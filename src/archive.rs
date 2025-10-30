use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use tar::{Archive, Entry};

use crate::manifest;
use crate::tree::Node;
use crate::whiteout;

/// Extract abbreviated hash from layer name
/// Example: "abc123def456.../layer.tar" -> Some("abc123d")
fn extract_layer_hash(layer_name: &str, length: usize) -> Option<String> {
    // Layer names are typically like "abc123def.../layer.tar" or "abc123.tar.gz"
    // Extract the hash portion (directory name or filename without extension)
    let path = layer_name.trim_end_matches("/layer.tar")
                         .trim_end_matches(".tar.gz")
                         .trim_end_matches(".tar");

    // Get the last component (the hash)
    let hash = path.rsplit('/').next().unwrap_or(path);

    if hash.is_empty() {
        None
    } else {
        Some(hash.chars().take(length).collect())
    }
}

/// Process a Docker archive and build the merged filesystem tree
pub fn process_archive(archive_path: &Path, show_layers: bool) -> Result<Node> {
    let file = File::open(archive_path)
        .with_context(|| format!("Failed to open archive: {}", archive_path.display()))?;

    let mut archive = Archive::new(file);

    // First pass: extract manifest and layer files to temporary directory
    let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
    let mut layer_paths: HashMap<String, std::path::PathBuf> = HashMap::new();
    let mut manifest_bytes: Option<Vec<u8>> = None;

    for entry in archive.entries().context("Failed to read archive entries")? {
        let mut entry = entry.context("Failed to read entry")?;
        let path = entry.path().context("Failed to read entry path")?;
        let path_str = path.to_string_lossy();

        // Check if this is manifest.json
        if path_str == "manifest.json" {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).context("Failed to read manifest.json")?;
            manifest_bytes = Some(buf);
            continue;
        }

        // Check if this is a layer tar file
        if path_str.ends_with("/layer.tar") || path_str.ends_with(".tar") || path_str.ends_with(".tar.gz") {
            // Save layer to temp file, preserving the extension
            let layer_name = path_str.to_string();
            let extension = if path_str.ends_with(".tar.gz") {
                ".tar.gz"
            } else {
                ".tar"
            };
            let temp_path = temp_dir.path().join(format!("layer-{}{}", layer_paths.len(), extension));

            let mut temp_file = File::create(&temp_path)
                .context("Failed to create temp file")?;
            std::io::copy(&mut entry, &mut temp_file)
                .context("Failed to copy layer to temp file")?;

            layer_paths.insert(layer_name, temp_path);
        }
    }

    let manifest_bytes = manifest_bytes.context("manifest.json not found in archive")?;
    let layers = manifest::parse_manifest(&manifest_bytes)?;

    // Second pass: apply layers in manifest order
    let mut root = Node::new_dir(0o755, 0, 0);

    for layer_name in layers.iter() {
        let temp_path = layer_paths.get(layer_name)
            .with_context(|| format!("Layer {} not found in archive", layer_name))?;

        let layer_hash = if show_layers {
            // Extract hash from layer name (e.g., "abc123def.../layer.tar" -> "abc123d")
            extract_layer_hash(layer_name, 7)
        } else {
            None
        };

        apply_layer(&mut root, temp_path, layer_hash.as_deref())?;
    }

    Ok(root)
}

/// Apply a single layer tar to the filesystem tree
fn apply_layer(root: &mut Node, layer_path: &Path, layer_hash: Option<&str>) -> Result<()> {
    let file = File::open(layer_path)
        .with_context(|| format!("Failed to open layer: {}", layer_path.display()))?;

    // Check if layer is gzipped
    let is_gzipped = layer_path.to_string_lossy().ends_with(".gz");

    if is_gzipped {
        let decoder = GzDecoder::new(file);
        let mut archive = Archive::new(decoder);
        archive.set_ignore_zeros(true);
        archive.set_unpack_xattrs(false);
        process_layer_entries(root, &mut archive, layer_hash)?;
    } else {
        let mut archive = Archive::new(file);
        archive.set_ignore_zeros(true);
        archive.set_unpack_xattrs(false);
        process_layer_entries(root, &mut archive, layer_hash)?;
    }

    Ok(())
}

/// Process entries from a layer archive
fn process_layer_entries<R: Read>(
    root: &mut Node,
    archive: &mut Archive<R>,
    layer_hash: Option<&str>,
) -> Result<()> {
    for entry in archive.entries().context("Failed to read layer entries")? {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => {
                // Skip corrupted entries but continue processing
                eprintln!("Warning: Skipping corrupted entry: {}", err);
                continue;
            }
        };

        if let Err(err) = apply_entry(root, entry, layer_hash) {
            eprintln!("Warning: Failed to apply entry: {}", err);
            continue;
        }
    }

    Ok(())
}

/// Apply a single tar entry to the tree
fn apply_entry<R: Read>(
    root: &mut Node,
    entry: Entry<R>,
    layer_hash: Option<&str>,
) -> Result<()> {
    let header = entry.header();
    let path = entry.path().context("Failed to read entry path")?;
    let path_str = path.to_string_lossy();

    // Normalize path (strip leading ./ segments)
    let normalized_path = path_str.trim_start_matches("./").trim_end_matches('/');

    if normalized_path.is_empty() {
        return Ok(());
    }

    // Extract metadata from tar header
    let mode = header.mode().context("Failed to read mode")?;
    let uid = header.uid().context("Failed to read uid")?;
    let gid = header.gid().context("Failed to read gid")?;
    let entry_type = header.entry_type();

    // Handle whiteouts
    if whiteout::is_whiteout(normalized_path) {
        if whiteout::is_opaque(normalized_path) {
            let dir_path = whiteout::opaque_dir(normalized_path);
            root.mark_opaque(dir_path);
        } else {
            let target = whiteout::whiteout_target(normalized_path);
            root.remove(&target);
        }
        return Ok(());
    }

    // Apply regular entries
    match entry_type {
        tar::EntryType::Directory => {
            root.ensure_path(normalized_path, mode, uid, gid, layer_hash);
        }
        tar::EntryType::Regular => {
            root.put_file(normalized_path, mode, uid, gid, false, None, layer_hash);
        }
        tar::EntryType::Symlink => {
            let link_target = header.link_name()
                .context("Failed to read symlink target")?
                .map(|p| p.to_string_lossy().to_string());
            root.put_file(normalized_path, mode, uid, gid, true, link_target, layer_hash);
        }
        tar::EntryType::Link => {
            // Hard link support
            let link_target = header.link_name()
                .context("Failed to read hard link target")?
                .map(|p| p.to_string_lossy().to_string());

            // Create the file node first
            root.put_file(normalized_path, mode, uid, gid, false, None, layer_hash);

            // Then set the hard link target
            if let Some(target) = link_target {
                if let Err(e) = root.set_hardlink_target(normalized_path, target) {
                    // Log warning but don't fail - the file still exists
                    eprintln!("Warning: Failed to set hard link target: {}", e);
                }
            }
        }
        _ => {
            // Skip other entry types (char devices, block devices, fifos, etc.)
        }
    }

    Ok(())
}

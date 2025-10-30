use std::collections::HashMap;
use crate::utils;

/// Represents a node in the merged filesystem tree
#[derive(Debug, Clone)]
pub struct Node {
    /// Child entries keyed by basename
    pub children: HashMap<String, Node>,
    /// File metadata
    pub metadata: NodeMetadata,
}

#[derive(Debug, Clone)]
pub struct NodeMetadata {
    /// true for files/symlinks, false for directories
    pub is_file: bool,
    /// true if this is a symbolic link
    pub is_symlink: bool,
    /// Symlink target path
    pub symlink_target: Option<String>,
    /// Hard link target path (relative to archive root)
    pub hardlink_target: Option<String>,
    /// Unix file mode (permissions and type bits)
    pub mode: u32,
    /// User ID of the owner
    pub uid: u64,
    /// Group ID of the owner
    pub gid: u64,
    /// User name (if available)
    #[allow(dead_code)]
    pub uname: Option<String>,
    /// Group name (if available)
    #[allow(dead_code)]
    pub gname: Option<String>,
    /// Layer hash that added/modified this entry
    pub layer_hash: Option<String>,
}

impl Node {
    /// Create a new directory node
    pub fn new_dir(mode: u32, uid: u64, gid: u64) -> Self {
        Node {
            children: HashMap::new(),
            metadata: NodeMetadata {
                is_file: false,
                is_symlink: false,
                symlink_target: None,
                hardlink_target: None,
                mode,
                uid,
                gid,
                uname: None,
                gname: None,
                layer_hash: None,
            },
        }
    }

    /// Create a new file node
    pub fn new_file(mode: u32, uid: u64, gid: u64) -> Self {
        Node {
            children: HashMap::new(),
            metadata: NodeMetadata {
                is_file: true,
                is_symlink: false,
                symlink_target: None,
                hardlink_target: None,
                mode,
                uid,
                gid,
                uname: None,
                gname: None,
                layer_hash: None,
            },
        }
    }

    /// Ensure a directory path exists in the tree, creating intermediate dirs as needed
    pub fn ensure_path(&mut self, path: &str, mode: u32, uid: u64, gid: u64, layer_hash: Option<&str>) {
        if path.is_empty() || path == "." {
            return;
        }

        let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty() && *p != ".").collect();
        let mut current = self;

        for part in parts {
            current = current.children
                .entry(part.to_string())
                .or_insert_with(|| Node::new_dir(mode, uid, gid));
            current.metadata.layer_hash = layer_hash.map(|s| s.to_string());
        }
    }

    /// Add or update a file at the given path
    #[allow(clippy::too_many_arguments)]
    pub fn put_file(
        &mut self,
        path: &str,
        mode: u32,
        uid: u64,
        gid: u64,
        is_symlink: bool,
        link_target: Option<String>,
        layer_hash: Option<&str>,
    ) {
        let (dir_path, basename) = utils::split_path(path);

        // Ensure parent directory exists
        if !dir_path.is_empty() {
            self.ensure_path(dir_path, 0o755, 0, 0, layer_hash);
        }

        // Navigate to parent directory
        let mut parent = self;
        if !dir_path.is_empty() {
            for part in dir_path.split('/').filter(|p| !p.is_empty() && *p != ".") {
                // If parent doesn't exist, create it
                parent = parent.children
                    .entry(part.to_string())
                    .or_insert_with(|| Node::new_dir(0o755, 0, 0));
            }
        }

        // Create or update the file node
        let mut file_node = Node::new_file(mode, uid, gid);
        file_node.metadata.is_symlink = is_symlink;
        file_node.metadata.symlink_target = link_target;
        file_node.metadata.layer_hash = layer_hash.map(|s| s.to_string());

        parent.children.insert(basename.to_string(), file_node);
    }

    /// Set hard link target for a file node
    /// Returns Ok(()) if successful, Err if the path doesn't exist
    pub fn set_hardlink_target(&mut self, path: &str, target: String) -> anyhow::Result<()> {
        let (dir_path, basename) = utils::split_path(path);

        let mut parent = self;
        if !dir_path.is_empty() {
            for part in dir_path.split('/').filter(|p| !p.is_empty() && *p != ".") {
                parent = parent.children
                    .get_mut(part)
                    .ok_or_else(|| anyhow::anyhow!("Parent directory '{}' not found", part))?;
            }
        }

        let node = parent.children
            .get_mut(basename)
            .ok_or_else(|| anyhow::anyhow!("File '{}' not found", basename))?;

        node.metadata.hardlink_target = Some(target);
        Ok(())
    }

    /// Remove a node at the given path (for whiteouts)
    pub fn remove(&mut self, path: &str) {
        let (dir_path, basename) = utils::split_path(path);

        let mut parent = self;
        if !dir_path.is_empty() {
            for part in dir_path.split('/').filter(|p| !p.is_empty() && *p != ".") {
                if let Some(node) = parent.children.get_mut(part) {
                    parent = node;
                } else {
                    return; // Path doesn't exist, nothing to remove
                }
            }
        }

        parent.children.remove(basename);
    }

    /// Mark a directory as opaque by clearing all its children
    pub fn mark_opaque(&mut self, path: &str) {
        if path.is_empty() || path == "." {
            self.children.clear();
            return;
        }

        let mut current = self;
        for part in path.split('/').filter(|p| !p.is_empty() && *p != ".") {
            if let Some(node) = current.children.get_mut(part) {
                current = node;
            } else {
                return; // Path doesn't exist
            }
        }

        current.children.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ensure_path() {
        let mut root = Node::new_dir(0o755, 0, 0);
        root.ensure_path("foo/bar/baz", 0o755, 0, 0, None);

        assert!(root.children.contains_key("foo"));
        assert!(root.children["foo"].children.contains_key("bar"));
        assert!(root.children["foo"].children["bar"].children.contains_key("baz"));
    }

    #[test]
    fn test_put_file() {
        let mut root = Node::new_dir(0o755, 0, 0);
        root.put_file("foo/bar.txt", 0o644, 1000, 1000, false, None, None);

        assert!(root.children.contains_key("foo"));
        assert!(root.children["foo"].children.contains_key("bar.txt"));
        assert!(root.children["foo"].children["bar.txt"].metadata.is_file);
    }
}

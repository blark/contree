use anyhow::{Context, Result};
use serde::Deserialize;

/// Docker manifest.json structure
/// The manifest is an array of image descriptors
#[derive(Debug, Deserialize)]
pub struct ManifestEntry {
    #[serde(rename = "Config")]
    #[allow(dead_code)]
    pub config: Option<String>,

    #[serde(rename = "RepoTags")]
    #[allow(dead_code)]
    pub repo_tags: Option<Vec<String>>,

    #[serde(rename = "Layers")]
    pub layers: Vec<String>,
}

/// Parse the manifest.json to extract ordered layer paths
pub fn parse_manifest(manifest_bytes: &[u8]) -> Result<Vec<String>> {
    let manifest: Vec<ManifestEntry> = serde_json::from_slice(manifest_bytes)
        .context("Failed to parse manifest.json")?;

    // Get the first manifest entry (most archives have only one)
    let entry = manifest.into_iter().next()
        .ok_or_else(|| anyhow::anyhow!("Empty manifest"))?;

    Ok(entry.layers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_manifest() {
        let manifest_json = r#"[{
            "Config": "abc123.json",
            "RepoTags": ["alpine:latest"],
            "Layers": [
                "layer1/layer.tar",
                "layer2/layer.tar",
                "layer3/layer.tar"
            ]
        }]"#;

        let layers = parse_manifest(manifest_json.as_bytes()).unwrap();
        assert_eq!(layers.len(), 3);
        assert_eq!(layers[0], "layer1/layer.tar");
        assert_eq!(layers[1], "layer2/layer.tar");
        assert_eq!(layers[2], "layer3/layer.tar");
    }
}

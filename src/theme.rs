use anyhow::{Context, Result};
use serde::{Deserialize, Deserializer};

/// Color theme configuration
#[derive(Debug, Clone, Deserialize)]
pub struct Theme {
    #[serde(default = "default_directory", deserialize_with = "deserialize_color")]
    pub directory: String,

    #[serde(default = "default_executable", deserialize_with = "deserialize_color")]
    pub executable: String,

    #[serde(default = "default_symlink", deserialize_with = "deserialize_color")]
    pub symlink: String,

    #[serde(default = "default_tree_chars", deserialize_with = "deserialize_color")]
    pub tree_chars: String,

    #[serde(default = "default_permissions", deserialize_with = "deserialize_color")]
    pub permissions: String,

    #[serde(default = "default_ownership", deserialize_with = "deserialize_color")]
    pub ownership: String,

    #[serde(default = "default_layer_separator", deserialize_with = "deserialize_color")]
    pub layer_separator: String,

    #[serde(default = "default_hardlink", deserialize_with = "deserialize_color")]
    pub hardlink: String,
}

/// Deserialize a color from either hex string (#RRGGBB) or RGB array [r, g, b]
fn deserialize_color<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ColorValue {
        Hex(String),
        Rgb([u8; 3]),
    }

    let value = ColorValue::deserialize(deserializer)?;

    match value {
        ColorValue::Hex(hex) => {
            // Parse hex color like "#7daea3" or "7daea3"
            let hex = hex.trim_start_matches('#');
            if hex.len() != 6 {
                return Err(D::Error::custom(format!("Invalid hex color: {}", hex)));
            }

            let r = u8::from_str_radix(&hex[0..2], 16)
                .map_err(|_| D::Error::custom(format!("Invalid hex color: {}", hex)))?;
            let g = u8::from_str_radix(&hex[2..4], 16)
                .map_err(|_| D::Error::custom(format!("Invalid hex color: {}", hex)))?;
            let b = u8::from_str_radix(&hex[4..6], 16)
                .map_err(|_| D::Error::custom(format!("Invalid hex color: {}", hex)))?;

            Ok(rgb_to_ansi(r, g, b))
        }
        ColorValue::Rgb([r, g, b]) => {
            Ok(rgb_to_ansi(r, g, b))
        }
    }
}

/// Convert RGB values to ANSI escape code
fn rgb_to_ansi(r: u8, g: u8, b: u8) -> String {
    format!("\x1b[38;2;{};{};{}m", r, g, b)
}

// Default Gruvbox Material Dark theme colors
fn default_directory() -> String {
    "\x1b[38;2;125;174;163m".to_string() // #7daea3
}

fn default_executable() -> String {
    "\x1b[38;2;169;182;101m".to_string() // #a9b665
}

fn default_symlink() -> String {
    "\x1b[38;2;137;180;130m".to_string() // #89b482
}

fn default_tree_chars() -> String {
    "\x1b[38;2;146;131;116m".to_string() // #928374
}

fn default_permissions() -> String {
    "\x1b[38;2;221;199;161m".to_string() // #ddc7a1
}

fn default_ownership() -> String {
    "\x1b[38;2;216;166;87m".to_string() // #d8a657
}

fn default_layer_separator() -> String {
    "\x1b[38;2;211;134;155m".to_string() // #d3869b
}

fn default_hardlink() -> String {
    "\x1b[38;2;146;131;116m".to_string() // #928374
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            directory: default_directory(),
            executable: default_executable(),
            symlink: default_symlink(),
            tree_chars: default_tree_chars(),
            permissions: default_permissions(),
            ownership: default_ownership(),
            layer_separator: default_layer_separator(),
            hardlink: default_hardlink(),
        }
    }
}

impl Theme {
    /// Parse a theme from a JSON string
    pub fn from_json(json: &str) -> Result<Self> {
        let theme: Theme = serde_json::from_str(json)
            .context("Failed to parse theme JSON")?;

        Ok(theme)
    }

    /// Get the default Gruvbox Material Dark theme
    #[allow(dead_code)]
    pub fn gruvbox_dark() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_theme() {
        let theme = Theme::default();
        assert!(theme.directory.contains("125;174;163"));
        assert!(theme.executable.contains("169;182;101"));
    }

    #[test]
    fn test_parse_hex_color() {
        let json = r##"{"directory": "#ff0000"}"##;
        let theme: Theme = serde_json::from_str(json).unwrap();
        assert_eq!(theme.directory, "\x1b[38;2;255;0;0m");
        // Other fields should have defaults
        assert!(theme.executable.contains("169;182;101"));
    }

    #[test]
    fn test_parse_rgb_array() {
        let json = r#"{"executable": [255, 128, 64]}"#;
        let theme: Theme = serde_json::from_str(json).unwrap();
        assert_eq!(theme.executable, "\x1b[38;2;255;128;64m");
        // Other fields should have defaults
        assert!(theme.directory.contains("125;174;163"));
    }

    #[test]
    fn test_parse_hex_without_hash() {
        let json = r#"{"symlink": "89b482"}"#;
        let theme: Theme = serde_json::from_str(json).unwrap();
        assert_eq!(theme.symlink, "\x1b[38;2;137;180;130m");
    }
}

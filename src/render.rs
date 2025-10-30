use crate::theme::Theme;
use crate::tree::Node;
use std::io::{self, Write};

const COLOR_RESET: &str = "\x1b[0m";

pub struct RenderOptions {
    pub show_long: bool,
    pub show_layers: bool,
    pub use_color: bool,
    pub icon_style: IconStyle,
    pub theme: Theme,
}

#[derive(Clone, Copy)]
pub enum IconStyle {
    None,
    Emoji,
    Nerd,
}

impl IconStyle {
    pub fn from_str(s: &str) -> Self {
        match s {
            "emoji" => IconStyle::Emoji,
            "nerd" => IconStyle::Nerd,
            _ => IconStyle::None,
        }
    }

    fn file_icon(&self) -> &'static str {
        match self {
            IconStyle::None => "",
            IconStyle::Emoji => "📄 ",
            IconStyle::Nerd => "\u{f15b} ", // nf-fa-file_o
        }
    }

    fn dir_icon(&self) -> &'static str {
        match self {
            IconStyle::None => "",
            IconStyle::Emoji => "📁 ",
            IconStyle::Nerd => "\u{f115} ", // nf-fa-folder
        }
    }
}

pub fn render_tree(root: &Node, options: &RenderOptions) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    // Calculate max ownership width if showing long format
    let max_ownership_width = if options.show_long {
        calculate_max_ownership_width(root)
    } else {
        0
    };

    render_node(&mut handle, root, "", options, None, max_ownership_width)?;
    handle.flush()
}

/// Calculate the maximum width needed for the ownership column
fn calculate_max_ownership_width(node: &Node) -> usize {
    let mut max_width = 0;

    for child in node.children.values() {
        let owner_str = format!("{}:{}", child.metadata.uid, child.metadata.gid);
        max_width = max_width.max(owner_str.len());

        // Recursively check children
        if !child.metadata.is_file {
            let child_max = calculate_max_ownership_width(child);
            max_width = max_width.max(child_max);
        }
    }

    max_width
}

fn render_node<W: Write>(
    writer: &mut W,
    node: &Node,
    prefix: &str,
    options: &RenderOptions,
    prev_layer: Option<&str>,
    max_ownership_width: usize,
) -> io::Result<Option<String>> {
    // Collect and sort children
    let mut children: Vec<_> = node.children.iter().collect();
    children.sort_by_key(|(name, _)| *name);

    let mut last_layer = prev_layer.map(|s| s.to_string());

    for (idx, (name, child)) in children.iter().enumerate() {
        let is_last = idx + 1 == children.len();

        // Check if we need to print a layer separator
        if options.show_layers {
            let current_layer = child.metadata.layer_hash.as_deref();
            if current_layer != last_layer.as_deref() {
                if let Some(layer) = current_layer {
                    write_layer_separator(writer, layer, options)?;
                    last_layer = Some(layer.to_string());
                }
            }
        }

        // Show permissions and ownership first if requested
        if options.show_long {
            let perms = format_permissions(child.metadata.mode, child.metadata.is_file);
            // Right-align ownership using the calculated max width
            let owner_str = format!("{}:{}", child.metadata.uid, child.metadata.gid);
            let owner = format!("{:>width$}", owner_str, width = max_ownership_width);

            if options.use_color {
                write!(writer, "{}{}{} {}{}{} ",
                    options.theme.permissions, perms, COLOR_RESET,
                    options.theme.ownership, owner, COLOR_RESET)?;
            } else {
                write!(writer, "{} {} ", perms, owner)?;
            }
        }

        // Draw tree structure
        let branch = if is_last { "└── " } else { "├── " };

        if options.use_color {
            write!(writer, "{}{}{}{}{}",
                options.theme.tree_chars, prefix, branch, COLOR_RESET, "")?;
        } else {
            write!(writer, "{}{}", prefix, branch)?;
        }

        // Determine color based on file type
        let color = if options.use_color {
            if child.metadata.is_symlink {
                &options.theme.symlink
            } else if !child.metadata.is_file {
                &options.theme.directory
            } else if child.metadata.mode & 0o111 != 0 {
                &options.theme.executable
            } else {
                ""
            }
        } else {
            ""
        };

        // Draw icon with same color as filename
        let icon = if child.metadata.is_file {
            options.icon_style.file_icon()
        } else {
            options.icon_style.dir_icon()
        };

        if !color.is_empty() {
            write!(writer, "{}{}", color, icon)?;
        } else {
            write!(writer, "{}", icon)?;
        }

        // Print filename with same color
        if !color.is_empty() {
            write!(writer, "{}{}", name, COLOR_RESET)?;
        } else {
            write!(writer, "{}", name)?;
        }

        // Show symlink target
        if child.metadata.is_symlink {
            if let Some(ref target) = child.metadata.symlink_target {
                if options.use_color {
                    write!(writer, " -> {}{}{}", options.theme.symlink, target, COLOR_RESET)?;
                } else {
                    write!(writer, " -> {}", target)?;
                }
            }
        }

        // Show hard link target
        if let Some(ref target) = child.metadata.hardlink_target {
            if options.use_color {
                write!(writer, " => {}{}{}", options.theme.hardlink, target, COLOR_RESET)?;
            } else {
                write!(writer, " => {}", target)?;
            }
        }

        writeln!(writer)?;

        // Recurse into directories
        if !child.metadata.is_file && !child.children.is_empty() {
            let new_prefix = if is_last {
                format!("{}    ", prefix)
            } else if options.use_color {
                format!("{}{}│{}   ", prefix, options.theme.tree_chars, COLOR_RESET)
            } else {
                format!("{}│   ", prefix)
            };

            last_layer = render_node(writer, child, &new_prefix, options, last_layer.as_deref(), max_ownership_width)?
                .or(last_layer);
        }
    }

    Ok(last_layer)
}

fn write_layer_separator<W: Write>(
    writer: &mut W,
    layer_hash: &str,
    options: &RenderOptions,
) -> io::Result<()> {
    let label = format!(" Layer {} ", layer_hash);
    let total_width: usize = 60;
    let padding = total_width.saturating_sub(label.len()) / 2;
    let right_padding = total_width.saturating_sub(label.len() + padding);

    writeln!(writer)?;

    if options.use_color {
        write!(writer, "{}", options.theme.layer_separator)?;
        write!(writer, "{}", "─".repeat(padding))?;
        write!(writer, "{}", label)?;
        write!(writer, "{}", "─".repeat(right_padding))?;
        writeln!(writer, "{}", COLOR_RESET)?;
    } else {
        write!(writer, "{}", "─".repeat(padding))?;
        write!(writer, "{}", label)?;
        writeln!(writer, "{}", "─".repeat(right_padding))?;
    }

    Ok(())
}

fn format_permissions(mode: u32, is_file: bool) -> String {
    let file_type = if is_file { '-' } else { 'd' };

    format!(
        "{}{}{}{}{}{}{}{}{}{}",
        file_type,
        if mode & 0o400 != 0 { 'r' } else { '-' },
        if mode & 0o200 != 0 { 'w' } else { '-' },
        if mode & 0o100 != 0 { 'x' } else { '-' },
        if mode & 0o040 != 0 { 'r' } else { '-' },
        if mode & 0o020 != 0 { 'w' } else { '-' },
        if mode & 0o010 != 0 { 'x' } else { '-' },
        if mode & 0o004 != 0 { 'r' } else { '-' },
        if mode & 0o002 != 0 { 'w' } else { '-' },
        if mode & 0o001 != 0 { 'x' } else { '-' },
    )
}

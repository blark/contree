use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

mod archive;
mod manifest;
mod render;
mod theme;
mod tree;
mod whiteout;

#[derive(Parser)]
#[command(name = "contree")]
#[command(about = "Visualize the merged filesystem tree of Docker image archives")]
#[command(version = "0.1.0")]
struct Cli {
    /// Docker archive tar file to visualize
    archive: PathBuf,

    /// Show permissions and ownership information
    #[arg(short, long)]
    long: bool,

    /// When to colorize output: auto, always, never
    #[arg(long, default_value = "auto")]
    color: String,

    /// Icon style: none, emoji, nerd
    #[arg(long, default_value = "nerd")]
    icons: String,

    /// Show layer separators with abbreviated hash
    #[arg(long)]
    layers: bool,

    /// Custom theme as JSON string (e.g., '{"directory":"#7daea3"}')
    #[arg(long)]
    theme: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Determine if we should use color
    let use_color = match cli.color.as_str() {
        "always" => true,
        "never" => false,
        _ => atty::is(atty::Stream::Stdout),
    };

    // Load theme
    let theme = if let Some(theme_json) = cli.theme {
        theme::Theme::from_json(&theme_json)?
    } else {
        theme::Theme::default()
    };

    // Process the Docker archive
    let root = archive::process_archive(&cli.archive, cli.layers)?;

    // Render the tree
    let options = render::RenderOptions {
        show_long: cli.long,
        show_layers: cli.layers,
        use_color,
        icon_style: render::IconStyle::from_str(&cli.icons),
        theme,
    };

    render::render_tree(&root, &options)?;

    Ok(())
}

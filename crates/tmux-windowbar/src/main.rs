mod commands;
mod config;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "tmux-windowbar",
    about = "Clickable window list with [+][x] for tmux status bar"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initial setup: set window-status-format and bindings
    Init,
    /// Re-apply window bar settings
    Apply,
    /// Handle mouse click (called by tmux internally)
    Click {
        /// The mouse_status_range value
        range: String,
    },
    /// Handle mouse double-click (called by tmux internally)
    Dblclick {
        /// The mouse_status_range value
        range: String,
    },
    /// Render window list (called by tmux-sessionbar internally)
    Render,
    /// Output view switcher string (called by tmux-sessionbar)
    RenderView,
    /// Save current window/pane layout
    LayoutSave {
        /// Layout name
        name: String,
    },
    /// Restore a saved layout
    LayoutLoad {
        /// Layout name
        name: String,
    },
    /// List saved layouts
    LayoutList,
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init => commands::init::run(),
        Commands::Apply => commands::apply::run(),
        Commands::Click { range } => commands::click::run(&range),
        Commands::Dblclick { range } => commands::click::run_dblclick(&range),
        Commands::Render => commands::render::run(),
        Commands::RenderView => {
            print!("{}", commands::render::render_view_switcher());
            Ok(())
        }
        Commands::LayoutSave { name } => commands::layout::save(&name),
        Commands::LayoutLoad { name } => commands::layout::load(&name),
        Commands::LayoutList => commands::layout::list(),
    };

    if let Err(e) = result {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

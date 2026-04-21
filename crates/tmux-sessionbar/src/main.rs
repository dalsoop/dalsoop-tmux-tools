mod ai_status;
mod commands;
mod config;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "tmux-sessionbar",
    about = "Clickable session list for tmux status bar"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initial setup: generate config and .tmux.conf
    Init,
    /// Regenerate .tmux.conf from config.toml and reload
    Apply,
    /// Show current tmux status and diagnostics
    Status,
    /// Render status bar segment (called by tmux internally)
    RenderStatus {
        /// Segment to render: "left" or "right"
        segment: String,
    },
    /// Handle mouse click on status bar (called by tmux internally)
    Click {
        /// The mouse_status_range value
        range: String,
    },
    /// Sync tmux-tools config to all system accounts
    Sync,
    /// Add a tmux plugin (e.g. tmux-plugins/tmux-yank)
    PluginAdd {
        /// Plugin name (e.g. tmux-plugins/tmux-yank)
        name: String,
    },
    /// Remove a tmux plugin
    PluginRm {
        /// Plugin name
        name: String,
    },
    /// List installed plugins
    PluginList,
    /// Install all enabled plugins via TPM
    PluginInstall,
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init => commands::init::run(),
        Commands::Apply => commands::apply::run(),
        Commands::Status => commands::status::run(),
        Commands::RenderStatus { segment } => commands::render::run(&segment),
        Commands::Click { range } => commands::click::run(&range),
        Commands::Sync => commands::sync::run(),
        Commands::PluginAdd { name } => commands::plugin::add(&name),
        Commands::PluginRm { name } => commands::plugin::remove(&name),
        Commands::PluginList => commands::plugin::list(),
        Commands::PluginInstall => commands::plugin::install(),
    };

    if let Err(e) = result {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

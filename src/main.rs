mod commands;
mod config;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "tmux-sessionbar", about = "Clickable session list for tmux status bar")]
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
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init => commands::init::run(),
        Commands::Apply => commands::apply::run(),
        Commands::Status => commands::status::run(),
        Commands::RenderStatus { segment } => commands::render::run(&segment),
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

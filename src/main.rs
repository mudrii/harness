mod cli;
mod config;

use clap::Parser;

fn main() {
    let cli = cli::Cli::parse();
    println!("Harness CLI v{}", env!("CARGO_PKG_VERSION"));
    match cli.command {
        cli::Commands::Analyze { path, format: _ } => {
            println!("analyze requested for {}", path.display());
        }
        cli::Commands::Suggest { path, .. } => {
            println!("suggest requested for {}", path.display());
        }
        cli::Commands::Init { path, profile, .. } => {
            println!("init requested for {} with profile {:?}", path.display(), profile);
        }
        cli::Commands::Apply { path, .. } => {
            println!("apply requested for {}", path.display());
        }
        cli::Commands::Optimize { path, .. } => {
            println!("optimize requested for {}", path.display());
        }
        cli::Commands::Bench { path, .. } => {
            println!("bench requested for {}", path.display());
        }
        cli::Commands::Lint { path, .. } => {
            println!("lint requested for {}", path.display());
        }
    }
}


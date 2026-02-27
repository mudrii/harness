mod cli;
mod config;

use clap::Parser;

fn main() {
    let cli = cli::Cli::parse();
    println!("Harness CLI v{}", env!("CARGO_PKG_VERSION"));
    match cli.command {
        cli::Commands::Analyze(cmd) => {
            println!("analyze requested for {}", cmd.path.display());
        }
        cli::Commands::Suggest(cmd) => {
            println!("suggest requested for {}", cmd.path.display());
        }
        cli::Commands::Init(cmd) => {
            println!(
                "init requested for {} with profile {:?}",
                cmd.path.display(),
                cmd.profile
            );
        }
        cli::Commands::Apply(cmd) => {
            println!("apply requested for {}", cmd.path.display());
        }
        cli::Commands::Optimize(cmd) => {
            println!("optimize requested for {}", cmd.path.display());
        }
        cli::Commands::Bench(cmd) => {
            println!("bench requested for {}", cmd.path.display());
        }
        cli::Commands::Lint(cmd) => {
            println!("lint requested for {}", cmd.path.display());
        }
    }
}

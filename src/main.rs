mod cli;
mod config;
mod error;
mod types;
mod scan;
mod analyze;
mod report;
mod guardrails;
// Deferred modules (uncomment when implementing):
// mod optimization;
// mod generator;
// mod trace;

use clap::Parser;
use crate::error::HarnessError;

pub mod exit_code {
    pub const SUCCESS: i32 = 0;
    pub const WARNINGS: i32 = 1;
    pub const BLOCKING: i32 = 2;
    pub const RUNTIME_FAILURE: i32 = 3;
}

fn run() -> Result<i32, HarnessError> {
    let cli = cli::Cli::parse();
    println!("Harness CLI v{}", env!("CARGO_PKG_VERSION"));
    match cli.command {
        cli::Commands::Analyze(cmd) => {
            println!(
                "analyze requested for {} (min_impact={:?})",
                cmd.path.display(),
                cmd.min_impact
            );
            Ok(exit_code::WARNINGS) // Output 1 to simulate warnings found
        }
        cli::Commands::Suggest(cmd) => {
            println!("suggest requested for {}", cmd.path.display());
            Ok(exit_code::SUCCESS)
        }
        cli::Commands::Init(cmd) => {
            println!(
                "init requested for {} with profile {:?}",
                cmd.path.display(),
                cmd.profile
            );
            Ok(exit_code::SUCCESS)
        }
        cli::Commands::Apply(cmd) => {
            println!(
                "apply requested for {} (mode={:?}, plan_all={})",
                cmd.path.display(),
                cmd.apply_mode,
                cmd.plan_all
            );
            Ok(exit_code::SUCCESS)
        }
        cli::Commands::Optimize(cmd) => {
            println!("optimize requested for {}", cmd.path.display());
            Ok(exit_code::SUCCESS)
        }
        cli::Commands::Bench(cmd) => {
            println!("bench requested for {}", cmd.path.display());
            Ok(exit_code::SUCCESS)
        }
        cli::Commands::Lint(cmd) => {
            println!("lint requested for {}", cmd.path.display());
            Ok(exit_code::BLOCKING) // Output 2 to simulate blocking lint rule
        }
    }
}

fn main() {
    match run() {
        Ok(code) => {
            if code != 0 {
                std::process::exit(code);
            }
        }
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(exit_code::RUNTIME_FAILURE);
        }
    }
}

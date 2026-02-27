mod analyze;
mod cli;
mod config;
mod error;
mod generator;
mod guardrails;
mod report;
mod scan;
mod types;
// Deferred modules (uncomment when implementing):
// mod optimization;
// mod trace;

use crate::error::HarnessError;
use clap::Parser;

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
            if !cmd.path.exists() {
                return Err(HarnessError::PathNotFound(cmd.path.display().to_string()));
            }
            if !cmd.path.join(".git").exists() {
                return Err(HarnessError::NotGitRepo(cmd.path.display().to_string()));
            }

            let loaded = config::load_config(&cmd.path)?;
            let model = scan::discover(&cmd.path, loaded.as_ref());
            let mut harness_report = analyze::analyze(&model, loaded.as_ref());

            if matches!(cmd.min_impact, cli::MinImpact::Safe) {
                harness_report.recommendations.retain(|recommendation| {
                    matches!(recommendation.risk, types::report::Risk::Safe)
                });
            }

            let output_format = match cmd.format {
                cli::ReportFormat::Json => report::OutputFormat::Json,
                cli::ReportFormat::Md => report::OutputFormat::Md,
                cli::ReportFormat::Sarif => report::OutputFormat::Sarif,
            };
            let rendered = report::render(&harness_report, output_format)?;
            println!("{rendered}");

            let has_blocking = harness_report
                .findings
                .iter()
                .any(|finding| finding.blocking);
            let has_warnings = !harness_report.findings.is_empty();
            let missing_config = loaded.is_none();

            if missing_config {
                eprintln!("warning: no harness.toml found in {}", cmd.path.display());
            }

            if has_blocking {
                Ok(exit_code::BLOCKING)
            } else if missing_config || has_warnings {
                Ok(exit_code::WARNINGS)
            } else {
                Ok(exit_code::SUCCESS)
            }
        }
        cli::Commands::Suggest(cmd) => {
            if !cmd.path.exists() {
                return Err(HarnessError::PathNotFound(cmd.path.display().to_string()));
            }
            if !cmd.path.join(".git").exists() {
                return Err(HarnessError::NotGitRepo(cmd.path.display().to_string()));
            }

            let loaded = config::load_config(&cmd.path)?;
            let model = scan::discover(&cmd.path, loaded.as_ref());
            let report = analyze::analyze(&model, loaded.as_ref());

            if report.recommendations.is_empty() {
                println!("suggest: no recommendations");
                return Ok(exit_code::SUCCESS);
            }

            println!("suggestions:");
            for recommendation in &report.recommendations {
                println!(
                    "- {} [{} {:?}/{:?}]",
                    recommendation.id,
                    recommendation.title,
                    recommendation.impact,
                    recommendation.risk
                );
            }

            if cmd.export_diff {
                let ids = report
                    .recommendations
                    .iter()
                    .filter(|recommendation| {
                        matches!(recommendation.risk, types::report::Risk::Safe)
                    })
                    .map(|recommendation| recommendation.id.clone())
                    .collect::<Vec<_>>();
                let plan = generator::manifest::SuggestPlan::new(ids);
                let path = generator::manifest::write_plan(&cmd.path, &plan)?;
                println!("plan file: {}", path.display());
            }

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
            if !cmd.path.exists() {
                return Err(HarnessError::PathNotFound(cmd.path.display().to_string()));
            }
            if !cmd.path.join(".git").exists() {
                return Err(HarnessError::NotGitRepo(cmd.path.display().to_string()));
            }
            generator::writer::execute_apply(&cmd)?;
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
            if !cmd.path.exists() {
                return Err(HarnessError::PathNotFound(cmd.path.display().to_string()));
            }
            if !cmd.path.join(".git").exists() {
                return Err(HarnessError::NotGitRepo(cmd.path.display().to_string()));
            }

            let loaded = config::load_config(&cmd.path)?;
            let model = scan::discover(&cmd.path, loaded.as_ref());
            let findings = analyze::lint::lint_findings(&model, loaded.as_ref());

            if findings.is_empty() {
                println!("lint: no findings");
                return Ok(exit_code::SUCCESS);
            }

            for finding in &findings {
                let level = if finding.blocking { "BLOCKING" } else { "WARN" };
                println!("[{}] {}: {}", level, finding.id, finding.title);
                println!("  {}", finding.body);
            }

            if findings.iter().any(|finding| finding.blocking) {
                Ok(exit_code::BLOCKING)
            } else {
                Ok(exit_code::WARNINGS)
            }
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

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
use serde::Serialize;

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
            if !cmd.path.exists() {
                if cmd.dry_run {
                    println!("init target would be created: {}", cmd.path.display());
                } else {
                    std::fs::create_dir_all(&cmd.path).map_err(HarnessError::Io)?;
                }
            }

            let profile = match cmd.profile {
                cli::Profile::General => "general",
                cli::Profile::Agent => "agent",
            };

            let files = vec![
                (
                    cmd.path.join("harness.toml"),
                    init_harness_toml(profile).to_string(),
                ),
                (cmd.path.join("AGENTS.md"), init_agents_md().to_string()),
                (
                    cmd.path.join("docs/context/INDEX.md"),
                    init_context_index().to_string(),
                ),
            ];

            println!("init plan:");
            for (path, _) in &files {
                println!("- {}", path.display());
            }

            if cmd.dry_run {
                println!("dry-run: no files were written");
                return Ok(exit_code::SUCCESS);
            }

            for (path, content) in files {
                if path.exists() && cmd.no_overwrite {
                    println!("skip existing: {}", path.display());
                    continue;
                }
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(HarnessError::Io)?;
                }
                std::fs::write(&path, content).map_err(HarnessError::Io)?;
            }
            println!("init complete");
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
            if !cmd.path.exists() {
                return Err(HarnessError::PathNotFound(cmd.path.display().to_string()));
            }
            if !cmd.path.join(".git").exists() {
                return Err(HarnessError::NotGitRepo(cmd.path.display().to_string()));
            }

            let loaded = config::load_config(&cmd.path)?;
            let model = scan::discover(&cmd.path, loaded.as_ref());
            let report = analyze::analyze(&model, loaded.as_ref());

            let out_dir = cmd
                .trace_dir
                .clone()
                .unwrap_or_else(|| cmd.path.join(".harness/optimize"));
            std::fs::create_dir_all(&out_dir).map_err(HarnessError::Io)?;
            let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
            let out_path = out_dir.join(format!("optimize-{stamp}.md"));
            let content = render_optimize_report(&report);
            std::fs::write(&out_path, content).map_err(HarnessError::Io)?;
            println!("optimize report: {}", out_path.display());
            Ok(exit_code::SUCCESS)
        }
        cli::Commands::Bench(cmd) => {
            if !cmd.path.exists() {
                return Err(HarnessError::PathNotFound(cmd.path.display().to_string()));
            }
            if !cmd.path.join(".git").exists() {
                return Err(HarnessError::NotGitRepo(cmd.path.display().to_string()));
            }

            let loaded = config::load_config(&cmd.path)?;
            let model = scan::discover(&cmd.path, loaded.as_ref());
            let mut run_results = Vec::new();
            for run_index in 0..cmd.runs {
                let report = analyze::analyze(&model, loaded.as_ref());
                run_results.push(BenchRunResult {
                    run: run_index + 1,
                    overall_score: report.overall_score,
                });
            }

            let context = BenchContext {
                os: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
                toolchain: detect_toolchain(),
                repo_ref: detect_repo_ref(&cmd.path),
                repo_dirty: detect_repo_dirty(&cmd.path),
                harness_version: env!("CARGO_PKG_VERSION").to_string(),
                suite: cmd.suite.clone().unwrap_or_else(|| "default".to_string()),
                timestamp: chrono::Utc::now().to_rfc3339(),
            };

            let report = BenchReport {
                bench_context: context,
                runs: run_results,
            };
            let report_path = write_bench_report(&cmd.path, &report)?;
            println!("bench report: {}", report_path.display());
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

fn init_harness_toml(profile: &str) -> &'static str {
    match profile {
        "agent" => {
            r#"[project]
name = "harness-project"
profile = "agent"

[tools.baseline]
commands = ["rg", "fd", "git"]
overlap_clusters = [["rg", "grep"], ["fd", "find"]]
destructive = ["git push --force", "rm -rf"]
forbidden = ["git push --force", "git reset --hard", "rm -rf"]

[verification]
required = ["cargo fmt --check", "cargo test"]
pre_completion_required = true
loop_guard_enabled = true
"#
        }
        _ => {
            r#"[project]
name = "harness-project"
profile = "general"

[tools.baseline]
commands = ["rg", "fd", "git"]
overlap_clusters = [["rg", "grep"], ["fd", "find"]]
destructive = ["git push --force", "rm -rf"]
forbidden = ["git push --force", "git reset --hard", "rm -rf"]

[verification]
required = ["cargo fmt --check", "cargo test"]
pre_completion_required = true
loop_guard_enabled = true
"#
        }
    }
}

fn init_agents_md() -> &'static str {
    r#"# Generated by harness
# Agents

- Context index: docs/context/INDEX.md
"#
}

fn init_context_index() -> &'static str {
    r#"# Generated by harness
# Context Index

- AGENTS.md
- harness.toml
"#
}

#[derive(Debug, Serialize)]
struct BenchContext {
    os: String,
    toolchain: String,
    repo_ref: String,
    repo_dirty: bool,
    harness_version: String,
    suite: String,
    timestamp: String,
}

#[derive(Debug, Serialize)]
struct BenchRunResult {
    run: u32,
    overall_score: f32,
}

#[derive(Debug, Serialize)]
struct BenchReport {
    bench_context: BenchContext,
    runs: Vec<BenchRunResult>,
}

fn detect_toolchain() -> String {
    let output = std::process::Command::new("rustc")
        .arg("--version")
        .output();
    match output {
        Ok(result) if result.status.success() => {
            String::from_utf8_lossy(&result.stdout).trim().to_string()
        }
        _ => "unknown".to_string(),
    }
}

fn detect_repo_ref(root: &std::path::Path) -> String {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output();
    match output {
        Ok(result) if result.status.success() => {
            String::from_utf8_lossy(&result.stdout).trim().to_string()
        }
        _ => "unknown".to_string(),
    }
}

fn detect_repo_dirty(root: &std::path::Path) -> bool {
    let output = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(root)
        .output();
    match output {
        Ok(result) if result.status.success() => {
            !String::from_utf8_lossy(&result.stdout).trim().is_empty()
        }
        _ => true,
    }
}

fn write_bench_report(
    root: &std::path::Path,
    report: &BenchReport,
) -> Result<std::path::PathBuf, HarnessError> {
    let dir = root.join(".harness/bench");
    std::fs::create_dir_all(&dir).map_err(HarnessError::Io)?;
    let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
    let out = dir.join(format!("bench-{stamp}.json"));
    let payload = serde_json::to_string_pretty(report)?;
    std::fs::write(&out, payload).map_err(HarnessError::Io)?;
    Ok(out)
}

fn render_optimize_report(report: &types::report::HarnessReport) -> String {
    let mut ordered_report = report.clone();
    ordered_report.sort_recommendations();

    let mut lines = vec![
        "# Harness Optimize Report".to_string(),
        String::new(),
        format!("Overall score: {:.3}", ordered_report.overall_score),
        String::new(),
        "## Top Recommendations".to_string(),
    ];

    if ordered_report.recommendations.is_empty() {
        lines.push("- No recommendations available.".to_string());
    } else {
        for recommendation in ordered_report.recommendations.iter().take(10) {
            lines.push(format!(
                "- `{}`: {} (impact: {:?}, effort: {:?}, risk: {:?}, confidence: {:.2})",
                recommendation.id,
                recommendation.summary,
                recommendation.impact,
                recommendation.effort,
                recommendation.risk,
                recommendation.confidence
            ));
        }
    }
    lines.push(String::new());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::report::{Effort, HarnessReport, Impact, Recommendation, Risk};
    use crate::types::scoring::ScoreCard;

    #[test]
    fn render_optimize_report_orders_recommendations_by_priority() {
        let report = HarnessReport {
            overall_score: 0.5,
            category_scores: ScoreCard::new(0.5, 0.5, 0.5, 0.5, 0.5),
            findings: vec![],
            recommendations: vec![
                Recommendation::new(
                    "low",
                    "Low",
                    "low summary",
                    Impact::Low,
                    Effort::Xs,
                    Risk::Safe,
                    0.6,
                ),
                Recommendation::new(
                    "high",
                    "High",
                    "high summary",
                    Impact::High,
                    Effort::S,
                    Risk::Safe,
                    0.9,
                ),
            ],
        };

        let rendered = render_optimize_report(&report);
        let high_pos = rendered
            .find("`high`")
            .expect("high recommendation should render");
        let low_pos = rendered
            .find("`low`")
            .expect("low recommendation should render");
        assert!(
            high_pos < low_pos,
            "high-priority recommendation should appear first"
        );
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

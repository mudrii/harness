mod analyze;
mod cli;
mod config;
mod continuity;
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
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

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
            let mut continuity_logger = continuity::ContinuityLogger::new(&cmd.path, loaded.as_ref());
            continuity_milestone(
                &mut continuity_logger,
                "analyze",
                "start",
                &[format!("path={}", cmd.path.display())],
                "running",
            );
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
            continuity_progress(
                &mut continuity_logger,
                "analyze",
                "report_rendered",
                &[
                    format!("findings={}", harness_report.findings.len()),
                    format!("recommendations={}", harness_report.recommendations.len()),
                ],
                "running",
            );

            let has_blocking = harness_report
                .findings
                .iter()
                .any(|finding| finding.blocking);
            let has_warnings = !harness_report.findings.is_empty();
            let missing_config = loaded.is_none();

            if missing_config {
                eprintln!("warning: no harness.toml found in {}", cmd.path.display());
            }

            let exit = if has_blocking {
                exit_code::BLOCKING
            } else if missing_config || has_warnings {
                exit_code::WARNINGS
            } else {
                exit_code::SUCCESS
            };
            continuity_milestone(
                &mut continuity_logger,
                "analyze",
                "complete",
                &[format!("exit_code={exit}")],
                "done",
            );
            Ok(exit)
        }
        cli::Commands::Suggest(cmd) => {
            if !cmd.path.exists() {
                return Err(HarnessError::PathNotFound(cmd.path.display().to_string()));
            }
            if !cmd.path.join(".git").exists() {
                return Err(HarnessError::NotGitRepo(cmd.path.display().to_string()));
            }

            let loaded = config::load_config(&cmd.path)?;
            let mut continuity_logger = continuity::ContinuityLogger::new(&cmd.path, loaded.as_ref());
            continuity_milestone(
                &mut continuity_logger,
                "suggest",
                "start",
                &[format!("path={}", cmd.path.display())],
                "running",
            );
            let model = scan::discover(&cmd.path, loaded.as_ref());
            let report = analyze::analyze(&model, loaded.as_ref());

            if report.recommendations.is_empty() {
                println!("suggest: no recommendations");
                continuity_milestone(
                    &mut continuity_logger,
                    "suggest",
                    "complete",
                    &[format!("exit_code={}", exit_code::SUCCESS)],
                    "done",
                );
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
                continuity_progress(
                    &mut continuity_logger,
                    "suggest",
                    "plan_exported",
                    &[format!("plan={}", path.display())],
                    "running",
                );
            }

            continuity_milestone(
                &mut continuity_logger,
                "suggest",
                "complete",
                &[
                    format!("recommendations={}", report.recommendations.len()),
                    format!("exit_code={}", exit_code::SUCCESS),
                ],
                "done",
            );
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

            let mut continuity_logger = continuity::ContinuityLogger::new(&cmd.path, None);
            continuity_milestone(
                &mut continuity_logger,
                "init",
                "start",
                &[format!("path={}", cmd.path.display())],
                "running",
            );

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
                continuity_milestone(
                    &mut continuity_logger,
                    "init",
                    "complete",
                    &[
                        "dry_run=true".to_string(),
                        format!("exit_code={}", exit_code::SUCCESS),
                    ],
                    "done",
                );
                return Ok(exit_code::SUCCESS);
            }

            for (path, content) in files {
                if path.exists() && cmd.no_overwrite {
                    println!("skip existing: {}", path.display());
                    continuity_progress(
                        &mut continuity_logger,
                        "init",
                        "skip_existing",
                        &[format!("file={}", path.display())],
                        "running",
                    );
                    continue;
                }
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(HarnessError::Io)?;
                }
                std::fs::write(&path, content).map_err(HarnessError::Io)?;
                continuity_progress(
                    &mut continuity_logger,
                    "init",
                    "file_written",
                    &[format!("file={}", path.display())],
                    "running",
                );
            }
            println!("init complete");
            continuity_milestone(
                &mut continuity_logger,
                "init",
                "complete",
                &[format!("exit_code={}", exit_code::SUCCESS)],
                "done",
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
            match generator::writer::execute_apply(&cmd) {
                Ok(()) => {
                    let mut continuity_logger = continuity::ContinuityLogger::new(&cmd.path, None);
                    continuity_milestone(
                        &mut continuity_logger,
                        "apply",
                        "complete",
                        &[format!("exit_code={}", exit_code::SUCCESS)],
                        "done",
                    );
                    Ok(exit_code::SUCCESS)
                }
                Err(error) => {
                    let mut continuity_logger = continuity::ContinuityLogger::new(&cmd.path, None);
                    continuity_milestone(
                        &mut continuity_logger,
                        "apply",
                        "failed",
                        &[format!("error={}", error)],
                        "blocked",
                    );
                    Err(error)
                }
            }
        }
        cli::Commands::Optimize(cmd) => {
            if !cmd.path.exists() {
                return Err(HarnessError::PathNotFound(cmd.path.display().to_string()));
            }
            if !cmd.path.join(".git").exists() {
                return Err(HarnessError::NotGitRepo(cmd.path.display().to_string()));
            }

            let loaded = config::load_config(&cmd.path)?;
            let mut continuity_logger = continuity::ContinuityLogger::new(&cmd.path, loaded.as_ref());
            continuity_milestone(
                &mut continuity_logger,
                "optimize",
                "start",
                &[format!("path={}", cmd.path.display())],
                "running",
            );
            let thresholds = loaded
                .as_ref()
                .map(types::config::HarnessConfig::optimization_thresholds)
                .unwrap_or_default();

            let trace_dir = cmd
                .trace_dir
                .clone()
                .unwrap_or_else(|| cmd.path.join(".harness/traces"));
            let trace_data = scan_traces(&trace_dir, thresholds.trace_staleness_days)?;
            continuity_progress(
                &mut continuity_logger,
                "optimize",
                "trace_scanned",
                &[
                    format!("recent={}", trace_data.stats.recent),
                    format!("stale={}", trace_data.stats.stale),
                    format!("malformed={}", trace_data.stats.malformed),
                ],
                "running",
            );
            let optimize_delta = compute_optimize_delta(&trace_data.recent, thresholds);

            let model = scan::discover(&cmd.path, loaded.as_ref());
            let report = analyze::analyze(&model, loaded.as_ref());

            let out_dir = cmd.path.join(".harness/optimize");
            std::fs::create_dir_all(&out_dir).map_err(HarnessError::Io)?;
            let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
            let out_path = out_dir.join(format!("optimize-{stamp}.md"));
            let content = render_optimize_report(
                &report,
                trace_data.stats,
                thresholds,
                &trace_dir,
                &optimize_delta,
            );
            std::fs::write(&out_path, content).map_err(HarnessError::Io)?;
            println!("optimize report: {}", out_path.display());
            continuity_milestone(
                &mut continuity_logger,
                "optimize",
                "complete",
                &[
                    format!("status={:?}", optimize_delta.status),
                    format!("exit_code={}", exit_code::SUCCESS),
                ],
                "done",
            );
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
            let mut continuity_logger = continuity::ContinuityLogger::new(&cmd.path, loaded.as_ref());
            continuity_milestone(
                &mut continuity_logger,
                "bench",
                "start",
                &[format!("path={}", cmd.path.display())],
                "running",
            );
            let model = scan::discover(&cmd.path, loaded.as_ref());
            let mut run_results = Vec::new();
            for run_index in 0..cmd.runs {
                let report = analyze::analyze(&model, loaded.as_ref());
                run_results.push(BenchRunResult {
                    run: run_index + 1,
                    overall_score: report.overall_score,
                });
            }
            continuity_progress(
                &mut continuity_logger,
                "bench",
                "runs_completed",
                &[format!("runs={}", run_results.len())],
                "running",
            );

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

            if let Some(compare_path) = &cmd.compare {
                let baseline = load_bench_report(compare_path)?;
                validate_bench_compare_compatibility(
                    &report.bench_context,
                    &baseline.bench_context,
                    cmd.force_compare,
                )?;
                let current_avg = average_overall_score(&report.runs);
                let baseline_avg = average_overall_score(&baseline.runs);
                println!(
                    "bench compare: baseline={:.3}, current={:.3}, delta={:.3}",
                    baseline_avg,
                    current_avg,
                    current_avg - baseline_avg
                );
            }

            let report_path = write_bench_report(&cmd.path, &report)?;
            println!("bench report: {}", report_path.display());
            continuity_milestone(
                &mut continuity_logger,
                "bench",
                "complete",
                &[
                    format!("report={}", report_path.display()),
                    format!("exit_code={}", exit_code::SUCCESS),
                ],
                "done",
            );
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
            let mut continuity_logger = continuity::ContinuityLogger::new(&cmd.path, loaded.as_ref());
            continuity_milestone(
                &mut continuity_logger,
                "lint",
                "start",
                &[format!("path={}", cmd.path.display())],
                "running",
            );
            let model = scan::discover(&cmd.path, loaded.as_ref());
            let findings = analyze::lint::lint_findings(&model, loaded.as_ref());

            if findings.is_empty() {
                println!("lint: no findings");
                continuity_milestone(
                    &mut continuity_logger,
                    "lint",
                    "complete",
                    &[format!("exit_code={}", exit_code::SUCCESS)],
                    "done",
                );
                return Ok(exit_code::SUCCESS);
            }

            for finding in &findings {
                let level = if finding.blocking { "BLOCKING" } else { "WARN" };
                println!("[{}] {}: {}", level, finding.id, finding.title);
                println!("  {}", finding.body);
            }

            let exit = if findings.iter().any(|finding| finding.blocking) {
                exit_code::BLOCKING
            } else {
                exit_code::WARNINGS
            };
            continuity_progress(
                &mut continuity_logger,
                "lint",
                "findings_emitted",
                &[format!("findings={}", findings.len())],
                "running",
            );
            continuity_milestone(
                &mut continuity_logger,
                "lint",
                "complete",
                &[format!("exit_code={exit}")],
                "done",
            );
            Ok(exit)
        }
    }
}

fn continuity_milestone(
    logger: &mut continuity::ContinuityLogger,
    feature: &str,
    action: &str,
    evidence: &[String],
    next_state: &str,
) {
    if let Err(error) = logger.record_milestone(feature, action, evidence, next_state) {
        eprintln!("warning: continuity milestone logging failed: {}", error);
    }
}

fn continuity_progress(
    logger: &mut continuity::ContinuityLogger,
    feature: &str,
    action: &str,
    evidence: &[String],
    next_state: &str,
) {
    if let Err(error) = logger.record_progress(feature, action, evidence, next_state) {
        eprintln!("warning: continuity progress logging failed: {}", error);
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

#[derive(Debug, Serialize, Deserialize)]
struct BenchContext {
    os: String,
    toolchain: String,
    repo_ref: String,
    repo_dirty: bool,
    harness_version: String,
    suite: String,
    timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct BenchRunResult {
    run: u32,
    overall_score: f32,
}

#[derive(Debug, Serialize, Deserialize)]
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

fn load_bench_report(path: &std::path::Path) -> Result<BenchReport, HarnessError> {
    let payload = std::fs::read_to_string(path).map_err(HarnessError::Io)?;
    serde_json::from_str(&payload).map_err(HarnessError::Json)
}

fn average_overall_score(runs: &[BenchRunResult]) -> f32 {
    if runs.is_empty() {
        return 0.0;
    }
    let sum: f32 = runs.iter().map(|run| run.overall_score).sum();
    sum / runs.len() as f32
}

fn validate_bench_compare_compatibility(
    current: &BenchContext,
    baseline: &BenchContext,
    force_compare: bool,
) -> Result<(), HarnessError> {
    let mut mismatches = Vec::new();
    if current.os != baseline.os {
        mismatches.push(format!("os (baseline={}, current={})", baseline.os, current.os));
    }
    if current.toolchain != baseline.toolchain {
        mismatches.push(format!(
            "toolchain (baseline={}, current={})",
            baseline.toolchain, current.toolchain
        ));
    }
    if current.repo_dirty != baseline.repo_dirty {
        mismatches.push(format!(
            "repo_dirty (baseline={}, current={})",
            baseline.repo_dirty, current.repo_dirty
        ));
    }

    if !mismatches.is_empty() && !force_compare {
        return Err(HarnessError::ConfigParse(format!(
            "bench compare blocked due to incompatible context: {}. Re-run with --force-compare to override.",
            mismatches.join(", ")
        )));
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct TraceRecord {
    timestamp: String,
    task_id: Option<String>,
    revision: Option<String>,
    outcome: Option<String>,
    steps: Option<u32>,
    tool_calls: Option<u32>,
    token_est: Option<u64>,
    wall_ms: Option<u64>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct TraceScanStats {
    recent: usize,
    stale: usize,
    malformed: usize,
}

#[derive(Debug, Clone)]
struct RecentTraceRecord {
    timestamp: chrono::DateTime<chrono::Utc>,
    task_id: String,
    revision: String,
    outcome: String,
    steps: Option<u32>,
    token_est: Option<u64>,
}

#[derive(Debug, Clone)]
struct TraceData {
    stats: TraceScanStats,
    recent: Vec<RecentTraceRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OptimizeDeltaStatus {
    Improvement,
    Regression,
    Neutral,
    InsufficientData,
}

#[derive(Debug, Clone)]
struct OptimizeDelta {
    status: OptimizeDeltaStatus,
    baseline_revision: Option<String>,
    current_revision: Option<String>,
    completion_delta: f32,
    token_delta_rel: f32,
    step_delta_rel: f32,
    task_overlap: f32,
    reason: Option<String>,
}

#[derive(Debug, Default)]
struct RevisionAccumulator {
    total: usize,
    success: usize,
    steps_sum: f64,
    steps_count: usize,
    tokens_sum: f64,
    tokens_count: usize,
    tasks: BTreeSet<String>,
    latest_ts: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug)]
struct RevisionMetrics {
    revision: String,
    total: usize,
    completion_rate: f32,
    avg_steps: f32,
    avg_tokens: f32,
    tasks: BTreeSet<String>,
    latest_ts: chrono::DateTime<chrono::Utc>,
}

impl RevisionAccumulator {
    fn add(&mut self, trace: &RecentTraceRecord) {
        self.total += 1;
        if trace.outcome == "success" {
            self.success += 1;
        }
        if let Some(steps) = trace.steps {
            self.steps_sum += f64::from(steps);
            self.steps_count += 1;
        }
        if let Some(token_est) = trace.token_est {
            self.tokens_sum += token_est as f64;
            self.tokens_count += 1;
        }
        self.tasks.insert(trace.task_id.clone());
        self.latest_ts = Some(self.latest_ts.map_or(trace.timestamp, |current| {
            if trace.timestamp > current {
                trace.timestamp
            } else {
                current
            }
        }));
    }

    fn into_metrics(self, revision: String) -> Option<RevisionMetrics> {
        let latest_ts = self.latest_ts?;
        let completion_rate = if self.total == 0 {
            0.0
        } else {
            self.success as f32 / self.total as f32
        };
        let avg_steps = if self.steps_count == 0 {
            0.0
        } else {
            (self.steps_sum / self.steps_count as f64) as f32
        };
        let avg_tokens = if self.tokens_count == 0 {
            0.0
        } else {
            (self.tokens_sum / self.tokens_count as f64) as f32
        };
        Some(RevisionMetrics {
            revision,
            total: self.total,
            completion_rate,
            avg_steps,
            avg_tokens,
            tasks: self.tasks,
            latest_ts,
        })
    }
}

fn scan_traces(trace_dir: &std::path::Path, max_age_days: u32) -> Result<TraceData, HarnessError> {
    if !trace_dir.exists() {
        return Ok(TraceData {
            stats: TraceScanStats::default(),
            recent: Vec::new(),
        });
    }

    let now = chrono::Utc::now();
    let max_age = i64::from(max_age_days);
    let mut stats = TraceScanStats::default();
    let mut recent = Vec::new();

    for entry_result in std::fs::read_dir(trace_dir).map_err(HarnessError::Io)? {
        let entry = entry_result.map_err(HarnessError::Io)?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let extension = path.extension().and_then(|value| value.to_str());
        if !matches!(extension, Some("jsonl" | "json")) {
            continue;
        }

        let content = std::fs::read_to_string(&path).map_err(HarnessError::Io)?;
        for line in content.lines().map(str::trim).filter(|line| !line.is_empty()) {
            let record = match serde_json::from_str::<TraceRecord>(line) {
                Ok(record) => record,
                Err(_) => {
                    stats.malformed += 1;
                    continue;
                }
            };
            let timestamp = match chrono::DateTime::parse_from_rfc3339(&record.timestamp) {
                Ok(value) => value.with_timezone(&chrono::Utc),
                Err(_) => {
                    stats.malformed += 1;
                    continue;
                }
            };
            let age_days = now.signed_duration_since(timestamp).num_days();
            if age_days <= max_age {
                stats.recent += 1;
                if let (Some(task_id), Some(revision), Some(outcome)) =
                    (record.task_id, record.revision, record.outcome)
                {
                    recent.push(RecentTraceRecord {
                        timestamp,
                        task_id,
                        revision,
                        outcome,
                        steps: record.steps,
                        token_est: record.token_est,
                    });
                }
            } else {
                stats.stale += 1;
            }
        }
    }
    Ok(TraceData { stats, recent })
}

fn count_recent_traces(
    trace_dir: &std::path::Path,
    max_age_days: u32,
) -> Result<TraceScanStats, HarnessError> {
    scan_traces(trace_dir, max_age_days).map(|data| data.stats)
}

fn compute_task_overlap(a: &BTreeSet<String>, b: &BTreeSet<String>) -> f32 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let intersection = a.intersection(b).count() as f32;
    let union = a.union(b).count() as f32;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

fn relative_delta(baseline: f32, current: f32) -> f32 {
    if baseline.abs() < f32::EPSILON {
        0.0
    } else {
        (current - baseline) / baseline
    }
}

fn compute_optimize_delta(
    traces: &[RecentTraceRecord],
    thresholds: types::config::OptimizationThresholds,
) -> OptimizeDelta {
    let mut per_revision: BTreeMap<String, RevisionAccumulator> = BTreeMap::new();
    for trace in traces {
        per_revision
            .entry(trace.revision.clone())
            .or_default()
            .add(trace);
    }

    let mut revisions = per_revision
        .into_iter()
        .filter_map(|(revision, accumulator)| accumulator.into_metrics(revision))
        .collect::<Vec<_>>();

    if revisions.len() < 2 {
        return OptimizeDelta {
            status: OptimizeDeltaStatus::InsufficientData,
            baseline_revision: None,
            current_revision: None,
            completion_delta: 0.0,
            token_delta_rel: 0.0,
            step_delta_rel: 0.0,
            task_overlap: 0.0,
            reason: Some("need traces from at least two revisions".to_string()),
        };
    }

    revisions.sort_by(|a, b| a.latest_ts.cmp(&b.latest_ts));
    let baseline = &revisions[revisions.len() - 2];
    let current = &revisions[revisions.len() - 1];

    if baseline.total < thresholds.min_traces as usize || current.total < thresholds.min_traces as usize {
        return OptimizeDelta {
            status: OptimizeDeltaStatus::InsufficientData,
            baseline_revision: Some(baseline.revision.clone()),
            current_revision: Some(current.revision.clone()),
            completion_delta: 0.0,
            token_delta_rel: 0.0,
            step_delta_rel: 0.0,
            task_overlap: 0.0,
            reason: Some(format!(
                "need at least {} traces per revision (baseline={}, current={})",
                thresholds.min_traces, baseline.total, current.total
            )),
        };
    }

    let overlap = compute_task_overlap(&baseline.tasks, &current.tasks);
    if overlap < thresholds.task_overlap_threshold {
        return OptimizeDelta {
            status: OptimizeDeltaStatus::InsufficientData,
            baseline_revision: Some(baseline.revision.clone()),
            current_revision: Some(current.revision.clone()),
            completion_delta: 0.0,
            token_delta_rel: 0.0,
            step_delta_rel: 0.0,
            task_overlap: overlap,
            reason: Some(format!(
                "task overlap {:.2} is below threshold {:.2}",
                overlap, thresholds.task_overlap_threshold
            )),
        };
    }

    let completion_delta = current.completion_rate - baseline.completion_rate;
    let token_delta_rel = relative_delta(baseline.avg_tokens, current.avg_tokens);
    let step_delta_rel = relative_delta(baseline.avg_steps, current.avg_steps);

    let completion_signal = if completion_delta >= thresholds.min_uplift_abs {
        1
    } else if completion_delta <= -thresholds.min_uplift_abs {
        -1
    } else {
        0
    };
    let token_signal = if token_delta_rel <= -thresholds.min_uplift_rel {
        1
    } else if token_delta_rel >= thresholds.min_uplift_rel {
        -1
    } else {
        0
    };
    let step_signal = if step_delta_rel <= -thresholds.min_uplift_rel {
        1
    } else if step_delta_rel >= thresholds.min_uplift_rel {
        -1
    } else {
        0
    };
    let total_signal = completion_signal + token_signal + step_signal;

    let (status, reason) = if total_signal > 0 {
        (OptimizeDeltaStatus::Improvement, None)
    } else if total_signal < 0 {
        (OptimizeDeltaStatus::Regression, None)
    } else {
        (
            OptimizeDeltaStatus::Neutral,
            Some("changes are below configured uplift thresholds".to_string()),
        )
    };

    OptimizeDelta {
        status,
        baseline_revision: Some(baseline.revision.clone()),
        current_revision: Some(current.revision.clone()),
        completion_delta,
        token_delta_rel,
        step_delta_rel,
        task_overlap: overlap,
        reason,
    }
}

fn render_optimize_report(
    report: &types::report::HarnessReport,
    trace_scan: TraceScanStats,
    thresholds: types::config::OptimizationThresholds,
    trace_dir: &std::path::Path,
    delta: &OptimizeDelta,
) -> String {
    let mut ordered_report = report.clone();
    ordered_report.sort_recommendations();

    let mut lines = vec![
        "# Harness Optimize Report".to_string(),
        String::new(),
        format!("Overall score: {:.3}", ordered_report.overall_score),
        format!("Trace directory: {}", trace_dir.display()),
        format!(
            "Trace records: recent={}, stale={}, malformed={}",
            trace_scan.recent, trace_scan.stale, trace_scan.malformed
        ),
        format!(
            "Recent traces required for optimization: {}",
            thresholds.min_traces
        ),
        String::new(),
    ];

    if trace_scan.malformed > 0 {
        lines.push(format!(
            "Warning: ignored malformed trace records: {}",
            trace_scan.malformed
        ));
    }

    if trace_scan.recent < thresholds.min_traces as usize {
        lines.push(
            "Status: insufficient data for optimization recommendations.".to_string(),
        );
        lines.push(format!(
            "Need at least {} recent traces before computing optimize deltas.",
            thresholds.min_traces
        ));
        lines.push(String::new());
        return lines.join("\n");
    }

    lines.push("## Optimization Delta".to_string());
    if let (Some(baseline), Some(current)) = (&delta.baseline_revision, &delta.current_revision) {
        lines.push(format!(
            "- revisions compared: baseline=`{}`, current=`{}`",
            baseline, current
        ));
    }
    lines.push(format!("- task overlap: {:.2}", delta.task_overlap));
    lines.push(format!(
        "- completion delta: {:+.3}, token delta (rel): {:+.3}, step delta (rel): {:+.3}",
        delta.completion_delta, delta.token_delta_rel, delta.step_delta_rel
    ));
    match delta.status {
        OptimizeDeltaStatus::Improvement => {
            lines.push("Status: improvement detected.".to_string());
        }
        OptimizeDeltaStatus::Regression => {
            lines.push("Status: regression warning.".to_string());
        }
        OptimizeDeltaStatus::Neutral => {
            lines.push("Status: stable; changes are below uplift thresholds.".to_string());
        }
        OptimizeDeltaStatus::InsufficientData => {
            lines.push("Status: insufficient comparative data for optimize deltas.".to_string());
        }
    }
    if let Some(reason) = &delta.reason {
        lines.push(format!("Reason: {}", reason));
    }
    lines.push(String::new());

    if matches!(delta.status, OptimizeDeltaStatus::InsufficientData) {
        return lines.join("\n");
    }

    lines.push("## Top Recommendations".to_string());

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::report::{Effort, HarnessReport, Impact, Recommendation, Risk};
    use crate::types::scoring::ScoreCard;

    fn make_bench_context(os: &str, toolchain: &str, repo_dirty: bool) -> BenchContext {
        BenchContext {
            os: os.to_string(),
            toolchain: toolchain.to_string(),
            repo_ref: "abc".to_string(),
            repo_dirty,
            harness_version: "0.1.0".to_string(),
            suite: "default".to_string(),
            timestamp: "2026-02-27T00:00:00Z".to_string(),
        }
    }

    fn default_thresholds() -> types::config::OptimizationThresholds {
        types::config::OptimizationThresholds::default()
    }

    fn neutral_delta() -> OptimizeDelta {
        OptimizeDelta {
            status: OptimizeDeltaStatus::Neutral,
            baseline_revision: Some("rev-a".to_string()),
            current_revision: Some("rev-b".to_string()),
            completion_delta: 0.0,
            token_delta_rel: 0.0,
            step_delta_rel: 0.0,
            task_overlap: 1.0,
            reason: Some("changes are below configured uplift thresholds".to_string()),
        }
    }

    fn make_recent_trace(
        revision: &str,
        task_id: &str,
        outcome: &str,
        steps: u32,
        token_est: u64,
    ) -> RecentTraceRecord {
        RecentTraceRecord {
            timestamp: chrono::Utc::now(),
            task_id: task_id.to_string(),
            revision: revision.to_string(),
            outcome: outcome.to_string(),
            steps: Some(steps),
            token_est: Some(token_est),
        }
    }

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

        let rendered = render_optimize_report(
            &report,
            TraceScanStats {
                recent: 30,
                stale: 0,
                malformed: 0,
            },
            default_thresholds(),
            std::path::Path::new(".harness/traces"),
            &neutral_delta(),
        );
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

    #[test]
    fn render_optimize_report_shows_insufficient_data_gate() {
        let report = HarnessReport {
            overall_score: 0.5,
            category_scores: ScoreCard::new(0.5, 0.5, 0.5, 0.5, 0.5),
            findings: vec![],
            recommendations: vec![Recommendation::new(
                "high",
                "High",
                "high summary",
                Impact::High,
                Effort::S,
                Risk::Safe,
                0.9,
            )],
        };

        let rendered = render_optimize_report(
            &report,
            TraceScanStats {
                recent: 2,
                stale: 0,
                malformed: 0,
            },
            default_thresholds(),
            std::path::Path::new(".harness/traces"),
            &neutral_delta(),
        );
        assert!(rendered.contains("insufficient data"));
        assert!(!rendered.contains("## Top Recommendations"));
    }

    #[test]
    fn render_optimize_report_surfaces_malformed_trace_warning() {
        let report = HarnessReport {
            overall_score: 0.5,
            category_scores: ScoreCard::new(0.5, 0.5, 0.5, 0.5, 0.5),
            findings: vec![],
            recommendations: vec![],
        };

        let rendered = render_optimize_report(
            &report,
            TraceScanStats {
                recent: 30,
                stale: 1,
                malformed: 2,
            },
            default_thresholds(),
            std::path::Path::new(".harness/traces"),
            &neutral_delta(),
        );

        assert!(rendered.contains("ignored malformed trace records: 2"));
    }

    #[test]
    fn count_recent_traces_reports_recent_stale_and_malformed_records() {
        let dir = tempfile::TempDir::new().expect("temp dir should be created");
        let now = chrono::Utc::now();
        let content = format!(
            "{{\"timestamp\":\"{}\"}}\n{{\"timestamp\":\"{}\"}}\nnot-json\n{{\"timestamp\":\"invalid\"}}\n",
            now.to_rfc3339(),
            (now - chrono::Duration::days(120)).to_rfc3339(),
        );
        std::fs::write(dir.path().join("traces.jsonl"), content).expect("trace file should write");

        let stats = count_recent_traces(dir.path(), 90).expect("trace scan should succeed");
        assert_eq!(
            stats,
            TraceScanStats {
                recent: 1,
                stale: 1,
                malformed: 2,
            }
        );
    }

    #[test]
    fn compute_optimize_delta_detects_improvement() {
        let thresholds = types::config::OptimizationThresholds {
            min_traces: 1,
            min_uplift_abs: 0.05,
            min_uplift_rel: 0.10,
            trace_staleness_days: 90,
            task_overlap_threshold: 0.50,
        };
        let traces = vec![
            make_recent_trace("rev-a", "task-1", "failure", 20, 200),
            make_recent_trace("rev-a", "task-2", "success", 20, 200),
            make_recent_trace("rev-b", "task-1", "success", 10, 100),
            make_recent_trace("rev-b", "task-2", "success", 10, 100),
        ];
        let delta = compute_optimize_delta(&traces, thresholds);
        assert_eq!(delta.status, OptimizeDeltaStatus::Improvement);
        assert!(delta.completion_delta > 0.0);
        assert!(delta.token_delta_rel < 0.0);
        assert!(delta.step_delta_rel < 0.0);
    }

    #[test]
    fn compute_optimize_delta_detects_regression() {
        let thresholds = types::config::OptimizationThresholds {
            min_traces: 1,
            min_uplift_abs: 0.05,
            min_uplift_rel: 0.10,
            trace_staleness_days: 90,
            task_overlap_threshold: 0.50,
        };
        let traces = vec![
            make_recent_trace("rev-a", "task-1", "success", 10, 100),
            make_recent_trace("rev-a", "task-2", "success", 10, 100),
            make_recent_trace("rev-b", "task-1", "failure", 20, 220),
            make_recent_trace("rev-b", "task-2", "success", 20, 220),
        ];
        let delta = compute_optimize_delta(&traces, thresholds);
        assert_eq!(delta.status, OptimizeDeltaStatus::Regression);
        assert!(delta.completion_delta < 0.0);
        assert!(delta.token_delta_rel > 0.0);
        assert!(delta.step_delta_rel > 0.0);
    }

    #[test]
    fn compute_optimize_delta_rejects_low_task_overlap() {
        let thresholds = types::config::OptimizationThresholds {
            min_traces: 1,
            min_uplift_abs: 0.05,
            min_uplift_rel: 0.10,
            trace_staleness_days: 90,
            task_overlap_threshold: 0.80,
        };
        let traces = vec![
            make_recent_trace("rev-a", "task-1", "success", 10, 100),
            make_recent_trace("rev-a", "task-2", "success", 10, 100),
            make_recent_trace("rev-b", "task-3", "success", 10, 100),
            make_recent_trace("rev-b", "task-4", "success", 10, 100),
        ];
        let delta = compute_optimize_delta(&traces, thresholds);
        assert_eq!(delta.status, OptimizeDeltaStatus::InsufficientData);
        let reason = delta.reason.expect("reason should exist");
        assert!(reason.contains("task overlap"));
    }

    #[test]
    fn bench_compare_rejects_mismatched_context_without_force() {
        let current = make_bench_context("linux-x86_64", "rustc 1.77.0", false);
        let baseline = make_bench_context("darwin-aarch64", "rustc 1.77.0", false);

        let err = validate_bench_compare_compatibility(&current, &baseline, false)
            .expect_err("compare should be blocked");
        assert!(err.to_string().contains("bench compare blocked"));
        assert!(err.to_string().contains("os"));
    }

    #[test]
    fn bench_compare_allows_mismatched_context_with_force() {
        let current = make_bench_context("linux-x86_64", "rustc 1.77.0", false);
        let baseline = make_bench_context("darwin-aarch64", "rustc 1.77.0", false);

        let result = validate_bench_compare_compatibility(&current, &baseline, true);
        assert!(result.is_ok());
    }

    #[test]
    fn bench_average_overall_score_handles_empty_and_non_empty_runs() {
        assert!((average_overall_score(&[]) - 0.0).abs() < 0.001);
        let runs = vec![
            BenchRunResult {
                run: 1,
                overall_score: 0.6,
            },
            BenchRunResult {
                run: 2,
                overall_score: 0.8,
            },
        ];
        assert!((average_overall_score(&runs) - 0.7).abs() < 0.001);
    }
}

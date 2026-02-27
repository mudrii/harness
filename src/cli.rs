use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "harness", version, about = "AI agent harness analysis and optimization CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Init(InitCommand),
    Analyze(AnalyzeCommand),
    Suggest(SuggestCommand),
    Apply(ApplyCommand),
    Optimize(OptimizeCommand),
    Bench(BenchCommand),
    Lint(LintCommand),
}

#[derive(Args)]
pub struct InitCommand {
    pub path: PathBuf,
    #[arg(long, default_value = "general")]
    pub profile: String,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub no_overwrite: bool,
}

#[derive(Args)]
pub struct AnalyzeCommand {
    pub path: PathBuf,
    #[arg(short, long, value_enum, default_value = "md")]
    pub format: ReportFormat,
    #[arg(long, default_value = "all")]
    pub min_impact: String,
}

#[derive(Args)]
pub struct SuggestCommand {
    pub path: PathBuf,
    #[arg(long)]
    pub export_diff: bool,
}

#[derive(Args)]
pub struct ApplyCommand {
    pub path: PathBuf,
    #[arg(long)]
    pub plan_file: Option<String>,
    #[arg(long, default_value = "preview")]
    pub apply_mode: String,
}

#[derive(Args)]
pub struct OptimizeCommand {
    pub path: PathBuf,
    #[arg(long)]
    pub trace_dir: Option<PathBuf>,
}

#[derive(Args)]
pub struct BenchCommand {
    pub path: PathBuf,
    #[arg(long)]
    pub suite: Option<String>,
    #[arg(long, default_value_t = 1)]
    pub runs: u32,
}

#[derive(Args)]
pub struct LintCommand {
    pub path: PathBuf,
}

#[derive(Clone, ValueEnum)]
pub enum ReportFormat {
    Json,
    Md,
    Sarif,
}

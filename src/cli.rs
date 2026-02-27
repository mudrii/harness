use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "harness",
    version,
    about = "AI agent harness analysis and optimization CLI"
)]
pub struct Cli {
    /// Increase verbosity (-v for info, -vv for debug)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Suppress all output except errors
    #[arg(short, long, global = true, conflicts_with = "verbose")]
    pub quiet: bool,

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

#[derive(Clone, Debug, ValueEnum)]
pub enum Profile {
    General,
    Agent,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum MinImpact {
    Safe,
    All,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum ApplyMode {
    Preview,
    Apply,
}

#[derive(Args)]
pub struct InitCommand {
    pub path: PathBuf,
    #[arg(long, value_enum, default_value = "general")]
    pub profile: Profile,
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
    #[arg(long, value_enum, default_value = "all")]
    pub min_impact: MinImpact,
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

    #[arg(
        long,
        required_unless_present = "plan_all",
        conflicts_with = "plan_all"
    )]
    pub plan_file: Option<String>,

    #[arg(
        long,
        required_unless_present = "plan_file",
        conflicts_with = "plan_file"
    )]
    pub plan_all: bool,

    #[arg(long, value_enum, default_value = "preview")]
    pub apply_mode: ApplyMode,
    #[arg(long)]
    pub allow_dirty: bool,
    #[arg(long, short)]
    pub yes: bool,
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

pub mod json;
pub mod md;
pub mod sarif;

use crate::error::HarnessError;
use crate::types::report::HarnessReport;

#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Json,
    Md,
    Sarif,
}

pub fn render(report: &HarnessReport, format: OutputFormat) -> Result<String, HarnessError> {
    match format {
        OutputFormat::Json => json::to_json(report).map_err(HarnessError::Json),
        OutputFormat::Md => Ok(md::to_markdown(report)),
        OutputFormat::Sarif => sarif::to_sarif(report).map_err(HarnessError::Json),
    }
}

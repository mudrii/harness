use crate::types::report::HarnessReport;
use serde_json::json;

pub fn to_sarif(report: &HarnessReport) -> Result<String, serde_json::Error> {
    let results: Vec<_> = report
        .findings
        .iter()
        .map(|finding| {
            json!({
                "ruleId": finding.id,
                "level": if finding.blocking { "error" } else { "warning" },
                "message": { "text": finding.body },
            })
        })
        .collect();

    let sarif = json!({
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "harness"
                }
            },
            "results": results
        }]
    });

    serde_json::to_string_pretty(&sarif)
}

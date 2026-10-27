use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetCheck {
    pub label: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetResult {
    pub target: String,
    pub checks: Vec<TargetCheck>,
    pub ping: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyReport {
    pub strategy_id: String,
    pub strategy_name: String,
    pub ok_count: i32,
    pub err_count: i32,
    pub results: Vec<TargetResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestSummary {
    pub best_strategy: Option<String>,
    pub reports: Vec<StrategyReport>,
}

impl StrategyReport {
    pub fn from_results(
        strategy_id: impl Into<String>,
        strategy_name: impl Into<String>,
        results: Vec<TargetResult>,
    ) -> Self {
        let ok_count = results
            .iter()
            .filter(|r| r.checks.iter().any(|c| c.status == "OK"))
            .count() as i32;
        let err_count = results
            .iter()
            .flat_map(|r| r.checks.iter())
            .filter(|c| c.status == "ERR" || c.status == "SSL")
            .count() as i32;
        Self {
            strategy_id: strategy_id.into(),
            strategy_name: strategy_name.into(),
            ok_count,
            err_count,
            results,
        }
    }
}

/// Strategy with most OK checks wins; first max wins ties (stable).
pub fn pick_best(reports: &[StrategyReport]) -> Option<String> {
    let mut best: Option<&StrategyReport> = None;
    for r in reports {
        match best {
            Some(b) if b.ok_count >= r.ok_count => {}
            _ => best = Some(r),
        }
    }
    best.map(|r| r.strategy_id.clone())
}

pub fn summarize(reports: Vec<StrategyReport>) -> TestSummary {
    let best_strategy = pick_best(&reports);
    TestSummary {
        best_strategy,
        reports,
    }
}

/// Build a full report from raw results (helper for FFI/JSON flows).
pub fn report_json(id: &str, name: &str, results: Vec<TargetResult>) -> String {
    serde_json::to_string(&StrategyReport::from_results(id, name, results)).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn res(target: &str, statuses: &[&str]) -> TargetResult {
        TargetResult {
            target: target.into(),
            checks: statuses
                .iter()
                .map(|s| TargetCheck {
                    label: "HTTP".into(),
                    status: s.to_string(),
                })
                .collect(),
            ping: "1 ms".into(),
        }
    }

    #[test]
    fn scoring_counts_ok_and_err() {
        let r = StrategyReport::from_results(
            "general",
            "General",
            vec![
                res("Discord", &["OK", "OK", "ERR"]),
                res("YouTube", &["ERR", "ERR", "ERR"]),
            ],
        );
        assert_eq!(r.ok_count, 1);
        assert_eq!(r.err_count, 4);
    }

    #[test]
    fn pick_best_prefers_more_ok_and_stable_ties() {
        let a = StrategyReport::from_results("a", "A", vec![res("D", &["OK"])]);
        let b = StrategyReport::from_results(
            "b",
            "B",
            vec![res("D", &["OK"]), res("Y", &["OK"])],
        );
        let c = StrategyReport::from_results(
            "c",
            "C",
            vec![res("D", &["OK"]), res("Y", &["OK"])],
        );
        let best = pick_best(&[a, b, c]).unwrap();
        assert_eq!(best, "b"); // first max wins on tie
    }

    #[test]
    fn pick_best_none_when_empty() {
        assert_eq!(pick_best(&[]), None);
    }
}

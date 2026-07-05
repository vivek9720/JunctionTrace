use crate::catalog;
use crate::model::{Bundle, OccupancyState};
use crate::route;
use crate::script;

#[derive(Clone, Debug)]
pub struct AnalysisReport {
    pub findings: Vec<AnalysisFinding>,
    pub route_count: usize,
    pub frame_count: usize,
    pub occupancy_events: usize,
    pub journal_records: usize,
    pub script_steps: usize,
}

#[derive(Clone, Debug)]
pub struct AnalysisFinding {
    pub code: u32,
    pub severity: u8,
    pub message: String,
    pub route_id: Option<u16>,
}

pub struct Analyzer;

impl Analyzer {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze(&self, bundle: &Bundle) -> AnalysisReport {
        let mut findings = Vec::new();
        for route in &bundle.routes {
            if route.nodes.len() > 24 && route.releases.is_empty() {
                findings.push(AnalysisFinding {
                    code: 1001,
                    severity: 4,
                    message: "long route without release rule".to_string(),
                    route_id: Some(route.route_id),
                });
            }
            if let Some(first) = route.nodes.first() {
                if !route::route_contains_signal(route, first.signal_id) {
                    findings.push(AnalysisFinding {
                        code: 1002,
                        severity: 2,
                        message: "route signal catalog mismatch".to_string(),
                        route_id: Some(route.route_id),
                    });
                }
            }
            if catalog::interlocks::interlock_pressure(route.route_id, route.score) > 900_000 {
                findings.push(AnalysisFinding {
                    code: 1003,
                    severity: 3,
                    message: "route has high interlock pressure".to_string(),
                    route_id: Some(route.route_id),
                });
            }
        }

        for message in &bundle.messages {
            if let Some(byte) = message.audit_byte {
                if byte & 0xf0 == 0xf0 {
                    findings.push(AnalysisFinding {
                        code: 2001,
                        severity: 5,
                        message: "handoff audit byte indicates blocked authority".to_string(),
                        route_id: Some(message.route_id),
                    });
                }
            }
        }

        let mut occupancy_events = 0usize;
        for batch in &bundle.occupancy {
            occupancy_events += batch.events.len();
            let occupied = batch
                .events
                .iter()
                .filter(|e| e.state == OccupancyState::Occupied)
                .count();
            let shunted = batch
                .events
                .iter()
                .filter(|e| e.state == OccupancyState::Shunted)
                .count();
            if occupied > 8 && shunted > 2 {
                findings.push(AnalysisFinding {
                    code: 3001,
                    severity: 4,
                    message: "occupancy trace mixes sustained occupancy and shunts".to_string(),
                    route_id: Some(batch.route_id),
                });
            }
            if batch.measurements.iter().any(|m| m.count_ones() > 24) {
                findings.push(AnalysisFinding {
                    code: 3002,
                    severity: 3,
                    message: "occupancy replay measurement is noisy".to_string(),
                    route_id: Some(batch.route_id),
                });
            }
        }

        let mut journal_records = 0usize;
        for journal in &bundle.journals {
            journal_records += journal.records.len();
            if journal.attachments_seen > 8 && !journal.committed {
                findings.push(AnalysisFinding {
                    code: 4001,
                    severity: 3,
                    message: "journal has many uncommitted attachments".to_string(),
                    route_id: None,
                });
            }
        }

        let mut script_steps = 0usize;
        for program in &bundle.scripts {
            let report = script::run_program(program);
            script_steps += report.steps;
            if report.emitted.len() > 16 && report.accumulator & 0xff == 0 {
                findings.push(AnalysisFinding {
                    code: 5001,
                    severity: 2,
                    message: "controller script emitted a zero-biased accumulator".to_string(),
                    route_id: None,
                });
            }
        }

        AnalysisReport {
            findings,
            route_count: bundle.routes.len(),
            frame_count: bundle.frames.len(),
            occupancy_events,
            journal_records,
            script_steps,
        }
    }
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}

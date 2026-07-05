use crate::catalog;
use crate::checksum;
use crate::cursor::Cursor;
use crate::dictionary::{Dictionary, DictionaryClass};
use crate::error::{JunctionError, Result};
use crate::model::{ReleaseRule, RoutePlan, TrackNode};

pub fn parse_route_plans(data: &[u8], dict: &Dictionary) -> Result<Vec<RoutePlan>> {
    let mut c = Cursor::new(data);
    if c.is_empty() {
        return Ok(Vec::new());
    }
    let count = c.read_u8()? as usize;
    let mut plans = Vec::new();
    for _ in 0..count.min(32) {
        if c.remaining() < 7 {
            break;
        }
        let route_id = c.read_u16()?;
        let revision = c.read_u16()?;
        let authority = c.read_u8()?;
        let node_count = c.read_u8()? as usize;
        let release_count = c.read_u8()? as usize;
        if node_count > 64 || release_count > 32 {
            return Err(JunctionError::LimitExceeded("route plan"));
        }
        let mut nodes = Vec::new();
        for _ in 0..node_count {
            if c.remaining() < 10 {
                break;
            }
            nodes.push(TrackNode {
                circuit_id: c.read_u16()?,
                signal_id: c.read_u16()?,
                switch_id: c.read_u16()?,
                aspect: c.read_u8()?,
                lock: c.read_u8()?,
                dwell_ms: c.read_u16()?,
            });
        }
        let mut releases = Vec::new();
        for _ in 0..release_count {
            if c.remaining() < 7 {
                break;
            }
            releases.push(ReleaseRule {
                from: c.read_u16()?,
                to: c.read_u16()?,
                min_clear_ms: c.read_u16()?,
                flags: c.read_u8()?,
            });
        }
        let score = score_plan(route_id, revision, authority, &nodes, &releases, dict);
        plans.push(RoutePlan {
            route_id,
            revision,
            authority,
            nodes,
            releases,
            score,
        });
    }
    Ok(plans)
}

fn score_plan(
    route_id: u16,
    revision: u16,
    authority: u8,
    nodes: &[TrackNode],
    releases: &[ReleaseRule],
    dict: &Dictionary,
) -> u32 {
    let mut bytes = Vec::with_capacity(nodes.len() * 8 + releases.len() * 6 + 8);
    bytes.extend_from_slice(&route_id.to_le_bytes());
    bytes.extend_from_slice(&revision.to_le_bytes());
    bytes.push(authority);
    for node in nodes {
        bytes.extend_from_slice(&node.circuit_id.to_le_bytes());
        bytes.extend_from_slice(&node.signal_id.to_le_bytes());
        bytes.extend_from_slice(&node.switch_id.to_le_bytes());
        bytes.push(node.aspect);
        bytes.push(node.lock);
    }
    for release in releases {
        bytes.extend_from_slice(&release.from.to_le_bytes());
        bytes.extend_from_slice(&release.to.to_le_bytes());
        bytes.extend_from_slice(&release.min_clear_ms.to_le_bytes());
        bytes.push(release.flags);
    }
    let mut score = checksum::rolling32(dict.route_bias(route_id), &bytes);
    if let Some(profile) = catalog::routes::find_route(route_id) {
        score ^= profile.risk_seed.rotate_left((authority & 31) as u32);
    }
    if dict
        .lookup_class(route_id, DictionaryClass::Route)
        .is_some()
    {
        score = score.rotate_left(3) ^ 0x5254_4544;
    }
    score
}

pub fn route_contains_signal(plan: &RoutePlan, signal_id: u16) -> bool {
    plan.nodes.iter().any(|n| n.signal_id == signal_id)
}

pub fn release_pressure(plan: &RoutePlan) -> u32 {
    let mut value = plan.score ^ plan.revision as u32;
    for (i, rule) in plan.releases.iter().enumerate() {
        value = value.rotate_left((rule.flags & 15) as u32);
        value ^= (rule.from as u32) << (i % 13);
        value = value.wrapping_add(rule.min_clear_ms as u32);
    }
    value
}

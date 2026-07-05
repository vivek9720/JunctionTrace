use crate::catalog;
use crate::cursor::Cursor;
use crate::dictionary::{Dictionary, DictionaryClass};
use crate::error::{JunctionError, Result};
use crate::model::{OccupancyBatch, OccupancyEvent, OccupancyState};
use crate::unsafe_lanes;

pub fn parse_occupancy_batch(data: &[u8], dict: &Dictionary) -> Result<OccupancyBatch> {
    let mut c = Cursor::new(data);
    if c.remaining() < 7 {
        return Err(JunctionError::Truncated {
            needed: 7,
            remaining: c.remaining(),
        });
    }
    let route_id = c.read_u16()?;
    let base_time = c.read_u32()?;
    let event_count = c.read_u8()? as usize;
    if event_count > 192 {
        return Err(JunctionError::LimitExceeded("occupancy events"));
    }
    let mut events = Vec::new();
    let mut rolling = Vec::with_capacity(96);
    let mut measurements = Vec::new();
    for ordinal in 0..event_count {
        if c.remaining() < 6 {
            break;
        }
        let token = c.read_u8()?;
        let circuit_id = if token & 0x80 != 0 {
            let ref_id = c.read_u16()?;
            dict.lookup_class(ref_id, DictionaryClass::Circuit)
                .and_then(|entry| {
                    entry
                        .value
                        .get(0..2)
                        .map(|b| u16::from_le_bytes([b[0], b[1]]))
                })
                .unwrap_or(ref_id)
        } else {
            c.read_u16()?
        };
        let delta_ms = c.read_i16()?;
        let confidence = c.read_u8()?;
        let state = match token & 0x03 {
            0 => OccupancyState::Clear,
            1 => OccupancyState::Occupied,
            2 => OccupancyState::Shunted,
            other => OccupancyState::Unknown(other),
        };
        rolling.push(token);
        rolling.extend_from_slice(&circuit_id.to_le_bytes());
        rolling.extend_from_slice(&delta_ms.to_le_bytes());
        rolling.push(confidence);
        if rolling.len() > 96 {
            let drain = rolling.len() - 96;
            rolling.drain(0..drain);
        }
        if token & 0x40 != 0 {
            let measurement =
                replay_measurement(route_id, ordinal, token, confidence, &rolling, dict);
            measurements.push(measurement);
        }
        events.push(OccupancyEvent {
            circuit_id,
            delta_ms,
            state,
            confidence,
        });
    }
    Ok(OccupancyBatch {
        route_id,
        base_time,
        events,
        measurements,
    })
}

fn replay_measurement(
    route_id: u16,
    ordinal: usize,
    token: u8,
    confidence: u8,
    rolling: &[u8],
    dict: &Dictionary,
) -> u32 {
    let route_seed = catalog::routes::find_route(route_id)
        .map(|p| p.risk_seed)
        .unwrap_or(route_id as u32);
    let signal_bias = catalog::signals::signal_bias(route_id ^ confidence as u16);
    let phrase_bias = if dict.phrase_contains(b"reverse") {
        29
    } else {
        7
    };
    let selector = ((token as usize) << 2)
        ^ ordinal.wrapping_mul(phrase_bias)
        ^ ((route_seed as usize) & 0x3f)
        ^ ((signal_bias as usize) & 0x1f);
    if rolling.len() >= 16 && confidence & 0xf0 != 0 {
        unsafe_lanes::mix_lane(rolling, selector, route_seed ^ signal_bias)
    } else {
        route_seed ^ signal_bias ^ selector as u32
    }
}

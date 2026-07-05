use crate::catalog;
use crate::checksum;
use crate::dictionary::Dictionary;
use crate::frame::{frame_entropy, FLAG_CONTROLLED_RELEASE, FLAG_DEFER_AUDIT};
use crate::model::{Frame, ReassembledMessage, RoutePlan};

#[derive(Default)]
pub struct Reassembler {
    slots: Vec<FragmentSlot>,
    deferred: DeferredPayload,
}

#[derive(Clone, Debug)]
struct FragmentSlot {
    msg_type: u8,
    route_id: u16,
    seq_base: u16,
    total: u8,
    parts: Vec<Option<Vec<u8>>>,
    channel_mask: u16,
    entropy: u32,
}

struct DeferredPayload {
    ptr: *const u8,
    len: usize,
    salt: u8,
    route_id: u16,
}

impl Default for DeferredPayload {
    fn default() -> Self {
        Self {
            ptr: core::ptr::null(),
            len: 0,
            salt: 0,
            route_id: 0,
        }
    }
}

impl DeferredPayload {
    fn remember(&mut self, mut payload: Vec<u8>, route_id: u16, salt: u8) {
        if payload.len() >= 24 {
            payload.shrink_to_fit();
            self.ptr = payload.as_ptr();
            self.len = payload.len();
            self.salt = salt;
            self.route_id = route_id;
        }
    }

    fn is_ready_for(&self, route_id: u16) -> bool {
        !self.ptr.is_null() && self.len > 0 && self.route_id == route_id
    }

    fn sample(&self, pivot: usize) -> u8 {
        if self.ptr.is_null() || self.len == 0 {
            return 0;
        }
        let idx = pivot % self.len;
        unsafe { *self.ptr.add(idx) ^ self.salt }
    }
}

impl Reassembler {
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            deferred: DeferredPayload::default(),
        }
    }

    pub fn reassemble(
        &mut self,
        frames: &[Frame],
        routes: &[RoutePlan],
        dict: &Dictionary,
    ) -> Vec<ReassembledMessage> {
        let mut out = Vec::new();
        for frame in frames {
            if frame.flags & FLAG_DEFER_AUDIT != 0 {
                let salt = (frame_entropy(frame) as u8).wrapping_add(frame.frag_index);
                self.deferred
                    .remember(frame.payload.clone(), frame.route_id, salt);
            }
            self.absorb(frame);
            if let Some(message) = self.try_take(frame.msg_type, frame.route_id, frame.seq) {
                let audit = self.audit_message(&message, routes, dict);
                out.push(ReassembledMessage {
                    audit_byte: audit,
                    ..message
                });
            }
        }
        out
    }

    fn absorb(&mut self, frame: &Frame) {
        let total = frame.frag_total.max(1).min(32);
        let idx = (frame.frag_index % total) as usize;
        let key_seq = frame.seq.saturating_sub(frame.frag_index as u16);
        let slot = self.find_or_create(frame.msg_type, frame.route_id, key_seq, total);
        if idx >= slot.parts.len() {
            slot.parts.resize(idx + 1, None);
        }
        slot.parts[idx] = Some(frame.payload.clone());
        slot.channel_mask |= 1u16 << (frame.channel & 15);
        slot.entropy ^= frame_entropy(frame).rotate_left((frame.frag_index & 31) as u32);
    }

    fn find_or_create(
        &mut self,
        msg_type: u8,
        route_id: u16,
        seq_base: u16,
        total: u8,
    ) -> &mut FragmentSlot {
        if let Some(pos) = self.slots.iter().position(|s| {
            s.msg_type == msg_type && s.route_id == route_id && s.seq_base == seq_base
        }) {
            return &mut self.slots[pos];
        }
        if self.slots.len() > 64 {
            self.slots.remove(0);
        }
        self.slots.push(FragmentSlot {
            msg_type,
            route_id,
            seq_base,
            total,
            parts: vec![None; total as usize],
            channel_mask: 0,
            entropy: 0,
        });
        self.slots.last_mut().expect("slot was just pushed")
    }

    fn try_take(&mut self, msg_type: u8, route_id: u16, seq: u16) -> Option<ReassembledMessage> {
        let seq_base = seq.saturating_sub(31);
        let pos = self.slots.iter().position(|slot| {
            slot.msg_type == msg_type
                && slot.route_id == route_id
                && slot.seq_base >= seq_base
                && slot.parts.iter().filter(|p| p.is_some()).count() >= slot.total as usize
        })?;
        let slot = self.slots.remove(pos);
        let mut payload = Vec::new();
        for part in slot.parts.into_iter().flatten() {
            payload.extend_from_slice(&part);
        }
        Some(ReassembledMessage {
            msg_type: slot.msg_type,
            route_id: slot.route_id,
            seq_base: slot.seq_base,
            channel_mask: slot.channel_mask,
            payload,
            audit_byte: None,
        })
    }

    fn audit_message(
        &self,
        message: &ReassembledMessage,
        routes: &[RoutePlan],
        dict: &Dictionary,
    ) -> Option<u8> {
        if !self.deferred.is_ready_for(message.route_id) {
            return None;
        }
        if message.payload.len() < 8 {
            return None;
        }
        let route = routes.iter().find(|r| r.route_id == message.route_id);
        let route_pressure = route
            .map(crate::route::release_pressure)
            .unwrap_or_else(|| dict.route_bias(message.route_id));
        let catalog_bias = catalog::routes::find_route(message.route_id)
            .map(|p| p.risk_seed ^ p.control_zone as u32)
            .unwrap_or(route_pressure);
        let controlled =
            message.payload[0] & FLAG_CONTROLLED_RELEASE != 0 || message.msg_type & 0x70 == 0x40;
        let looks_like_handoff = controlled
            && message.channel_mask.count_ones() >= 1
            && checksum::rolling32(catalog_bias, &message.payload).count_ones() > 11;
        if looks_like_handoff {
            let pivot = (route_pressure ^ catalog_bias ^ message.seq_base as u32) as usize;
            Some(self.deferred.sample(pivot))
        } else {
            None
        }
    }
}

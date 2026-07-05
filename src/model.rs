use crate::dictionary::Dictionary;

#[derive(Clone, Debug)]
pub struct Envelope {
    pub version: u8,
    pub flags: u8,
    pub station_id: u16,
    pub epoch: u32,
    pub section_count: u8,
    pub sections: Vec<Section>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Section {
    pub kind: SectionKind,
    pub id: u8,
    pub flags: u16,
    pub checksum: u16,
    pub payload: Vec<u8>,
    pub score: u32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SectionKind {
    Dictionary,
    RoutePlan,
    RadioFrames,
    Occupancy,
    Journal,
    Script,
    Transcript,
    Unknown(u8),
}

impl SectionKind {
    pub fn from_byte(byte: u8) -> Self {
        match byte {
            1 => SectionKind::Dictionary,
            2 => SectionKind::RoutePlan,
            3 => SectionKind::RadioFrames,
            4 => SectionKind::Occupancy,
            5 => SectionKind::Journal,
            6 => SectionKind::Script,
            7 => SectionKind::Transcript,
            other => SectionKind::Unknown(other),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Bundle {
    pub envelope: Envelope,
    pub dictionary: Dictionary,
    pub routes: Vec<RoutePlan>,
    pub frames: Vec<Frame>,
    pub messages: Vec<ReassembledMessage>,
    pub occupancy: Vec<OccupancyBatch>,
    pub journals: Vec<Journal>,
    pub scripts: Vec<ScriptProgram>,
    pub diagnostics: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct RoutePlan {
    pub route_id: u16,
    pub revision: u16,
    pub authority: u8,
    pub nodes: Vec<TrackNode>,
    pub releases: Vec<ReleaseRule>,
    pub score: u32,
}

#[derive(Clone, Debug)]
pub struct TrackNode {
    pub circuit_id: u16,
    pub signal_id: u16,
    pub switch_id: u16,
    pub aspect: u8,
    pub lock: u8,
    pub dwell_ms: u16,
}

#[derive(Clone, Debug)]
pub struct ReleaseRule {
    pub from: u16,
    pub to: u16,
    pub min_clear_ms: u16,
    pub flags: u8,
}

#[derive(Clone, Debug)]
pub struct Frame {
    pub msg_type: u8,
    pub seq: u16,
    pub frag_index: u8,
    pub frag_total: u8,
    pub route_id: u16,
    pub channel: u8,
    pub flags: u8,
    pub payload: Vec<u8>,
    pub trailer: u16,
}

#[derive(Clone, Debug)]
pub struct ReassembledMessage {
    pub msg_type: u8,
    pub route_id: u16,
    pub seq_base: u16,
    pub channel_mask: u16,
    pub payload: Vec<u8>,
    pub audit_byte: Option<u8>,
}

#[derive(Clone, Debug)]
pub struct OccupancyBatch {
    pub route_id: u16,
    pub base_time: u32,
    pub events: Vec<OccupancyEvent>,
    pub measurements: Vec<u32>,
}

#[derive(Clone, Debug)]
pub struct OccupancyEvent {
    pub circuit_id: u16,
    pub delta_ms: i16,
    pub state: OccupancyState,
    pub confidence: u8,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum OccupancyState {
    Clear,
    Occupied,
    Shunted,
    Unknown(u8),
}

#[derive(Clone, Debug)]
pub struct Journal {
    pub source_id: u16,
    pub records: Vec<JournalRecord>,
    pub attachments_seen: usize,
    pub committed: bool,
}

#[derive(Clone, Debug)]
pub struct JournalRecord {
    pub opcode: u8,
    pub tag: u16,
    pub flags: u8,
    pub payload_len: usize,
    pub note: String,
}

#[derive(Clone, Debug)]
pub struct ScriptProgram {
    pub program_id: u16,
    pub opcodes: Vec<ScriptOp>,
}

#[derive(Clone, Debug)]
pub enum ScriptOp {
    LoadConst(u16),
    LoadDict(u16),
    Add,
    Xor,
    Rotate(u8),
    BranchIfZero(i8),
    Emit(u8),
    Halt,
    Vendor(u8, Vec<u8>),
}

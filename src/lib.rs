//! JunctionTrace decodes offline rail interlocking evidence bundles.
//!
//! The crate models the kind of binary files produced by field equipment when a
//! rail operator needs to investigate a routing or signaling incident without
//! connecting the controller to a central service.

pub mod analysis;
pub mod attachment;
pub mod catalog;
pub mod checksum;
pub mod cursor;
pub mod dictionary;
pub mod envelope;
pub mod error;
pub mod frame;
pub mod journal;
pub mod model;
pub mod occupancy;
pub mod reassembler;
pub mod route;
pub mod script;
pub mod stream;
pub mod unsafe_lanes;

pub use analysis::{AnalysisFinding, AnalysisReport};
pub use envelope::BundleDecoder;
pub use error::{JunctionError, Result};
pub use model::{Bundle, Envelope, Frame, Journal, OccupancyBatch, RoutePlan, ScriptProgram};

pub fn parse_bundle(data: &[u8]) -> Result<Bundle> {
    BundleDecoder::new().parse(data)
}

pub fn decode_and_analyze_bundle(data: &[u8]) -> Result<AnalysisReport> {
    let bundle = parse_bundle(data)?;
    Ok(analysis::Analyzer::new().analyze(&bundle))
}

pub fn decode_stream(data: &[u8]) -> Result<Bundle> {
    let mut decoder = stream::StreamDecoder::new();
    decoder.feed(data)?;
    decoder.finish()
}

pub fn decode_journal_bytes(data: &[u8]) -> Result<Journal> {
    journal::parse_journal(data, &dictionary::Dictionary::new())
}

pub fn run_script_bytes(data: &[u8]) -> Result<script::ExecutionReport> {
    script::compile_and_run(data)
}

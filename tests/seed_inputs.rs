#[test]
fn bundle_seed_decodes_and_analyzes() {
    let data = include_bytes!("../fuzz/corpus/bundle_fuzzer/seed_valid_bundle.jtrc");
    let report = junctiontrace::decode_and_analyze_bundle(data).expect("bundle seed should parse");
    assert!(report.route_count >= 1);
    assert!(report.frame_count >= 1);
}

#[test]
fn stream_seed_decodes_frames() {
    let data = include_bytes!("../fuzz/corpus/stream_fuzzer/seed_frames.bin");
    let bundle = junctiontrace::decode_stream(data).expect("stream seed should parse");
    assert!(!bundle.frames.is_empty());
}

#[test]
fn journal_seed_decodes_records() {
    let data = include_bytes!("../fuzz/corpus/journal_fuzzer/seed_journal.bin");
    let journal = junctiontrace::decode_journal_bytes(data).expect("journal seed should parse");
    assert!(journal.records.len() >= 2);
}

#[test]
fn script_seed_executes() {
    let data = include_bytes!("../fuzz/corpus/script_fuzzer/seed_script.bin");
    let report = junctiontrace::run_script_bytes(data).expect("script seed should run");
    assert!(report.steps > 0);
}

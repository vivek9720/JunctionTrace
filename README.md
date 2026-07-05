# JunctionTrace

JunctionTrace is an offline Rust library for decoding rail interlocking evidence
bundles recovered from wayside controllers, dispatch laptops, and signal
maintenance recorders. A bundle contains a binary envelope, local dictionaries,
fragmented radio frames, route plans, occupancy traces, maintenance journal
transactions, and small controller scripts.

The project is intentionally dependency-free for the main crate. The fuzzing
package uses a local `libfuzzer-sys` shim so the ClusterFuzzLite build can run in
locked offline mode and link against the runner-provided libFuzzer engine.

## Protocol Sketch

Each `JTRC` bundle is decoded in stages:

1. Envelope and section table.
2. Dictionary pages for circuits, routes, signals, and maintenance vocabulary.
3. Route-plan sections with switch locks, signal aspects, and release windows.
4. Fragmented radio frames reassembled into dispatcher and wayside messages.
5. Occupancy streams using compact replay tokens and rolling windows.
6. Maintenance journals with attachment pages and rollback snapshots.
7. Controller bytecode used to replay route and signal state transitions.

The public APIs used by tools and fuzzers are:

- `parse_bundle`
- `decode_and_analyze_bundle`
- `decode_stream`
- `decode_journal_bytes`
- `run_script_bytes`

## Fuzzing

Harnesses live under `fuzz/fuzz_targets/`:

- `bundle_fuzzer`
- `stream_fuzzer`
- `journal_fuzzer`
- `script_fuzzer`

ClusterFuzzLite metadata is under `.clusterfuzzlite/`, and seeds are under
`fuzz/corpus/`, which is one of the locations listed in the Fenrir guide.

//! Generates SCALE-encoded verification vectors for this pallet's category-0 example schema:
//! `PassportRecordV1` for a head record's body, `LifecycleEventV1` for an event's body. Each
//! vector is written as its own JSON file (bytes, parsed form, a blake2-256 fingerprint of the
//! bytes, and provenance) into the directory named on the command line.
//!
//! Both schema structures are not part of this pallet's own storage types: this pallet stores
//! `body` as an opaque byte string and never parses it (see `HeadRecord` and `EventRecord` in
//! `src/lib.rs`). They are the example schema documented on the passport pallet's own
//! `pilier.dev` page and already exercised by this crate's own `tests.rs`.
//!
//! Run: cargo run --example generate_vectors -p pallet-pilier-dpp -- <output-dir>

use codec::Encode;
use sp_core::blake2_256;
use std::{env, fs, path::Path};

#[derive(Encode, Clone)]
struct CompositionLine {
    fibre: Vec<u8>,
    percentage_bps: u16,
}

#[derive(Encode, Clone)]
struct PassportRecordV1 {
    count: u32,
    composition: Vec<CompositionLine>,
    composition_source: u8,
    certificate_fingerprints: Vec<[u8; 32]>,
}

#[derive(Encode, Clone)]
struct LifecycleEventV1 {
    event_type: u8,
    occurred_at_unix_ms: u64,
    gs1_event_hash: [u8; 32],
}

const MAX_RECORD_BODY_LEN: usize = 4 * 1024;
const MAX_EVENT_LEN: usize = 128;
const RUNTIME_SPEC_VERSION: u32 = 105;

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(2 + bytes.len() * 2);
    s.push_str("0x");
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

fn fingerprint_of(bytes: &[u8]) -> String {
    hex(&blake2_256(bytes))
}

fn fibre_str(fibre: &[u8]) -> String {
    String::from_utf8_lossy(fibre).into_owned()
}

fn composition_json(lines: &[CompositionLine]) -> String {
    let items: Vec<String> = lines
        .iter()
        .map(|l| {
            format!(
                "{{ \"fibre\": \"{}\", \"percentage_bps\": {} }}",
                fibre_str(&l.fibre),
                l.percentage_bps
            )
        })
        .collect();
    format!("[{}]", items.join(", "))
}

fn fingerprints_json(fps: &[[u8; 32]]) -> String {
    let items: Vec<String> = fps.iter().map(|f| format!("\"{}\"", hex(f))).collect();
    format!("[{}]", items.join(", "))
}

fn write_json(dir: &Path, file_name: &str, json: &str) {
    fs::write(dir.join(file_name), json).unwrap_or_else(|e| panic!("write {file_name}: {e}"));
    println!("wrote {}", dir.join(file_name).display());
}

fn write_record_vector(dir: &Path, name: &str, record: &PassportRecordV1, notes: &str) {
    let bytes = record.encode();
    let json = format!(
        "{{\n  \
         \"name\": \"{name}\",\n  \
         \"schema\": \"PassportRecordV1 (category 0, version 1)\",\n  \
         \"bytes\": \"{bytes_hex}\",\n  \
         \"length_bytes\": {len},\n  \
         \"parsed\": {{\n    \
             \"count\": {count},\n    \
             \"composition\": {composition},\n    \
             \"composition_source\": {composition_source},\n    \
             \"certificate_fingerprints\": {fingerprints}\n  \
         }},\n  \
         \"blake2_256_of_bytes\": \"{content_fingerprint}\",\n  \
         \"provenance\": {{\n    \
             \"generated_by\": \"cargo run --example generate_vectors -p pallet-pilier-dpp\",\n    \
             \"source_commit\": \"{commit}\",\n    \
             \"runtime_spec_version\": {spec},\n    \
             \"notes\": \"{notes}\"\n  \
         }}\n\
         }}\n",
        name = name,
        bytes_hex = hex(&bytes),
        len = bytes.len(),
        count = record.count,
        composition = composition_json(&record.composition),
        composition_source = record.composition_source,
        fingerprints = fingerprints_json(&record.certificate_fingerprints),
        content_fingerprint = fingerprint_of(&bytes),
        commit = option_env!("GENERATOR_SOURCE_COMMIT").unwrap_or("unknown"),
        spec = RUNTIME_SPEC_VERSION,
        notes = notes,
    );
    write_json(dir, &format!("record-{name}.json"), &json);
}

fn write_event_vector_decodable(dir: &Path, name: &str, event: &LifecycleEventV1, notes: &str) {
    let bytes = event.encode();
    let json = format!(
        "{{\n  \
         \"name\": \"{name}\",\n  \
         \"schema\": \"LifecycleEventV1 (category 0, version 1)\",\n  \
         \"bytes\": \"{bytes_hex}\",\n  \
         \"length_bytes\": {len},\n  \
         \"parsed\": {{\n    \
             \"event_type\": {event_type},\n    \
             \"occurred_at_unix_ms\": {occurred_at},\n    \
             \"gs1_event_hash\": \"{gs1_hash}\"\n  \
         }},\n  \
         \"blake2_256_of_bytes\": \"{content_fingerprint}\",\n  \
         \"provenance\": {{\n    \
             \"generated_by\": \"cargo run --example generate_vectors -p pallet-pilier-dpp\",\n    \
             \"source_commit\": \"{commit}\",\n    \
             \"runtime_spec_version\": {spec},\n    \
             \"notes\": \"{notes}\"\n  \
         }}\n\
         }}\n",
        name = name,
        bytes_hex = hex(&bytes),
        len = bytes.len(),
        event_type = event.event_type,
        occurred_at = event.occurred_at_unix_ms,
        gs1_hash = hex(&event.gs1_event_hash),
        content_fingerprint = fingerprint_of(&bytes),
        commit = option_env!("GENERATOR_SOURCE_COMMIT").unwrap_or("unknown"),
        spec = RUNTIME_SPEC_VERSION,
        notes = notes,
    );
    write_json(dir, &format!("event-{name}.json"), &json);
}

fn write_event_vector_raw(dir: &Path, name: &str, bytes: &[u8], notes: &str) {
    let json = format!(
        "{{\n  \
         \"name\": \"{name}\",\n  \
         \"schema\": null,\n  \
         \"bytes\": \"{bytes_hex}\",\n  \
         \"length_bytes\": {len},\n  \
         \"parsed\": null,\n  \
         \"blake2_256_of_bytes\": \"{content_fingerprint}\",\n  \
         \"provenance\": {{\n    \
             \"generated_by\": \"cargo run --example generate_vectors -p pallet-pilier-dpp\",\n    \
             \"source_commit\": \"{commit}\",\n    \
             \"runtime_spec_version\": {spec},\n    \
             \"notes\": \"{notes}\"\n  \
         }}\n\
         }}\n",
        name = name,
        bytes_hex = hex(bytes),
        len = bytes.len(),
        content_fingerprint = fingerprint_of(bytes),
        commit = option_env!("GENERATOR_SOURCE_COMMIT").unwrap_or("unknown"),
        spec = RUNTIME_SPEC_VERSION,
        notes = notes,
    );
    write_json(dir, &format!("event-{name}.json"), &json);
}

/// Pads the first composition line's fibre name by `pad` extra bytes and appends `n` synthetic
/// filler certificate fingerprints, then reports the resulting encoded length.
fn ceiling_candidate(composition: &[CompositionLine], pad: usize, n: u32) -> PassportRecordV1 {
    let mut composition = composition.to_vec();
    composition[0].fibre.extend(std::iter::repeat_n(b'x', pad));
    let certificate_fingerprints: Vec<[u8; 32]> = (0..n)
        .map(|i| {
            let mut input = b"passport-reader ceiling vector filler ".to_vec();
            input.extend_from_slice(&i.to_le_bytes());
            blake2_256(&input)
        })
        .collect();
    PassportRecordV1 {
        count: 999_999,
        composition,
        composition_source: 0,
        certificate_fingerprints,
    }
}

/// Finds a combination of composition-padding bytes and filler certificate fingerprints whose
/// total encoded length is exactly `MAX_RECORD_BODY_LEN`, by direct measurement rather than
/// hand-derived arithmetic (the compact-length prefix's own width changes at 64 elements, which
/// shifts where an exact match falls). Panics if none is found in the searched range, so a future
/// change to the base composition or to the codec's own overhead is caught immediately instead of
/// silently producing a vector that is not at the ceiling.
fn build_ceiling_record(composition: Vec<CompositionLine>) -> PassportRecordV1 {
    for pad in 0..32usize {
        for n in 0..200u32 {
            let candidate = ceiling_candidate(&composition, pad, n);
            if candidate.encode().len() == MAX_RECORD_BODY_LEN {
                return candidate;
            }
        }
    }
    panic!("no (pad, n) combination reaches exactly {MAX_RECORD_BODY_LEN} bytes; widen the search");
}

fn main() {
    let out_dir = env::args()
        .nth(1)
        .expect("usage: generate_vectors <output-dir>");
    let out_dir = Path::new(&out_dir);
    fs::create_dir_all(out_dir).expect("create output dir");

    // --- Records ---

    write_record_vector(
        out_dir,
        "minimal",
        &PassportRecordV1 {
            count: 0,
            composition: vec![],
            composition_source: 0,
            certificate_fingerprints: vec![],
        },
        "Synthetic: every variable-length field empty. The smallest legal encoding of this schema. Never published on chain.",
    );

    // The real, permanently published worked example on the passport pallet's own pilier.dev
    // page and on the live testnet, at key (DOCS-EXAMPLE-0001, 00000000000017). Reproduced here
    // field-for-field from that page; the two certificate fingerprints are copied as bytes
    // (not recomputed from their private source text) because the source strings that produced
    // them are not themselves public.
    let typical_record = PassportRecordV1 {
        count: 500,
        composition: vec![
            CompositionLine {
                fibre: b"cotton".to_vec(),
                percentage_bps: 6000,
            },
            CompositionLine {
                fibre: b"polyester".to_vec(),
                percentage_bps: 2500,
            },
            CompositionLine {
                fibre: b"elastane".to_vec(),
                percentage_bps: 500,
            },
            CompositionLine {
                fibre: b"viscose".to_vec(),
                percentage_bps: 1000,
            },
        ],
        composition_source: 0,
        certificate_fingerprints: vec![
            hex_literal_32("1e28ba7ac941ad264440690254d7524f0c0259cfd4c11b1711c3d38b07249d3a"),
            hex_literal_32("6fffd7cd93a5a34a08125764ac5bcf64e3c4ab9c1d3e7ede43d53e5ab65d60a8"),
        ],
    };
    let typical_bytes = typical_record.encode();
    assert_eq!(
        hex(&typical_bytes),
        "0xf40100001018636f74746f6e701724706f6c796573746572c40920656c617374616e65f4011c766973636f7365e80300081e28ba7ac941ad264440690254d7524f0c0259cfd4c11b1711c3d38b07249d3a6fffd7cd93a5a34a08125764ac5bcf64e3c4ab9c1d3e7ede43d53e5ab65d60a8",
        "reconstructed typical record must match the already-published live record byte for byte"
    );
    write_record_vector(
        out_dir,
        "typical",
        &typical_record,
        "The real, permanently published worked example on the passport pallet's own pilier.dev documentation page and on the live testnet (wss://rpc.pilier.dev), at key (DOCS-EXAMPLE-0001, 00000000000017). Byte-for-byte identical to the on-chain record; reconstruction checked against it before this file was written.",
    );

    let ceiling_record = build_ceiling_record(vec![
        CompositionLine {
            fibre: b"cotton".to_vec(),
            percentage_bps: 2500,
        },
        CompositionLine {
            fibre: b"polyester".to_vec(),
            percentage_bps: 2500,
        },
        CompositionLine {
            fibre: b"elastane".to_vec(),
            percentage_bps: 2500,
        },
        CompositionLine {
            fibre: b"viscose".to_vec(),
            percentage_bps: 2500,
        },
    ]);
    assert_eq!(ceiling_record.encode().len(), MAX_RECORD_BODY_LEN);
    write_record_vector(
        out_dir,
        "ceiling",
        &ceiling_record,
        "Synthetic: padded with filler certificate fingerprints (not real certificates) solely to reach MaxRecordBodyLen (4096 bytes) exactly. Never published on chain.",
    );

    // --- Events ---

    // The same live testnet passport's own two lifecycle events, in order.
    write_event_vector_decodable(
        out_dir,
        "minimal",
        &LifecycleEventV1 {
            event_type: 0,
            occurred_at_unix_ms: 1_795_000_000_000,
            gs1_event_hash: hex_literal_32(
                "632c1cd2d012f72ff418a5acfa66bfa95de1345b630fd4232d6c05b84669f94b",
            ),
        },
        "The real first lifecycle event (index 0, event_type 0 = created) of the passport pallet's own live testnet worked example, at key (DOCS-EXAMPLE-0001, 00000000000017). Named 'minimal' because every LifecycleEventV1 encodes to the same fixed 41 bytes regardless of field values, so this and 'typical' differ only in content, not in shape or length.",
    );
    write_event_vector_decodable(
        out_dir,
        "typical",
        &LifecycleEventV1 {
            event_type: 1,
            occurred_at_unix_ms: 1_795_600_000_000,
            gs1_event_hash: hex_literal_32(
                "5d521b62b039d6c3d78d8ca6c71a351b88ea00edc0daaa9e801894d7c956928f",
            ),
        },
        "The real second lifecycle event (index 1, event_type 1 = shipped) of the same live testnet worked example.",
    );

    let ceiling_event_bytes = vec![0xEEu8; MAX_EVENT_LEN];
    write_event_vector_raw(
        out_dir,
        "ceiling",
        &ceiling_event_bytes,
        "Synthetic raw fill at exactly MaxEventLen (128 bytes). Deliberately NOT a decodable LifecycleEventV1: that schema is fixed at 41 bytes for any field values, so a ceiling-sized instance of it does not exist. This vector demonstrates only the pallet's own raw length ceiling on an event body, the same way this pallet's own tests.rs exercises it.",
    );

    println!("done");
}

fn hex_literal_32(hex_str: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = u8::from_str_radix(&hex_str[i * 2..i * 2 + 2], 16).expect("valid hex");
    }
    out
}

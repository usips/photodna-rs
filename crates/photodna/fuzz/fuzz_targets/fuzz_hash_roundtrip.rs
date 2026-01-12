//! Fuzz target for hash roundtrip (slice -> hex -> hash -> slice).
//!
//! This target ensures that hashes maintain data integrity through
//! all conversion operations.

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use photodna::Hash;

/// Structured input for roundtrip testing
#[derive(Debug, Arbitrary)]
struct RoundtripInput {
    /// Raw bytes (will be truncated to HASH_SIZE)
    bytes: Vec<u8>,
    /// Whether to use uppercase hex
    uppercase: bool,
}

fuzz_target!(|input: RoundtripInput| {
    // Limit bytes to valid hash size
    let bytes: Vec<u8> = input.bytes.into_iter().take(photodna::HASH_SIZE).collect();

    if bytes.is_empty() {
        return;
    }

    // Create hash from bytes
    let hash = match Hash::from_slice(&bytes) {
        Some(h) => h,
        None => return,
    };

    // Convert to hex (upper or lower)
    let hex = if input.uppercase {
        hash.to_hex_upper()
    } else {
        hash.to_hex()
    };

    // Convert back to hash
    let roundtrip = Hash::from_hex(&hex).expect("Roundtrip hex parsing failed");

    // Verify data integrity
    assert_eq!(
        roundtrip.as_bytes(),
        hash.as_bytes(),
        "Hash data changed during roundtrip"
    );
    assert_eq!(
        roundtrip.len(),
        hash.len(),
        "Hash length changed during roundtrip"
    );

    // Verify equality
    assert_eq!(hash, roundtrip, "Hash equality failed after roundtrip");
});

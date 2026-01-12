//! Fuzz target for `Hash::from_hex` parsing.
//!
//! This target tests that `Hash::from_hex` correctly handles all possible
//! input strings without panicking or causing undefined behavior.

#![no_main]

use libfuzzer_sys::fuzz_target;
use photodna::Hash;

fuzz_target!(|data: &str| {
    // Attempt to parse the input as a hex string
    // This should never panic, only return None for invalid input
    let result = Hash::from_hex(data);

    // If parsing succeeded, verify the hash is valid
    if let Some(hash) = result {
        // Verify length is consistent
        assert!(hash.len() <= photodna::HASH_SIZE);

        // Verify roundtrip: hex -> hash -> hex -> hash
        let hex = hash.to_hex();
        let roundtrip = Hash::from_hex(&hex);
        assert!(roundtrip.is_some(), "Roundtrip failed for valid hash");
        assert_eq!(roundtrip.unwrap().as_bytes(), hash.as_bytes());
    }
});

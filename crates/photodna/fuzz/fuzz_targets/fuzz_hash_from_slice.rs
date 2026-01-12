//! Fuzz target for `Hash::from_slice` parsing.
//!
//! This target tests that `Hash::from_slice` correctly handles all possible
//! byte sequences without panicking or causing undefined behavior.

#![no_main]

use libfuzzer_sys::fuzz_target;
use photodna::Hash;

fuzz_target!(|data: &[u8]| {
    // Attempt to create a hash from arbitrary bytes
    // This should never panic, only return None for input > HASH_SIZE
    let result = Hash::from_slice(data);

    if let Some(hash) = result {
        // Verify length matches input
        assert_eq!(hash.len(), data.len());

        // Verify data matches
        assert_eq!(hash.as_bytes(), data);

        // Verify we can convert to hex and back
        let hex = hash.to_hex();
        assert_eq!(hex.len(), data.len() * 2);
    } else {
        // If None, the input was too large
        assert!(data.len() > photodna::HASH_SIZE);
    }
});

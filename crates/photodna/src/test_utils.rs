//! # PhotoDNA Test Utilities
//!
//! This module provides test fixtures and utilities for testing code
//! that integrates with PhotoDNA. It is only available when the
//! `test-utils` feature is enabled.
//!
//! ## Purpose
//!
//! PhotoDNA integration testing can be challenging because:
//!
//! 1. The PhotoDNA SDK may not be available in CI environments
//! 2. Creating real hashes requires valid images
//! 3. Hash comparison logic needs deterministic test data
//!
//! This module provides mock hashes and utilities that allow testing
//! PhotoDNA integration code without the actual SDK.
//!
//! ## Usage
//!
//! Enable the `test-utils` feature in your `Cargo.toml`:
//!
//! ```toml
//! [dev-dependencies]
//! photodna = { version = "1.5", features = ["test-utils"] }
//! ```
//!
//! Then use the fixtures in your tests:
//!
//! ```rust,ignore
//! use photodna::test_utils::fixtures;
//!
//! #[test]
//! fn test_hash_comparison() {
//!     let hash1 = fixtures::sample_hash_a();
//!     let hash2 = fixtures::sample_hash_a_variant();
//!     
//!     // These should be "similar" (same image with modifications)
//!     assert!(hash1.distance(&hash2) < 0.1);
//! }
//! ```
//!
//! ## Available Utilities
//!
//! - [`MockHashBuilder`]: Builder for creating custom test hashes
//! - [`fixtures`]: Pre-built sample hashes for common test scenarios
//! - [`generators`]: Proptest strategies for property-based testing
//!
//! ## Important Notes
//!
//! - Mock hashes do NOT represent real PhotoDNA output
//! - Do not use these utilities to bypass PhotoDNA in production
//! - These are for testing integration code, not the PhotoDNA algorithm

use crate::{Hash, HASH_SIZE};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// A builder for creating mock PhotoDNA hashes.
///
/// This provides a fluent API for constructing hashes with specific
/// characteristics for testing purposes.
///
/// # Examples
///
/// ```rust
/// use photodna::test_utils::MockHashBuilder;
///
/// let hash = MockHashBuilder::new()
///     .with_seed(12345)
///     .with_pattern(0xAB)
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct MockHashBuilder {
    /// Seed for random generation (for reproducibility)
    seed: Option<u64>,
    /// Fill pattern for the hash bytes
    pattern: Option<u8>,
    /// Custom bytes to use
    custom_bytes: Option<Vec<u8>>,
    /// Length of the hash (defaults to HASH_SIZE)
    length: usize,
}

impl Default for MockHashBuilder {
    fn default() -> Self {
        Self {
            seed: None,
            pattern: None,
            custom_bytes: None,
            length: HASH_SIZE,
        }
    }
}

impl MockHashBuilder {
    /// Creates a new mock hash builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets a random seed for reproducible hash generation.
    ///
    /// Using the same seed will always produce the same hash.
    #[must_use]
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Fills the hash with a repeating byte pattern.
    #[must_use]
    pub fn with_pattern(mut self, pattern: u8) -> Self {
        self.pattern = Some(pattern);
        self
    }

    /// Uses custom bytes for the hash content.
    ///
    /// If the provided bytes are shorter than HASH_SIZE, they will be
    /// repeated to fill the hash.
    #[must_use]
    pub fn with_bytes(mut self, bytes: Vec<u8>) -> Self {
        self.custom_bytes = Some(bytes);
        self
    }

    /// Sets a custom length for the hash.
    ///
    /// This can be used to test handling of partial hashes.
    #[must_use]
    pub fn with_length(mut self, length: usize) -> Self {
        self.length = length.min(HASH_SIZE);
        self
    }

    /// Builds the mock hash.
    ///
    /// Returns a `Hash` with the configured properties.
    pub fn build(self) -> Hash {
        let mut bytes = [0u8; HASH_SIZE];
        let len = self.length;

        if let Some(custom) = self.custom_bytes {
            // Repeat custom bytes to fill the hash
            for (i, b) in bytes[..len].iter_mut().enumerate() {
                *b = custom[i % custom.len()];
            }
        } else if let Some(pattern) = self.pattern {
            // Fill with pattern
            bytes[..len].fill(pattern);
        } else if let Some(seed) = self.seed {
            // Generate random bytes from seed
            let mut rng = StdRng::seed_from_u64(seed);
            rng.fill(&mut bytes[..len]);
        } else {
            // Generate random bytes
            let mut rng = rand::thread_rng();
            rng.fill(&mut bytes[..len]);
        }

        Hash::from_slice(&bytes[..len]).expect("valid hash length")
    }

    /// Creates a variant of a hash with small random changes.
    ///
    /// This simulates what might happen when the same image is
    /// slightly modified (resized, compressed, etc.)
    ///
    /// # Arguments
    ///
    /// * `base` - The base hash to create a variant of
    /// * `variance` - Amount of variance (0.0-1.0), where higher means more different
    pub fn variant(base: &Hash, variance: f64) -> Hash {
        let mut rng = rand::thread_rng();
        let mut bytes = [0u8; HASH_SIZE];
        bytes.copy_from_slice(&base.as_bytes()[..base.len().min(HASH_SIZE)]);

        let change_probability = variance.clamp(0.0, 1.0);
        let len = base.len().min(HASH_SIZE);

        for b in bytes[..len].iter_mut() {
            if rng.gen::<f64>() < change_probability {
                // Apply small random change
                let delta: i16 = rng.gen_range(-20..=20);
                *b = (*b as i16).saturating_add(delta).clamp(0, 255) as u8;
            }
        }

        Hash::from_slice(&bytes[..len]).expect("valid hash length")
    }
}

/// Pre-built sample hashes for common test scenarios.
///
/// These fixtures provide consistent, reproducible hashes for testing
/// without needing to compute real PhotoDNA hashes.
pub mod fixtures {
    use super::*;

    /// Returns a sample hash representing "Image A".
    ///
    /// This hash is deterministic (same every time) and can be used
    /// as a reference point in tests.
    pub fn sample_hash_a() -> Hash {
        MockHashBuilder::new()
            .with_seed(0xDEADBEEF_CAFEBABE)
            .build()
    }

    /// Returns a variant of sample hash A.
    ///
    /// This represents the same image with minor modifications
    /// (resize, slight compression, etc.)
    pub fn sample_hash_a_variant() -> Hash {
        let base = sample_hash_a();
        MockHashBuilder::variant(&base, 0.05)
    }

    /// Returns a sample hash representing "Image B".
    ///
    /// This is completely different from Image A.
    pub fn sample_hash_b() -> Hash {
        MockHashBuilder::new()
            .with_seed(0x12345678_9ABCDEF0)
            .build()
    }

    /// Returns an empty hash (all zeros).
    ///
    /// Useful for testing error handling or edge cases.
    pub fn empty_hash() -> Hash {
        MockHashBuilder::new().with_pattern(0x00).build()
    }

    /// Returns a "full" hash (all 0xFF).
    ///
    /// Useful for boundary testing.
    pub fn full_hash() -> Hash {
        MockHashBuilder::new().with_pattern(0xFF).build()
    }

    /// Returns a hash with an alternating bit pattern.
    ///
    /// Useful for testing bit-level operations.
    pub fn alternating_hash() -> Hash {
        MockHashBuilder::new().with_bytes(vec![0xAA, 0x55]).build()
    }

    /// Returns a partial hash (less than HASH_SIZE).
    ///
    /// Useful for testing handling of truncated or partial hashes.
    pub fn partial_hash() -> Hash {
        MockHashBuilder::new()
            .with_seed(0xBEEF1234)
            .with_length(100)
            .build()
    }

    /// Returns a sequence of hashes with increasing "distance".
    ///
    /// Useful for testing distance calculation or threshold logic.
    /// Returns: (base_hash, vec![slight_variant, moderate_variant, very_different])
    pub fn distance_sequence() -> (Hash, Vec<Hash>) {
        let base = sample_hash_a();
        let variants = vec![
            MockHashBuilder::variant(&base, 0.02), // Slight variant
            MockHashBuilder::variant(&base, 0.15), // Moderate variant
            MockHashBuilder::variant(&base, 0.50), // Very different
        ];
        (base, variants)
    }
}

/// Generators for property-based testing with proptest.
///
/// These provide strategies for generating random hashes with
/// specific properties.
///
/// # Examples
///
/// ```rust,ignore
/// use proptest::prelude::*;
/// use photodna::test_utils::generators;
///
/// proptest! {
///     #[test]
///     fn test_hash_roundtrip(hash in generators::any_hash()) {
///         let hex = hash.to_hex();
///         let parsed = Hash::from_hex(&hex).unwrap();
///         assert_eq!(hash, parsed);
///     }
/// }
/// ```
#[cfg(feature = "test-utils")]
pub mod generators {
    use super::*;
    use std::ops::Range;

    /// Generates a random hash with full length.
    pub fn random_hash() -> Hash {
        MockHashBuilder::new().build()
    }

    /// Generates a hash with the given seed.
    pub fn seeded_hash(seed: u64) -> Hash {
        MockHashBuilder::new().with_seed(seed).build()
    }

    /// Generates a hash with length in the given range.
    pub fn hash_with_length_range(range: Range<usize>) -> Hash {
        let mut rng = rand::thread_rng();
        let length = rng.gen_range(range);
        MockHashBuilder::new()
            .with_length(length.min(HASH_SIZE))
            .build()
    }

    /// Generates a pair of similar hashes (representing the same image).
    pub fn similar_hash_pair(variance: f64) -> (Hash, Hash) {
        let base = random_hash();
        let variant = MockHashBuilder::variant(&base, variance);
        (base, variant)
    }

    /// Generates a pair of different hashes (representing different images).
    pub fn different_hash_pair() -> (Hash, Hash) {
        let mut rng = rand::thread_rng();
        let hash1 = seeded_hash(rng.gen());
        let hash2 = seeded_hash(rng.gen());
        (hash1, hash2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_with_seed_is_deterministic() {
        let hash1 = MockHashBuilder::new().with_seed(12345).build();
        let hash2 = MockHashBuilder::new().with_seed(12345).build();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_builder_different_seeds_different_hashes() {
        let hash1 = MockHashBuilder::new().with_seed(1).build();
        let hash2 = MockHashBuilder::new().with_seed(2).build();
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_builder_with_pattern() {
        let hash = MockHashBuilder::new().with_pattern(0xAB).build();
        assert!(hash.as_bytes().iter().all(|&b| b == 0xAB));
    }

    #[test]
    fn test_builder_with_custom_bytes() {
        let hash = MockHashBuilder::new()
            .with_bytes(vec![1, 2, 3])
            .with_length(9)
            .build();
        assert_eq!(&hash.as_bytes()[..9], &[1, 2, 3, 1, 2, 3, 1, 2, 3]);
    }

    #[test]
    fn test_builder_with_length() {
        let hash = MockHashBuilder::new()
            .with_seed(42)
            .with_length(100)
            .build();
        assert_eq!(hash.len(), 100);
    }

    #[test]
    fn test_variant_is_similar() {
        let base = fixtures::sample_hash_a();
        let variant = MockHashBuilder::variant(&base, 0.05);

        // Most bytes should be the same or very close
        let differences: usize = base
            .as_bytes()
            .iter()
            .zip(variant.as_bytes())
            .map(|(a, b)| if a != b { 1 } else { 0 })
            .sum();

        // With 5% variance, expect ~5% of bytes to differ
        let max_expected_diffs = (base.len() as f64 * 0.15) as usize;
        assert!(
            differences <= max_expected_diffs,
            "Too many differences: {} > {}",
            differences,
            max_expected_diffs
        );
    }

    #[test]
    fn test_fixtures_are_consistent() {
        let a1 = fixtures::sample_hash_a();
        let a2 = fixtures::sample_hash_a();
        assert_eq!(a1, a2);

        let b = fixtures::sample_hash_b();
        assert_ne!(a1, b);
    }

    #[test]
    fn test_empty_hash() {
        let hash = fixtures::empty_hash();
        assert!(hash.as_bytes().iter().all(|&b| b == 0));
    }

    #[test]
    fn test_full_hash() {
        let hash = fixtures::full_hash();
        assert!(hash.as_bytes().iter().all(|&b| b == 0xFF));
    }

    #[test]
    fn test_distance_sequence_ordering() {
        let (base, variants) = fixtures::distance_sequence();

        // Each variant should be more different than the previous
        // We measure this by counting byte differences
        let mut last_diff = 0usize;
        for variant in &variants {
            let diff: usize = base
                .as_bytes()
                .iter()
                .zip(variant.as_bytes())
                .map(|(a, b)| (*a as i16 - *b as i16).unsigned_abs() as usize)
                .sum();
            assert!(
                diff >= last_diff,
                "Variants should have increasing difference"
            );
            last_diff = diff;
        }
    }
}

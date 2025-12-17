//! PhotoDNA hash types and operations.
//!
//! This module provides the [`Hash`] type, a fixed-size container for
//! PhotoDNA perceptual hashes with zero-copy semantics.

use std::fmt;

/// Size of PhotoDNA Edge V2 hash in bytes (binary format).
///
/// This is the standard hash size for all PhotoDNA Edge V2 hashes.
pub const HASH_SIZE: usize = photodna_sys::PHOTODNA_HASH_SIZE_EDGE_V2;

/// Maximum possible hash buffer size.
///
/// Use this when you need to support any hash format, including Base64.
pub const HASH_SIZE_MAX: usize = photodna_sys::PHOTODNA_HASH_SIZE_MAX;

/// A PhotoDNA perceptual hash.
///
/// This type wraps a fixed-size byte array containing the raw hash bytes.
/// It is designed for high-performance use cases:
///
/// - **Zero-copy**: The hash is stored inline on the stack (no heap allocation).
/// - **Copy-safe**: Implements `Copy` for trivial duplication.
/// - **Hashable**: Can be used as a key in hash maps and sets.
///
/// # Size
///
/// The hash is 924 bytes (Edge V2 binary format). For Base64-encoded hashes,
/// use the raw byte methods and encode/decode as needed.
///
/// # Examples
///
/// ```rust
/// use photodna::Hash;
///
/// // Create an empty hash (all zeros)
/// let hash = Hash::default();
/// assert!(hash.is_empty());
///
/// // Access raw bytes
/// let bytes: &[u8] = hash.as_bytes();
/// assert_eq!(bytes.len(), photodna::HASH_SIZE);
///
/// // Format as hex string
/// let hex = hash.to_hex();
/// ```
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Hash {
    /// The raw hash bytes.
    bytes: [u8; HASH_SIZE],
    /// The actual length of valid hash data (may be less than HASH_SIZE).
    len: usize,
}

impl Hash {
    /// Creates a new hash from raw bytes.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The raw hash bytes. Must be exactly [`HASH_SIZE`] bytes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use photodna::{Hash, HASH_SIZE};
    ///
    /// let data = [0u8; HASH_SIZE];
    /// let hash = Hash::new(data);
    /// ```
    #[inline]
    pub const fn new(bytes: [u8; HASH_SIZE]) -> Self {
        Self {
            bytes,
            len: HASH_SIZE,
        }
    }

    /// Creates a hash from a slice, copying the bytes.
    ///
    /// # Arguments
    ///
    /// * `slice` - A byte slice containing hash data. Must not exceed [`HASH_SIZE`] bytes.
    ///
    /// # Returns
    ///
    /// Returns `Some(Hash)` if the slice length is valid, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use photodna::Hash;
    ///
    /// let data = [0xAB; 100];
    /// let hash = Hash::from_slice(&data).unwrap();
    /// assert_eq!(hash.len(), 100);
    /// ```
    pub fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() > HASH_SIZE {
            return None;
        }

        let mut bytes = [0u8; HASH_SIZE];
        bytes[..slice.len()].copy_from_slice(slice);

        Some(Self {
            bytes,
            len: slice.len(),
        })
    }

    /// Returns the hash bytes as a slice.
    ///
    /// The returned slice contains only the valid hash bytes (up to `len()`).
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    /// Returns the full hash buffer as a fixed-size array reference.
    ///
    /// This includes any padding bytes if the hash is shorter than [`HASH_SIZE`].
    #[inline]
    pub const fn as_array(&self) -> &[u8; HASH_SIZE] {
        &self.bytes
    }

    /// Returns the length of valid hash bytes.
    #[inline]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if all hash bytes are zero.
    ///
    /// An empty hash typically indicates that no hash was computed.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.bytes[..self.len].iter().all(|&b| b == 0)
    }

    /// Formats the hash as a lowercase hexadecimal string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use photodna::Hash;
    ///
    /// let data = [0xAB; 4];
    /// let hash = Hash::from_slice(&data).unwrap();
    /// assert_eq!(&hash.to_hex()[..8], "abababab");
    /// ```
    pub fn to_hex(&self) -> String {
        let mut hex = String::with_capacity(self.len * 2);
        for byte in &self.bytes[..self.len] {
            use std::fmt::Write;
            let _ = write!(hex, "{:02x}", byte);
        }
        hex
    }

    /// Formats the hash as an uppercase hexadecimal string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use photodna::Hash;
    ///
    /// let data = [0xAB; 4];
    /// let hash = Hash::from_slice(&data).unwrap();
    /// assert_eq!(&hash.to_hex_upper()[..8], "ABABABAB");
    /// ```
    pub fn to_hex_upper(&self) -> String {
        let mut hex = String::with_capacity(self.len * 2);
        for byte in &self.bytes[..self.len] {
            use std::fmt::Write;
            let _ = write!(hex, "{:02X}", byte);
        }
        hex
    }

    /// Parses a hash from a hexadecimal string.
    ///
    /// # Arguments
    ///
    /// * `hex` - A hexadecimal string (case-insensitive).
    ///
    /// # Returns
    ///
    /// Returns `Some(Hash)` if parsing succeeds, `None` if the string
    /// contains invalid characters or has an invalid length.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use photodna::Hash;
    ///
    /// let hash = Hash::from_hex("abcdef01").unwrap();
    /// assert_eq!(hash.len(), 4);
    /// assert_eq!(hash.as_bytes(), &[0xAB, 0xCD, 0xEF, 0x01]);
    /// ```
    pub fn from_hex(hex: &str) -> Option<Self> {
        // Hex string must have even length
        if hex.len() % 2 != 0 {
            return None;
        }

        let byte_len = hex.len() / 2;
        if byte_len > HASH_SIZE {
            return None;
        }

        let mut bytes = [0u8; HASH_SIZE];

        for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
            let high = hex_digit_value(chunk[0])?;
            let low = hex_digit_value(chunk[1])?;
            bytes[i] = (high << 4) | low;
        }

        Some(Self {
            bytes,
            len: byte_len,
        })
    }

    /// Returns a mutable slice to the entire hash buffer.
    ///
    /// This is useful for passing to FFI functions that write directly
    /// to the buffer.
    ///
    /// # Safety
    ///
    /// After modifying the buffer via this method, you may need to update
    /// the logical length using [`set_len`](Self::set_len) if the actual
    /// data size has changed.
    #[inline]
    pub fn as_mut_bytes(&mut self) -> &mut [u8; HASH_SIZE] {
        &mut self.bytes
    }

    /// Sets the length of valid hash data.
    ///
    /// # Panics
    ///
    /// Panics if `len > HASH_SIZE`.
    #[inline]
    pub fn set_len(&mut self, len: usize) {
        assert!(len <= HASH_SIZE, "length exceeds maximum hash size");
        self.len = len;
    }

    /// Creates a new hash with uninitialized content.
    ///
    /// This is useful for performance-critical code where the hash
    /// will be immediately overwritten by FFI.
    ///
    /// # Safety
    ///
    /// The caller must ensure the hash is fully initialized before
    /// reading from it.
    #[inline]
    pub const fn zeroed() -> Self {
        Self {
            bytes: [0u8; HASH_SIZE],
            len: 0,
        }
    }
}

impl Default for Hash {
    fn default() -> Self {
        Self {
            bytes: [0u8; HASH_SIZE],
            len: HASH_SIZE,
        }
    }
}

impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Show first 16 bytes as hex for readability
        let preview_len = 16.min(self.len);
        let preview: String = self.bytes[..preview_len]
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect();

        if self.len > preview_len {
            write!(f, "Hash({}..., {} bytes)", preview, self.len)
        } else {
            write!(f, "Hash({})", preview)
        }
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl From<[u8; HASH_SIZE]> for Hash {
    fn from(bytes: [u8; HASH_SIZE]) -> Self {
        Self::new(bytes)
    }
}

impl TryFrom<&[u8]> for Hash {
    type Error = ();

    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        Self::from_slice(slice).ok_or(())
    }
}

/// Converts a hex character to its numeric value.
#[inline]
fn hex_digit_value(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_size_constant() {
        assert_eq!(HASH_SIZE, 924);
    }

    #[test]
    fn test_hash_new() {
        let data = [0xAB; HASH_SIZE];
        let hash = Hash::new(data);
        assert_eq!(hash.len(), HASH_SIZE);
        assert!(!hash.is_empty());
    }

    #[test]
    fn test_hash_default() {
        let hash = Hash::default();
        assert!(hash.is_empty());
        assert_eq!(hash.len(), HASH_SIZE);
    }

    #[test]
    fn test_hash_from_slice() {
        let data = [0xAB; 100];
        let hash = Hash::from_slice(&data).unwrap();
        assert_eq!(hash.len(), 100);
        assert_eq!(&hash.as_bytes()[..100], &data);
    }

    #[test]
    fn test_hash_from_slice_too_large() {
        let data = [0xAB; HASH_SIZE + 1];
        assert!(Hash::from_slice(&data).is_none());
    }

    #[test]
    fn test_hash_to_hex() {
        let data = [0xAB, 0xCD, 0xEF, 0x01];
        let hash = Hash::from_slice(&data).unwrap();
        assert_eq!(hash.to_hex(), "abcdef01");
        assert_eq!(hash.to_hex_upper(), "ABCDEF01");
    }

    #[test]
    fn test_hash_from_hex() {
        let hash = Hash::from_hex("abcdef01").unwrap();
        assert_eq!(hash.len(), 4);
        assert_eq!(hash.as_bytes(), &[0xAB, 0xCD, 0xEF, 0x01]);
    }

    #[test]
    fn test_hash_from_hex_invalid() {
        assert!(Hash::from_hex("abc").is_none()); // Odd length
        assert!(Hash::from_hex("ghij").is_none()); // Invalid chars
    }

    #[test]
    fn test_hash_copy() {
        let hash1 = Hash::from_slice(&[1, 2, 3, 4]).unwrap();
        let hash2 = hash1; // Copy
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_debug() {
        let hash = Hash::from_slice(&[0xAB; 20]).unwrap();
        let debug = format!("{:?}", hash);
        assert!(debug.contains("Hash("));
        assert!(debug.contains("20 bytes"));
    }

    #[test]
    fn test_hash_display() {
        let hash = Hash::from_slice(&[0xAB, 0xCD]).unwrap();
        assert_eq!(format!("{}", hash), "abcd");
    }

    #[test]
    fn test_hash_equality() {
        let hash1 = Hash::from_slice(&[1, 2, 3, 4]).unwrap();
        let hash2 = Hash::from_slice(&[1, 2, 3, 4]).unwrap();
        let hash3 = Hash::from_slice(&[1, 2, 3, 5]).unwrap();

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_hash_in_hashset() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(Hash::from_slice(&[1, 2, 3]).unwrap());
        set.insert(Hash::from_slice(&[4, 5, 6]).unwrap());

        assert!(set.contains(&Hash::from_slice(&[1, 2, 3]).unwrap()));
        assert!(!set.contains(&Hash::from_slice(&[7, 8, 9]).unwrap()));
    }
}

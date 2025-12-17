//! Error types for the PhotoDNA library.
//!
//! This module provides typed, ergonomic error handling for all PhotoDNA operations.

// Allow non-standard constant names from photodna-sys (C-style naming)
#![allow(non_upper_case_globals)]

use thiserror::Error;

/// Result type alias for PhotoDNA operations.
pub type Result<T> = std::result::Result<T, PhotoDnaError>;

/// Error type for PhotoDNA operations.
///
/// This enum provides strongly-typed errors for all failure modes
/// in the PhotoDNA library, with human-readable descriptions.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum PhotoDnaError {
    /// Failed to load or initialize the PhotoDNA library.
    #[error("failed to initialize PhotoDNA library: {0}")]
    InitializationFailed(String),

    /// An undetermined error occurred within the library.
    #[error("an undetermined error occurred (error code: -7000)")]
    Unknown,

    /// Failed to allocate memory.
    #[error("failed to allocate memory")]
    MemoryAllocationFailed,

    /// General failure within the library.
    #[error("general failure within the library")]
    LibraryFailure,

    /// System memory exception occurred.
    #[error("system memory exception occurred")]
    MemoryAccess,

    /// Hash that does not conform to PhotoDNA specifications.
    #[error("hash does not conform to PhotoDNA specifications")]
    InvalidHash,

    /// An invalid character was contained in a Base64 or Hex hash.
    #[error("invalid character in Base64 or Hex hash")]
    HashFormatInvalidCharacters,

    /// Provided image had a dimension less than 50 pixels.
    #[error("image dimension is less than 50 pixels (minimum: 50x50)")]
    ImageTooSmall,

    /// A border was not detected for the image.
    #[error("no border was detected for the image")]
    NoBorder,

    /// An invalid argument was passed to the function.
    #[error("an invalid argument was passed")]
    BadArgument,

    /// The image has few or no gradients.
    #[error("image has few or no gradients (image is flat)")]
    ImageIsFlat,

    /// Provided image had a dimension less than 50 pixels after border removal.
    #[error("image too small after border removal (minimum: 50x50)")]
    NoBorderImageTooSmall,

    /// Not a known source image format.
    #[error("not a known source image format")]
    SourceFormatUnknown,

    /// Stride should be 0, or greater than or equal to width in bytes.
    #[error("invalid stride: must be 0 or >= width in bytes")]
    InvalidStride,

    /// The sub region area is not within the boundaries of the image.
    #[error("sub region is not within image boundaries")]
    InvalidSubImage,

    /// Image data buffer is too small for the specified dimensions.
    #[error("image buffer too small: expected at least {expected} bytes, got {actual}")]
    BufferTooSmall {
        /// Expected minimum buffer size in bytes.
        expected: usize,
        /// Actual buffer size provided.
        actual: usize,
    },

    /// Invalid image dimensions (width or height is zero or negative).
    #[error("invalid image dimensions: {width}x{height}")]
    InvalidDimensions {
        /// The width provided.
        width: i32,
        /// The height provided.
        height: i32,
    },

    /// An unknown error code was returned by the library.
    #[error("unknown error code: {0}")]
    UnknownErrorCode(i32),
}

impl PhotoDnaError {
    /// Creates an error from a PhotoDNA library error code.
    ///
    /// Negative error codes indicate failures. A zero or positive value
    /// typically indicates success and should not be converted to an error.
    ///
    /// # Arguments
    ///
    /// * `code` - The error code returned by the PhotoDNA library.
    ///
    /// # Returns
    ///
    /// The corresponding `PhotoDnaError` variant for the given code.
    pub fn from_error_code(code: i32) -> Self {
        use photodna_sys::*;

        match code {
            PhotoDna_ErrorUnknown => Self::Unknown,
            PhotoDna_ErrorMemoryAllocationFailed => Self::MemoryAllocationFailed,
            PhotoDna_ErrorLibraryFailure => Self::LibraryFailure,
            PhotoDna_ErrorMemoryAccess => Self::MemoryAccess,
            PhotoDna_ErrorInvalidHash => Self::InvalidHash,
            PhotoDna_ErrorHashFormatInvalidCharacters => Self::HashFormatInvalidCharacters,
            PhotoDna_ErrorImageTooSmall => Self::ImageTooSmall,
            PhotoDna_ErrorNoBorder => Self::NoBorder,
            PhotoDna_ErrorBadArgument => Self::BadArgument,
            PhotoDna_ErrorImageIsFlat => Self::ImageIsFlat,
            PhotoDna_ErrorNoBorderImageTooSmall => Self::NoBorderImageTooSmall,
            PhotoDna_ErrorSourceFormatUnknown => Self::SourceFormatUnknown,
            PhotoDna_ErrorInvalidStride => Self::InvalidStride,
            PhotoDna_ErrorInvalidSubImage => Self::InvalidSubImage,
            _ => Self::UnknownErrorCode(code),
        }
    }

    /// Returns the original PhotoDNA error code, if applicable.
    ///
    /// Returns `None` for errors that don't map to a specific library code.
    pub fn error_code(&self) -> Option<i32> {
        use photodna_sys::*;

        match self {
            Self::Unknown => Some(PhotoDna_ErrorUnknown),
            Self::MemoryAllocationFailed => Some(PhotoDna_ErrorMemoryAllocationFailed),
            Self::LibraryFailure => Some(PhotoDna_ErrorLibraryFailure),
            Self::MemoryAccess => Some(PhotoDna_ErrorMemoryAccess),
            Self::InvalidHash => Some(PhotoDna_ErrorInvalidHash),
            Self::HashFormatInvalidCharacters => Some(PhotoDna_ErrorHashFormatInvalidCharacters),
            Self::ImageTooSmall => Some(PhotoDna_ErrorImageTooSmall),
            Self::NoBorder => Some(PhotoDna_ErrorNoBorder),
            Self::BadArgument => Some(PhotoDna_ErrorBadArgument),
            Self::ImageIsFlat => Some(PhotoDna_ErrorImageIsFlat),
            Self::NoBorderImageTooSmall => Some(PhotoDna_ErrorNoBorderImageTooSmall),
            Self::SourceFormatUnknown => Some(PhotoDna_ErrorSourceFormatUnknown),
            Self::InvalidStride => Some(PhotoDna_ErrorInvalidStride),
            Self::InvalidSubImage => Some(PhotoDna_ErrorInvalidSubImage),
            Self::UnknownErrorCode(code) => Some(*code),
            Self::InitializationFailed(_)
            | Self::BufferTooSmall { .. }
            | Self::InvalidDimensions { .. } => None,
        }
    }

    /// Returns `true` if this is a recoverable error that might succeed on retry.
    ///
    /// Memory allocation failures and library failures may be transient.
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::MemoryAllocationFailed | Self::LibraryFailure | Self::MemoryAccess
        )
    }

    /// Returns `true` if this error indicates invalid input data.
    ///
    /// These errors typically require the caller to fix their input.
    pub fn is_input_error(&self) -> bool {
        matches!(
            self,
            Self::ImageTooSmall
                | Self::ImageIsFlat
                | Self::BadArgument
                | Self::InvalidStride
                | Self::InvalidSubImage
                | Self::SourceFormatUnknown
                | Self::BufferTooSmall { .. }
                | Self::InvalidDimensions { .. }
                | Self::NoBorderImageTooSmall
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_from_code() {
        assert_eq!(
            PhotoDnaError::from_error_code(photodna_sys::PhotoDna_ErrorImageTooSmall),
            PhotoDnaError::ImageTooSmall
        );
        assert_eq!(
            PhotoDnaError::from_error_code(-9999),
            PhotoDnaError::UnknownErrorCode(-9999)
        );
    }

    #[test]
    fn test_error_code_round_trip() {
        let error = PhotoDnaError::ImageTooSmall;
        let code = error.error_code().unwrap();
        assert_eq!(PhotoDnaError::from_error_code(code), error);
    }

    #[test]
    fn test_error_display() {
        let error = PhotoDnaError::ImageTooSmall;
        assert!(error.to_string().contains("50 pixels"));
    }

    #[test]
    fn test_is_recoverable() {
        assert!(PhotoDnaError::MemoryAllocationFailed.is_recoverable());
        assert!(!PhotoDnaError::ImageTooSmall.is_recoverable());
    }

    #[test]
    fn test_is_input_error() {
        assert!(PhotoDnaError::ImageTooSmall.is_input_error());
        assert!(PhotoDnaError::InvalidStride.is_input_error());
        assert!(!PhotoDnaError::MemoryAllocationFailed.is_input_error());
    }
}

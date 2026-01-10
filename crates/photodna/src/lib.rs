//! # photodna
//!
//! Safe, high-level Rust bindings for the Microsoft PhotoDNA Edge Hash Generator.
//!
//! ## Overview
//!
//! PhotoDNA is a perceptual hashing technology that creates a compact 924-byte
//! "fingerprint" of an image. This fingerprint identifies visually similar images
//! even after modifications like resizing, cropping, color adjustment, or format
//! conversion. It is widely used in content moderation and safety systems.
//!
//! ## Key Types
//!
//! | Type | Purpose |
//! |------|---------|
//! | [`Generator`] | Loads the PhotoDNA library and computes hashes |
//! | [`Hash`][struct@Hash] | 924-byte perceptual hash with zero-copy semantics |
//! | [`PixelFormat`] | Specifies input image pixel layout (RGB, RGBA, etc.) |
//! | [`PhotoDnaError`] | Comprehensive typed error handling |
//! | [`HashOptions`] | Fine-grained control over hash computation |
//!
//! ## Features
//!
//! - **Safe API**: All unsafe FFI operations encapsulated behind safe interface
//! - **Zero-Copy Hashes**: `Hash` uses fixed-size stack array (no heap allocation)
//! - **Typed Errors**: Every failure mode has a specific error variant
//! - **Builder Pattern**: Ergonomic configuration via `GeneratorOptions` and `HashOptions`
//! - **Test Utilities**: Mock hashes and fixtures for testing (via `test-utils` feature)
//!
//! ## Requirements
//!
//! **This crate requires the proprietary Microsoft PhotoDNA SDK (not included).**
//!
//! Set `PHOTODNA_SDK_ROOT` before building:
//!
//! ```bash
//! export PHOTODNA_SDK_ROOT=/path/to/PhotoDNA.EdgeHashGeneration-1.05.001
//! cargo build
//! ```
//!
//! See [`photodna-sys`](https://docs.rs/photodna-sys) for detailed SDK setup.
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use photodna::{Generator, GeneratorOptions, Hash, Result};
//!
//! fn main() -> Result<()> {
//!     // Initialize the generator (loads PhotoDNA library)
//!     let generator = Generator::new(GeneratorOptions::default())?;
//!
//!     // Load image as raw RGB pixels (use `image` crate, etc.)
//!     let image_data: Vec<u8> = load_rgb_image("photo.jpg");
//!     let (width, height) = (640, 480);
//!
//!     // Compute the hash
//!     let hash: Hash = generator.compute_hash_rgb(&image_data, width, height)?;
//!
//!     // Store or compare the hash
//!     println!("Hash: {}", hash.to_hex());
//!     Ok(())
//! }
//! ```
//!
//! ## Image Requirements
//!
//! | Requirement | Value |
//! |-------------|-------|
//! | Minimum size | 50×50 pixels |
//! | Supported formats | RGB, RGBA, BGRA, ARGB, ABGR, CMYK, Gray8, Gray32, YCbCr, YUV420P |
//! | Content | Must have sufficient gradients (flat/solid images will fail) |
//!
//! ## Pixel Format Selection
//!
//! ```rust,ignore
//! use photodna::{Generator, GeneratorOptions, HashOptions, PixelFormat};
//!
//! let generator = Generator::new(GeneratorOptions::default())?;
//!
//! // For BGRA images (common in Windows/OpenCV)
//! let options = HashOptions::new().pixel_format(PixelFormat::Bgra);
//! let hash = generator.compute_hash(&bgra_data, 640, 480, options)?;
//! ```
//!
//! ## Border Detection
//!
//! PhotoDNA can detect and remove borders from images:
//!
//! ```rust,ignore
//! let result = generator.compute_hash_with_border_detection(&data, 640, 480, options)?;
//!
//! println!("Original hash: {}", result.primary);
//! if let Some(borderless) = result.borderless {
//!     println!("Without border: {}", borderless);
//!     println!("Content region: {:?}", result.content_region);
//! }
//! ```
//!
//! ## Thread Safety
//!
//! [`Generator`] is `Send` but not `Sync`. For concurrent access:
//!
//! - Create one `Generator` per thread (recommended), or
//! - Wrap in `Arc<Mutex<Generator>>` for shared access
//!
//! The `max_threads` option controls the underlying library's thread pool.
//! Operations exceeding this limit will block until a slot becomes available.
//!
//! ## Test Utilities
//!
//! For testing without the PhotoDNA SDK:
//!
//! ```toml
//! [dev-dependencies]
//! photodna = { version = "1.5", features = ["test-utils"] }
//! ```
//!
//! ```rust,ignore
//! use photodna::test_utils::{MockHashBuilder, fixtures};
//!
//! let hash = MockHashBuilder::new().with_seed(42).build();
//! let sample = fixtures::sample_hash_a();
//! ```
//!
//! ## Error Handling
//!
//! All operations return [`Result<T, PhotoDnaError>`]:
//!
//! ```rust,ignore
//! match generator.compute_hash_rgb(&data, width, height) {
//!     Ok(hash) => println!("Success: {}", hash),
//!     Err(PhotoDnaError::ImageTooSmall) => eprintln!("Image must be >= 50x50"),
//!     Err(PhotoDnaError::ImageIsFlat) => eprintln!("Image needs more contrast"),
//!     Err(e) => eprintln!("Error: {}", e),
//! }
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

mod error;
mod hash;

// Test utilities module (available with `test-utils` feature or in tests)
#[cfg(any(test, feature = "test-utils"))]
#[cfg_attr(docsrs, doc(cfg(feature = "test-utils")))]
pub mod test_utils;

pub use error::{PhotoDnaError, Result};
pub use hash::{Hash, HASH_SIZE, HASH_SIZE_MAX};

use photodna_sys::{self as sys, PhotoDnaOptions};
use std::ffi::c_void;

// Re-export commonly used constants from sys
pub use photodna_sys::PHOTODNA_LIBRARY_VERSION as LIBRARY_VERSION;

/// Pixel format for raw image data.
///
/// This specifies how color components are arranged in the pixel buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PixelFormat {
    /// RGB format: 3 bytes per pixel (Red, Green, Blue).
    ///
    /// This is the default format.
    #[default]
    Rgb,

    /// BGR format: 3 bytes per pixel (Blue, Green, Red).
    ///
    /// Common in Windows BMP files and OpenCV.
    Bgr,

    /// RGBA format: 4 bytes per pixel (Red, Green, Blue, Alpha).
    Rgba,

    /// RGBA with pre-multiplied alpha: 4 bytes per pixel.
    RgbaPremultiplied,

    /// BGRA format: 4 bytes per pixel (Blue, Green, Red, Alpha).
    ///
    /// Common in Windows GDI and many image libraries.
    Bgra,

    /// ARGB format: 4 bytes per pixel (Alpha, Red, Green, Blue).
    Argb,

    /// ABGR format: 4 bytes per pixel (Alpha, Blue, Green, Red).
    Abgr,

    /// CMYK format: 4 bytes per pixel (Cyan, Magenta, Yellow, Key/Black).
    Cmyk,

    /// 8-bit grayscale: 1 byte per pixel.
    Gray8,

    /// 32-bit grayscale: 4 bytes per pixel.
    Gray32,

    /// YCbCr color space: 3 bytes per pixel.
    YCbCr,

    /// YUV420P planar format.
    ///
    /// Y plane: 1 byte per pixel, U and V planes: 1 byte per 4 pixels.
    Yuv420p,
}

impl PixelFormat {
    /// Returns the number of bytes per pixel for this format.
    ///
    /// For planar formats like [`Yuv420p`](Self::Yuv420p), returns the
    /// average bytes per pixel.
    pub const fn bytes_per_pixel(self) -> usize {
        match self {
            Self::Rgb | Self::Bgr | Self::YCbCr => 3,
            Self::Rgba
            | Self::RgbaPremultiplied
            | Self::Bgra
            | Self::Argb
            | Self::Abgr
            | Self::Cmyk
            | Self::Gray32 => 4,
            Self::Gray8 => 1,
            Self::Yuv420p => 2, // Average: Y=1 + (U+V)/4 = 1.5, rounded up
        }
    }

    /// Converts this pixel format to the PhotoDNA options flag.
    fn to_options(self) -> PhotoDnaOptions {
        match self {
            Self::Rgb | Self::Bgr => sys::PhotoDna_Rgb,
            Self::Rgba | Self::Bgra => sys::PhotoDna_Rgba,
            Self::RgbaPremultiplied => sys::PhotoDna_RgbaPm,
            Self::Argb | Self::Abgr => sys::PhotoDna_Argb,
            Self::Cmyk => sys::PhotoDna_Cmyk,
            Self::Gray8 => sys::PhotoDna_Grey8,
            Self::Gray32 => sys::PhotoDna_Grey32,
            Self::YCbCr => sys::PhotoDna_YCbCr,
            Self::Yuv420p => sys::PhotoDna_Yuv420p,
        }
    }
}

/// Options for configuring the PhotoDNA generator.
///
/// Use the builder methods to customize the generator behavior.
///
/// # Examples
///
/// ```rust
/// use photodna::GeneratorOptions;
///
/// let options = GeneratorOptions::default()
///     .max_threads(4)
///     .library_dir("/custom/path/to/lib");
/// ```
#[derive(Debug, Clone)]
pub struct GeneratorOptions {
    /// Maximum number of concurrent threads for hash computation.
    max_threads: i32,

    /// Custom path to the library directory.
    library_dir: Option<String>,
}

impl Default for GeneratorOptions {
    fn default() -> Self {
        Self {
            max_threads: 4,
            library_dir: None,
        }
    }
}

impl GeneratorOptions {
    /// Creates new options with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the maximum number of concurrent threads.
    ///
    /// Hash computations exceeding this limit will block until a
    /// slot becomes available. Default is 4.
    ///
    /// # Arguments
    ///
    /// * `threads` - The maximum number of threads (must be > 0).
    pub fn max_threads(mut self, threads: i32) -> Self {
        self.max_threads = threads.max(1);
        self
    }

    /// Sets a custom library directory path.
    ///
    /// By default, the library is loaded from the path configured
    /// at build time via `PHOTODNA_SDK_ROOT`.
    ///
    /// # Arguments
    ///
    /// * `path` - The directory containing the PhotoDNA library.
    pub fn library_dir(mut self, path: impl Into<String>) -> Self {
        self.library_dir = Some(path.into());
        self
    }
}

/// Options for a single hash computation.
///
/// These options modify the behavior of individual hash operations.
#[derive(Debug, Clone, Copy, Default)]
pub struct HashOptions {
    /// The pixel format of the input image.
    pixel_format: PixelFormat,

    /// Whether to detect and remove borders from the image.
    remove_border: bool,

    /// Disable checking for rotated and flipped orientations.
    no_rotate_flip: bool,

    /// Enable verbose debug output to stderr.
    verbose: bool,

    /// Enable memory checking (may impact performance).
    check_memory: bool,
}

impl HashOptions {
    /// Creates new hash options with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the pixel format of the input image.
    ///
    /// Default is [`PixelFormat::Rgb`].
    pub fn pixel_format(mut self, format: PixelFormat) -> Self {
        self.pixel_format = format;
        self
    }

    /// Enables border detection and removal.
    ///
    /// When enabled, the library will attempt to detect and remove
    /// borders from the image before computing the hash.
    pub fn remove_border(mut self, enable: bool) -> Self {
        self.remove_border = enable;
        self
    }

    /// Disables rotation and flip detection.
    ///
    /// By default, the library checks for rotated and flipped
    /// versions of the image. This option disables that behavior.
    pub fn no_rotate_flip(mut self, disable: bool) -> Self {
        self.no_rotate_flip = disable;
        self
    }

    /// Enables verbose debug output.
    ///
    /// Debug messages will be written to stderr.
    pub fn verbose(mut self, enable: bool) -> Self {
        self.verbose = enable;
        self
    }

    /// Enables memory checking.
    ///
    /// This validates that data pointers reference valid allocated memory.
    /// Note: This may negatively impact performance.
    pub fn check_memory(mut self, enable: bool) -> Self {
        self.check_memory = enable;
        self
    }

    /// Converts these options to PhotoDNA library flags.
    fn to_sys_options(self) -> PhotoDnaOptions {
        let mut opts = sys::PhotoDna_HashFormatEdgeV2;
        opts |= self.pixel_format.to_options();

        if self.remove_border {
            opts |= sys::PhotoDna_RemoveBorder;
        }
        if self.no_rotate_flip {
            opts |= sys::PhotoDna_NoRotateFlip;
        }
        if self.verbose {
            opts |= sys::PhotoDna_Verbose;
        }
        if self.check_memory {
            opts |= sys::PhotoDna_CheckMemory;
        }

        opts
    }
}

/// The result of a hash computation with border detection.
///
/// Contains the primary hash and optionally a secondary hash
/// computed from the image with borders removed.
#[derive(Debug, Clone)]
pub struct BorderHashResult {
    /// The hash of the original image.
    pub primary: Hash,

    /// The hash with borders removed, if a border was detected.
    pub borderless: Option<Hash>,

    /// The detected border region (x, y, width, height).
    ///
    /// This describes the content area after border removal.
    pub content_region: Option<(i32, i32, i32, i32)>,
}

/// The PhotoDNA hash generator.
///
/// This struct manages the underlying PhotoDNA library instance and provides
/// safe methods for computing perceptual hashes.
///
/// # Thread Safety
///
/// The generator is [`Send`] but not [`Sync`]. To use from multiple threads,
/// either:
/// - Create one `Generator` per thread, or
/// - Wrap in `Arc<Mutex<Generator>>` for shared access
///
/// # Examples
///
/// ```rust,ignore
/// use photodna::{Generator, GeneratorOptions};
///
/// // Create a generator with default options
/// let generator = Generator::new(GeneratorOptions::default())?;
///
/// // Check library version
/// if let Some(version) = generator.library_version_text() {
///     println!("PhotoDNA library version: {}", version);
/// }
///
/// // Compute a hash
/// let hash = generator.compute_hash_rgb(&image_data, 640, 480)?;
/// ```
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
pub struct Generator {
    /// The underlying sys-level generator.
    inner: sys::EdgeHashGenerator,
}

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
impl Generator {
    /// Creates a new PhotoDNA generator with the given options.
    ///
    /// This loads the PhotoDNA library and initializes the internal state.
    ///
    /// # Arguments
    ///
    /// * `options` - Configuration options for the generator.
    ///
    /// # Errors
    ///
    /// Returns an error if the library cannot be loaded or initialized.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use photodna::{Generator, GeneratorOptions};
    ///
    /// let generator = Generator::new(GeneratorOptions::default())?;
    /// ```
    pub fn new(options: GeneratorOptions) -> Result<Self> {
        let inner =
            sys::EdgeHashGenerator::new(options.library_dir.as_deref(), options.max_threads)
                .map_err(PhotoDnaError::InitializationFailed)?;

        Ok(Self { inner })
    }

    /// Returns the last error number from the library.
    ///
    /// This can be useful for debugging after a failed operation.
    pub fn last_error_code(&self) -> i32 {
        self.inner.get_error_number()
    }

    /// Returns a human-readable description for an error code.
    pub fn error_description(&self, code: i32) -> Option<&str> {
        self.inner.get_error_string(code)
    }

    /// Returns the library version as a packed integer.
    ///
    /// High 16 bits = major version, low 16 bits = minor version.
    pub fn library_version(&self) -> i32 {
        self.inner.library_version()
    }

    /// Returns the major version number.
    pub fn library_version_major(&self) -> i32 {
        self.inner.library_version_major()
    }

    /// Returns the minor version number.
    pub fn library_version_minor(&self) -> i32 {
        self.inner.library_version_minor()
    }

    /// Returns the patch version number.
    pub fn library_version_patch(&self) -> i32 {
        self.inner.library_version_patch()
    }

    /// Returns the library version as a human-readable string.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let generator = Generator::new(GeneratorOptions::default())?;
    /// println!("Version: {}", generator.library_version_text().unwrap_or("unknown"));
    /// ```
    pub fn library_version_text(&self) -> Option<&str> {
        self.inner.library_version_text()
    }

    /// Computes a PhotoDNA hash from RGB pixel data.
    ///
    /// This is a convenience method that calls [`compute_hash`](Self::compute_hash)
    /// with RGB pixel format.
    ///
    /// # Arguments
    ///
    /// * `image_data` - Raw RGB pixel data (3 bytes per pixel).
    /// * `width` - Image width in pixels (minimum 50).
    /// * `height` - Image height in pixels (minimum 50).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Image dimensions are too small (< 50 pixels)
    /// - Buffer is too small for the specified dimensions
    /// - Image has insufficient gradient (flat image)
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let hash = generator.compute_hash_rgb(&rgb_pixels, 640, 480)?;
    /// ```
    pub fn compute_hash_rgb(&self, image_data: &[u8], width: u32, height: u32) -> Result<Hash> {
        self.compute_hash(image_data, width, height, HashOptions::default())
    }

    /// Computes a PhotoDNA hash from pixel data with custom options.
    ///
    /// # Arguments
    ///
    /// * `image_data` - Raw pixel data in the format specified by `options`.
    /// * `width` - Image width in pixels (minimum 50).
    /// * `height` - Image height in pixels (minimum 50).
    /// * `options` - Hash computation options.
    ///
    /// # Errors
    ///
    /// Returns an error if the hash cannot be computed. See [`PhotoDnaError`]
    /// for possible error types.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use photodna::{Generator, GeneratorOptions, HashOptions, PixelFormat};
    ///
    /// let generator = Generator::new(GeneratorOptions::default())?;
    ///
    /// let options = HashOptions::new()
    ///     .pixel_format(PixelFormat::Bgra)
    ///     .remove_border(true);
    ///
    /// let hash = generator.compute_hash(&bgra_pixels, 640, 480, options)?;
    /// ```
    pub fn compute_hash(
        &self,
        image_data: &[u8],
        width: u32,
        height: u32,
        options: HashOptions,
    ) -> Result<Hash> {
        self.compute_hash_with_stride(image_data, width, height, 0, options)
    }

    /// Computes a PhotoDNA hash with explicit stride.
    ///
    /// Use this when the image has padding bytes between rows (common in
    /// windowed framebuffers and some image formats).
    ///
    /// # Arguments
    ///
    /// * `image_data` - Raw pixel data.
    /// * `width` - Image width in pixels (minimum 50).
    /// * `height` - Image height in pixels (minimum 50).
    /// * `stride` - Row stride in bytes, or 0 to auto-calculate.
    /// * `options` - Hash computation options.
    ///
    /// # Errors
    ///
    /// Returns an error if the hash cannot be computed.
    pub fn compute_hash_with_stride(
        &self,
        image_data: &[u8],
        width: u32,
        height: u32,
        stride: u32,
        options: HashOptions,
    ) -> Result<Hash> {
        let width_i32 = width as i32;
        let height_i32 = height as i32;
        let stride_i32 = stride as i32;

        // Validate dimensions
        if width == 0 || height == 0 {
            return Err(PhotoDnaError::InvalidDimensions {
                width: width_i32,
                height: height_i32,
            });
        }

        // Calculate expected buffer size
        let bytes_per_pixel = options.pixel_format.bytes_per_pixel();
        let expected_stride = if stride == 0 {
            (width as usize) * bytes_per_pixel
        } else {
            stride as usize
        };
        let expected_size = expected_stride * (height as usize);

        if image_data.len() < expected_size {
            return Err(PhotoDnaError::BufferTooSmall {
                expected: expected_size,
                actual: image_data.len(),
            });
        }

        let sys_options = options.to_sys_options();

        // Allocate hash buffer on the stack
        let mut hash_buffer = [0u8; HASH_SIZE];

        // SAFETY: We have validated the buffer sizes and dimensions.
        // The sys library will validate the image data internally.
        let result = unsafe {
            self.inner.photo_dna_edge_hash(
                image_data.as_ptr(),
                hash_buffer.as_mut_ptr(),
                width_i32,
                height_i32,
                stride_i32,
                sys_options,
            )
        };

        if result < 0 {
            return Err(PhotoDnaError::from_error_code(result));
        }

        Ok(Hash::new(hash_buffer))
    }

    /// Computes a hash for a sub-region of an image.
    ///
    /// # Arguments
    ///
    /// * `image_data` - Raw pixel data for the full image.
    /// * `width` - Full image width in pixels.
    /// * `height` - Full image height in pixels.
    /// * `stride` - Row stride in bytes, or 0 to auto-calculate.
    /// * `region` - The sub-region to hash: (x, y, width, height).
    /// * `options` - Hash computation options.
    ///
    /// # Errors
    ///
    /// Returns an error if the region is outside the image bounds or
    /// if the hash cannot be computed.
    pub fn compute_hash_subregion(
        &self,
        image_data: &[u8],
        width: u32,
        height: u32,
        stride: u32,
        region: (u32, u32, u32, u32),
        options: HashOptions,
    ) -> Result<Hash> {
        let (rx, ry, rw, rh) = region;

        // Validate region bounds
        if rx + rw > width || ry + rh > height {
            return Err(PhotoDnaError::InvalidSubImage);
        }

        let width_i32 = width as i32;
        let height_i32 = height as i32;
        let stride_i32 = stride as i32;

        // Validate dimensions
        if width == 0 || height == 0 || rw == 0 || rh == 0 {
            return Err(PhotoDnaError::InvalidDimensions {
                width: rw as i32,
                height: rh as i32,
            });
        }

        // Calculate expected buffer size for the full image
        let bytes_per_pixel = options.pixel_format.bytes_per_pixel();
        let expected_stride = if stride == 0 {
            (width as usize) * bytes_per_pixel
        } else {
            stride as usize
        };
        let expected_size = expected_stride * (height as usize);

        if image_data.len() < expected_size {
            return Err(PhotoDnaError::BufferTooSmall {
                expected: expected_size,
                actual: image_data.len(),
            });
        }

        let sys_options = options.to_sys_options();
        let mut hash_buffer = [0u8; HASH_SIZE];

        // SAFETY: Buffer sizes validated, region bounds checked.
        let result = unsafe {
            self.inner.photo_dna_edge_hash_sub(
                image_data.as_ptr(),
                hash_buffer.as_mut_ptr(),
                width_i32,
                height_i32,
                stride_i32,
                rx as i32,
                ry as i32,
                rw as i32,
                rh as i32,
                sys_options,
            )
        };

        if result < 0 {
            return Err(PhotoDnaError::from_error_code(result));
        }

        Ok(Hash::new(hash_buffer))
    }

    /// Computes a hash with automatic border detection.
    ///
    /// This method returns both the original hash and a hash computed
    /// after removing detected borders.
    ///
    /// # Arguments
    ///
    /// * `image_data` - Raw pixel data.
    /// * `width` - Image width in pixels (minimum 50).
    /// * `height` - Image height in pixels (minimum 50).
    /// * `options` - Hash computation options.
    ///
    /// # Returns
    ///
    /// A [`BorderHashResult`] containing the primary hash and optionally
    /// a hash with borders removed.
    ///
    /// # Errors
    ///
    /// Returns an error if the hash cannot be computed.
    pub fn compute_hash_with_border_detection(
        &self,
        image_data: &[u8],
        width: u32,
        height: u32,
        options: HashOptions,
    ) -> Result<BorderHashResult> {
        let width_i32 = width as i32;
        let height_i32 = height as i32;

        // Validate dimensions
        if width == 0 || height == 0 {
            return Err(PhotoDnaError::InvalidDimensions {
                width: width_i32,
                height: height_i32,
            });
        }

        // Calculate expected buffer size
        let bytes_per_pixel = options.pixel_format.bytes_per_pixel();
        let expected_size = (width as usize) * (height as usize) * bytes_per_pixel;

        if image_data.len() < expected_size {
            return Err(PhotoDnaError::BufferTooSmall {
                expected: expected_size,
                actual: image_data.len(),
            });
        }

        let sys_options = options.to_sys_options();

        // Allocate result buffer for up to 2 hashes
        let mut hash_results = [sys::HashResult::default(); 2];

        // SAFETY: Buffer validated, hash_results array is properly sized.
        let count = unsafe {
            self.inner.photo_dna_edge_hash_border(
                image_data.as_ptr(),
                hash_results.as_mut_ptr(),
                2,
                width_i32,
                height_i32,
                0, // auto stride
                sys_options,
            )
        };

        if count < 0 {
            return Err(PhotoDnaError::from_error_code(count));
        }

        // Extract primary hash (always present if count >= 1)
        let primary = extract_hash_from_result(&hash_results[0])?;

        // Extract borderless hash if a border was detected (count == 2)
        let (borderless, content_region) = if count >= 2 {
            let hash = extract_hash_from_result(&hash_results[1])?;
            let region = (
                hash_results[1].header_dimensions_image_x,
                hash_results[1].header_dimensions_image_y,
                hash_results[1].header_dimensions_image_w,
                hash_results[1].header_dimensions_image_h,
            );
            (Some(hash), Some(region))
        } else {
            (None, None)
        };

        Ok(BorderHashResult {
            primary,
            borderless,
            content_region,
        })
    }

    /// Returns the raw library instance pointer.
    ///
    /// This is intended for advanced use cases that need direct FFI access.
    ///
    /// # Safety
    ///
    /// The returned pointer is only valid while this `Generator` is alive.
    /// Do not use after dropping the generator.
    pub fn raw_instance(&self) -> *mut c_void {
        self.inner.raw_instance()
    }
}

// SAFETY: The Generator can be sent between threads. The internal library
// handle is thread-safe for single-owner usage (ownership transfer).
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
unsafe impl Send for Generator {}

// Note: Generator is NOT Sync because the underlying library may maintain
// thread-local state. Use Mutex if concurrent access is needed.

/// Extracts a Hash from a sys::HashResult.
fn extract_hash_from_result(result: &sys::HashResult) -> Result<Hash> {
    // Copy packed field to avoid unaligned access
    let result_code = result.result;
    if result_code < 0 {
        return Err(PhotoDnaError::from_error_code(result_code));
    }

    // The hash is stored in the first HASH_SIZE bytes
    let mut hash_bytes = [0u8; HASH_SIZE];
    hash_bytes.copy_from_slice(&result.hash[..HASH_SIZE]);

    Ok(Hash::new(hash_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pixel_format_bytes_per_pixel() {
        assert_eq!(PixelFormat::Rgb.bytes_per_pixel(), 3);
        assert_eq!(PixelFormat::Rgba.bytes_per_pixel(), 4);
        assert_eq!(PixelFormat::Gray8.bytes_per_pixel(), 1);
    }

    #[test]
    fn test_generator_options_builder() {
        let options = GeneratorOptions::new()
            .max_threads(8)
            .library_dir("/custom/path");

        assert_eq!(options.max_threads, 8);
        assert_eq!(options.library_dir, Some("/custom/path".to_string()));
    }

    #[test]
    fn test_hash_options_builder() {
        let options = HashOptions::new()
            .pixel_format(PixelFormat::Bgra)
            .remove_border(true)
            .verbose(true);

        assert_eq!(options.pixel_format, PixelFormat::Bgra);
        assert!(options.remove_border);
        assert!(options.verbose);
    }

    #[test]
    fn test_hash_options_to_sys_options() {
        let options = HashOptions::new()
            .pixel_format(PixelFormat::Rgba)
            .remove_border(true)
            .no_rotate_flip(true);

        let sys_opts = options.to_sys_options();

        // Verify flags are set
        assert!(sys_opts & sys::PhotoDna_HashFormatEdgeV2 != 0);
        assert!(sys_opts & sys::PhotoDna_Rgba != 0);
        assert!(sys_opts & sys::PhotoDna_RemoveBorder != 0);
        assert!(sys_opts & sys::PhotoDna_NoRotateFlip != 0);
    }

    #[test]
    fn test_generator_options_max_threads_minimum() {
        let options = GeneratorOptions::new().max_threads(-5);
        assert_eq!(options.max_threads, 1);

        let options = GeneratorOptions::new().max_threads(0);
        assert_eq!(options.max_threads, 1);
    }
}

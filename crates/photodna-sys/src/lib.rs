//! # photodna-sys
//!
//! Low-level, unsafe FFI bindings to the Microsoft PhotoDNA Edge Hash Generator library.
//!
//! ## Purpose
//!
//! This crate provides raw Rust bindings to the proprietary Microsoft PhotoDNA SDK,
//! enabling computation of perceptual image hashes for content identification. PhotoDNA
//! generates a compact 924-byte "fingerprint" that identifies visually similar images
//! even after modifications like resizing, cropping, or format conversion.
//!
//! **Primary use cases:**
//! - Content moderation and safety systems
//! - Detection of known illegal content (CSAM, etc.)
//! - Image deduplication at scale
//! - Visual similarity search
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                      Your Application                           │
//! └─────────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                   photodna (safe wrapper)                       │
//! │  Generator, Hash, PixelFormat, PhotoDnaError                    │
//! └─────────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                   photodna-sys (this crate)                     │
//! │  EdgeHashGenerator, HashResult, constants, FFI types            │
//! └─────────────────────────────────────────────────────────────────┘
//!                              │
//!                    ┌─────────┴─────────┐
//!                    ▼                   ▼
//!     ┌──────────────────────┐  ┌─────────────────────┐
//!     │ Native Library       │  │ WASM Module (BSD)   │
//!     │ .dll / .so / .dylib  │  │ photoDnaEdgeHash    │
//!     └──────────────────────┘  └─────────────────────┘
//! ```
//!
//! ## Library Loading Model
//!
//! The PhotoDNA library uses **runtime dynamic loading** (`dlopen`/`LoadLibrary`),
//! not compile-time linking. This crate provides:
//!
//! | Component | Description |
//! |-----------|-------------|
//! | [`EdgeHashGenerator`] | Main wrapper that loads the library and exposes function access |
//! | [`HashResult`] | C-compatible struct for border detection results |
//! | `PhotoDnaOptions` | Bitmask flags for pixel format, hash format, and behavior |
//! | `Fn*` types | Function pointer types matching the C API signatures |
//!
//! ## Requirements
//!
//! **This crate requires the proprietary Microsoft PhotoDNA SDK (not included).**
//!
//! Set `PHOTODNA_SDK_ROOT` environment variable before building:
//!
//! ```bash
//! export PHOTODNA_SDK_ROOT=/path/to/PhotoDNA.EdgeHashGeneration-1.05.001
//! ```
//!
//! Expected SDK directory structure:
//! ```text
//! $PHOTODNA_SDK_ROOT/
//! ├── clientlibrary/
//! │   ├── libEdgeHashGenerator.so.1.05          # Linux x86_64
//! │   ├── libEdgeHashGenerator-arm64.so.1.05    # Linux ARM64
//! │   ├── libEdgeHashGenerator.1.05.dll         # Windows x86_64
//! │   └── c/PhotoDnaEdgeHashGenerator.h         # C header
//! └── webassembly/photoDnaEdgeHash.wasm         # WASM for BSD
//! ```
//!
//! ## Platform Support
//!
//! | Platform | Architecture | Backend | Notes |
//! |----------|--------------|---------|-------|
//! | Windows  | x86_64, x86, ARM64 | Native `.dll` | Default |
//! | Linux    | x86_64, x86, ARM64 | Native `.so` | Default |
//! | macOS    | x86_64, ARM64 | Native `.so` | ARM64 via Rosetta or native |
//! | OpenBSD/FreeBSD | any | WebAssembly | Requires `wasm` feature |
//!
//! ## Features
//!
//! | Feature | Default | Description |
//! |---------|---------|-------------|
//! | `native` | ✓ | Runtime loading of native dynamic libraries |
//! | `wasm` | | Embeds WebAssembly module for BSD platforms |
//! | `bindgen` | | Regenerate bindings from C headers (requires clang) |
//!
//! ## Safety Requirements
//!
//! All FFI functions are `unsafe`. Callers **must** ensure:
//!
//! 1. Library is initialized via [`EdgeHashGenerator::new`] before any calls
//! 2. All pointers point to valid, sufficiently-sized memory
//! 3. Image buffers match specified dimensions: `height * stride` bytes minimum
//! 4. Stride is 0 (auto-calculate) or `>= width * bytes_per_pixel`
//! 5. The `EdgeHashGenerator` instance outlives all operations using it
//!
//! ## Error Codes
//!
//! | Code | Constant | Meaning |
//! |------|----------|---------|
//! | -7000 | `PhotoDna_ErrorUnknown` | Undetermined internal error |
//! | -7001 | `PhotoDna_ErrorMemoryAllocationFailed` | Memory allocation failed |
//! | -7006 | `PhotoDna_ErrorImageTooSmall` | Image dimension < 50 pixels |
//! | -7009 | `PhotoDna_ErrorImageIsFlat` | Insufficient gradients in image |
//! | -7012 | `PhotoDna_ErrorInvalidStride` | Invalid stride value |
//!
//! See individual constant documentation for the complete list.
//!
//! ## Example
//!
//! ```rust,ignore
//! use photodna_sys::*;
//!
//! // Initialize (loads native library, allocates thread pool)
//! let lib = EdgeHashGenerator::new(None, 4)?; // 4 concurrent threads max
//!
//! // Prepare image: RGB format, 640x480, tightly packed rows
//! let width = 640;
//! let height = 480;
//! let image_data: Vec<u8> = vec![0u8; width * height * 3];
//!
//! // Compute hash
//! let mut hash = [0u8; PHOTODNA_HASH_SIZE_MAX];
//! let result = unsafe {
//!     lib.photo_dna_edge_hash(
//!         image_data.as_ptr(),
//!         hash.as_mut_ptr(),
//!         width as i32,
//!         height as i32,
//!         0,              // stride=0 means auto-calculate
//!         PhotoDna_Rgb | PhotoDna_HashFormatEdgeV2,
//!     )
//! };
//!
//! match result {
//!     r if r >= 0 => println!("Hash computed: {} bytes", PHOTODNA_HASH_SIZE_EDGE_V2),
//!     PhotoDna_ErrorImageTooSmall => eprintln!("Image must be >= 50x50 pixels"),
//!     PhotoDna_ErrorImageIsFlat => eprintln!("Image lacks sufficient gradients"),
//!     code => eprintln!("Error: {}", error_code_description(code)),
//! }
//! ```

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
// Allow dead code for constants that may not be used by all consumers
#![allow(dead_code)]
// FFI functions must match the C API signature exactly
#![allow(clippy::too_many_arguments)]

use std::ffi::{c_char, c_void, CStr};

#[cfg(not(photodna_no_sdk))]
use std::ffi::CString;

// ============================================================================
// Constants
// ============================================================================

/// Size of PhotoDNA Edge V2 hash in bytes (binary format).
pub const PHOTODNA_HASH_SIZE_EDGE_V2: usize = 0x39c; // 924 bytes

/// Size of PhotoDNA Edge V2 hash in bytes (Base64 format).
pub const PHOTODNA_HASH_SIZE_EDGE_V2_BASE64: usize = 0x4d0; // 1232 bytes

/// Maximum hash buffer size required.
pub const PHOTODNA_HASH_SIZE_MAX: usize = 0x4d0; // 1232 bytes

/// Library version string.
pub const PHOTODNA_LIBRARY_VERSION: &str = "1.05";

/// The SDK root path (set at compile time from PHOTODNA_SDK_ROOT environment variable).
/// Only available on native platforms (Windows, Linux, macOS) with SDK configured at build time.
#[cfg(all(
    any(target_os = "windows", target_os = "linux", target_os = "macos"),
    not(photodna_no_sdk)
))]
pub const PHOTODNA_SDK_ROOT: &str = env!("PHOTODNA_SDK_ROOT");

/// The client library directory path.
/// Only available on native platforms (Windows, Linux, macOS) with SDK configured at build time.
#[cfg(all(
    any(target_os = "windows", target_os = "linux", target_os = "macos"),
    not(photodna_no_sdk)
))]
pub const PHOTODNA_LIB_DIR: &str = env!("PHOTODNA_LIB_DIR");

// ============================================================================
// Error Codes
// ============================================================================

/// Type alias for PhotoDNA error codes.
pub type ErrorCode = u32;

/// An undetermined error occurred.
pub const PhotoDna_ErrorUnknown: i32 = -7000;

/// Failed to allocate memory.
pub const PhotoDna_ErrorMemoryAllocationFailed: i32 = -7001;

/// Alias for memory allocation failure (host-side).
pub const PhotoDna_ErrorHostMemoryAllocationFailed: i32 = -7001;

/// General failure within the library.
pub const PhotoDna_ErrorLibraryFailure: i32 = -7002;

/// System memory exception occurred.
pub const PhotoDna_ErrorMemoryAccess: i32 = -7003;

/// Hash that does not conform to PhotoDNA specifications.
pub const PhotoDna_ErrorInvalidHash: i32 = -7004;

/// An invalid character was contained in a Base64 or Hex hash.
pub const PhotoDna_ErrorHashFormatInvalidCharacters: i32 = -7005;

/// Provided image had a dimension less than 50 pixels.
pub const PhotoDna_ErrorImageTooSmall: i32 = -7006;

/// A border was not detected for the image.
pub const PhotoDna_ErrorNoBorder: i32 = -7007;

/// An invalid argument was passed to the function.
pub const PhotoDna_ErrorBadArgument: i32 = -7008;

/// The image has few or no gradients.
pub const PhotoDna_ErrorImageIsFlat: i32 = -7009;

/// Provided image had a dimension less than 50 pixels (no border variant).
pub const PhotoDna_ErrorNoBorderImageTooSmall: i32 = -7010;

/// Not a known source image format.
pub const PhotoDna_ErrorSourceFormatUnknown: i32 = -7011;

/// Stride should be 0, or greater than or equal to width in bytes.
pub const PhotoDna_ErrorInvalidStride: i32 = -7012;

/// The sub region area is not within the boundaries of the image.
pub const PhotoDna_ErrorInvalidSubImage: i32 = -7013;

// ============================================================================
// Hash Size Constants
// ============================================================================

/// Type alias for hash size identifiers.
pub type HashSize = u32;

/// Edge V2 format hash size.
pub const PhotoDna_EdgeV2: HashSize = 0x0000039c;

/// Edge V2 format hash size (Base64 encoded).
pub const PhotoDna_EdgeV2Base64: HashSize = 0x000004d0;

/// Maximum hash size.
pub const PhotoDna_MaxSize: HashSize = 0x000004d0;

// ============================================================================
// PhotoDNA Options (Flags)
// ============================================================================

/// Type alias for PhotoDNA option flags.
pub type PhotoDnaOptions = u32;

/// No options specified. See description for default behavior.
pub const PhotoDna_OptionNone: PhotoDnaOptions = 0x00000000;

/// Default options. The matcher will return all results found.
pub const PhotoDna_Default: PhotoDnaOptions = 0x00000000;

/// Mask to isolate the hash format bits.
pub const PhotoDna_HashFormatMask: PhotoDnaOptions = 0x000000f0;

/// Hash output format: PhotoDNA Edge Hash V2.
pub const PhotoDna_HashFormatEdgeV2: PhotoDnaOptions = 0x00000080;

/// Hash output format: PhotoDNA Edge Hash V2 Base64.
pub const PhotoDna_HashFormatEdgeV2Base64: PhotoDnaOptions = 0x00000090;

/// Mask to isolate the pixel layout bits.
pub const PhotoDna_PixelLayoutMask: PhotoDnaOptions = 0x00001f00;

/// Pixel layout: RGB, 3 bytes per pixel.
pub const PhotoDna_Rgb: PhotoDnaOptions = 0x00000000;

/// Pixel layout: BGR, 3 bytes per pixel.
pub const PhotoDna_Bgr: PhotoDnaOptions = 0x00000000;

/// Pixel layout: RGBA, 4 bytes per pixel.
pub const PhotoDna_Rgba: PhotoDnaOptions = 0x00000100;

/// Pixel layout: RGBA with pre-multiplied alpha, 4 bytes per pixel.
pub const PhotoDna_RgbaPm: PhotoDnaOptions = 0x00000700;

/// Pixel layout: BGRA, 4 bytes per pixel.
pub const PhotoDna_Bgra: PhotoDnaOptions = 0x00000100;

/// Pixel layout: ARGB, 4 bytes per pixel.
pub const PhotoDna_Argb: PhotoDnaOptions = 0x00000200;

/// Pixel layout: ABGR, 4 bytes per pixel.
pub const PhotoDna_Abgr: PhotoDnaOptions = 0x00000200;

/// Pixel layout: CMYK, 4 bytes per pixel.
pub const PhotoDna_Cmyk: PhotoDnaOptions = 0x00000300;

/// Pixel layout: Grayscale 8-bit, 1 byte per pixel.
pub const PhotoDna_Grey8: PhotoDnaOptions = 0x00000400;

/// Pixel layout: Grayscale 32-bit, 4 bytes per pixel.
pub const PhotoDna_Grey32: PhotoDnaOptions = 0x00000500;

/// Pixel layout: YCbCr, 3 bytes per pixel.
pub const PhotoDna_YCbCr: PhotoDnaOptions = 0x00000600;

/// Pixel layout: YUV420P planar format.
/// Y = 1 byte per pixel, U = 1 byte per 4 pixels, V = 1 byte per 4 pixels.
pub const PhotoDna_Yuv420p: PhotoDnaOptions = 0x00000800;

/// Check for and remove borders from the image.
pub const PhotoDna_RemoveBorder: PhotoDnaOptions = 0x00200000;

/// Prevent checks for rotated and/or flipped orientations.
pub const PhotoDna_NoRotateFlip: PhotoDnaOptions = 0x01000000;

/// Check data pointers for valid allocated memory.
/// Note: This may negatively impact performance.
pub const PhotoDna_CheckMemory: PhotoDnaOptions = 0x20000000;

/// Enable debug output to stderr.
pub const PhotoDna_Verbose: PhotoDnaOptions = 0x40000000;

/// Equivalent to Verbose + CheckMemory.
pub const PhotoDna_Test: PhotoDnaOptions = 0x60000000;

/// Use same options as specified for primary parameter.
pub const PhotoDna_Other: PhotoDnaOptions = 0xffffffff; // -1 as u32

// ============================================================================
// Structures
// ============================================================================

/// Result structure returned by border detection hash functions.
///
/// When a border is found, the hash with the border removed will be in the
/// second instance of the results array.
///
/// # Result Values
/// - `< 0`: An error occurred (see error codes)
/// - `1`: No border was found
/// - `2`: A border was found
#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct HashResult {
    /// Error code if less than 0, otherwise indicates border detection result.
    pub result: i32,
    /// Hash format used for this result.
    pub hash_format: i32,
    /// Left position (X) within the provided image.
    pub header_dimensions_image_x: i32,
    /// Top position (Y) within the provided image.
    pub header_dimensions_image_y: i32,
    /// Width within the provided image.
    pub header_dimensions_image_w: i32,
    /// Height within the provided image.
    pub header_dimensions_image_h: i32,
    /// The computed hash in the requested format.
    pub hash: [u8; PHOTODNA_HASH_SIZE_MAX],
    /// Reserved for future use.
    pub reserved0: i32,
    /// Reserved for future use.
    pub reserved1: i32,
    /// Reserved for future use.
    pub reserved2: i32,
    /// Reserved for future use.
    pub reserved3: i32,
    /// Reserved for future use.
    pub reserved4: i32,
    /// Reserved for future use.
    pub reserved5: i32,
}

impl Default for HashResult {
    fn default() -> Self {
        Self {
            result: 0,
            hash_format: 0,
            header_dimensions_image_x: 0,
            header_dimensions_image_y: 0,
            header_dimensions_image_w: 0,
            header_dimensions_image_h: 0,
            hash: [0u8; PHOTODNA_HASH_SIZE_MAX],
            reserved0: 0,
            reserved1: 0,
            reserved2: 0,
            reserved3: 0,
            reserved4: 0,
            reserved5: 0,
        }
    }
}

impl core::fmt::Debug for HashResult {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Copy packed fields to avoid unaligned references
        let result = self.result;
        let hash_format = self.hash_format;
        let x = self.header_dimensions_image_x;
        let y = self.header_dimensions_image_y;
        let w = self.header_dimensions_image_w;
        let h = self.header_dimensions_image_h;

        f.debug_struct("HashResult")
            .field("result", &result)
            .field("hash_format", &hash_format)
            .field("x", &x)
            .field("y", &y)
            .field("w", &w)
            .field("h", &h)
            .field("hash", &"[...]")
            .finish()
    }
}

// ============================================================================
// Function Pointer Types
// ============================================================================

/// Function pointer type for EdgeHashGeneratorInit.
pub type FnEdgeHashGeneratorInit =
    unsafe extern "C" fn(library_path: *const c_char, max_threads: i32) -> *mut c_void;

/// Function pointer type for EdgeHashGeneratorRelease.
pub type FnEdgeHashGeneratorRelease = unsafe extern "C" fn(library_instance: *mut c_void);

/// Function pointer type for GetErrorNumber.
pub type FnGetErrorNumber = unsafe extern "C" fn(library_instance: *mut c_void) -> i32;

/// Function pointer type for GetErrorString.
pub type FnGetErrorString =
    unsafe extern "C" fn(library_instance: *mut c_void, error: i32) -> *const c_char;

/// Function pointer type for LibraryVersion.
pub type FnLibraryVersion = unsafe extern "C" fn(library_instance: *mut c_void) -> i32;

/// Function pointer type for LibraryVersionMajor.
pub type FnLibraryVersionMajor = unsafe extern "C" fn(library_instance: *mut c_void) -> i32;

/// Function pointer type for LibraryVersionMinor.
pub type FnLibraryVersionMinor = unsafe extern "C" fn(library_instance: *mut c_void) -> i32;

/// Function pointer type for LibraryVersionPatch.
pub type FnLibraryVersionPatch = unsafe extern "C" fn(library_instance: *mut c_void) -> i32;

/// Function pointer type for LibraryVersionText.
pub type FnLibraryVersionText =
    unsafe extern "C" fn(library_instance: *mut c_void) -> *const c_char;

/// Function pointer type for PhotoDnaEdgeHash.
pub type FnPhotoDnaEdgeHash = unsafe extern "C" fn(
    library_instance: *mut c_void,
    image_data: *const u8,
    hash_value: *mut u8,
    width: i32,
    height: i32,
    stride: i32,
    options: PhotoDnaOptions,
) -> i32;

/// Function pointer type for PhotoDnaEdgeHashBorder.
pub type FnPhotoDnaEdgeHashBorder = unsafe extern "C" fn(
    library_instance: *mut c_void,
    image_data: *const u8,
    hash_results: *mut HashResult,
    max_hash_count: i32,
    width: i32,
    height: i32,
    stride: i32,
    options: PhotoDnaOptions,
) -> i32;

/// Function pointer type for PhotoDnaEdgeHashBorderSub.
pub type FnPhotoDnaEdgeHashBorderSub = unsafe extern "C" fn(
    library_instance: *mut c_void,
    image_data: *const u8,
    hash_results: *mut HashResult,
    max_hash_count: i32,
    width: i32,
    height: i32,
    stride: i32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    options: PhotoDnaOptions,
) -> i32;

/// Function pointer type for PhotoDnaEdgeHashSub.
pub type FnPhotoDnaEdgeHashSub = unsafe extern "C" fn(
    library_instance: *mut c_void,
    image_data: *const u8,
    hash_value: *mut u8,
    width: i32,
    height: i32,
    stride: i32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    options: PhotoDnaOptions,
) -> i32;

// ============================================================================
// Native Library Loading (Windows, Linux, macOS)
// ============================================================================

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
mod native {
    use super::*;

    /// Returns the platform-specific library filename.
    pub fn get_library_filename() -> String {
        #[cfg(target_os = "windows")]
        {
            #[cfg(target_arch = "x86_64")]
            {
                format!("libEdgeHashGenerator.{}.dll", PHOTODNA_LIBRARY_VERSION)
            }
            #[cfg(target_arch = "aarch64")]
            {
                format!(
                    "libEdgeHashGenerator-arm64.{}.dll",
                    PHOTODNA_LIBRARY_VERSION
                )
            }
            #[cfg(target_arch = "x86")]
            {
                format!("libEdgeHashGenerator-x86.{}.dll", PHOTODNA_LIBRARY_VERSION)
            }
            #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64", target_arch = "x86")))]
            {
                format!("libEdgeHashGenerator.{}.dll", PHOTODNA_LIBRARY_VERSION)
            }
        }
        #[cfg(target_os = "linux")]
        {
            #[cfg(target_arch = "x86_64")]
            {
                format!("libEdgeHashGenerator.so.{}", PHOTODNA_LIBRARY_VERSION)
            }
            #[cfg(target_arch = "aarch64")]
            {
                format!("libEdgeHashGenerator-arm64.so.{}", PHOTODNA_LIBRARY_VERSION)
            }
            #[cfg(target_arch = "x86")]
            {
                format!("libEdgeHashGenerator-x86.so.{}", PHOTODNA_LIBRARY_VERSION)
            }
            #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64", target_arch = "x86")))]
            {
                format!("libEdgeHashGenerator.so.{}", PHOTODNA_LIBRARY_VERSION)
            }
        }
        #[cfg(target_os = "macos")]
        {
            #[cfg(target_arch = "aarch64")]
            {
                format!(
                    "libEdgeHashGenerator-arm64-macos.so.{}",
                    PHOTODNA_LIBRARY_VERSION
                )
            }
            #[cfg(not(target_arch = "aarch64"))]
            {
                format!("libEdgeHashGenerator-macos.so.{}", PHOTODNA_LIBRARY_VERSION)
            }
        }
    }
}

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
pub use native::*;

// ============================================================================
// Edge Hash Generator
// ============================================================================

/// The PhotoDNA Edge Hash Generator library wrapper.
///
/// This struct handles loading the native library and provides access to all
/// library functions through type-safe function pointers.
///
/// # Example
///
/// ```rust,ignore
/// use photodna_sys::*;
///
/// let lib = EdgeHashGenerator::new(None, 4)?;
/// println!("Library version: {}", lib.library_version_text());
/// ```
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
pub struct EdgeHashGenerator {
    /// Handle to the loaded dynamic library.
    _library: libloading::Library,
    /// Handle to the PhotoDNA library instance.
    library_instance: *mut c_void,
    /// Function pointer: EdgeHashGeneratorRelease
    fn_release: libloading::Symbol<'static, FnEdgeHashGeneratorRelease>,
    /// Function pointer: GetErrorNumber
    fn_get_error_number: libloading::Symbol<'static, FnGetErrorNumber>,
    /// Function pointer: GetErrorString
    fn_get_error_string: libloading::Symbol<'static, FnGetErrorString>,
    /// Function pointer: LibraryVersion
    fn_library_version: libloading::Symbol<'static, FnLibraryVersion>,
    /// Function pointer: LibraryVersionMajor
    fn_library_version_major: libloading::Symbol<'static, FnLibraryVersionMajor>,
    /// Function pointer: LibraryVersionMinor
    fn_library_version_minor: libloading::Symbol<'static, FnLibraryVersionMinor>,
    /// Function pointer: LibraryVersionPatch
    fn_library_version_patch: libloading::Symbol<'static, FnLibraryVersionPatch>,
    /// Function pointer: LibraryVersionText
    fn_library_version_text: libloading::Symbol<'static, FnLibraryVersionText>,
    /// Function pointer: PhotoDnaEdgeHash
    fn_photo_dna_edge_hash: libloading::Symbol<'static, FnPhotoDnaEdgeHash>,
    /// Function pointer: PhotoDnaEdgeHashBorder
    fn_photo_dna_edge_hash_border: libloading::Symbol<'static, FnPhotoDnaEdgeHashBorder>,
    /// Function pointer: PhotoDnaEdgeHashBorderSub
    fn_photo_dna_edge_hash_border_sub: libloading::Symbol<'static, FnPhotoDnaEdgeHashBorderSub>,
    /// Function pointer: PhotoDnaEdgeHashSub
    fn_photo_dna_edge_hash_sub: libloading::Symbol<'static, FnPhotoDnaEdgeHashSub>,
}

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
impl EdgeHashGenerator {
    /// Creates a new EdgeHashGenerator by loading the native library.
    ///
    /// # Parameters
    ///
    /// - `library_dir`: Directory containing the library. If `None`, uses the path from `PHOTODNA_LIB_DIR`.
    /// - `max_threads`: Maximum number of concurrent threads. Calls exceeding this
    ///   will block until a previous call completes.
    ///
    /// # Returns
    ///
    /// A Result containing the EdgeHashGenerator or an error message.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Use default library path
    /// let lib = EdgeHashGenerator::new(None, 4)?;
    ///
    /// // Use custom library path
    /// let lib = EdgeHashGenerator::new(Some("/path/to/libs"), 4)?;
    /// ```
    pub fn new(library_dir: Option<&str>, max_threads: i32) -> Result<Self, String> {
        #[cfg(photodna_no_sdk)]
        {
            let _ = (library_dir, max_threads); // Suppress unused warnings
            Err(
                "PhotoDNA SDK not available: PHOTODNA_SDK_ROOT was not set at build time. \
                 Please rebuild with PHOTODNA_SDK_ROOT environment variable set to the SDK directory."
                    .to_string(),
            )
        }

        #[cfg(not(photodna_no_sdk))]
        {
            let lib_dir = library_dir.unwrap_or(PHOTODNA_LIB_DIR);
            let lib_filename = get_library_filename();
            let lib_path = format!("{}/{}", lib_dir, lib_filename);

            unsafe {
                // Load the dynamic library using libloading
                let library = libloading::Library::new(&lib_path)
                    .map_err(|e| format!("Failed to load library '{}': {}", lib_path, e))?;

                // Get function pointers using libloading
                let fn_init: libloading::Symbol<FnEdgeHashGeneratorInit> = library
                    .get(b"EdgeHashGeneratorInit\0")
                    .map_err(|e| format!("Failed to find symbol 'EdgeHashGeneratorInit': {}", e))?;
                let fn_release: libloading::Symbol<FnEdgeHashGeneratorRelease> = library
                    .get(b"EdgeHashGeneratorRelease\0")
                    .map_err(|e| {
                        format!("Failed to find symbol 'EdgeHashGeneratorRelease': {}", e)
                    })?;
                let fn_get_error_number: libloading::Symbol<FnGetErrorNumber> = library
                    .get(b"GetErrorNumber\0")
                    .map_err(|e| format!("Failed to find symbol 'GetErrorNumber': {}", e))?;
                let fn_get_error_string: libloading::Symbol<FnGetErrorString> = library
                    .get(b"GetErrorString\0")
                    .map_err(|e| format!("Failed to find symbol 'GetErrorString': {}", e))?;
                let fn_library_version: libloading::Symbol<FnLibraryVersion> = library
                    .get(b"LibraryVersion\0")
                    .map_err(|e| format!("Failed to find symbol 'LibraryVersion': {}", e))?;
                let fn_library_version_major: libloading::Symbol<FnLibraryVersionMajor> = library
                    .get(b"LibraryVersionMajor\0")
                    .map_err(|e| format!("Failed to find symbol 'LibraryVersionMajor': {}", e))?;
                let fn_library_version_minor: libloading::Symbol<FnLibraryVersionMinor> = library
                    .get(b"LibraryVersionMinor\0")
                    .map_err(|e| format!("Failed to find symbol 'LibraryVersionMinor': {}", e))?;
                let fn_library_version_patch: libloading::Symbol<FnLibraryVersionPatch> = library
                    .get(b"LibraryVersionPatch\0")
                    .map_err(|e| format!("Failed to find symbol 'LibraryVersionPatch': {}", e))?;
                let fn_library_version_text: libloading::Symbol<FnLibraryVersionText> = library
                    .get(b"LibraryVersionText\0")
                    .map_err(|e| format!("Failed to find symbol 'LibraryVersionText': {}", e))?;
                let fn_photo_dna_edge_hash: libloading::Symbol<FnPhotoDnaEdgeHash> = library
                    .get(b"PhotoDnaEdgeHash\0")
                    .map_err(|e| format!("Failed to find symbol 'PhotoDnaEdgeHash': {}", e))?;
                let fn_photo_dna_edge_hash_border: libloading::Symbol<FnPhotoDnaEdgeHashBorder> =
                    library.get(b"PhotoDnaEdgeHashBorder\0").map_err(|e| {
                        format!("Failed to find symbol 'PhotoDnaEdgeHashBorder': {}", e)
                    })?;
                let fn_photo_dna_edge_hash_border_sub: libloading::Symbol<
                    FnPhotoDnaEdgeHashBorderSub,
                > = library
                    .get(b"PhotoDnaEdgeHashBorderSub\0")
                    .map_err(|e| {
                        format!("Failed to find symbol 'PhotoDnaEdgeHashBorderSub': {}", e)
                    })?;
                let fn_photo_dna_edge_hash_sub: libloading::Symbol<FnPhotoDnaEdgeHashSub> =
                    library.get(b"PhotoDnaEdgeHashSub\0").map_err(|e| {
                        format!("Failed to find symbol 'PhotoDnaEdgeHashSub': {}", e)
                    })?;

                // Initialize the library
                let c_lib_dir = CString::new(lib_dir).map_err(|e| e.to_string())?;
                let library_instance = fn_init(c_lib_dir.as_ptr(), max_threads);

                if library_instance.is_null() {
                    return Err("Failed to initialize PhotoDNA library".to_string());
                }

                // Convert symbols to 'static lifetime for storage
                // This is safe because the library will remain loaded for the lifetime of Self
                #[allow(clippy::missing_transmute_annotations)]
                let fn_release = std::mem::transmute(fn_release);
                #[allow(clippy::missing_transmute_annotations)]
                let fn_get_error_number = std::mem::transmute(fn_get_error_number);
                #[allow(clippy::missing_transmute_annotations)]
                let fn_get_error_string = std::mem::transmute(fn_get_error_string);
                #[allow(clippy::missing_transmute_annotations)]
                let fn_library_version = std::mem::transmute(fn_library_version);
                #[allow(clippy::missing_transmute_annotations)]
                let fn_library_version_major = std::mem::transmute(fn_library_version_major);
                #[allow(clippy::missing_transmute_annotations)]
                let fn_library_version_minor = std::mem::transmute(fn_library_version_minor);
                #[allow(clippy::missing_transmute_annotations)]
                let fn_library_version_patch = std::mem::transmute(fn_library_version_patch);
                #[allow(clippy::missing_transmute_annotations)]
                let fn_library_version_text = std::mem::transmute(fn_library_version_text);
                #[allow(clippy::missing_transmute_annotations)]
                let fn_photo_dna_edge_hash = std::mem::transmute(fn_photo_dna_edge_hash);
                #[allow(clippy::missing_transmute_annotations)]
                let fn_photo_dna_edge_hash_border =
                    std::mem::transmute(fn_photo_dna_edge_hash_border);
                #[allow(clippy::missing_transmute_annotations)]
                let fn_photo_dna_edge_hash_border_sub =
                    std::mem::transmute(fn_photo_dna_edge_hash_border_sub);
                #[allow(clippy::missing_transmute_annotations)]
                let fn_photo_dna_edge_hash_sub = std::mem::transmute(fn_photo_dna_edge_hash_sub);

                Ok(Self {
                    _library: library,
                    library_instance,
                    fn_release,
                    fn_get_error_number,
                    fn_get_error_string,
                    fn_library_version,
                    fn_library_version_major,
                    fn_library_version_minor,
                    fn_library_version_patch,
                    fn_library_version_text,
                    fn_photo_dna_edge_hash,
                    fn_photo_dna_edge_hash_border,
                    fn_photo_dna_edge_hash_border_sub,
                    fn_photo_dna_edge_hash_sub,
                })
            }
        }
    }

    /// Returns the raw library instance handle.
    ///
    /// # Safety
    ///
    /// The returned pointer is only valid while this EdgeHashGenerator is alive.
    pub fn raw_instance(&self) -> *mut c_void {
        self.library_instance
    }

    /// Retrieves the last error number from the library.
    pub fn get_error_number(&self) -> i32 {
        unsafe { (self.fn_get_error_number)(self.library_instance) }
    }

    /// Returns a human-readable description for an error code.
    ///
    /// Returns `None` if the error code is unknown.
    pub fn get_error_string(&self, error: i32) -> Option<&str> {
        unsafe {
            let ptr = (self.fn_get_error_string)(self.library_instance, error);
            if ptr.is_null() {
                None
            } else {
                CStr::from_ptr(ptr).to_str().ok()
            }
        }
    }

    /// Returns the library version as a packed integer.
    ///
    /// High 16 bits = major, low 16 bits = minor.
    pub fn library_version(&self) -> i32 {
        unsafe { (self.fn_library_version)(self.library_instance) }
    }

    /// Returns the major version number.
    pub fn library_version_major(&self) -> i32 {
        unsafe { (self.fn_library_version_major)(self.library_instance) }
    }

    /// Returns the minor version number.
    pub fn library_version_minor(&self) -> i32 {
        unsafe { (self.fn_library_version_minor)(self.library_instance) }
    }

    /// Returns the patch version number.
    pub fn library_version_patch(&self) -> i32 {
        unsafe { (self.fn_library_version_patch)(self.library_instance) }
    }

    /// Returns the library version as a human-readable string.
    pub fn library_version_text(&self) -> Option<&str> {
        unsafe {
            let ptr = (self.fn_library_version_text)(self.library_instance);
            if ptr.is_null() {
                None
            } else {
                CStr::from_ptr(ptr).to_str().ok()
            }
        }
    }

    /// Computes the PhotoDNA Edge Hash of an image.
    ///
    /// # Parameters
    ///
    /// - `image_data`: Pointer to pixel data in the format specified by `options`.
    /// - `hash_value`: Output buffer for the computed hash. Must be at least
    ///   [`PHOTODNA_HASH_SIZE_MAX`] bytes.
    /// - `width`: Image width in pixels (minimum 50).
    /// - `height`: Image height in pixels (minimum 50).
    /// - `stride`: Row stride in bytes, or 0 to calculate from dimensions.
    /// - `options`: Combination of [`PhotoDnaOptions`] flags.
    ///
    /// # Returns
    ///
    /// 0 on success, or a negative error code.
    ///
    /// # Safety
    ///
    /// - `image_data` must point to valid pixel data of size `height * stride` bytes.
    /// - `hash_value` must point to a buffer of at least `PHOTODNA_HASH_SIZE_MAX` bytes.
    pub unsafe fn photo_dna_edge_hash(
        &self,
        image_data: *const u8,
        hash_value: *mut u8,
        width: i32,
        height: i32,
        stride: i32,
        options: PhotoDnaOptions,
    ) -> i32 {
        (self.fn_photo_dna_edge_hash)(
            self.library_instance,
            image_data,
            hash_value,
            width,
            height,
            stride,
            options,
        )
    }

    /// Computes the PhotoDNA Edge Hash with border detection.
    ///
    /// Returns hashes for both the original image and the image with
    /// borders removed (if detected).
    ///
    /// # Parameters
    ///
    /// - `image_data`: Pointer to pixel data in the format specified by `options`.
    /// - `hash_results`: Output array for computed hashes (at least 2 entries).
    /// - `max_hash_count`: Size of the `hash_results` array.
    /// - `width`: Image width in pixels (minimum 50).
    /// - `height`: Image height in pixels (minimum 50).
    /// - `stride`: Row stride in bytes, or 0 to calculate from dimensions.
    /// - `options`: Combination of [`PhotoDnaOptions`] flags.
    ///
    /// # Returns
    ///
    /// Number of hashes returned (1 or 2), or a negative error code.
    ///
    /// # Safety
    ///
    /// - `image_data` must point to valid pixel data.
    /// - `hash_results` must point to an array of at least `max_hash_count` elements.
    pub unsafe fn photo_dna_edge_hash_border(
        &self,
        image_data: *const u8,
        hash_results: *mut HashResult,
        max_hash_count: i32,
        width: i32,
        height: i32,
        stride: i32,
        options: PhotoDnaOptions,
    ) -> i32 {
        (self.fn_photo_dna_edge_hash_border)(
            self.library_instance,
            image_data,
            hash_results,
            max_hash_count,
            width,
            height,
            stride,
            options,
        )
    }

    /// Computes the PhotoDNA Edge Hash for a sub-region with border detection.
    ///
    /// # Safety
    ///
    /// - All pointer parameters must be valid.
    /// - The sub-region must be within the image bounds.
    pub unsafe fn photo_dna_edge_hash_border_sub(
        &self,
        image_data: *const u8,
        hash_results: *mut HashResult,
        max_hash_count: i32,
        width: i32,
        height: i32,
        stride: i32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        options: PhotoDnaOptions,
    ) -> i32 {
        (self.fn_photo_dna_edge_hash_border_sub)(
            self.library_instance,
            image_data,
            hash_results,
            max_hash_count,
            width,
            height,
            stride,
            x,
            y,
            w,
            h,
            options,
        )
    }

    /// Computes the PhotoDNA Edge Hash for a sub-region of an image.
    ///
    /// # Safety
    ///
    /// - All pointer parameters must be valid.
    /// - The sub-region must be within the image bounds.
    pub unsafe fn photo_dna_edge_hash_sub(
        &self,
        image_data: *const u8,
        hash_value: *mut u8,
        width: i32,
        height: i32,
        stride: i32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        options: PhotoDnaOptions,
    ) -> i32 {
        (self.fn_photo_dna_edge_hash_sub)(
            self.library_instance,
            image_data,
            hash_value,
            width,
            height,
            stride,
            x,
            y,
            w,
            h,
            options,
        )
    }
}

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
impl Drop for EdgeHashGenerator {
    fn drop(&mut self) {
        unsafe {
            // Release the library instance
            (self.fn_release)(self.library_instance);
            // The library is automatically unloaded when _library is dropped
        }
    }
}

// EdgeHashGenerator is not Send/Sync by default due to raw pointers.
// The library may or may not be thread-safe internally.
// Users should wrap in appropriate synchronization primitives if needed.

// ============================================================================
// WebAssembly Module (BSD and other platforms)
// ============================================================================

/// WebAssembly module support for platforms without native library binaries.
///
/// This module provides the embedded WASM binary for use with a WASM runtime
/// like `wasmtime`. The consuming crate is responsible for instantiating and
/// calling the WASM module.
///
/// Note: This module is only available when the SDK was present at build time
/// and the `wasm` feature is enabled, or when building for BSD targets with SDK.
/// If building without SDK for BSD targets, the WASM module must be loaded at runtime.
#[cfg(any(
    all(
        feature = "wasm",
        not(any(target_os = "windows", target_os = "linux", target_os = "macos"))
    ),
    target_os = "openbsd",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "dragonfly",
))]
pub mod wasm {
    /// The PhotoDNA Edge Hash Generator WebAssembly module bytes.
    ///
    /// This constant contains the complete WASM module that can be instantiated
    /// with a WASM runtime (e.g., `wasmtime`, `wasmer`).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use wasmtime::*;
    ///
    /// let engine = Engine::default();
    /// let module = Module::new(&engine, photodna_sys::wasm::PHOTODNA_WASM_BYTES)?;
    /// // ... instantiate and call functions
    /// ```
    ///
    /// # Note
    ///
    /// The WASM module exports the same functions as the native library.
    /// Consult the PhotoDNA documentation for the expected calling conventions.
    pub const PHOTODNA_WASM_BYTES: &[u8] = include_bytes!(env!("PHOTODNA_WASM_PATH"));

    /// Size of the embedded WASM module in bytes.
    pub const PHOTODNA_WASM_SIZE: usize = PHOTODNA_WASM_BYTES.len();
}

#[cfg(any(
    all(
        feature = "wasm",
        not(any(target_os = "windows", target_os = "linux", target_os = "macos"))
    ),
    target_os = "openbsd",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "dragonfly",
))]
pub use wasm::*;

// ============================================================================
// Utility Functions
// ============================================================================

/// Returns a human-readable description of a PhotoDNA error code.
///
/// This is a compile-time lookup that doesn't require a library instance.
pub const fn error_code_description(code: i32) -> &'static str {
    match code {
        0 => "Success",
        PhotoDna_ErrorUnknown => "An undetermined error occurred",
        PhotoDna_ErrorMemoryAllocationFailed => "Failed to allocate memory",
        PhotoDna_ErrorLibraryFailure => "General failure within the library",
        PhotoDna_ErrorMemoryAccess => "System memory exception occurred",
        PhotoDna_ErrorInvalidHash => "Hash does not conform to PhotoDNA specifications",
        PhotoDna_ErrorHashFormatInvalidCharacters => "Invalid character in Base64 or Hex hash",
        PhotoDna_ErrorImageTooSmall => "Image dimension is less than 50 pixels",
        PhotoDna_ErrorNoBorder => "No border was detected for the image",
        PhotoDna_ErrorBadArgument => "An invalid argument was passed to the function",
        PhotoDna_ErrorImageIsFlat => "Image has few or no gradients",
        PhotoDna_ErrorNoBorderImageTooSmall => "No border; image too small after border removal",
        PhotoDna_ErrorSourceFormatUnknown => "Not a known source image format",
        PhotoDna_ErrorInvalidStride => "Stride should be 0 or >= width in bytes",
        PhotoDna_ErrorInvalidSubImage => "Sub region is not within image boundaries",
        _ => "Unknown error code",
    }
}

/// Returns the expected hash size for the given options.
///
/// # Parameters
///
/// - `options`: The PhotoDNA options flags.
///
/// # Returns
///
/// The hash size in bytes based on the format specified in options.
pub const fn hash_size_for_options(options: PhotoDnaOptions) -> usize {
    let format = options & PhotoDna_HashFormatMask;
    if format == PhotoDna_HashFormatEdgeV2Base64 {
        PHOTODNA_HASH_SIZE_EDGE_V2_BASE64
    } else {
        PHOTODNA_HASH_SIZE_EDGE_V2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_result_size() {
        // Verify the struct is packed correctly
        assert_eq!(
            core::mem::size_of::<HashResult>(),
            4 + 4 + 4 + 4 + 4 + 4 + PHOTODNA_HASH_SIZE_MAX + 4 * 6
        );
    }

    #[test]
    fn test_error_code_descriptions() {
        assert_eq!(error_code_description(0), "Success");
        assert_eq!(
            error_code_description(PhotoDna_ErrorImageTooSmall),
            "Image dimension is less than 50 pixels"
        );
    }

    #[test]
    fn test_hash_size_for_options() {
        assert_eq!(
            hash_size_for_options(PhotoDna_Default),
            PHOTODNA_HASH_SIZE_EDGE_V2
        );
        assert_eq!(
            hash_size_for_options(PhotoDna_HashFormatEdgeV2Base64),
            PHOTODNA_HASH_SIZE_EDGE_V2_BASE64
        );
    }

    #[test]
    fn test_constants() {
        assert_eq!(PHOTODNA_HASH_SIZE_EDGE_V2, 924);
        assert_eq!(PHOTODNA_HASH_SIZE_EDGE_V2_BASE64, 1232);
        assert_eq!(PhotoDna_EdgeV2 as usize, PHOTODNA_HASH_SIZE_EDGE_V2);
    }

    #[test]
    #[cfg(all(
        any(target_os = "windows", target_os = "linux", target_os = "macos"),
        not(photodna_no_sdk)
    ))]
    fn test_sdk_paths() {
        // Verify SDK paths are set at compile time (only for native targets with SDK)
        assert!(!PHOTODNA_SDK_ROOT.is_empty());
        assert!(!PHOTODNA_LIB_DIR.is_empty());
    }
}

# photodna-rs

[![Crates.io](https://img.shields.io/crates/v/photodna.svg)](https://crates.io/crates/photodna)
[![Documentation](https://docs.rs/photodna/badge.svg)](https://docs.rs/photodna)
[![License](https://img.shields.io/crates/l/photodna.svg)](LICENSE)

Safe, high-level Rust bindings for the Microsoft PhotoDNA Edge Hash Generator.

## Overview

PhotoDNA is a perceptual hashing technology developed by Microsoft that creates a compact 924-byte "fingerprint" of an image. This fingerprint can identify visually similar images even after modifications like resizing, cropping, color adjustment, or format conversion.

This workspace provides two crates:

| Crate | Purpose |
|-------|---------|
| [`photodna`](crates/photodna) | Safe, high-level API for hash computation |
| [`photodna-sys`](crates/photodna-sys) | Low-level, unsafe FFI bindings |

## Requirements

**This crate requires the proprietary Microsoft PhotoDNA SDK (not included).**

Set the `PHOTODNA_SDK_ROOT` environment variable before building:

```bash
export PHOTODNA_SDK_ROOT=/path/to/PhotoDNA.EdgeHashGeneration-1.05.001
cargo build
```

## Quick Start

```rust,ignore
use photodna::{Generator, GeneratorOptions, Hash, Result};

fn main() -> Result<()> {
    // Initialize the generator (loads PhotoDNA library)
    let generator = Generator::new(GeneratorOptions::default())?;

    // Load image as raw RGB pixels
    let image_data: Vec<u8> = load_rgb_image("photo.jpg");
    let (width, height) = (640, 480);

    // Compute the hash
    let hash: Hash = generator.compute_hash_rgb(&image_data, width, height)?;

    // Use the hash
    println!("PhotoDNA hash: {}", hash.to_hex());
    Ok(())
}
```

## Architecture

```text
┌─────────────────────────────────────────────────────────────────┐
│                      Your Application                           │
└─────────────────────────────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                   photodna (safe wrapper)                       │
│  Generator, Hash, PixelFormat, PhotoDnaError                    │
└─────────────────────────────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                   photodna-sys (FFI bindings)                   │
│  EdgeHashGenerator, HashResult, constants, FFI types            │
└─────────────────────────────────────────────────────────────────┘
                             │
                   ┌─────────┴─────────┐
                   ▼                   ▼
    ┌──────────────────────┐  ┌─────────────────────┐
    │ Native Library       │  │ WASM Module (BSD)   │
    │ .dll / .so / .dylib  │  │ photoDnaEdgeHash    │
    └──────────────────────┘  └─────────────────────┘
```

## Safety Documentation

This section documents the safety analysis performed on this crate, including audit results, unsafe code analysis, and memory ownership semantics.

### Cargo Audit Results

Last run: January 2026

```
$ cargo audit
Scanning Cargo.lock for vulnerabilities (63 crate dependencies)
✓ No vulnerabilities found
```

All dependencies have been reviewed for known security advisories. No vulnerabilities were detected in the dependency tree.

### Cargo Geiger Analysis

This analysis shows the unsafe code usage in both crates:

#### photodna-sys (FFI bindings)

```
Functions  Expressions  Impls  Traits  Methods  Dependency
0/0        161/169      0/0    0/0     4/4      ☢️  photodna-sys 1.5.1
0/4        115/291      8/12   0/0     11/16    ☢️  └── libloading 0.8.9
0/0        0/0          0/0    0/0     0/0      ❓     └── cfg-if 1.0.4
```

**Unsafe Code Justification (photodna-sys):**

| Location | Lines | Purpose | Safety Rationale |
|----------|-------|---------|-----------------|
| `EdgeHashGenerator::new` | 660-750 | Library loading | Uses `libloading` for dynamic symbol resolution. All symbols are validated before use. Library instance validity checked via null pointer test. |
| `EdgeHashGenerator::photo_dna_*` | 780-900 | FFI calls | Direct calls to C library. Safety guaranteed by caller validation of buffer sizes in the `photodna` wrapper. |
| `EdgeHashGenerator::drop` | 1010-1015 | Resource cleanup | Calls library's release function. Instance pointer validated during construction. |
| `std::mem::transmute` | 720-745 | Symbol lifetime extension | Extends `Symbol<'a>` to `'static` for storage. Safe because `_library` field keeps the library loaded. |

#### photodna (safe wrapper)

```
Functions  Expressions  Impls  Traits  Methods  Dependency
0/0        16/16        1/1    0/0     0/0      ☢️  photodna 1.5.1
```

**Unsafe Code Justification (photodna):**

| Location | Lines | Purpose | Safety Rationale |
|----------|-------|---------|-----------------|
| `Generator::compute_hash_*` | 600-680 | FFI wrapper | All buffer sizes validated before FFI call. Dimensions checked for minimum requirements. Stride calculation ensures sufficient buffer space. |
| `unsafe impl Send for Generator` | 843 | Thread safety | `Generator` owns the library handle exclusively. Single-owner semantics make `Send` safe. Not `Sync` due to potential thread-local state in library. |

### Miri Validation

All safe code paths have been validated with Miri (undefined behavior detector):

```bash
$ rustup run nightly cargo miri test --package photodna-sys
running 4 tests
test tests::test_constants ... ok
test tests::test_error_code_descriptions ... ok
test tests::test_hash_result_size ... ok
test tests::test_hash_size_for_options ... ok

$ rustup run nightly cargo miri test --package photodna
running 33 tests
# All tests pass
```

**Note:** Miri cannot validate FFI calls to the proprietary library, but all Rust-side pointer handling and memory operations pass Miri's checks.

### Memory Ownership Model

This section documents who owns what memory and when.

#### Buffer Ownership

| Buffer | Owner | Lifetime | Notes |
|--------|-------|----------|-------|
| Input image data (`&[u8]`) | Caller | Duration of FFI call | Borrowed immutably. Library does not store reference. |
| Hash output buffer | `Hash` struct | Until `Hash` dropped | Stack-allocated 924-byte array. Zero-copy. |
| `HashResult` array | Caller (stack) | Duration of FFI call | Passed by mutable pointer. Library writes results. |
| Library instance (`*mut c_void`) | `EdgeHashGenerator` | Until `Drop` | Created by library, released on drop. |

#### FFI Call Contracts

All FFI functions follow these contracts:

1. **Image data pointer**: Must point to `height * stride` readable bytes
2. **Hash output pointer**: Must point to `PHOTODNA_HASH_SIZE_MAX` (1232) writable bytes
3. **HashResult array**: Must have at least `max_hash_count` elements
4. **Stride**: Either 0 (auto-calculate) or `>= width * bytes_per_pixel`

The `photodna` crate validates all these requirements before making FFI calls.

#### Thread Safety

| Type | `Send` | `Sync` | Notes |
|------|--------|--------|-------|
| `Generator` | ✅ | ❌ | Owns library handle. Transfer between threads safe. |
| `Hash` | ✅ | ✅ | Pure data, no shared state. |
| `EdgeHashGenerator` | ❌ | ❌ | Raw pointers prevent auto-impl. Wrap in `Mutex` if needed. |

### Fuzz Testing

Hash parsing is fuzz-tested using `cargo-fuzz`:

```bash
cd crates/photodna/fuzz
cargo +nightly fuzz run fuzz_hash_from_hex    # Hex string parsing
cargo +nightly fuzz run fuzz_hash_from_slice  # Byte slice handling
cargo +nightly fuzz run fuzz_hash_roundtrip   # Serialization roundtrip
```

All parsing functions are designed to never panic on arbitrary input.

## Platform Support

| Platform | Architecture | Backend | Status |
|----------|--------------|---------|--------|
| Windows  | x86_64, x86, ARM64 | Native `.dll` | ✅ Supported |
| Linux    | x86_64, x86, ARM64 | Native `.so` | ✅ Supported |
| macOS    | x86_64, ARM64 | Native `.so` | ✅ Supported |
| OpenBSD/FreeBSD | any | WebAssembly | 🔧 Experimental |

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `test-utils` | ❌ | Mock hashes and fixtures for testing |

## Image Requirements

| Requirement | Value |
|-------------|-------|
| Minimum size | 50×50 pixels |
| Supported formats | RGB, RGBA, BGRA, ARGB, ABGR, CMYK, Gray8, Gray32, YCbCr, YUV420P |
| Content | Must have sufficient gradients (flat/solid images will fail) |

## Error Handling

All operations return typed errors:

```rust,ignore
match generator.compute_hash_rgb(&data, width, height) {
    Ok(hash) => println!("Success: {}", hash),
    Err(PhotoDnaError::ImageTooSmall) => eprintln!("Image must be >= 50x50"),
    Err(PhotoDnaError::ImageIsFlat) => eprintln!("Image needs more contrast"),
    Err(e) => eprintln!("Error: {}", e),
}
```

## Testing

```bash
# Run all tests
cargo test

# Run tests with Miri (UB detection)
rustup run nightly cargo miri test

# Run fuzz tests
cd crates/photodna/fuzz
cargo +nightly fuzz run fuzz_hash_from_hex -- -runs=100000

# Security audit
cargo audit

# Unsafe code analysis
cargo geiger
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Disclaimer

This crate provides bindings to the proprietary Microsoft PhotoDNA SDK. The SDK itself is not included and must be obtained separately from Microsoft. PhotoDNA is typically available only to organizations working on child safety initiatives.

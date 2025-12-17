# photodna

Safe, high-level Rust bindings for the Microsoft PhotoDNA Edge Hash Generator.

[![Crates.io](https://img.shields.io/crates/v/photodna.svg)](https://crates.io/crates/photodna)
[![Documentation](https://docs.rs/photodna/badge.svg)](https://docs.rs/photodna)
[![License](https://img.shields.io/crates/l/photodna.svg)](LICENSE)

## Overview

PhotoDNA is a perceptual hashing technology that creates a compact "fingerprint" of an image. This fingerprint can be used to identify visually similar images even after modifications like resizing, cropping, color adjustments, or format conversion.

This crate provides a safe, ergonomic Rust API on top of the low-level `photodna-sys` bindings.

## Features

- **Safe API** – All unsafe FFI operations are encapsulated behind a safe interface
- **Zero-Copy Hashes** – The `Hash` type uses a fixed-size stack array (no heap allocation)
- **Typed Errors** – Comprehensive error handling via `PhotoDnaError`
- **Builder Pattern** – Ergonomic configuration via `GeneratorOptions` and `HashOptions`

## Requirements

**This crate requires the proprietary Microsoft PhotoDNA SDK.**

Before building, you must set the `PHOTODNA_SDK_ROOT` environment variable to point to the SDK installation directory:

```bash
export PHOTODNA_SDK_ROOT=/path/to/PhotoDNA.EdgeHashGeneration-1.05.001
```

See the [`photodna-sys`](../photodna-sys/README.md) crate documentation for detailed setup instructions.

## Quick Start

```rust
use photodna::{Generator, GeneratorOptions, Hash, Result};

fn main() -> Result<()> {
    // Initialize the generator (loads the PhotoDNA library)
    let generator = Generator::new(GeneratorOptions::default())?;

    // Load your image as raw RGB pixels (e.g., using the `image` crate)
    let image_data: Vec<u8> = load_image_as_rgb("photo.jpg");
    let width = 640;
    let height = 480;

    // Compute the hash
    let hash: Hash = generator.compute_hash_rgb(&image_data, width, height)?;

    // Use the hash (e.g., store in database as hex)
    println!("Hash: {}", hash.to_hex());

    Ok(())
}
```

## API Overview

### Generator

The main entry point. Manages the PhotoDNA library lifecycle:

```rust
use photodna::{Generator, GeneratorOptions};

let generator = Generator::new(
    GeneratorOptions::new()
        .max_threads(4)
)?;

// Check library version
println!("PhotoDNA v{}", generator.library_version_text().unwrap_or("?"));
```

### Hash

A fixed-size (924 bytes) perceptual hash with zero-copy semantics:

```rust
use photodna::Hash;

// Format as hex for storage
let hex: String = hash.to_hex();

// Parse from hex
let hash = Hash::from_hex(&hex).unwrap();

// Access raw bytes
let bytes: &[u8] = hash.as_bytes();

// Use as HashMap key (implements Hash trait)
use std::collections::HashSet;
let mut seen = HashSet::new();
seen.insert(hash);
```

### Pixel Formats

Multiple pixel formats are supported:

```rust
use photodna::{HashOptions, PixelFormat};

let options = HashOptions::new()
    .pixel_format(PixelFormat::Bgra)  // Common in Windows
    .remove_border(true);             // Auto-detect and remove borders
```

Supported formats:
- `Rgb`, `Bgr` (3 bytes/pixel)
- `Rgba`, `Bgra`, `Argb`, `Abgr` (4 bytes/pixel)
- `Gray8` (1 byte/pixel), `Gray32` (4 bytes/pixel)
- `Cmyk`, `YCbCr`, `Yuv420p`

### Border Detection

Automatically detect and remove borders from images:

```rust
let result = generator.compute_hash_with_border_detection(
    &image_data,
    width,
    height,
    HashOptions::default(),
)?;

println!("Primary hash: {}", result.primary.to_hex());

if let Some(borderless) = result.borderless {
    println!("Borderless hash: {}", borderless.to_hex());
    if let Some((x, y, w, h)) = result.content_region {
        println!("Content region: {}x{} at ({}, {})", w, h, x, y);
    }
}
```

## Error Handling

All operations return `Result<T, PhotoDnaError>`:

```rust
use photodna::{PhotoDnaError, Result};

fn process_image(data: &[u8], w: u32, h: u32) -> Result<()> {
    match generator.compute_hash_rgb(data, w, h) {
        Ok(hash) => println!("Success: {}", hash),
        Err(PhotoDnaError::ImageTooSmall) => {
            eprintln!("Image must be at least 50x50 pixels");
        }
        Err(PhotoDnaError::ImageIsFlat) => {
            eprintln!("Image has insufficient detail for hashing");
        }
        Err(e) => return Err(e),
    }
    Ok(())
}
```

## Thread Safety

`Generator` implements `Send` but not `Sync`. Options for concurrent use:

```rust
// Option 1: One generator per thread
let generator = Generator::new(GeneratorOptions::new().max_threads(1))?;

// Option 2: Shared with Mutex
use std::sync::{Arc, Mutex};
let generator = Arc::new(Mutex::new(Generator::new(GeneratorOptions::default())?));

// Option 3: Use max_threads for internal parallelism
let generator = Generator::new(GeneratorOptions::new().max_threads(8))?;
// Internal operations can use up to 8 threads
```

## Platform Support

| Platform | Architecture | Support |
|----------|--------------|---------|
| Windows  | x86_64, x86, arm64 | ✅ Native library |
| Linux    | x86_64, x86, arm64 | ✅ Native library |
| macOS    | x86_64, arm64 | ✅ Native library |
| BSD      | any | 🔧 WebAssembly (via `photodna-sys`) |

## License

This crate is licensed under MIT OR Apache-2.0.

**Note:** The PhotoDNA library itself is proprietary software from Microsoft. Usage of PhotoDNA requires a separate license agreement with Microsoft.

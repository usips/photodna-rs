# photodna-sys

Low-level, unsafe FFI bindings to the Microsoft PhotoDNA Edge Hash Generator library.

## Overview

This crate provides raw Rust bindings to the proprietary Microsoft PhotoDNA library,
which computes perceptual hashes of images for content identification purposes.
PhotoDNA is commonly used by organizations to detect known illegal content.

**Note:** This crate only provides the FFI bindings. You must obtain the PhotoDNA SDK
separately from Microsoft.

## Requirements

### PhotoDNA SDK

This crate requires the proprietary Microsoft PhotoDNA Edge Hash Generator SDK
(version 1.05.001). Contact Microsoft to obtain a license and the SDK files.

Set the `PHOTODNA_SDK_ROOT` environment variable to point to the SDK installation:

```bash
export PHOTODNA_SDK_ROOT=/path/to/PhotoDNA.EdgeHashGeneration-1.05.001
```

Expected directory structure:
```
$PHOTODNA_SDK_ROOT/
├── clientlibrary/
│   ├── libEdgeHashGenerator.so.1.05          # Linux x86_64
│   ├── libEdgeHashGenerator-arm64.so.1.05    # Linux ARM64
│   ├── libEdgeHashGenerator-macos.so.1.05    # macOS x86_64
│   ├── libEdgeHashGenerator-arm64-macos.so.1.05  # macOS ARM64
│   ├── c/
│   │   └── PhotoDnaEdgeHashGenerator.h
│   └── ...
└── webassembly/
    └── photoDnaEdgeHash.wasm                 # For BSD platforms
```

### Build Dependencies

- Rust 1.70 or later
- For the `bindgen` feature: LLVM/Clang development headers

## Platform Support

| Platform | Architecture | Support |
|----------|--------------|---------|
| Windows  | x86_64, x86, ARM64 | Native library |
| Linux    | x86_64, x86, ARM64 | Native library |
| macOS    | x86_64, ARM64 | Native library |
| OpenBSD  | any | WebAssembly (requires `wasm` feature) |
| FreeBSD  | any | WebAssembly (requires `wasm` feature) |

## Features

- **`native`** (default): Links against native dynamic libraries (`.dll`/`.so`).
- **`wasm`**: Embeds the WebAssembly module for platforms without native library support.
- **`bindgen`**: Regenerates bindings from C headers at build time (requires clang).

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
photodna-sys = "1.5"
```

### Example

```rust
use photodna_sys::*;
use std::ffi::CString;

fn compute_hash(image_data: &[u8], width: i32, height: i32) -> Result<Vec<u8>, i32> {
    unsafe {
        let library_path = CString::new(".").unwrap();
        let mut error: i32 = 0;
        
        // Initialize the library
        let instance = EdgeHashGeneratorInit_internal(
            library_path.as_ptr(),
            4, // max concurrent threads
        );
        
        if instance.is_null() {
            return Err(PhotoDna_ErrorLibraryFailure);
        }
        
        // Prepare output buffer
        let mut hash = vec![0u8; PHOTODNA_HASH_SIZE_MAX];
        
        // Compute the hash (RGB format, calculate stride automatically)
        let result = PhotoDnaEdgeHash(
            instance,
            image_data.as_ptr(),
            hash.as_mut_ptr(),
            width,
            height,
            0, // auto-calculate stride
            PhotoDna_Default,
        );
        
        // Clean up
        EdgeHashGeneratorRelease(instance);
        
        if result < 0 {
            Err(result)
        } else {
            hash.truncate(PHOTODNA_HASH_SIZE_EDGE_V2);
            Ok(hash)
        }
    }
}
```

### BSD / WebAssembly Usage

On BSD platforms, use a WASM runtime to execute the PhotoDNA module:

```rust
#[cfg(any(target_os = "openbsd", target_os = "freebsd"))]
fn example() {
    use photodna_sys::wasm::PHOTODNA_WASM_BYTES;
    
    // Use wasmtime, wasmer, or another WASM runtime
    // to instantiate and call the module
    println!("WASM module size: {} bytes", PHOTODNA_WASM_BYTES.len());
}
```

## Safety

All FFI functions in this crate are `unsafe`. Callers must ensure:

- The library has been properly initialized via `EdgeHashGeneratorInit`.
- All pointers passed to functions are valid and point to sufficient memory.
- Image data buffers match the specified dimensions, stride, and pixel format.
- The library instance is not used after `EdgeHashGeneratorRelease` is called.

## Error Handling

Functions return negative error codes on failure. Use `error_code_description()`
to get a human-readable description:

```rust
use photodna_sys::{error_code_description, PhotoDna_ErrorImageTooSmall};

let description = error_code_description(PhotoDna_ErrorImageTooSmall);
assert_eq!(description, "Image dimension is less than 50 pixels");
```

## Version Compatibility

| Crate Version | SDK Version |
|---------------|-------------|
| 1.5.x         | 1.05.001    |

## License

The Rust bindings in this crate are provided under the MIT license.
However, the PhotoDNA library itself is proprietary software owned by Microsoft.
You must obtain a separate license from Microsoft to use the PhotoDNA SDK.

## Related Projects

- [photodna](https://crates.io/crates/photodna) - Safe, high-level wrapper

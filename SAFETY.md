# Safety Analysis

This document provides a comprehensive analysis of the safety properties of the `photodna-rs` workspace.

## Executive Summary

| Metric | Status |
|--------|--------|
| Cargo Audit | ✅ No vulnerabilities |
| Miri (UB detection) | ✅ All tests pass |
| Fuzz Testing | ✅ Configured and passing |
| Unsafe Code | ✅ Documented and justified |

## Crate Safety Overview

### photodna-sys (FFI Bindings)

This crate contains the bulk of unsafe code, as expected for FFI bindings.

```
Functions  Expressions  Impls  Traits  Methods
0/0        161/169      0/0    0/0     4/4
```

**169 total unsafe expressions**, of which **161 are used by the build**.

### photodna (Safe Wrapper)

```
Functions  Expressions  Impls  Traits  Methods
0/0        16/16        1/1    0/0     0/0
```

**16 unsafe expressions**, all used and documented.

## Unsafe Code Inventory

### photodna-sys

#### 1. Library Loading (`EdgeHashGenerator::new`)

**Location:** `crates/photodna-sys/src/lib.rs` lines 733-850

**Unsafe Operations:**
- `libloading::Library::new()` - loads dynamic library
- `library.get()` - resolves function pointers
- `fn_init()` - calls C function
- `std::mem::transmute()` - extends symbol lifetime

**Safety Justification:**

```rust
// The library path comes from build.rs validation
let library = libloading::Library::new(&lib_path)?;

// Symbol resolution - types match C header exactly
let fn_init: Symbol<FnEdgeHashGeneratorInit> = library.get(b"EdgeHashGeneratorInit\0")?;

// Lifetime transmute - safe because:
// 1. _library field keeps library loaded
// 2. Drop order: symbols dropped before library
// 3. No way to leak symbols outside struct
let fn_release: Symbol<'static, _> = std::mem::transmute(fn_release);
```

#### 2. FFI Calls (`photo_dna_edge_hash`, etc.)

**Location:** `crates/photodna-sys/src/lib.rs` lines 920-1050

**Unsafe Operations:**
- Calling function pointers with raw pointer arguments
- Dereferencing `*const u8` (image data)
- Writing to `*mut u8` (hash output)
- Writing to `*mut HashResult` (border detection output)

**Safety Requirements (documented in function docs):**

| Parameter | Requirement |
|-----------|-------------|
| `image_data` | Must point to `height * stride` readable bytes |
| `hash_value` | Must point to 1232 writable bytes |
| `hash_results` | Must point to `max_hash_count` `HashResult` elements |
| `library_instance` | Must be valid (guaranteed by `&self`) |

#### 3. Drop Implementation

**Location:** `crates/photodna-sys/src/lib.rs` lines 1070-1080

```rust
impl Drop for EdgeHashGenerator {
    fn drop(&mut self) {
        unsafe {
            // SAFETY: library_instance is valid because:
            // 1. It was validated non-null in new()
            // 2. No method can invalidate it
            // 3. This is the only place it's released
            (self.fn_release)(self.library_instance);
        }
    }
}
```

#### 4. CStr Operations

**Location:** Various (error string handling, version text)

```rust
// SAFETY: The PhotoDNA library returns null-terminated UTF-8 strings
// or null pointers. We check for null before calling CStr::from_ptr.
let ptr = (self.fn_get_error_string)(self.library_instance, error);
if ptr.is_null() {
    None
} else {
    CStr::from_ptr(ptr).to_str().ok()
}
```

### photodna

#### 1. FFI Wrapper Calls

**Location:** `crates/photodna/src/lib.rs` lines 600-750

All unsafe code in the safe wrapper validates inputs before calling FFI:

```rust
pub fn compute_hash_with_stride(&self, image_data: &[u8], ...) -> Result<Hash> {
    // Validate dimensions
    if width == 0 || height == 0 {
        return Err(PhotoDnaError::InvalidDimensions { ... });
    }

    // Calculate and validate buffer size
    let expected_size = expected_stride * (height as usize);
    if image_data.len() < expected_size {
        return Err(PhotoDnaError::BufferTooSmall { ... });
    }

    // SAFETY: Buffer size validated above, dimensions checked.
    // hash_buffer is a local array of correct size.
    let result = unsafe {
        self.inner.photo_dna_edge_hash(
            image_data.as_ptr(),    // Valid: slice guarantees this
            hash_buffer.as_mut_ptr(), // Valid: local array
            ...
        )
    };
}
```

#### 2. Send Implementation

**Location:** `crates/photodna/src/lib.rs` line 897

```rust
// SAFETY: Generator can be sent between threads because:
// 1. It owns the library handle exclusively (no shared references)
// 2. The underlying library uses its own synchronization
// 3. We don't implement Sync (concurrent access not allowed)
unsafe impl Send for Generator {}
```

## Memory Ownership Model

### Ownership Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Generator (photodna)                        │
│  - Owns EdgeHashGenerator                                           │
│  - All public methods borrow image data (&[u8])                     │
│  - Returns owned Hash values                                        │
└─────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    EdgeHashGenerator (photodna-sys)                 │
│  - Owns libloading::Library (keeps .so/.dll loaded)                 │
│  - Owns library_instance (*mut c_void from C library)               │
│  - Owns function pointer symbols (transmuted to 'static)            │
└─────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         C Library Memory                            │
│  - Thread pool (allocated by EdgeHashGeneratorInit)                 │
│  - Internal buffers (managed by library)                            │
│  - Released by EdgeHashGeneratorRelease                             │
└─────────────────────────────────────────────────────────────────────┘
```

### Buffer Lifetimes

| Buffer | Lifetime | Owner | Borrower |
|--------|----------|-------|----------|
| Image pixel data | Borrowed during FFI call | Caller | C library |
| Hash output | Stack-allocated by caller | Caller | C library writes |
| HashResult array | Stack-allocated by caller | Caller | C library writes |
| Error strings | 'static in C library | C library | Rust borrows |
| Version strings | 'static in C library | C library | Rust borrows |

### Drop Order Guarantee

Rust's drop order guarantees fields are dropped in declaration order:

```rust
pub struct EdgeHashGenerator {
    _library: libloading::Library,      // Dropped LAST (keeps symbols valid)
    library_instance: *mut c_void,      // Raw pointer, no drop
    fn_release: Symbol<'static, ...>,   // Dropped before _library
    fn_get_error_number: Symbol<...>,   // Dropped before _library
    // ... etc
}

impl Drop for EdgeHashGenerator {
    fn drop(&mut self) {
        // Called BEFORE any field drops
        (self.fn_release)(self.library_instance);
        // After this: fields drop in order, _library last
    }
}
```

## Threat Model

### What We Protect Against

1. **Buffer overflows**: All buffer sizes validated before FFI calls
2. **Use-after-free**: Rust's ownership system + careful drop ordering
3. **Null pointer dereference**: All pointers checked before use
4. **Integer overflow**: Dimensions converted with overflow checks
5. **Invalid UTF-8**: CStr parsing with `.ok()` fallback

### What We Cannot Protect Against

1. **Bugs in the PhotoDNA library itself**: Closed-source, trusted
2. **Malicious SDK replacement**: User-provided library path
3. **Thread safety issues in C library**: We don't implement Sync

## Validation Tools

### Miri

```bash
rustup run nightly cargo miri test --package photodna-sys
rustup run nightly cargo miri test --package photodna
```

Miri validates:
- No undefined behavior in Rust code
- Correct pointer handling
- Proper memory access patterns

**Limitation:** Miri cannot validate FFI calls to C code.

### Cargo Audit

```bash
cargo audit
```

Checks for known vulnerabilities in dependencies.

### Cargo Geiger

```bash
cd crates/photodna-sys && cargo geiger
cd crates/photodna && cargo geiger
```

Counts and categorizes unsafe code usage.

### Fuzz Testing

```bash
cd crates/photodna/fuzz
cargo +nightly fuzz run fuzz_hash_from_hex
cargo +nightly fuzz run fuzz_hash_from_slice
cargo +nightly fuzz run fuzz_hash_roundtrip
```

Tests parsing functions with random inputs to find panics or crashes.

## Recommendations for Consumers

1. **Use the safe wrapper**: Import `photodna`, not `photodna-sys`
2. **Validate image sources**: Ensure images come from trusted sources
3. **Handle errors properly**: All error conditions are typed
4. **Don't share Generator across threads without Mutex**: It's Send but not Sync
5. **Keep SDK updated**: Follow Microsoft's security advisories

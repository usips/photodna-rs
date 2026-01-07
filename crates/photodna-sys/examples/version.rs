//! Example demonstrating basic usage of photodna-sys
//!
//! This example shows how to initialize the library and query its version.
//!
//! Run with:
//! ```bash
//! PHOTODNA_SDK_ROOT=/path/to/PhotoDNA.EdgeHashGeneration-1.05.001 cargo run --example version
//! ```

use photodna_sys::*;

fn main() {
    println!("PhotoDNA-sys Example");
    println!("====================");
    println!();

    // SDK paths only available on native targets where SDK was present at build time
    #[cfg(all(
        any(target_os = "windows", target_os = "linux", target_os = "macos"),
        not(photodna_no_sdk)
    ))]
    {
        println!("SDK Root: {}", PHOTODNA_SDK_ROOT);
        println!("Library Directory: {}", PHOTODNA_LIB_DIR);
        println!();
    }

    #[cfg(any(
        not(any(target_os = "windows", target_os = "linux", target_os = "macos")),
        photodna_no_sdk
    ))]
    {
        println!("SDK paths not available (build without SDK or BSD/WASM-only build)");
        println!();
    }

    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    {
        println!("Attempting to load PhotoDNA library...");

        match EdgeHashGenerator::new(None, 4) {
            Ok(lib) => {
                println!("✓ Library loaded successfully!");
                println!();
                println!("Library Version Information:");
                println!("  Version (packed): 0x{:08x}", lib.library_version());
                println!("  Major: {}", lib.library_version_major());
                println!("  Minor: {}", lib.library_version_minor());
                println!("  Patch: {}", lib.library_version_patch());
                if let Some(text) = lib.library_version_text() {
                    println!("  Text: {}", text);
                }
                println!();
                println!("Hash sizes:");
                println!("  Edge V2 (binary): {} bytes", PHOTODNA_HASH_SIZE_EDGE_V2);
                println!(
                    "  Edge V2 (base64): {} bytes",
                    PHOTODNA_HASH_SIZE_EDGE_V2_BASE64
                );
            }
            Err(e) => {
                eprintln!("✗ Failed to load library: {}", e);
                std::process::exit(1);
            }
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        println!("Native library not available on this platform.");
        println!("Use the 'wasm' feature with a WASM runtime for BSD platforms.");
    }
}

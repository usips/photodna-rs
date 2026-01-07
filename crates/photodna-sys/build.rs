//! Build script for photodna-sys
//!
//! This script handles:
//! - Platform detection and native library path verification
//! - Optional bindgen-based header parsing
//! - WebAssembly module path configuration for BSD/wasm targets
//!
//! Note: The PhotoDNA library is designed for dynamic loading at runtime
//! (via dlopen/LoadLibrary), not compile-time linking. This build script
//! verifies the SDK exists and sets up environment variables for runtime loading.

use std::env;
use std::path::{Path, PathBuf};

/// The expected library version string (used in filenames)
const LIBRARY_VERSION: &str = "1.05";

fn main() {
    // Declare custom cfg for check-cfg lint
    println!("cargo::rustc-check-cfg=cfg(photodna_no_sdk)");

    // Re-run if environment changes
    println!("cargo:rerun-if-env-changed=PHOTODNA_SDK_ROOT");
    println!("cargo:rerun-if-env-changed=DOCS_RS");
    println!("cargo:rerun-if-changed=build.rs");

    // Skip SDK verification in docs.rs builds or when explicitly requested.
    // This allows the crate to be published and documented without the proprietary SDK.
    if env::var("DOCS_RS").is_ok() {
        eprintln!("cargo:warning=Building for docs.rs - skipping SDK verification");
        println!("cargo:rustc-cfg=photodna_no_sdk");
        return;
    }

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    // Determine if this is a BSD/WASM-only target
    let is_bsd_target = matches!(
        target_os.as_str(),
        "openbsd" | "freebsd" | "netbsd" | "dragonfly"
    );

    // Determine if this is a native-supported target
    let is_native_target = matches!(target_os.as_str(), "windows" | "linux" | "macos");

    // Try to get SDK root - required for native targets, optional for BSD
    let sdk_root = get_sdk_root_optional();

    match (&sdk_root, is_native_target, is_bsd_target) {
        // Native target with SDK - verify and configure
        (Some(root), true, _) => {
            match target_os.as_str() {
                "windows" => verify_windows(root, &target_arch),
                "linux" => verify_linux(root, &target_arch),
                "macos" => verify_macos(root, &target_arch),
                _ => {}
            }

            // Export SDK paths
            println!("cargo:rustc-env=PHOTODNA_SDK_ROOT={}", root.display());
            println!(
                "cargo:rustc-env=PHOTODNA_LIB_DIR={}",
                root.join("clientlibrary").display()
            );

            #[cfg(feature = "bindgen")]
            generate_bindings(root);
        }

        // Native target without SDK - warn and set no_sdk cfg
        // This allows the crate to compile (for publishing, docs, etc.)
        // but the library will need the SDK at runtime
        (None, true, _) => {
            eprintln!(
                "cargo:warning=photodna-sys: PHOTODNA_SDK_ROOT not set. \
                 The PhotoDNA SDK will need to be available at runtime."
            );
            println!("cargo:rustc-cfg=photodna_no_sdk");
        }

        // BSD target with SDK - configure WASM
        (Some(root), false, true) => {
            let wasm_path = root.join("webassembly").join("photoDnaEdgeHash.wasm");
            if !wasm_path.exists() {
                panic!(
                    "photodna-sys: WebAssembly module not found at: {}\n\
                     Ensure PHOTODNA_SDK_ROOT points to the PhotoDNA SDK root directory.",
                    wasm_path.display()
                );
            }
            println!("cargo:rustc-env=PHOTODNA_WASM_PATH={}", wasm_path.display());
            println!("cargo:rustc-env=PHOTODNA_SDK_ROOT={}", root.display());
            println!(
                "cargo:rustc-env=PHOTODNA_LIB_DIR={}",
                root.join("clientlibrary").display()
            );
        }

        // BSD target without SDK - use embedded WASM path placeholder
        (None, false, true) => {
            // For BSD without SDK at build time, we'll require WASM to be provided
            // at runtime or embedded separately. Set a placeholder.
            eprintln!(
                "cargo:warning=Building for BSD target without PHOTODNA_SDK_ROOT. \
                 The WASM module will need to be provided at runtime."
            );
            // Use empty placeholders - the code will need to handle this
            println!("cargo:rustc-cfg=photodna_no_sdk");
        }

        // Unknown target
        (sdk, false, false) => {
            if sdk.is_none() {
                eprintln!(
                    "cargo:warning=Unsupported target OS '{}' and PHOTODNA_SDK_ROOT not set. \
                     Consider enabling the 'wasm' feature for runtime emulation.",
                    target_os
                );
                println!("cargo:rustc-cfg=photodna_no_sdk");
            } else {
                let root = sdk.as_ref().unwrap();
                println!("cargo:rustc-env=PHOTODNA_SDK_ROOT={}", root.display());
                println!(
                    "cargo:rustc-env=PHOTODNA_LIB_DIR={}",
                    root.join("clientlibrary").display()
                );
            }
        }
    }

    // For WASM feature on any platform, try to set WASM path if SDK is available
    if cfg!(feature = "wasm") {
        if let Some(ref root) = sdk_root {
            let wasm_path = root.join("webassembly").join("photoDnaEdgeHash.wasm");
            if wasm_path.exists() {
                println!("cargo:rustc-env=PHOTODNA_WASM_PATH={}", wasm_path.display());
            }
        }
    }
}

/// Retrieves the SDK root directory from the environment variable.
/// Returns None if not set, rather than panicking.
fn get_sdk_root_optional() -> Option<PathBuf> {
    match env::var("PHOTODNA_SDK_ROOT") {
        Ok(path) => {
            let sdk_path = PathBuf::from(&path);
            if !sdk_path.exists() {
                eprintln!(
                    "cargo:warning=PHOTODNA_SDK_ROOT directory does not exist: {}",
                    path
                );
                return None;
            }
            if !sdk_path.join("clientlibrary").exists() {
                eprintln!(
                    "cargo:warning=PHOTODNA_SDK_ROOT does not contain 'clientlibrary' directory: {}",
                    path
                );
                return None;
            }
            Some(sdk_path)
        }
        Err(_) => None,
    }
}

/// Verifies library exists for Windows targets.
fn verify_windows(sdk_root: &Path, target_arch: &str) {
    let lib_dir = sdk_root.join("clientlibrary");

    // Determine architecture-specific library name suffix
    let arch_suffix = match target_arch {
        "x86_64" | "amd64" => "",
        "aarch64" => "-arm64",
        "arm" => "-arm32",
        "x86" => "-x86",
        _ => "",
    };

    let lib_name = format!(
        "libEdgeHashGenerator{}.{}.dll",
        arch_suffix, LIBRARY_VERSION
    );
    let lib_path = lib_dir.join(&lib_name);

    if !lib_path.exists() {
        panic!(
            "photodna-sys: Windows library not found: {}\n\
             Expected location: {}\n\
             Available files in clientlibrary/:\n{}",
            lib_name,
            lib_path.display(),
            list_library_files(&lib_dir)
        );
    }

    // Export the library path for runtime loading
    println!("cargo:rustc-env=PHOTODNA_NATIVE_LIB={}", lib_path.display());
}

/// Verifies library exists for Linux targets.
fn verify_linux(sdk_root: &Path, target_arch: &str) {
    let lib_dir = sdk_root.join("clientlibrary");

    // Determine architecture-specific library name suffix
    let arch_suffix = match target_arch {
        "x86_64" | "amd64" => "",
        "aarch64" => "-arm64",
        "arm" => "-arm32",
        "x86" => "-x86",
        _ => "",
    };

    let lib_name = format!("libEdgeHashGenerator{}.so.{}", arch_suffix, LIBRARY_VERSION);
    let lib_path = lib_dir.join(&lib_name);

    if !lib_path.exists() {
        panic!(
            "photodna-sys: Linux library not found: {}\n\
             Expected location: {}\n\
             Available files in clientlibrary/:\n{}",
            lib_name,
            lib_path.display(),
            list_library_files(&lib_dir)
        );
    }

    // Export the library path for runtime loading
    println!("cargo:rustc-env=PHOTODNA_NATIVE_LIB={}", lib_path.display());
}

/// Verifies library exists for macOS targets.
fn verify_macos(sdk_root: &Path, target_arch: &str) {
    let lib_dir = sdk_root.join("clientlibrary");

    // Determine architecture-specific library name
    let arch_suffix = match target_arch {
        "aarch64" => "-arm64-macos",
        _ => "-macos",
    };

    let lib_name = format!("libEdgeHashGenerator{}.so.{}", arch_suffix, LIBRARY_VERSION);
    let lib_path = lib_dir.join(&lib_name);

    if !lib_path.exists() {
        panic!(
            "photodna-sys: macOS library not found: {}\n\
             Expected location: {}\n\
             Available files in clientlibrary/:\n{}",
            lib_name,
            lib_path.display(),
            list_library_files(&lib_dir)
        );
    }

    // Export the library path for runtime loading
    println!("cargo:rustc-env=PHOTODNA_NATIVE_LIB={}", lib_path.display());
}

/// Lists library files in a directory for diagnostic output.
fn list_library_files(dir: &Path) -> String {
    if let Ok(entries) = std::fs::read_dir(dir) {
        entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                name.contains("EdgeHashGenerator")
                    || name.ends_with(".so")
                    || name.ends_with(".dll")
            })
            .map(|e| format!("  - {}", e.file_name().to_string_lossy()))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        "  (unable to read directory)".to_string()
    }
}

/// Generates Rust bindings from C headers using bindgen.
#[cfg(feature = "bindgen")]
fn generate_bindings(sdk_root: &Path) {
    let header_dir = sdk_root.join("clientlibrary").join("c");
    let header_path = header_dir.join("PhotoDnaEdgeHashGenerator.h");

    if !header_path.exists() {
        panic!(
            "photodna-sys: C header not found: {}\n\
             Ensure PHOTODNA_SDK_ROOT points to a valid SDK installation.",
            header_path.display()
        );
    }

    println!("cargo:rerun-if-changed={}", header_path.display());

    let bindings = bindgen::Builder::default()
        .header(header_path.to_string_lossy())
        .clang_arg(format!("-I{}", header_dir.display()))
        // Parse only the necessary types and constants, skip the inline wrapper functions
        .allowlist_type("HashResult")
        .allowlist_type("PhotoDnaOptions")
        .allowlist_type("ErrorCode")
        .allowlist_type("HashSize")
        .allowlist_type("ErrorCodeEnum")
        .allowlist_type("HashSizeEnum")
        .allowlist_type("PhotoDnaOptionsEnum")
        // Allow all PhotoDna constants
        .allowlist_var("PhotoDna_.*")
        // Derive useful traits
        .derive_default(true)
        .derive_eq(true)
        .derive_hash(true)
        .derive_ord(true)
        .derive_copy(true)
        .derive_debug(true)
        // Generate documentation from C comments
        .generate_comments(true)
        .clang_arg("-fparse-all-comments")
        // Use core types instead of std where possible
        .use_core()
        // Layout tests can be useful for debugging
        .layout_tests(true)
        // Generate Rust code
        .generate()
        .expect("Unable to generate bindings");

    // Write to the src directory for version control
    // In production, you might write to OUT_DIR instead
    let out_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("src")
        .join("bindings.rs");

    bindings
        .write_to_file(&out_path)
        .expect("Couldn't write bindings!");

    println!(
        "cargo:warning=Generated new bindings at {}",
        out_path.display()
    );
}

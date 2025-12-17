//! Example demonstrating hash computation with photodna-sys
//!
//! This example creates a simple test image and computes its PhotoDNA hash.
//!
//! Run with:
//! ```bash
//! PHOTODNA_SDK_ROOT=/path/to/PhotoDNA.EdgeHashGeneration-1.05.001 cargo run --example hash
//! ```

use photodna_sys::*;

fn main() {
    println!("PhotoDNA Hash Computation Example");
    println!("==================================");
    println!();

    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    {
        // Load the library
        let lib = match EdgeHashGenerator::new(None, 4) {
            Ok(lib) => {
                println!(
                    "✓ Library loaded: version {}",
                    lib.library_version_text().unwrap_or("unknown")
                );
                lib
            }
            Err(e) => {
                eprintln!("✗ Failed to load library: {}", e);
                std::process::exit(1);
            }
        };

        // Create a simple test image (gradient pattern)
        // Images must be at least 50x50 pixels
        let width = 100;
        let height = 100;
        let bytes_per_pixel = 3; // RGB format
        let stride = width * bytes_per_pixel;

        // Generate a simple gradient image
        let mut image_data = vec![0u8; (height * stride) as usize];
        for y in 0..height {
            for x in 0..width {
                let offset = ((y * stride) + (x * bytes_per_pixel)) as usize;
                // Create a diagonal gradient pattern
                let r = ((x * 255) / width) as u8;
                let g = ((y * 255) / height) as u8;
                let b = (((x + y) * 255) / (width + height)) as u8;
                image_data[offset] = r;
                image_data[offset + 1] = g;
                image_data[offset + 2] = b;
            }
        }

        println!(
            "Created test image: {}x{} RGB ({} bytes)",
            width,
            height,
            image_data.len()
        );
        println!();

        // Compute the hash
        let mut hash = [0u8; PHOTODNA_HASH_SIZE_MAX];

        let result = unsafe {
            lib.photo_dna_edge_hash(
                image_data.as_ptr(),
                hash.as_mut_ptr(),
                width,
                height,
                stride,
                PhotoDna_Default | PhotoDna_Rgb,
            )
        };

        if result < 0 {
            eprintln!(
                "✗ Hash computation failed: {} (code {})",
                error_code_description(result),
                result
            );
            if let Some(lib_msg) = lib.get_error_string(result) {
                eprintln!("  Library message: {}", lib_msg);
            }
            std::process::exit(1);
        }

        println!("✓ Hash computed successfully!");
        println!();

        // Display first 32 bytes of hash in hex
        let hash_preview: String = hash[..32]
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join(" ");

        println!("Hash (first 32 bytes): {}", hash_preview);
        println!("Hash size: {} bytes", PHOTODNA_HASH_SIZE_EDGE_V2);

        // Also demonstrate border detection
        println!();
        println!("Testing border detection...");

        let mut hash_results = [HashResult::default(); 2];

        let border_result = unsafe {
            lib.photo_dna_edge_hash_border(
                image_data.as_ptr(),
                hash_results.as_mut_ptr(),
                2,
                width,
                height,
                stride,
                PhotoDna_Default | PhotoDna_Rgb,
            )
        };

        if border_result < 0 {
            println!(
                "Border detection: {} (code {})",
                error_code_description(border_result),
                border_result
            );
        } else {
            println!("Border detection returned {} hash(es)", border_result);
            for i in 0..border_result as usize {
                let result = hash_results[i].result;
                let x = hash_results[i].header_dimensions_image_x;
                let y = hash_results[i].header_dimensions_image_y;
                let w = hash_results[i].header_dimensions_image_w;
                let h = hash_results[i].header_dimensions_image_h;
                println!(
                    "  Hash {}: result={}, region=({},{},{},{})",
                    i, result, x, y, w, h
                );
            }
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        println!("Native library not available on this platform.");
        println!("Use the 'wasm' feature with a WASM runtime for BSD platforms.");
    }
}

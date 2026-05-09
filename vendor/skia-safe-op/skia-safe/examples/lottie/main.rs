//! Lottie Animation Rendering Example
//!
//! This example demonstrates rendering Lottie animations using Skia's Skottie module.
//! It loads a Lottie JSON file and renders all frames as a PNG sequence.
//!
//! # Usage
//!
//! ```bash
//! # Download a sample animation
//! curl -O https://raw.githubusercontent.com/airbnb/lottie-web/master/test/animations/starfish.json
//!
//! # Run the example (outputs to current directory)
//! cargo run --example lottie --features "skottie" -- starfish.json
//!
//! # Or specify an output directory
//! cargo run --example lottie --features "skottie" -- starfish.json ./output
//! ```

#[cfg(not(feature = "skottie"))]
fn main() {
    println!("To run this example, use: cargo run --example lottie --features \"skottie\" -- <path-to-lottie.json> [output-dir]");
}

#[cfg(feature = "skottie")]
fn main() {
    use skia_safe::{skottie::Animation, surfaces, Color, EncodedImageFormat};
    use std::path::PathBuf;
    use std::{env, fs, fs::File, io::Write};

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let (file_path, output_dir) = match args.len() {
        2 => (args[1].clone(), PathBuf::from(".")),
        3 => (args[1].clone(), PathBuf::from(&args[2])),
        _ => {
            eprintln!(
                "Usage: cargo run --example lottie --features \"skottie\" -- <lottie.json> [output-dir]"
            );
            eprintln!();
            eprintln!("Arguments:");
            eprintln!("  <lottie.json>  Path to a Lottie animation JSON file");
            eprintln!(
                "  [output-dir]   Output directory for PNG frames (default: current directory)"
            );
            eprintln!();
            eprintln!("Example:");
            eprintln!("  curl -O https://raw.githubusercontent.com/airbnb/lottie-web/master/test/animations/starfish.json");
            eprintln!("  cargo run --example lottie --features \"skottie\" -- starfish.json");
            std::process::exit(1);
        }
    };

    // Create output directory if it doesn't exist
    if !output_dir.exists() {
        if let Err(e) = fs::create_dir_all(&output_dir) {
            eprintln!(
                "Error: Failed to create output directory '{}': {}",
                output_dir.display(),
                e
            );
            std::process::exit(1);
        }
    }

    // Load the animation
    let animation = match Animation::from_file(&file_path) {
        Some(anim) => anim,
        None => {
            eprintln!("Error: Failed to load animation from '{}'", file_path);
            eprintln!("Make sure the file exists and contains valid Lottie JSON.");
            std::process::exit(1);
        }
    };

    // Get animation properties
    let size = animation.size();
    let fps = animation.fps();
    let duration = animation.duration();
    let in_point = animation.in_point();
    let out_point = animation.out_point();
    let total_frames = (out_point - in_point).ceil() as u32;

    println!("Animation Properties:");
    println!("  File: {}", file_path);
    println!("  Version: {}", animation.version());
    println!("  Size: {}x{}", size.width, size.height);
    println!("  FPS: {}", fps);
    println!("  Duration: {:.2}s", duration);
    println!(
        "  Frame range: {} - {} ({} frames)",
        in_point, out_point, total_frames
    );
    println!("  Output: {}", output_dir.display());
    println!();

    // Create a raster surface matching the animation size
    let width = size.width.ceil() as i32;
    let height = size.height.ceil() as i32;

    let mut surface = match surfaces::raster_n32_premul((width, height)) {
        Some(s) => s,
        None => {
            eprintln!("Error: Failed to create rendering surface");
            std::process::exit(1);
        }
    };

    // Render all frames
    println!("Rendering {} frames...", total_frames);

    for frame in 0..total_frames {
        // Seek to frame
        animation.seek_frame(frame as f64 + in_point as f64);

        // Clear the canvas with a white background
        surface.canvas().clear(Color::WHITE);

        // Render
        animation.render(surface.canvas(), None);

        // Save as PNG
        let file_name = output_dir.join(format!("frame-{:04}.png", frame));
        let image = surface.image_snapshot();

        match image.encode(None, EncodedImageFormat::PNG, 100) {
            Some(data) => {
                let mut file = match File::create(&file_name) {
                    Ok(f) => f,
                    Err(e) => {
                        eprintln!(
                            "Error: Failed to create file '{}': {}",
                            file_name.display(),
                            e
                        );
                        continue;
                    }
                };
                if let Err(e) = file.write_all(data.as_bytes()) {
                    eprintln!("Error: Failed to write to '{}': {}", file_name.display(), e);
                    continue;
                }
            }
            None => {
                eprintln!("Error: Failed to encode frame {} as PNG", frame);
            }
        }

        // Progress indicator
        if (frame + 1) % 10 == 0 || frame + 1 == total_frames {
            println!("  {}/{} frames", frame + 1, total_frames);
        }
    }

    println!();
    println!(
        "Done! {} frames written to {}",
        total_frames,
        output_dir.display()
    );
}

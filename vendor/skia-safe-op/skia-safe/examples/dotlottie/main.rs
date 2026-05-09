//! DotLottie (.lottie) Animation Rendering Example
//!
//! This example demonstrates loading `.lottie` files using the Builder API
//! with a custom ResourceProvider. Supports both dotLottie v1.0 and v2.0 specs.
//!
//! A `.lottie` file is a ZIP archive containing:
//! - `manifest.json` - metadata describing animations (includes `version` field)
//! - Animation JSON files (v1: `animations/`, v2: `a/`)
//! - Embedded images (v1: `images/`, v2: `i/`)
//!
//! # Usage
//!
//! ```bash
//! # Download a sample .lottie file
//! curl -LO https://github.com/LottieFiles/dotlottie-android/raw/main/sample-compose/src/main/assets/animations/pigeon.lottie
//!
//! # Run the example (outputs to current directory)
//! cargo run --example dotlottie --features "skottie" -- pigeon.lottie
//!
//! # Or specify an output directory
//! cargo run --example dotlottie --features "skottie" -- pigeon.lottie ./output
//! ```

#[cfg(not(feature = "skottie"))]
fn main() {
    println!("To run this example, use: cargo run --example dotlottie --features \"skottie\" -- <path-to-file.lottie> [output-dir]");
}

#[cfg(feature = "skottie")]
fn main() {
    use serde::Deserialize;
    use skia_safe::{
        resources::ResourceProvider, skottie::Builder, surfaces, Color, Data, EncodedImageFormat,
        FontMgr, Typeface,
    };
    use std::collections::HashMap;
    use std::io::Read;
    use std::path::PathBuf;
    use std::{env, fs, fs::File, io::Write};
    use zip::ZipArchive;

    /// dotLottie manifest.json structure
    #[derive(Deserialize)]
    struct Manifest {
        #[serde(default)]
        version: String, // "1" or "2" (or empty for legacy v1)
        animations: Vec<AnimationInfo>,
    }

    /// Returns the directory prefixes based on dotLottie spec version
    fn get_paths(version: &str) -> (&'static str, &'static str) {
        match version {
            "2" => ("a", "i"),
            _ => ("animations", "images"), // v1 or unspecified
        }
    }

    /// Animation entry in the manifest
    #[derive(Deserialize)]
    struct AnimationInfo {
        id: String,
    }

    /// Resource provider that serves images from a .lottie ZIP archive.
    ///
    /// Images are pre-extracted into a HashMap because ZipArchive requires
    /// `&mut self` for reading, which conflicts with ResourceProvider's `&self` methods.
    struct DotLottieResourceProvider {
        /// Images extracted from the archive (name -> data)
        images: HashMap<String, Vec<u8>>,
        font_mgr: FontMgr,
    }

    impl DotLottieResourceProvider {
        fn new(images: HashMap<String, Vec<u8>>, font_mgr: FontMgr) -> Self {
            Self { images, font_mgr }
        }
    }

    impl ResourceProvider for DotLottieResourceProvider {
        fn load(&self, _resource_path: &str, resource_name: &str) -> Option<Data> {
            // Try to find the image by name in our pre-extracted HashMap
            // The resource_name might be just the filename or include a path
            let name = resource_name
                .rsplit('/')
                .next()
                .unwrap_or(resource_name)
                .to_string();

            self.images
                .get(&name)
                .or_else(|| self.images.get(resource_name))
                .map(|data| Data::new_copy(data))
        }

        fn load_typeface(&self, _name: &str, _url: &str) -> Option<Typeface> {
            // For simplicity, we don't support embedded fonts in this example
            None
        }

        fn font_mgr(&self) -> FontMgr {
            self.font_mgr.clone()
        }
    }

    /// Extract all images from the .lottie archive into a HashMap
    fn extract_images(archive: &mut ZipArchive<File>, img_dir: &str) -> HashMap<String, Vec<u8>> {
        let mut images = HashMap::new();
        let prefix = format!("{}/", img_dir);

        for i in 0..archive.len() {
            let mut file = match archive.by_index(i) {
                Ok(f) => f,
                Err(_) => continue,
            };

            let name = file.name().to_string();

            // Check if this is an image file
            if name.starts_with(&prefix) && !file.is_dir() {
                let mut data = Vec::new();
                if file.read_to_end(&mut data).is_ok() {
                    // Store by just the filename (without path)
                    let filename = name.rsplit('/').next().unwrap_or(&name).to_string();
                    println!("  Found image: {}", filename);
                    images.insert(filename, data);
                }
            }
        }

        images
    }

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let (file_path, output_dir) = match args.len() {
        2 => (args[1].clone(), PathBuf::from(".")),
        3 => (args[1].clone(), PathBuf::from(&args[2])),
        _ => {
            eprintln!(
                "Usage: cargo run --example dotlottie --features \"skottie\" -- <file.lottie> [output-dir]"
            );
            eprintln!();
            eprintln!("Arguments:");
            eprintln!("  <file.lottie>  Path to a .lottie animation file");
            eprintln!(
                "  [output-dir]   Output directory for PNG frames (default: current directory)"
            );
            eprintln!();
            eprintln!("Example:");
            eprintln!("  curl -LO https://github.com/LottieFiles/dotlottie-android/raw/main/sample-compose/src/main/assets/animations/pigeon.lottie");
            eprintln!("  cargo run --example dotlottie --features \"skottie\" -- pigeon.lottie");
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

    // Open the .lottie file as a ZIP archive
    let file = match File::open(&file_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Error: Failed to open '{}': {}", file_path, e);
            std::process::exit(1);
        }
    };

    let mut archive = match ZipArchive::new(file) {
        Ok(a) => a,
        Err(e) => {
            eprintln!(
                "Error: Failed to read '{}' as ZIP archive: {}",
                file_path, e
            );
            eprintln!("Make sure the file is a valid .lottie file.");
            std::process::exit(1);
        }
    };

    println!("Loading .lottie file: {}", file_path);

    // Parse manifest.json to get animation info
    let manifest: Manifest = {
        let file = match archive.by_name("manifest.json") {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Error: Failed to read manifest.json from archive: {}", e);
                std::process::exit(1);
            }
        };
        match serde_json::from_reader(file) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Error: Failed to parse manifest.json: {}", e);
                std::process::exit(1);
            }
        }
    };

    if manifest.animations.is_empty() {
        eprintln!("Error: No animations found in manifest.json");
        std::process::exit(1);
    }

    // Determine paths based on dotLottie spec version
    let (anim_dir, img_dir) = get_paths(&manifest.version);
    let version_info = if manifest.version.is_empty() {
        "v1 (legacy)"
    } else {
        &format!("v{}", manifest.version)
    };
    println!("dotLottie spec: {}", version_info);

    let anim_id = &manifest.animations[0].id;
    println!("Animation ID: {}", anim_id);

    // Extract animation JSON
    let anim_json = {
        let anim_path = format!("{}/{}.json", anim_dir, anim_id);
        let mut file = match archive.by_name(&anim_path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Error: Failed to read animation '{}': {}", anim_path, e);
                std::process::exit(1);
            }
        };
        let mut contents = String::new();
        if let Err(e) = file.read_to_string(&mut contents) {
            eprintln!("Error: Failed to read animation JSON: {}", e);
            std::process::exit(1);
        }
        contents
    };

    // Pre-extract images into HashMap
    println!("Extracting embedded images...");
    let images = extract_images(&mut archive, img_dir);
    println!("  Found {} images", images.len());

    // Create resource provider for embedded images
    let font_mgr = FontMgr::default();
    let provider = DotLottieResourceProvider::new(images, font_mgr.clone());

    // Build animation with resource provider
    let animation = match Builder::new()
        .set_font_manager(font_mgr)
        .set_resource_provider(provider)
        .make(&anim_json)
    {
        Some(anim) => anim,
        None => {
            eprintln!("Error: Failed to build animation from JSON");
            eprintln!("The animation JSON may be invalid or unsupported.");
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

    println!();
    println!("Animation Properties:");
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

        // Save as JPEG
        let file_name = output_dir.join(format!("frame-{:04}.jpg", frame));
        let image = surface.image_snapshot();

        match image.encode(None, EncodedImageFormat::JPEG, 80) {
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
                eprintln!("Error: Failed to encode frame {} as JPEG", frame);
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

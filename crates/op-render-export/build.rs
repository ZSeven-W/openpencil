fn main() {
    println!("cargo:rerun-if-changed=src/motion/mp4_avfoundation.m");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        cc::Build::new()
            .file("src/motion/mp4_avfoundation.m")
            .flags(["-fobjc-arc", "-fblocks", "-Wno-deprecated-declarations"])
            .compile("op_motion_avfoundation");
        for framework in ["Foundation", "AVFoundation", "CoreMedia", "CoreVideo"] {
            println!("cargo:rustc-link-lib=framework={framework}");
        }
    }
}

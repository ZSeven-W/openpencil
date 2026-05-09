use super::{generic, linux, prelude::*};

pub struct Musl;

impl PlatformDetails for Musl {
    fn uses_freetype(&self) -> bool {
        true
    }

    fn gn_args(&self, config: &BuildConfiguration, builder: &mut GnArgsBuilder) {
        generic::gn_args(config, builder);
        let target = &config.target;

        builder.cflags(flags(target));
    }

    fn bindgen_args(&self, target: &Target, builder: &mut BindgenArgsBuilder) {
        builder.args(flags(target))
    }

    fn link_libraries(&self, features: &Features) -> Vec<String> {
        linux::link_libraries(features)
    }
}

fn flags(target: &Target) -> Vec<String> {
    let arch = &target.architecture;
    match linux::get_stdlib_version() {
        None => {
            cargo::warning("unable to determine g++ stdlib version");
            vec![]
        }
        Some(stdlib_version) => vec![
            format!("-I/usr/include/c++/{stdlib_version}"),
            format!("-I/usr/include/c++/{stdlib_version}/{arch}-alpine-linux-musl"),
        ],
    }
}

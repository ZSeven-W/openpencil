use super::prelude::*;

pub struct Generic;

impl PlatformDetails for Generic {
    fn uses_freetype(&self) -> bool {
        true
    }

    fn gn_args(&self, config: &BuildConfiguration, builder: &mut GnArgsBuilder) {
        gn_args(config, builder)
    }

    fn link_libraries(&self, features: &Features) -> Vec<String> {
        link_libraries(features)
    }
}

pub fn gn_args(config: &BuildConfiguration, builder: &mut GnArgsBuilder) {
    builder.target_os_and_default_cpu(&config.target.system);
}

pub fn link_libraries(features: &Features) -> Vec<String> {
    if features[feature::GL] {
        vec!["GL".into()]
    } else {
        Vec::new()
    }
}

extern crate prost_build;

fn main() {
    let mut config = prost_build::Config::new();
    config.type_attribute("Sport", "#[derive(Eq, Hash, Copy)]");
    config.type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]");
    config
        .compile_protos(&["src/common/types.proto"], &["src/"])
        .unwrap();
}

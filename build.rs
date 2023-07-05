extern crate prost_build;

fn main() {
    prost_build::compile_protos(&["src/common/types.proto"], &["src/"]).unwrap();
}

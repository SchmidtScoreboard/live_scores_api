pub mod color;
pub mod data;
pub mod fetch;
pub mod processors;
pub mod proto_helpers;
pub mod team;

pub mod types {
    include!(concat!(env!("OUT_DIR"), "/common.types.rs"));
}

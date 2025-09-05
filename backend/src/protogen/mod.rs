// Include the compiled proto Rust code
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/protogen/example.rs"));

// Expose the compiled protobuf descriptor bytes for tonic reflection
pub const DESCRIPTOR_SET: &[u8] = include_bytes!("descriptor.bin");

// this generates the methods but cannot serialize timestamps
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_client(true)
        .build_server(true)
        .out_dir("src/protogen")
        .file_descriptor_set_path("src/protogen/descriptor.bin") 
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .extern_path(".google.protobuf.Timestamp", "crate::types::SerdeTimestamp")
        // .field_attribute(
        //     ".google.protobuf.Timestamp",
        //     "#[serde(with = \"crate::wkt::timestamp_serde\")]",
        // )

        .compile(&["../proto/message.proto"], &["../proto", "../proto/third_party"])?;
    Ok(())
}

/*
// this causes errors because the methods are not generated
use std::{env, path::PathBuf};
use prost_wkt_build::*;
//use prost_types::FileDescriptorSet;

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();

    tonic_build::configure()
        .build_server(true) // generate server code
        .build_client(true) // generate client code
        // Add serde derives on all messages
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .extern_path(".google.protobuf.Any", "::prost_wkt_types::Any")
        .extern_path(".google.protobuf.Timestamp", "::prost_wkt_types::Timestamp")
        .extern_path(".google.protobuf.Value", "::prost_wkt_types::Value")
        .extern_path(".google.protobuf.Struct", "::prost_wkt_types::Struct")
        .extern_path(".google.protobuf.ListValue", "::prost_wkt_types::ListValue")
        .extern_path(".google.protobuf.NullValue", "::prost_wkt_types::NullValue")
        .out_dir(&out_dir)
        .compile(&["../proto/message.proto"], &["../proto"])
        .expect("Failed to compile protos");
}
*/
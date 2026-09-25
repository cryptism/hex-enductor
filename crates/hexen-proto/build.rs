use std::env;
use std::path::PathBuf;

use protox::prost::Message;

// protox is a pure-Rust protobuf compiler, so building this crate needs
// no `protoc` on PATH — which also means it builds for wasm32 targets
// (apps/presentation) from any shell, nix devShell or not.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let schema_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?).join("../../schema");

    let proto_files = [
        "hexen/v1/grid.proto",
        "hexen/v1/content.proto",
        "hexen/v1/location.proto",
        "hexen/v1/project.proto",
        "hexen/v1/command.proto",
        "hexen/v1/session.proto",
    ];

    let descriptors = protox::compile(proto_files, [&schema_dir])?;
    let descriptor_bytes = descriptors.encode_to_vec();
    std::fs::write(out_dir.join("hexen_descriptor.bin"), &descriptor_bytes)?;

    prost_build::Config::new().compile_fds(descriptors)?;

    pbjson_build::Builder::new()
        .register_descriptors(&descriptor_bytes)?
        .build(&[".hexen.v1"])?;

    for f in &proto_files {
        println!("cargo:rerun-if-changed={}", schema_dir.join(f).display());
    }

    Ok(())
}

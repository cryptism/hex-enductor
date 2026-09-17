use std::env;
use std::path::PathBuf;

fn main() -> std::io::Result<()> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let descriptor_path = out_dir.join("hexen_descriptor.bin");

    let schema_dir = "../../schema";
    let proto_files = [
        "hexen/v1/grid.proto",
        "hexen/v1/content.proto",
        "hexen/v1/location.proto",
        "hexen/v1/project.proto",
        "hexen/v1/command.proto",
        "hexen/v1/session.proto",
    ]
    .map(|f| format!("{schema_dir}/{f}"));

    prost_build::Config::new()
        .file_descriptor_set_path(&descriptor_path)
        .compile_protos(&proto_files, &[schema_dir])?;

    let descriptor_bytes = std::fs::read(&descriptor_path)?;

    pbjson_build::Builder::new()
        .register_descriptors(&descriptor_bytes)?
        .build(&[".hexen.v1"])?;

    for f in &proto_files {
        println!("cargo:rerun-if-changed={f}");
    }

    Ok(())
}

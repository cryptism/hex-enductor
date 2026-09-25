//! Generated from schema/hexen/v1/*.proto by build.rs (prost for the
//! struct definitions, pbjson for protobuf's canonical JSON mapping,
//! which is also the .hexen.yml format on disk).
//!
//! Shared by apps/hexend (native) and apps/presentation (wasm32), so
//! both ends of the /ws session serialize through the same types and
//! there's no wireFormat.ts-style seam between them.

pub mod hexen {
    pub mod v1 {
        include!(concat!(env!("OUT_DIR"), "/hexen.v1.rs"));
        include!(concat!(env!("OUT_DIR"), "/hexen.v1.serde.rs"));
    }
}

/// The compiled schema as an encoded `google.protobuf.FileDescriptorSet`
/// (with source comments) — for tools that work from the schema itself,
/// like `hexen schema`'s JSON Schema generator.
pub const FILE_DESCRIPTOR_SET: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/hexen_descriptor.bin"));

//! Generated from schema/hexen/v1/*.proto by build.rs (prost for the
//! struct definitions, pbjson for protobuf's canonical JSON mapping —
//! the same JSON convention packages/hexen-proto-ts's generated
//! TypeScript speaks, so this is the one schema both sides read).
//!
//! Shared by apps/hexend (native) and apps/presentation-rs (wasm32), so
//! both ends of the /ws session serialize through the same types and
//! there's no wireFormat.ts-style seam between them.

pub mod hexen {
    pub mod v1 {
        include!(concat!(env!("OUT_DIR"), "/hexen.v1.rs"));
        include!(concat!(env!("OUT_DIR"), "/hexen.v1.serde.rs"));
    }
}

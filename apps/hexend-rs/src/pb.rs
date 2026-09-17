//! Generated from schema/hexen/v1/*.proto by build.rs (prost for the
//! struct definitions, pbjson for protobuf's canonical JSON mapping —
//! the same JSON convention packages/hexen-proto-ts's generated
//! TypeScript speaks, so this is the one schema both sides read).

pub mod hexen {
    pub mod v1 {
        include!(concat!(env!("OUT_DIR"), "/hexen.v1.rs"));
        include!(concat!(env!("OUT_DIR"), "/hexen.v1.serde.rs"));
    }
}

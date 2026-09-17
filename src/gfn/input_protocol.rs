//! NVST input-channel binary protocol.
//!
//! The implementation lives in the `opennow-core` workspace crate, which depends on nothing but
//! `std` and therefore builds and tests on an ordinary PC. The binary links SDL2, libopus and the
//! VitaSDK stubs unconditionally, so anything left in here can only ever be exercised on the
//! console itself - which is why the wire format, the key tables and the whole input mapping were
//! moved out. See `core/src/protocol.rs`.
pub use opennow_core::protocol::*;

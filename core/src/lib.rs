//! Pure logic shared with the Vita binary: the NVST wire format and the whole input mapping.
//!
//! Nothing in here links SDL2, libopus or the VitaSDK stubs, and nothing touches the filesystem.
//! That is the entire point: the binary crate cannot be built or tested without a cross-compiler
//! and a console, so every decision worth testing - which zone a finger landed in, what a button
//! means in a given profile, how a stick deflection becomes cursor pixels, what bytes go on the
//! wire - lives here instead, where `cargo test -p opennow-core` runs on any PC in seconds.

pub mod config;
pub mod input;
pub mod protocol;

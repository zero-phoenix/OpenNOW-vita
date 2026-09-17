//! The input mapping, in three layers.
//!
//! ```text
//! SDL events ──▶ physical::PadState / Finger ──▶ mapper ──▶ OutputEvent ──▶ protocol bytes
//!                                                  ▲
//!                                    layout + bindings + config
//! ```
//!
//! Layer 1 (reading SDL) stays in the binary, because only it knows what an `sdl2::event::Event`
//! is. Everything from `PadState` onwards is here and is pure.
//!
//! The rule that keeps the drawing and the hit-testing from drifting apart: [`layout::ZONES`] is a
//! single `const` table, and both the renderer and [`router::route_touch`] read *that same table*.
//! The previous design had two functions "derived from the same constants", which is not the same
//! thing and is how a button ends up drawn somewhere it cannot be pressed.

pub mod bindings;
pub mod layout;
pub mod mapper;
pub mod physical;
pub mod router;

pub use bindings::{Action, MouseTarget};
pub use layout::{Rect, Zone, ZoneId};
pub use mapper::{
    DesktopPad, ModifierLatch, OutputEvent, TriggerZones, press_is_tap, rear_tap_button,
    scale_pointer_delta,
};
pub use physical::{Finger, Panel, PadState};
pub use router::{StickSide, TouchOwner, route_touch};

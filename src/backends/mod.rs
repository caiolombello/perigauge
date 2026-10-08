//! Battery sources. Specialized backends speak a device protocol directly;
//! `upower` is the generic fallback for everything the system already knows.

pub mod galaxy_buds;
pub mod hidpp;
pub mod keychron;
pub mod upower;

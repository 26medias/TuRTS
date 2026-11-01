//! Task13 red contract for the slot-colour palette.
//!
//! This intentionally mirrors the current single-material renderer. Replace it with the
//! authored slot palette after VIS1 has recorded the one-colour red.

/// The current renderer's warm unit material, repeated for each slot until Task13 lands.
pub const SLOT_RGB: [[u8; 3]; 8] = [[204, 77, 51]; 8];

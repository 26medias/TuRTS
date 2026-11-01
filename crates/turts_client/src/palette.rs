//! Task13 red contract for the slot-colour palette.
//!
//! Default Okabe-Ito slot palette and common conversion helpers.

/// Default slot colours from the MC visual contract.
pub const SLOT_RGB: [[u8; 3]; 8] = [
    [0x00, 0x72, 0xB2],
    [0xE6, 0x9F, 0x00],
    [0x00, 0x9E, 0x73],
    [0xD5, 0x5E, 0x00],
    [0x56, 0xB4, 0xE9],
    [0xCC, 0x79, 0xA7],
    [0xF0, 0xE4, 0x42],
    [0x00, 0x00, 0x00],
];

/// Return the authored slot colour, clamping future slots to the final palette entry.
pub fn slot_rgb(slot: u8) -> [u8; 3] {
    SLOT_RGB[usize::from(slot).min(SLOT_RGB.len() - 1)]
}

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

/// Palette entry chosen for this slot by the authoritative match start.
pub(crate) fn colour_index(slot: u8, start: Option<&turts_net_protocol::MatchStart>) -> usize {
    start
        .and_then(|start| start.colours.get(usize::from(slot)))
        .copied()
        .unwrap_or(slot) as usize
}

#[cfg(test)]
mod lobby_colour_tests {
    use super::*;
    #[test]
    fn match_colours_map_slots_without_changing_their_owners() {
        let start = turts_net_protocol::MatchStart {
            start_tick: 1,
            teams: vec![0, 1],
            colours: vec![1, 0],
        };
        assert_eq!(colour_index(0, Some(&start)), 1);
        assert_eq!(colour_index(1, Some(&start)), 0);
        assert_eq!(colour_index(2, Some(&start)), 2);
        assert_eq!(colour_index(0, None), 0);
    }
}

//! Conservative B-tree node accounting for the locked Rust toolchain.
//!
//! A non-root node holds at least five entries. A root may hold just one.
//! This toolchain can retain an allocated root even after its map is empty.
//! Reserve one full root for the member map and each live gate map.
use super::Member;
use std::mem::size_of;

const MEMBER_NODE_BYTES: usize = 11 * (size_of::<Member>() + size_of::<u32>()) + 128;
pub(super) const MEMBER_ROW_BYTES: usize = MEMBER_NODE_BYTES.div_ceil(5);
const GATE_ROW_BYTES: usize = 256;
const GATE_NODE_BYTES: usize = super::public_passage::root_node_bound();
const _: () = assert!(GATE_NODE_BYTES <= 5 * GATE_ROW_BYTES);

pub(super) const fn root_allowance(gates: usize) -> usize {
    MEMBER_NODE_BYTES + gates * GATE_NODE_BYTES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_maps_still_reserve_their_possible_roots() {
        assert_eq!(root_allowance(0), MEMBER_NODE_BYTES);
        assert_eq!(root_allowance(1), MEMBER_NODE_BYTES + GATE_NODE_BYTES);
        assert_eq!(
            root_allowance(128),
            MEMBER_NODE_BYTES + 128 * GATE_NODE_BYTES
        );
    }
}

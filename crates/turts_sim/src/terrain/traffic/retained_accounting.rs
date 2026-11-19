//! Root-node allowances for independently allocated retained B-tree maps.
//!
//! The per-entry allowances cover non-root nodes under the locked toolchain's
//! minimum occupancy. A root may have just one entry, so it needs its own
//! allowance. The root bounds match the existing layout assertions in the
//! parent and public-passage modules.
pub(super) const MEMBER_ROOT_BYTES: usize = 5 * 512;
pub(super) const GATE_ROOT_BYTES: usize = 5 * 256;

pub(super) const fn root_allowance(has_members: bool, nonempty_gates: usize) -> usize {
    (has_members as usize) * MEMBER_ROOT_BYTES + nonempty_gates * GATE_ROOT_BYTES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_independent_nonempty_map_reserves_a_root() {
        assert_eq!(root_allowance(false, 0), 0);
        assert_eq!(root_allowance(true, 0), 2560);
        assert_eq!(root_allowance(false, 128), 163_840);
        assert_eq!(root_allowance(true, 128), 166_400);
    }
}

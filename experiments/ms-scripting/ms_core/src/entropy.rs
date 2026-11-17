//! Wasm32 entropy backends (spec 2.1). Both fill a fixed byte pattern, so nothing in the
//! module depends on host entropy, and the module needs no import for it.

fn fill(dest: &mut [u8]) {
    for (i, b) in dest.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(151).wrapping_add(17);
    }
}

fn fixed_02(dest: &mut [u8]) -> Result<(), getrandom02::Error> {
    fill(dest);
    Ok(())
}
getrandom02::register_custom_getrandom!(fixed_02);

/// getrandom 0.3 custom backend (`--cfg getrandom_backend="custom"`).
///
/// # Safety
/// getrandom calls it with a valid buffer of `len` bytes.
#[no_mangle]
unsafe extern "Rust" fn __getrandom_v03_custom(
    dest: *mut u8,
    len: usize,
) -> Result<(), getrandom03::Error> {
    fill(core::slice::from_raw_parts_mut(dest, len));
    Ok(())
}

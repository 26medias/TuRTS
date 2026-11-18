//! Certify captured movement against a fresh same-build command replay.
//! This complements the geometry oracle. It does not broaden the independent
//! economy auditor's advertised support for terrain detours.
use super::fixture::Frame;

pub fn certify(captured: &[u8], replay: &[Frame]) -> Result<(), String> {
    let captured: Vec<Frame> = serde_json::from_slice(captured)
        .map_err(|error| format!("invalid movement capture: {error}"))?;
    if captured.len() != replay.len() {
        return Err("movement capture tick count differs from command replay".into());
    }
    for (index, (recorded, actual)) in captured.iter().zip(replay).enumerate() {
        if recorded != actual {
            return Err(format!(
                "movement capture differs from command replay at tick{}",
                index + 1
            ));
        }
    }
    Ok(())
}

pub fn reject_forged_positions(captured: &[u8], replay: &[Frame]) {
    let original: serde_json::Value = serde_json::from_slice(captured).unwrap();
    let bits = original[0][0]["position"][0].as_u64().unwrap() as u32;
    // A legal-looking one-ULP displacement must fail just as a wall teleport
    // does. Geometry checks alone would not certify deterministic provenance.
    for replacement in [bits ^ 1, 31.5_f32.to_bits()] {
        let mut forged = original.clone();
        forged[0][0]["position"][0] = replacement.into();
        assert!(certify(&serde_json::to_vec(&forged).unwrap(), replay).is_err());
    }
    let mut shortened = original;
    shortened.as_array_mut().unwrap().pop();
    assert!(certify(&serde_json::to_vec(&shortened).unwrap(), replay).is_err());
}

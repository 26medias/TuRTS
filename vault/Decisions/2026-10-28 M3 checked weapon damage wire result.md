# M3 checked weapon damage wire result

T13b publishes `WeaponRecord.damage` as `Result<u64, CommandRejectReason>`. A successful checked effective damage value uses `Ok(value)`. If the simulation's checked base-plus-research addition fails, the wire record keeps the explicit `ArithmeticOverflow` reason.

This preserves the simulation's checked arithmetic and prevents the network layer from inventing a usable damage value or silently omitting an owned weapon record. It adds one result tag and an error reason to the Protocol 3 weapon record. T15b must present the error as an unavailable value, not as a number.

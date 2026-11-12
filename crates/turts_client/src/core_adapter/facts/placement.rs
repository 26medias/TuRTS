//! The placement facts of `UI-PLACE` (design 6.1, 7.2): sites, footprints and the build area, from the replica and the content.
//! L0-CORE made this file; `UI-PLACE` fills it. `build` updates the facts that the last frame left:
//! keep the replica tick of the last rebuild in `turts_interaction::placement::Facts` and skip the work
//! when the tick did not change (one rebuild for each applied delta, not for each frame).

use crate::{content::ClientContent, replica::Replica};

pub(in crate::core_adapter) fn build(
    _facts: &mut turts_interaction::placement::Facts,
    _replica: &Replica,
    _content: &ClientContent,
) {
}

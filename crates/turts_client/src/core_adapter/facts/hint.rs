//! The hint facts of `UI-HINT` (design 7.8): the data of the hint conditions, from the replica and the content.
//! L0-CORE made this file; `UI-HINT` fills it. `build` updates the facts that the last frame left:
//! keep the replica tick of the last rebuild in `turts_interaction::hints::Facts` and skip the work
//! when the tick did not change (one rebuild for each applied delta, not for each frame).

use crate::{content::ClientContent, replica::Replica};

pub(in crate::core_adapter) fn build(
    _facts: &mut turts_interaction::hints::Facts,
    _replica: &Replica,
    _content: &ClientContent,
) {
}

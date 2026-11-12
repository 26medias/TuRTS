//! The placement facts of `UI-PLACE` (design 6.1, 7.2): sites, footprints and the build area, from
//! the replica and the content. L0-CORE made this file; `UI-PLACE` fills it. The facts are rebuilt
//! once for each applied delta in a later task, not for each frame (plan, `UI-PLACE`).

use crate::{content::ClientContent, replica::Replica};

pub(in crate::core_adapter) fn build(
    _replica: &Replica,
    _content: &ClientContent,
) -> turts_interaction::placement::Facts {
    turts_interaction::placement::Facts::default()
}

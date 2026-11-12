//! The hint facts of `UI-HINT` (design 7.8): the data of the hint conditions, from the replica and
//! the content. L0-CORE made this file; `UI-HINT` fills it.

use crate::{content::ClientContent, replica::Replica};

pub(in crate::core_adapter) fn build(
    _replica: &Replica,
    _content: &ClientContent,
) -> turts_interaction::hints::Facts {
    turts_interaction::hints::Facts::default()
}

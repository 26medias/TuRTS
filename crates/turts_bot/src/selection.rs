//! A paged physical choice cannot choose a prefix or break a tie with an identifier.
use crate::{BasisId, PageState};
use std::cmp::Ordering;
use turts_mod_api::WorldPos;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionError {
    WrongBasis,
    InvalidPosition,
    Incomplete,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CandidateRank {
    distance_squared: f64,
    x: f32,
    y: f32,
    authored_rank: u32,
}
impl Eq for CandidateRank {}
impl CandidateRank {
    pub fn new(
        origin: WorldPos,
        position: WorldPos,
        authored_rank: u32,
    ) -> Result<Self, SelectionError> {
        if [origin.x, origin.y, position.x, position.y]
            .iter()
            .any(|value| !value.is_finite())
        {
            return Err(SelectionError::InvalidPosition);
        }
        let dx = f64::from(position.x) - f64::from(origin.x);
        let dy = f64::from(position.y) - f64::from(origin.y);
        Ok(Self {
            distance_squared: dx * dx + dy * dy,
            x: position.x,
            y: position.y,
            authored_rank,
        })
    }
}
fn coordinate_cmp(a: f32, b: f32) -> Ordering {
    if a == b {
        Ordering::Equal
    } else {
        a.total_cmp(&b)
    }
}
impl Ord for CandidateRank {
    fn cmp(&self, other: &Self) -> Ordering {
        self.distance_squared
            .total_cmp(&other.distance_squared)
            .then_with(|| coordinate_cmp(self.x, other.x))
            .then_with(|| coordinate_cmp(self.y, other.y))
            .then(self.authored_rank.cmp(&other.authored_rank))
    }
}
impl PartialOrd for CandidateRank {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CandidateChoice<T> {
    Selected(T),
    Ambiguous,
    Empty,
}
/// Constant-size best-candidate state. The basis lease belongs to the calling controller.
#[derive(Debug, Clone)]
pub struct CandidateScan<T> {
    basis: BasisId,
    best: Option<(u64, CandidateRank, T)>,
    ambiguous: bool,
}
impl<T> CandidateScan<T> {
    pub fn new(basis: BasisId) -> Self {
        Self {
            basis,
            best: None,
            ambiguous: false,
        }
    }
    pub fn basis(&self) -> BasisId {
        self.basis
    }
    pub fn consider(
        &mut self,
        basis: BasisId,
        identity: u64,
        rank: CandidateRank,
        candidate: T,
    ) -> Result<(), SelectionError> {
        if basis != self.basis {
            return Err(SelectionError::WrongBasis);
        }
        match self.best.as_ref().map(|(id, old, _)| (*id, old.cmp(&rank))) {
            None | Some((_, Ordering::Greater)) => {
                self.best = Some((identity, rank, candidate));
                self.ambiguous = false;
            }
            Some((old_id, Ordering::Equal)) if old_id != identity => self.ambiguous = true,
            _ => {}
        }
        Ok(())
    }
    pub fn choice(&self, state: &PageState) -> Result<CandidateChoice<&T>, SelectionError> {
        if !matches!(state, PageState::Complete) {
            return Err(SelectionError::Incomplete);
        }
        if self.ambiguous {
            return Ok(CandidateChoice::Ambiguous);
        }
        Ok(self
            .best
            .as_ref()
            .map_or(CandidateChoice::Empty, |(_, _, candidate)| {
                CandidateChoice::Selected(candidate)
            }))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::QueryCursor;
    fn rank(x: f32) -> CandidateRank {
        CandidateRank::new(WorldPos { x: 0., y: 0. }, WorldPos { x, y: 0. }, 0).unwrap()
    }
    #[test]
    fn a_later_page_can_replace_the_first_pages_best_candidate() {
        let mut scan = CandidateScan::new(7);
        scan.consider(7, 1, rank(8.), "first").unwrap();
        let more = PageState::More(QueryCursor { basis: 7, token: 1 });
        assert_eq!(scan.choice(&more), Err(SelectionError::Incomplete));
        scan.consider(7, 900, rank(3.), "later").unwrap();
        assert_eq!(
            scan.choice(&PageState::Complete),
            Ok(CandidateChoice::Selected(&"later"))
        );
    }
    #[test]
    fn a_tie_that_crosses_a_page_boundary_has_no_identifier_winner() {
        let mut scan = CandidateScan::new(7);
        scan.consider(7, 900, rank(3.), "high-id").unwrap();
        assert_eq!(
            scan.choice(&PageState::UnresolvedTie(QueryCursor {
                basis: 7,
                token: 1
            })),
            Err(SelectionError::Incomplete)
        );
        scan.consider(7, 1, rank(3.), "low-id").unwrap();
        assert_eq!(
            scan.choice(&PageState::Complete),
            Ok(CandidateChoice::Ambiguous)
        );
    }
    #[test]
    fn zero_signs_tie_and_a_different_basis_cannot_finish_the_choice() {
        let mut scan = CandidateScan::new(7);
        scan.consider(7, 1, rank(-0.), "negative").unwrap();
        scan.consider(7, 2, rank(0.), "positive").unwrap();
        assert_eq!(
            scan.choice(&PageState::Complete),
            Ok(CandidateChoice::Ambiguous)
        );
        assert_eq!(
            scan.consider(8, 3, rank(0.), "other"),
            Err(SelectionError::WrongBasis)
        );
    }
}

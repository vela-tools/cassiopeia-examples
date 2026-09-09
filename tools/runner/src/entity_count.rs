use strum::Display;

/// How many entities an output has to hold for a run to be considered correct.
///
/// The choice is about the claim, not about the source. An exact figure is a claim that the run
/// produces precisely this many entities, which is worth making whenever the number follows from
/// the mapping: a fixed file's rows, or the single entity a whole file merges into. A floor is a
/// claim that the run produces at least this many, which is what a feed whose size nobody controls
/// admits. Whether a difference from the declared figure fails the run or is only reported is a
/// separate question, answered by the datasets' [`crate::volatility::Volatility`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityCount {
    /// Exactly this many.
    Exactly(usize),

    /// At least this many.
    AtLeast(usize),
}

/// Whether a found count is near enough to the declared figure for a publisher's edits to explain
/// the difference.
///
/// This is the guard rail under the frozen-live rule: it is what keeps a live source from excusing
/// a mapping that has stopped producing entities at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display)]
#[strum(serialize_all = "kebab-case")]
pub enum Plausibility {
    /// Within the guard rail, so a publisher editing the source could have got here.
    Churnable,

    /// Outside it. No publisher halves or doubles a dataset between two runs, so a figure this far
    /// out is a mapping that stopped working rather than a source that moved.
    Implausible,
}

impl EntityCount {
    /// The declared figure, whichever way it is stated.
    pub const fn declared(self) -> usize {
        match self {
            EntityCount::Exactly(expected) => expected,
            EntityCount::AtLeast(fewest) => fewest,
        }
    }

    /// Whether a run that produced this many entities satisfies the count.
    pub const fn satisfied_by(self, found: usize) -> bool {
        match self {
            EntityCount::Exactly(expected) => found == expected,
            EntityCount::AtLeast(fewest) => found >= fewest,
        }
    }

    /// Whether a count that is not satisfied is still close enough to be the publisher's doing.
    pub const fn plausibility(self, found: usize) -> Plausibility {
        let churnable = match self {
            // An exact figure is bounded on both sides: half of it and twice it.
            EntityCount::Exactly(expected) => found.saturating_mul(2) >= expected && found <= expected.saturating_mul(2),
            // A floor is deliberately set below what the feed usually returns, so only the lower
            // rail says anything: the nationwide alert example clears a floor of 114 with 444
            // alerts on an ordinary day, and that is the feed working, not drifting.
            EntityCount::AtLeast(fewest) => found.saturating_mul(2) >= fewest,
        };

        if churnable { Plausibility::Churnable } else { Plausibility::Implausible }
    }
}

#[cfg(test)]
mod tests {
    use crate::entity_count::{EntityCount, Plausibility};

    #[test]
    fn an_exact_count_admits_only_that_many() {
        assert!(EntityCount::Exactly(118).satisfied_by(118));
        assert!(!EntityCount::Exactly(118).satisfied_by(117));
        assert!(!EntityCount::Exactly(118).satisfied_by(119));
    }

    #[test]
    fn a_floor_admits_anything_above_it() {
        assert!(EntityCount::AtLeast(500).satisfied_by(500));
        assert!(EntityCount::AtLeast(500).satisfied_by(6162));
        assert!(!EntityCount::AtLeast(500).satisfied_by(499));
    }

    #[test]
    fn both_forms_report_the_figure_they_were_written_with() {
        assert_eq!(EntityCount::Exactly(118).declared(), 118);
        assert_eq!(EntityCount::AtLeast(500).declared(), 500);
    }

    #[test]
    fn an_exact_count_stays_churnable_between_half_the_figure_and_twice_it() {
        assert_eq!(EntityCount::Exactly(7698).plausibility(7701), Plausibility::Churnable);
        assert_eq!(EntityCount::Exactly(7698).plausibility(3849), Plausibility::Churnable);
        assert_eq!(EntityCount::Exactly(7698).plausibility(15396), Plausibility::Churnable);
    }

    #[test]
    fn an_exact_count_outside_the_guard_rail_is_implausible() {
        assert_eq!(EntityCount::Exactly(7698).plausibility(3848), Plausibility::Implausible);
        assert_eq!(EntityCount::Exactly(7698).plausibility(15397), Plausibility::Implausible);
        assert_eq!(EntityCount::Exactly(1).plausibility(0), Plausibility::Implausible);
    }

    #[test]
    fn a_floor_has_no_upper_guard_rail() {
        assert_eq!(EntityCount::AtLeast(114).plausibility(444), Plausibility::Churnable);
        assert_eq!(EntityCount::AtLeast(114).plausibility(57), Plausibility::Churnable);
        assert_eq!(EntityCount::AtLeast(114).plausibility(56), Plausibility::Implausible);
    }
}

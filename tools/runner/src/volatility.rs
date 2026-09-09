use crate::entity_count::Plausibility;
use serde::Deserialize;
use strum::Display;

/// Whether a dataset's publisher has finished with the file an example reads.
///
/// This is what separates a broken example from a moving source. A frozen file is a closed range, a
/// dated revision, or a dump nobody edits any more: its record count is a fact, and a run that
/// finds another number found a defect. A live file still grows or still accepts corrections, so
/// the same difference is the publisher at work and the example is still doing its job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, Deserialize)]
#[strum(serialize_all = "kebab-case")]
#[serde(rename_all = "kebab-case")]
pub enum Volatility {
    /// The publisher has closed the file: a dated revision, a finished range, or a dormant dump.
    Frozen,

    /// The publisher still edits or grows the file.
    Live,
}

/// What a difference between what an example declares and what a run found amounts to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display)]
#[strum(serialize_all = "kebab-case")]
pub enum Mismatch {
    /// The example no longer works, and the run has to fail.
    Broken,

    /// The publisher moved under a working example. Worth reporting, never worth failing on.
    Churn,
}

impl Volatility {
    /// How a difference between a declared figure and a found one is reported.
    ///
    /// A figure far enough from the declared one is a broken mapping whatever the publisher does,
    /// so an implausible finding overrides the frozen-live distinction: no source halves or doubles
    /// itself between two runs of the same week.
    pub const fn classify(self, plausibility: Plausibility) -> Mismatch {
        match (self, plausibility) {
            (Volatility::Frozen, Plausibility::Churnable | Plausibility::Implausible) | (Volatility::Live, Plausibility::Implausible) => Mismatch::Broken,
            (Volatility::Live, Plausibility::Churnable) => Mismatch::Churn,
        }
    }
}

/// The volatility of a whole example, from what its datasets declare.
///
/// One live dataset is enough. A frozen source and a live one land in the same output file, so its
/// entity count moves with whichever of them the publisher is still editing. An example that
/// declares no dataset at all reads its input from a remote source as it runs, which nothing in the
/// descriptor pins, so it is live too.
pub fn combined<'declared>(declared: impl IntoIterator<Item = &'declared Volatility>) -> Volatility {
    declared
        .into_iter()
        .copied()
        .reduce(|left, right| match (left, right) {
            (Volatility::Live, Volatility::Frozen | Volatility::Live) | (Volatility::Frozen, Volatility::Live) => Volatility::Live,
            (Volatility::Frozen, Volatility::Frozen) => Volatility::Frozen,
        })
        // An example with no datasets fetches as it runs, so there is nothing here to freeze.
        .unwrap_or(Volatility::Live)
}

#[cfg(test)]
mod tests {
    use crate::{
        entity_count::Plausibility,
        volatility::{Mismatch, Volatility, combined},
    };
    use serde::Deserialize;

    /// The shape the setting is read in, since a bare enum is not a TOML document.
    #[derive(Debug, Deserialize)]
    struct Holder {
        volatility: Volatility,
    }

    fn read(text: &str) -> Option<Volatility> {
        toml::from_str(text).ok().map(|holder: Holder| holder.volatility)
    }

    #[test]
    fn a_closed_file_is_read_as_frozen() {
        assert_eq!(read("volatility = \"frozen\""), Some(Volatility::Frozen));
    }

    #[test]
    fn a_growing_file_is_read_as_live() {
        assert_eq!(read("volatility = \"live\""), Some(Volatility::Live));
    }

    #[test]
    fn a_setting_that_is_neither_is_rejected() {
        assert_eq!(read("volatility = \"occasional\""), None);
    }

    #[test]
    fn a_dataset_that_says_nothing_is_rejected() {
        assert_eq!(read("source = \"a publisher\""), None);
    }

    #[test]
    fn a_frozen_source_makes_any_difference_a_defect() {
        assert_eq!(Volatility::Frozen.classify(Plausibility::Churnable), Mismatch::Broken);
        assert_eq!(Volatility::Frozen.classify(Plausibility::Implausible), Mismatch::Broken);
    }

    #[test]
    fn a_live_source_makes_a_near_difference_churn_and_a_far_one_a_defect() {
        assert_eq!(Volatility::Live.classify(Plausibility::Churnable), Mismatch::Churn);
        assert_eq!(Volatility::Live.classify(Plausibility::Implausible), Mismatch::Broken);
    }

    #[test]
    fn one_live_dataset_makes_the_whole_example_live() {
        assert_eq!(combined([Volatility::Frozen, Volatility::Live, Volatility::Frozen].iter()), Volatility::Live);
    }

    #[test]
    fn an_example_whose_datasets_are_all_frozen_is_frozen() {
        assert_eq!(combined([Volatility::Frozen, Volatility::Frozen].iter()), Volatility::Frozen);
    }

    #[test]
    fn an_example_without_datasets_is_live() {
        assert_eq!(combined(&[]), Volatility::Live);
    }
}

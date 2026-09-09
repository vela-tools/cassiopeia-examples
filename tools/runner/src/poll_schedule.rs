use crate::poll_interval::PollInterval;
use serde::Deserialize;
use std::{
    num::NonZeroU32,
    time::{Duration, Instant},
};

/// The bound a scheduled example runs under.
///
/// A scheduled mapping runs until it is interrupted, which is what its page tells a reader to do by
/// hand. Nothing in continuous integration sits at a terminal, so the descriptor says how many
/// polls are worth watching and the runner interrupts the run once they have had time to happen.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct PollSchedule {
    /// How many polls the run is allowed before it is interrupted.
    pub cycles: NonZeroU32,

    /// The interval between them, matching the manifest's own schedule.
    pub interval: PollInterval,

    /// What the interval does not cover: the jitter the manifest adds before each poll, and the
    /// fetch, mapping, and delivery the last one still has to finish.
    pub grace: PollInterval,
}

impl PollSchedule {
    /// How long the run is left alone.
    ///
    /// The first poll fires immediately and each later one waits out the interval, so five cycles
    /// are four intervals apart, plus whatever the last of them needs to finish.
    pub const fn window(self) -> Duration {
        self.interval
            .as_duration()
            .saturating_mul(self.cycles.get().saturating_sub(1))
            .saturating_add(self.grace.as_duration())
    }

    /// When the run is interrupted, counted from the moment it started.
    pub fn deadline(self, started: Instant) -> Instant {
        started + self.window()
    }
}

#[cfg(test)]
mod tests {
    use crate::poll_schedule::PollSchedule;
    use std::time::{Duration, Instant};

    fn schedule(text: &str) -> Option<PollSchedule> {
        toml::from_str(text).ok()
    }

    #[test]
    fn five_five_minute_cycles_are_four_intervals_and_the_grace() {
        let declared = schedule("cycles = 5\ninterval = \"5m\"\ngrace = \"4m\"\n");

        assert_eq!(declared.map(PollSchedule::window), Some(Duration::from_mins(24)));
    }

    #[test]
    fn a_single_cycle_waits_only_for_the_grace() {
        let declared = schedule("cycles = 1\ninterval = \"5m\"\ngrace = \"90s\"\n");

        assert_eq!(declared.map(PollSchedule::window), Some(Duration::from_secs(90)));
    }

    #[test]
    fn the_deadline_is_the_window_after_the_run_started() {
        let started = Instant::now();
        let declared = schedule("cycles = 2\ninterval = \"1m\"\ngrace = \"30s\"\n");

        assert_eq!(declared.map(|declared| declared.deadline(started)), Some(started + Duration::from_secs(90)));
    }

    #[test]
    fn a_run_that_allows_no_cycles_is_rejected() {
        assert!(schedule("cycles = 0\ninterval = \"5m\"\ngrace = \"1m\"\n").is_none());
    }

    #[test]
    fn a_block_without_a_grace_is_rejected() {
        assert!(schedule("cycles = 5\ninterval = \"5m\"\n").is_none());
    }

    #[test]
    fn an_unknown_key_is_rejected() {
        assert!(schedule("cycles = 5\ninterval = \"5m\"\ngrace = \"1m\"\njitter = \"20s\"\n").is_none());
    }
}

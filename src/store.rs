//! Countdown state, independent of the UI so transitions can be tested.
use std::time::{Duration, Instant};

pub const FOCUS_SECONDS: u64 = 45 * 60;
const BREAK_SECONDS: u64 = 5 * 60;
const LONG_BREAK_SECONDS: u64 = 15 * 60;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    Focus,
    Break,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimerState {
    pub phase: Phase,
    pub session: u32,
    pub remaining: u64,
    pub running: bool,
    deadline: Option<Instant>,
}

impl Default for TimerState {
    fn default() -> Self {
        Self {
            phase: Phase::Focus,
            session: 1,
            remaining: FOCUS_SECONDS,
            running: false,
            deadline: None,
        }
    }
}

impl TimerState {
    pub fn duration(&self) -> u64 {
        match self.phase {
            Phase::Focus => FOCUS_SECONDS,
            Phase::Break if self.session == 4 => LONG_BREAK_SECONDS,
            Phase::Break => BREAK_SECONDS,
        }
    }

    pub fn toggle(&mut self, now: Instant) {
        if self.running {
            self.tick(now);
            self.running = false;
            self.deadline = None;
        } else {
            self.running = true;
            self.deadline = Some(now + Duration::from_secs(self.remaining));
        }
    }

    /// Returns true once when a countdown finishes. Late wakes use elapsed
    /// wall time instead of counting callbacks, preventing accumulated drift.
    pub fn tick(&mut self, now: Instant) -> bool {
        let Some(deadline) = self.deadline else {
            return false;
        };
        if now >= deadline {
            self.skip();
            return true;
        }
        self.remaining = deadline.duration_since(now).as_secs_f64().ceil() as u64;
        false
    }

    pub fn reset(&mut self) {
        self.running = false;
        self.deadline = None;
        self.remaining = self.duration();
    }

    pub fn skip(&mut self) {
        match self.phase {
            Phase::Focus => self.phase = Phase::Break,
            Phase::Break => {
                self.phase = Phase::Focus;
                self.session = self.session % 4 + 1;
            }
        }
        self.reset();
    }

    pub fn display(&self) -> String {
        format!("{:02} : {:02}", self.remaining / 60, self.remaining % 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_pauses_and_resets() {
        let now = Instant::now();
        let mut timer = TimerState::default();
        assert_eq!(timer.display(), "45 : 00");
        timer.toggle(now);
        timer.tick(now + Duration::from_secs(61));
        assert_eq!(timer.display(), "43 : 59");
        timer.toggle(now + Duration::from_secs(61));
        timer.tick(now + Duration::from_secs(120));
        assert_eq!(timer.remaining, FOCUS_SECONDS - 61);
        timer.reset();
        assert_eq!(timer.remaining, FOCUS_SECONDS);
        assert!(!timer.running);
    }

    #[test]
    fn resumes_from_the_paused_remaining_time() {
        let now = Instant::now();
        let mut timer = TimerState::default();
        timer.toggle(now);
        timer.toggle(now + Duration::from_secs(60));
        timer.toggle(now + Duration::from_secs(600));
        timer.tick(now + Duration::from_secs(610));
        assert_eq!(timer.remaining, FOCUS_SECONDS - 70);
        assert!(timer.running);
    }

    #[test]
    fn completion_advances_once_and_waits_for_start() {
        let now = Instant::now();
        let mut timer = TimerState::default();
        timer.toggle(now);
        assert!(timer.tick(now + Duration::from_secs(FOCUS_SECONDS + 20)));
        assert_eq!(timer.phase, Phase::Break);
        assert_eq!(timer.remaining, BREAK_SECONDS);
        assert!(!timer.running);
        assert!(!timer.tick(now + Duration::from_secs(FOCUS_SECONDS + 21)));
    }

    #[test]
    fn fourth_session_has_long_break_then_wraps() {
        let mut timer = TimerState::default();
        for _ in 0..6 {
            timer.skip();
        }
        assert_eq!(timer.session, 4);
        timer.skip();
        assert_eq!(timer.remaining, LONG_BREAK_SECONDS);
        timer.skip();
        assert_eq!(timer.session, 1);
        assert_eq!(timer.phase, Phase::Focus);
    }
}

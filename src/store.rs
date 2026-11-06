//! Countdown state, independent of the UI so transitions can be tested.
use std::time::{Duration, Instant};

pub const FOCUS_SECONDS: u64 = 25 * 60;
const BREAK_SECONDS: u64 = 5 * 60;
const LONG_BREAK_SECONDS: u64 = 15 * 60;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SpecialBreak {
    pub time: String,
    pub minutes: String,
}

impl SpecialBreak {
    pub fn schedule(&self) -> Option<(u32, u64)> {
        let (hour, minute) = self.time.split_once(':')?;
        let (hour, minute) = (hour.parse::<u32>().ok()?, minute.parse::<u32>().ok()?);
        let duration = self.minutes.parse::<u64>().ok()?;
        (hour < 24 && minute < 60 && (1..=180).contains(&duration))
            .then_some((hour * 60 + minute, duration * 60))
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TimerRules {
    pub focus_minutes: u32,
    pub short_break_minutes: u32,
    pub long_break_minutes: u32,
    pub rounds: u32,
    pub special_breaks: [SpecialBreak; 4],
}

impl Default for TimerRules {
    fn default() -> Self {
        Self {
            focus_minutes: (FOCUS_SECONDS / 60) as u32,
            short_break_minutes: (BREAK_SECONDS / 60) as u32,
            long_break_minutes: (LONG_BREAK_SECONDS / 60) as u32,
            rounds: 4,
            special_breaks: Default::default(),
        }
    }
}

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
    pub rules: TimerRules,
    pub note: String,
    pub completed: Vec<CompletedFocus>,
    started_at: Option<i64>,
    special_duration: Option<u64>,
    interrupted_focus: Option<u64>,
    last_special: [Option<(i32, u32)>; 4],
    deadline: Option<Instant>,
}

impl Default for TimerState {
    fn default() -> Self {
        Self {
            phase: Phase::Focus,
            session: 1,
            remaining: FOCUS_SECONDS,
            running: false,
            rules: TimerRules::default(),
            note: String::new(),
            completed: Vec::new(),
            started_at: None,
            special_duration: None,
            interrupted_focus: None,
            last_special: [None; 4],
            deadline: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompletedFocus {
    pub started_at: i64,
    pub finished_at: i64,
    pub seconds: u64,
    pub note: String,
}

impl TimerState {
    pub fn duration(&self) -> u64 {
        if let Some(duration) = self.special_duration {
            return duration;
        }
        u64::from(match self.phase {
            Phase::Focus => self.rules.focus_minutes,
            Phase::Break if self.session == self.rules.rounds => self.rules.long_break_minutes,
            Phase::Break => self.rules.short_break_minutes,
        }) * 60
    }

    /// Rule edits start a fresh cycle, keeping duration and progress consistent.
    pub fn configure(&mut self, edit: impl FnOnce(&mut TimerRules)) {
        edit(&mut self.rules);
        self.rules.focus_minutes = self.rules.focus_minutes.clamp(1, 120);
        self.rules.short_break_minutes = self.rules.short_break_minutes.clamp(1, 60);
        self.rules.long_break_minutes = self.rules.long_break_minutes.clamp(1, 60);
        self.rules.rounds = self.rules.rounds.clamp(1, 10);
        self.phase = Phase::Focus;
        self.session = 1;
        self.special_duration = None;
        self.interrupted_focus = None;
        self.reset();
    }

    /// A configured local time fires at most once per day, during active focus.
    pub fn check_special_break(&mut self, day: i32, minute: u32, now: Instant) {
        if !self.running || self.phase != Phase::Focus {
            return;
        }
        for index in 0..4 {
            let Some((scheduled, duration)) = self.rules.special_breaks[index].schedule() else {
                continue;
            };
            if minute == scheduled && self.last_special[index] != Some((day, minute)) {
                self.tick(now);
                if self.phase != Phase::Focus {
                    return;
                }
                self.last_special[index] = Some((day, minute));
                self.interrupted_focus = Some(self.remaining);
                self.special_duration = Some(duration);
                self.phase = Phase::Break;
                self.remaining = duration;
                self.deadline = Some(now + Duration::from_secs(duration));
                return;
            }
        }
    }

    pub fn toggle(&mut self, now: Instant) {
        if self.running {
            self.tick(now);
            self.running = false;
            self.deadline = None;
        } else {
            if self.phase == Phase::Focus && self.started_at.is_none() {
                self.started_at = Some(chrono::Utc::now().timestamp());
            }
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
            if self.phase == Phase::Focus {
                let finished_at = chrono::Utc::now().timestamp();
                self.completed.push(CompletedFocus {
                    started_at: self
                        .started_at
                        .unwrap_or(finished_at - self.duration() as i64),
                    finished_at,
                    seconds: u64::from(self.rules.focus_minutes) * 60,
                    note: self.note.clone(),
                });
            }
            self.skip();
            return true;
        }
        self.remaining = deadline.duration_since(now).as_secs_f64().ceil() as u64;
        false
    }

    pub fn reset(&mut self) {
        self.started_at = None;
        self.running = false;
        self.deadline = None;
        self.remaining = self.duration();
    }

    pub fn skip(&mut self) {
        if let Some(remaining) = self.interrupted_focus.take() {
            self.special_duration = None;
            self.phase = Phase::Focus;
            let started_at = self.started_at;
            self.reset();
            self.started_at = started_at;
            self.remaining = remaining;
            return;
        }
        match self.phase {
            Phase::Focus => self.phase = Phase::Break,
            Phase::Break => {
                self.phase = Phase::Focus;
                self.session = self.session % self.rules.rounds + 1;
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
        assert_eq!(timer.display(), "25 : 00");
        timer.toggle(now);
        timer.tick(now + Duration::from_secs(61));
        assert_eq!(timer.display(), "23 : 59");
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
    fn rule_edits_reset_a_running_cycle_and_control_rounds() {
        let mut timer = TimerState::default();
        timer.toggle(Instant::now());
        timer.configure(|rules| {
            rules.focus_minutes = 40;
            rules.short_break_minutes = 8;
            rules.long_break_minutes = 20;
            rules.rounds = 2;
        });
        assert!(!timer.running);
        assert_eq!(timer.remaining, 40 * 60);
        timer.skip();
        assert_eq!(timer.remaining, 8 * 60);
        timer.skip();
        assert_eq!(timer.session, 2);
        timer.skip();
        assert_eq!(timer.remaining, 20 * 60);
        timer.skip();
        assert_eq!(timer.session, 1);
        timer.configure(|rules| *rules = TimerRules::default());
        assert_eq!(timer.rules, TimerRules::default());
        assert_eq!(timer.remaining, FOCUS_SECONDS);
    }

    #[test]
    fn special_breaks_validate_times_and_durations() {
        let entry = |time: &str, minutes: &str| SpecialBreak {
            time: time.into(),
            minutes: minutes.into(),
        };
        assert_eq!(entry("09:30", "10").schedule(), Some((570, 600)));
        for invalid in [
            entry("24:00", "5"),
            entry("12:60", "5"),
            entry("12:00", "0"),
            entry("12:00", "181"),
            entry("", "5"),
        ] {
            assert_eq!(invalid.schedule(), None);
        }
    }

    #[test]
    fn scheduled_break_preserves_focus_and_only_fires_once_per_day() {
        let now = Instant::now();
        let mut timer = TimerState::default();
        timer.rules.special_breaks[0] = SpecialBreak {
            time: "09:30".into(),
            minutes: "10".into(),
        };
        timer.check_special_break(1, 570, now);
        assert_eq!(timer.phase, Phase::Focus);
        timer.toggle(now);
        timer.check_special_break(1, 570, now + Duration::from_secs(60));
        assert_eq!(timer.phase, Phase::Break);
        assert_eq!(timer.duration(), 600);
        assert_eq!(timer.remaining, 600);
        assert!(timer.running);
        assert!(timer.tick(now + Duration::from_secs(660)));
        assert_eq!(timer.phase, Phase::Focus);
        assert_eq!(timer.remaining, FOCUS_SECONDS - 60);
        assert_eq!(timer.session, 1);
        assert!(!timer.running);
        timer.toggle(now + Duration::from_secs(661));
        timer.check_special_break(1, 570, now + Duration::from_secs(662));
        assert_eq!(timer.phase, Phase::Focus);
        timer.check_special_break(2, 570, now + Duration::from_secs(663));
        assert_eq!(timer.phase, Phase::Break);
    }

    #[test]
    fn only_completed_focus_is_recorded_once_with_note() {
        let now = Instant::now();
        let mut timer = TimerState::default();
        timer.note = "Build profiles".into();
        timer.toggle(now);
        timer.tick(now + Duration::from_secs(FOCUS_SECONDS));
        assert_eq!(timer.completed.len(), 1);
        assert_eq!(timer.completed[0].note, "Build profiles");
        assert_eq!(timer.completed[0].seconds, FOCUS_SECONDS);
        timer.tick(now + Duration::from_secs(FOCUS_SECONDS + 1));
        timer.toggle(now + Duration::from_secs(FOCUS_SECONDS + 1));
        timer.tick(now + Duration::from_secs(FOCUS_SECONDS + BREAK_SECONDS + 1));
        assert_eq!(timer.completed.len(), 1);
        timer.toggle(now);
        timer.reset();
        timer.skip();
        assert_eq!(timer.completed.len(), 1);
    }

    #[test]
    fn scheduled_break_does_not_record_partial_focus() {
        let now = Instant::now();
        let mut timer = TimerState::default();
        timer.rules.special_breaks[0] = SpecialBreak {
            time: "09:30".into(),
            minutes: "10".into(),
        };
        timer.toggle(now);
        timer.check_special_break(1, 570, now + Duration::from_secs(60));
        timer.tick(now + Duration::from_secs(660));
        assert!(timer.completed.is_empty());
        timer.toggle(now + Duration::from_secs(661));
        timer.tick(now + Duration::from_secs(661 + FOCUS_SECONDS - 60));
        assert_eq!(timer.completed.len(), 1);
        assert_eq!(timer.completed[0].seconds, FOCUS_SECONDS);
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

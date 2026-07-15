//! Adaptive Scheduler — `docs/05_Vision_Engine.md` §5.7 (Idle 1fps / Active
//! 5fps / Interactive 10fps state machine). Driven by explicit `Instant`
//! inputs rather than reading the wall clock, so transitions are unit-testable.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

const RAPID_WINDOW: Duration = Duration::from_secs(1);
const RAPID_THRESHOLD: usize = 3; // >=3 changes within RAPID_WINDOW => Interactive
const IDLE_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureState {
    Idle,
    Active,
    Interactive,
}

pub struct AdaptiveScheduler {
    state: CaptureState,
    last_activity: Instant,
    recent_changes: VecDeque<Instant>,
}

impl AdaptiveScheduler {
    #[must_use]
    pub fn new(now: Instant) -> Self {
        Self {
            state: CaptureState::Idle,
            last_activity: now,
            recent_changes: VecDeque::new(),
        }
    }

    /// Window focus changed — Idle -> Active (docs/05 §5.7 state diagram).
    pub fn on_focus_change(&mut self, now: Instant) {
        self.last_activity = now;
        if self.state == CaptureState::Idle {
            self.state = CaptureState::Active;
        }
    }

    /// A capture tick completed; `changed` is the frame differencer's verdict.
    pub fn on_frame_change(&mut self, now: Instant, changed: bool) {
        if !changed {
            return;
        }
        self.last_activity = now;
        self.recent_changes.push_back(now);
        self.prune(now);
        if self.recent_changes.len() >= RAPID_THRESHOLD {
            self.state = CaptureState::Interactive;
        } else if self.state == CaptureState::Idle {
            self.state = CaptureState::Active;
        }
    }

    /// Re-evaluates timeouts (Active/Interactive -> Idle after 30s, Interactive
    /// -> Active once changes slow down) and returns the interval to sleep for.
    pub fn tick(&mut self, now: Instant) -> Duration {
        self.prune(now);
        if self.state != CaptureState::Idle
            && now.duration_since(self.last_activity) >= IDLE_TIMEOUT
        {
            self.state = CaptureState::Idle;
        } else if self.state == CaptureState::Interactive
            && self.recent_changes.len() < RAPID_THRESHOLD
        {
            self.state = CaptureState::Active;
        }
        self.interval()
    }

    #[must_use]
    pub fn interval(&self) -> Duration {
        match self.state {
            CaptureState::Idle => Duration::from_secs(1),
            CaptureState::Active => Duration::from_millis(200),
            CaptureState::Interactive => Duration::from_millis(100),
        }
    }

    #[must_use]
    pub fn state(&self) -> CaptureState {
        self.state
    }

    fn prune(&mut self, now: Instant) {
        while let Some(&front) = self.recent_changes.front() {
            if now.duration_since(front) > RAPID_WINDOW {
                self.recent_changes.pop_front();
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_idle() {
        let sched = AdaptiveScheduler::new(Instant::now());
        assert_eq!(sched.state(), CaptureState::Idle);
    }

    #[test]
    fn focus_change_moves_idle_to_active() {
        let t0 = Instant::now();
        let mut sched = AdaptiveScheduler::new(t0);
        sched.on_focus_change(t0);
        assert_eq!(sched.state(), CaptureState::Active);
    }

    #[test]
    fn rapid_changes_move_to_interactive() {
        let t0 = Instant::now();
        let mut sched = AdaptiveScheduler::new(t0);
        sched.on_frame_change(t0, true);
        sched.on_frame_change(t0 + Duration::from_millis(100), true);
        sched.on_frame_change(t0 + Duration::from_millis(200), true);
        assert_eq!(sched.state(), CaptureState::Interactive);
    }

    #[test]
    fn slowed_changes_fall_back_to_active() {
        let t0 = Instant::now();
        let mut sched = AdaptiveScheduler::new(t0);
        sched.on_frame_change(t0, true);
        sched.on_frame_change(t0 + Duration::from_millis(100), true);
        sched.on_frame_change(t0 + Duration::from_millis(200), true);
        assert_eq!(sched.state(), CaptureState::Interactive);

        // No more changes for 2s — rolling window empties, tick re-evaluates.
        let later = t0 + Duration::from_secs(2);
        assert_eq!(sched.tick(later), Duration::from_millis(200));
        assert_eq!(sched.state(), CaptureState::Active);
    }

    #[test]
    fn thirty_seconds_no_activity_falls_back_to_idle() {
        let t0 = Instant::now();
        let mut sched = AdaptiveScheduler::new(t0);
        sched.on_focus_change(t0);
        assert_eq!(sched.state(), CaptureState::Active);

        let later = t0 + Duration::from_secs(31);
        assert_eq!(sched.tick(later), Duration::from_secs(1));
        assert_eq!(sched.state(), CaptureState::Idle);
    }
}

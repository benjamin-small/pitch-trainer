//! Per-test adaptive staircase: 3 correct in a row → harder, 1 wrong → easier.

use serde::{Deserialize, Serialize};

pub const MIN_LEVEL: u8 = 1;
pub const MAX_LEVEL: u8 = 8;
const STREAK_TO_LEVEL_UP: u8 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Progress {
    pub level: u8,
    /// Consecutive correct answers since the last level change.
    pub streak: u8,
    pub best_level: u8,
    pub attempts: u32,
    pub correct: u32,
}

impl Default for Progress {
    fn default() -> Self {
        Progress {
            level: MIN_LEVEL,
            streak: 0,
            best_level: MIN_LEVEL,
            attempts: 0,
            correct: 0,
        }
    }
}

impl Progress {
    pub fn record(&mut self, correct: bool) {
        self.attempts += 1;
        if correct {
            self.correct += 1;
            self.streak += 1;
            if self.streak >= STREAK_TO_LEVEL_UP {
                self.level = (self.level + 1).min(MAX_LEVEL);
                self.streak = 0;
            }
        } else {
            self.level = self.level.saturating_sub(1).max(MIN_LEVEL);
            self.streak = 0;
        }
        self.best_level = self.best_level.max(self.level);
    }

    /// Repairs values loaded from untrusted saved data.
    pub fn sanitized(self) -> Self {
        let level = self.level.clamp(MIN_LEVEL, MAX_LEVEL);
        Progress {
            level,
            streak: self.streak.min(STREAK_TO_LEVEL_UP - 1),
            best_level: self.best_level.clamp(level, MAX_LEVEL),
            attempts: self.attempts,
            correct: self.correct.min(self.attempts),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at_level(level: u8) -> Progress {
        Progress {
            level,
            best_level: level,
            ..Progress::default()
        }
    }

    #[test]
    fn starts_at_level_one() {
        let p = Progress::default();
        assert_eq!(
            (p.level, p.streak, p.best_level, p.attempts, p.correct),
            (1, 0, 1, 0, 0)
        );
    }

    #[test]
    fn three_correct_levels_up_and_resets_streak() {
        let mut p = Progress::default();
        p.record(true);
        p.record(true);
        assert_eq!((p.level, p.streak), (1, 2));
        p.record(true);
        assert_eq!((p.level, p.streak), (2, 0));
    }

    #[test]
    fn wrong_levels_down_and_resets_streak() {
        let mut p = at_level(3);
        p.record(true);
        p.record(false);
        assert_eq!((p.level, p.streak), (2, 0));
    }

    #[test]
    fn clamps_at_bounds() {
        let mut low = Progress::default();
        low.record(false);
        assert_eq!(low.level, MIN_LEVEL);

        let mut high = at_level(MAX_LEVEL);
        for _ in 0..3 {
            high.record(true);
        }
        assert_eq!(high.level, MAX_LEVEL);
    }

    #[test]
    fn best_level_never_drops() {
        let mut p = Progress::default();
        for _ in 0..6 {
            p.record(true);
        }
        assert_eq!(p.level, 3);
        p.record(false);
        assert_eq!((p.level, p.best_level), (2, 3));
    }

    #[test]
    fn counts_attempts_and_correct() {
        let mut p = Progress::default();
        p.record(true);
        p.record(false);
        p.record(true);
        assert_eq!((p.attempts, p.correct), (3, 2));
    }

    #[test]
    fn sanitized_repairs_bad_values() {
        let bad = Progress {
            level: 0,
            streak: 9,
            best_level: 20,
            attempts: 2,
            correct: 5,
        };
        let fixed = bad.sanitized();
        assert_eq!(
            fixed,
            Progress {
                level: 1,
                streak: 2,
                best_level: 8,
                attempts: 2,
                correct: 2
            }
        );

        let behind = Progress {
            level: 5,
            best_level: 1,
            ..Progress::default()
        }
        .sanitized();
        assert_eq!(behind.best_level, 5);
    }
}

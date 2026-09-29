//! Generates rounds for the three test types and grades answers.
//!
//! Answer codes: Up/Down 0 = higher, 1 = lower; Pick Two 0 = first, 1 = second;
//! Sequence 0..N-1 = position in the sequence.

use serde::{Deserialize, Serialize};

use crate::difficulty::{MAX_LEVEL, MIN_LEVEL};
use crate::notes::{HIGHEST, LOWEST};
use crate::rng::Rng;
use crate::synth::{NoteEvent, timeline};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TestKind {
    UpDown,
    PickTwo,
    Sequence,
}

impl TestKind {
    pub const ALL: [TestKind; 3] = [TestKind::UpDown, TestKind::PickTwo, TestKind::Sequence];

    pub fn parse(name: &str) -> Option<TestKind> {
        match name {
            "upDown" => Some(TestKind::UpDown),
            "pickTwo" => Some(TestKind::PickTwo),
            "sequence" => Some(TestKind::Sequence),
            _ => None,
        }
    }
}

/// Semitone gap between the two notes (Up/Down, Pick Two), indexed by level - 1.
pub const PAIR_GAPS: [u8; 8] = [12, 9, 7, 5, 4, 3, 2, 1];
/// (length, closest spacing in semitones) for Sequence, indexed by level - 1.
pub const SEQUENCE_LEVELS: [(usize, u8); 8] = [
    (3, 5),
    (4, 5),
    (5, 5),
    (6, 4),
    (7, 3),
    (8, 3),
    (8, 2),
    (8, 1),
];
/// Most extra semitones added to any non-tight gap in a sequence, to keep runs compact.
const MAX_EXTRA_SPACING: u32 = 4;

#[derive(Clone, Debug, PartialEq)]
pub struct Round {
    pub kind: TestKind,
    /// Notes in playback order, excluding the target.
    pub notes: Vec<u8>,
    /// The note played again after the pause (Pick Two, Sequence).
    pub target: Option<u8>,
    /// Correct answer code (see the module docs for the encoding).
    pub answer: u32,
}

impl Round {
    pub fn generate(kind: TestKind, level: u8, rng: &mut Rng) -> Round {
        let index = (level.clamp(MIN_LEVEL, MAX_LEVEL) - 1) as usize;
        match kind {
            TestKind::UpDown => up_down(PAIR_GAPS[index], rng),
            TestKind::PickTwo => pick_two(PAIR_GAPS[index], rng),
            TestKind::Sequence => {
                let (len, spacing) = SEQUENCE_LEVELS[index];
                sequence(len, spacing, rng)
            }
        }
    }

    pub fn options(&self) -> u32 {
        match self.kind {
            TestKind::UpDown | TestKind::PickTwo => 2,
            TestKind::Sequence => self.notes.len() as u32,
        }
    }

    pub fn events(&self) -> Vec<NoteEvent> {
        timeline(&self.notes, self.target)
    }

    pub fn is_correct(&self, answer: u32) -> bool {
        answer == self.answer
    }
}

/// Two notes exactly `gap` apart in random order. Returns (first, second).
fn pair(gap: u8, rng: &mut Rng) -> (u8, u8) {
    let low = rng.range(LOWEST as u32, (HIGHEST - gap) as u32) as u8;
    if rng.coin() {
        (low, low + gap)
    } else {
        (low + gap, low)
    }
}

fn up_down(gap: u8, rng: &mut Rng) -> Round {
    let (a, b) = pair(gap, rng);
    Round {
        kind: TestKind::UpDown,
        notes: vec![a, b],
        target: None,
        answer: if b > a { 0 } else { 1 },
    }
}

fn pick_two(gap: u8, rng: &mut Rng) -> Round {
    let (a, b) = pair(gap, rng);
    let pick = rng.range(0, 1);
    let target = if pick == 0 { a } else { b };
    Round {
        kind: TestKind::PickTwo,
        notes: vec![a, b],
        target: Some(target),
        answer: pick,
    }
}

/// `len` distinct notes whose closest pair is exactly `spacing` apart, in random order.
fn sequence(len: usize, spacing: u8, rng: &mut Rng) -> Round {
    let range = (HIGHEST - LOWEST) as u32;
    let gap_count = len - 1;
    let tight = rng.range(0, gap_count as u32 - 1) as usize;
    let mut slack = range - gap_count as u32 * spacing as u32;
    let mut gaps = vec![spacing as u32; gap_count];
    for (i, gap) in gaps.iter_mut().enumerate() {
        if i != tight {
            let extra = rng.range(0, slack.min(MAX_EXTRA_SPACING));
            *gap += extra;
            slack -= extra;
        }
    }
    let span: u32 = gaps.iter().sum();
    let mut note = LOWEST as u32 + rng.range(0, range - span);
    let mut notes = vec![note as u8];
    for gap in gaps {
        note += gap;
        notes.push(note as u8);
    }
    rng.shuffle(&mut notes);
    let answer = rng.range(0, len as u32 - 1);
    Round {
        kind: TestKind::Sequence,
        target: Some(notes[answer as usize]),
        notes,
        answer,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEEDS: u64 = 300;

    fn in_range(notes: &[u8]) -> bool {
        notes.iter().all(|&n| (LOWEST..=HIGHEST).contains(&n))
    }

    fn closest_spacing(notes: &[u8]) -> u8 {
        let mut sorted = notes.to_vec();
        sorted.sort();
        sorted.windows(2).map(|w| w[1] - w[0]).min().unwrap()
    }

    fn each_round(kind: TestKind, mut check: impl FnMut(u8, &Round)) {
        for level in MIN_LEVEL..=MAX_LEVEL {
            for seed in 0..SEEDS {
                let mut rng = Rng::new(seed);
                let round = Round::generate(kind, level, &mut rng);
                check(level, &round);
            }
        }
    }

    #[test]
    fn up_down_rounds() {
        let mut saw = [false; 2];
        each_round(TestKind::UpDown, |level, r| {
            assert_eq!(r.notes.len(), 2);
            assert!(in_range(&r.notes));
            assert_eq!(r.target, None);
            assert_eq!(r.options(), 2);
            let (a, b) = (r.notes[0], r.notes[1]);
            assert_eq!(a.abs_diff(b), PAIR_GAPS[level as usize - 1]);
            assert_eq!(r.answer, if b > a { 0 } else { 1 });
            saw[r.answer as usize] = true;
        });
        assert_eq!(saw, [true, true]);
    }

    #[test]
    fn pick_two_rounds() {
        let mut saw = [false; 2];
        each_round(TestKind::PickTwo, |level, r| {
            assert_eq!(r.notes.len(), 2);
            assert!(in_range(&r.notes));
            assert_eq!(r.options(), 2);
            assert_eq!(
                r.notes[0].abs_diff(r.notes[1]),
                PAIR_GAPS[level as usize - 1]
            );
            assert_eq!(r.target, Some(r.notes[r.answer as usize]));
            saw[r.answer as usize] = true;
        });
        assert_eq!(saw, [true, true]);
    }

    #[test]
    fn sequence_rounds() {
        each_round(TestKind::Sequence, |level, r| {
            let (len, spacing) = SEQUENCE_LEVELS[level as usize - 1];
            assert_eq!(r.notes.len(), len);
            assert_eq!(r.options(), len as u32);
            assert!(in_range(&r.notes));
            let mut unique = r.notes.clone();
            unique.sort();
            unique.dedup();
            assert_eq!(unique.len(), len, "notes must be distinct: {:?}", r.notes);
            assert_eq!(closest_spacing(&r.notes), spacing, "notes {:?}", r.notes);
            assert!(r.answer < len as u32);
            assert_eq!(r.target, Some(r.notes[r.answer as usize]));
        });
    }

    #[test]
    fn sequence_answer_positions_vary() {
        let mut seen = [false; 8];
        for seed in 0..SEEDS {
            let r = Round::generate(TestKind::Sequence, 8, &mut Rng::new(seed));
            seen[r.answer as usize] = true;
        }
        assert!(seen.iter().all(|&s| s));
    }

    #[test]
    fn out_of_range_levels_are_clamped() {
        let mut rng = Rng::new(0);
        assert_eq!(
            Round::generate(TestKind::Sequence, 0, &mut rng).notes.len(),
            3
        );
        assert_eq!(
            Round::generate(TestKind::Sequence, 99, &mut rng)
                .notes
                .len(),
            8
        );
    }

    #[test]
    fn grading() {
        let r = Round::generate(TestKind::Sequence, 4, &mut Rng::new(9));
        for a in 0..r.options() {
            assert_eq!(r.is_correct(a), a == r.answer);
        }
    }

    #[test]
    fn events_include_target_last() {
        let r = Round::generate(TestKind::PickTwo, 1, &mut Rng::new(5));
        let events = r.events();
        assert_eq!(events.len(), 3);
        assert_eq!(Some(events[2].midi), r.target);
    }

    #[test]
    fn test_kind_names() {
        for kind in TestKind::ALL {
            let json = serde_json::to_string(&kind).unwrap();
            assert_eq!(TestKind::parse(json.trim_matches('"')), Some(kind));
        }
        assert_eq!(
            serde_json::to_string(&TestKind::UpDown).unwrap(),
            "\"upDown\""
        );
        assert_eq!(TestKind::parse("nope"), None);
    }
}

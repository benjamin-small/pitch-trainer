//! The engine's stateful core: owns progress, the current round, and rendering.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::difficulty::Progress;
use crate::rng::Rng;
use crate::rounds::{Round, TestKind};
use crate::synth::{NoteEvent, render, timeline};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SavedState {
    pub up_down: Progress,
    pub pick_two: Progress,
    pub sequence: Progress,
}

impl SavedState {
    /// Parses saved state, falling back to defaults on any problem.
    pub fn from_json(json: &str) -> SavedState {
        match serde_json::from_str::<SavedState>(json) {
            Ok(s) => SavedState {
                up_down: s.up_down.sanitized(),
                pick_two: s.pick_two.sanitized(),
                sequence: s.sequence.sanitized(),
            },
            Err(_) => SavedState::default(),
        }
    }

    fn progress(&self, kind: TestKind) -> &Progress {
        match kind {
            TestKind::UpDown => &self.up_down,
            TestKind::PickTwo => &self.pick_two,
            TestKind::Sequence => &self.sequence,
        }
    }

    fn progress_mut(&mut self, kind: TestKind) -> &mut Progress {
        match kind {
            TestKind::UpDown => &mut self.up_down,
            TestKind::PickTwo => &mut self.pick_two,
            TestKind::Sequence => &mut self.sequence,
        }
    }
}

/// What the UI may know about a round before it is answered.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoundView {
    pub kind: TestKind,
    pub prompt: &'static str,
    pub options: u32,
    /// Onset (seconds) of every sounding note, target last when present.
    pub onsets: Vec<f32>,
    pub has_target: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnswerResult {
    pub correct: bool,
    pub correct_answer: u32,
    /// Every sounding note with its onset, for the reveal replay.
    pub notes: Vec<NoteEvent>,
    pub progress: Progress,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TrainerError {
    NoActiveRound,
    AnswerOutOfRange { answer: u32, options: u32 },
}

impl fmt::Display for TrainerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrainerError::NoActiveRound => write!(f, "no round is waiting for an answer"),
            TrainerError::AnswerOutOfRange { answer, options } => {
                write!(f, "answer {answer} is out of range (round has {options} options)")
            }
        }
    }
}

impl std::error::Error for TrainerError {}

fn prompt(kind: TestKind) -> &'static str {
    match kind {
        TestKind::UpDown => "Was the second note higher or lower than the first?",
        TestKind::PickTwo => "Which note was played again?",
        TestKind::Sequence => "Which position was played again?",
    }
}

pub struct Trainer {
    sample_rate: f32,
    rng: Rng,
    state: SavedState,
    round: Option<Round>,
    answered: bool,
}

impl Trainer {
    pub fn new(sample_rate: f32, seed: u64, saved_json: Option<&str>) -> Trainer {
        Trainer {
            sample_rate,
            rng: Rng::new(seed),
            state: saved_json.map(SavedState::from_json).unwrap_or_default(),
            round: None,
            answered: false,
        }
    }

    pub fn new_round(&mut self, kind: TestKind) -> RoundView {
        let level = self.state.progress(kind).level;
        let round = Round::generate(kind, level, &mut self.rng);
        let view = RoundView {
            kind,
            prompt: prompt(kind),
            options: round.options(),
            onsets: round.events().iter().map(|e| e.onset).collect(),
            has_target: round.target.is_some(),
        };
        self.round = Some(round);
        self.answered = false;
        view
    }

    pub fn round_audio(&self) -> Result<Vec<f32>, TrainerError> {
        let round = self.round.as_ref().ok_or(TrainerError::NoActiveRound)?;
        Ok(self.render_events(&round.events()))
    }

    pub fn answer(&mut self, answer: u32) -> Result<AnswerResult, TrainerError> {
        let round = match &self.round {
            Some(round) if !self.answered => round,
            _ => return Err(TrainerError::NoActiveRound),
        };
        let options = round.options();
        if answer >= options {
            return Err(TrainerError::AnswerOutOfRange { answer, options });
        }
        let correct = round.is_correct(answer);
        let correct_answer = round.answer;
        let notes = round.events();
        let kind = round.kind;

        let progress = self.state.progress_mut(kind);
        progress.record(correct);
        let progress = *progress;
        self.answered = true;
        Ok(AnswerResult { correct, correct_answer, notes, progress })
    }

    pub fn note_audio(&self, midi: u8) -> Vec<f32> {
        self.render_events(&timeline(&[midi], None))
    }

    pub fn scale_notes(&self, from: u8, to: u8) -> Vec<NoteEvent> {
        let notes: Vec<u8> = (from.min(to)..=from.max(to)).collect();
        timeline(&notes, None)
    }

    pub fn render_events(&self, events: &[NoteEvent]) -> Vec<f32> {
        render(events, self.sample_rate)
    }

    pub fn progress(&self, kind: TestKind) -> Progress {
        *self.state.progress(kind)
    }

    pub fn state_json(&self) -> String {
        serde_json::to_string(&self.state).expect("saved state always serializes")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 8000.0;

    fn trainer() -> Trainer {
        Trainer::new(SR, 42, None)
    }

    fn right_answer(t: &Trainer) -> u32 {
        t.round.as_ref().unwrap().answer
    }

    #[test]
    fn view_has_slots_but_no_notes() {
        let mut t = trainer();
        let v = t.new_round(TestKind::PickTwo);
        assert_eq!(v.kind, TestKind::PickTwo);
        assert_eq!(v.options, 2);
        assert_eq!(v.onsets.len(), 3);
        assert!(v.has_target);
        assert!(!v.prompt.is_empty());
        let json = serde_json::to_string(&v).unwrap();
        assert!(!json.contains("midi") && !json.contains("answer"), "view leaks: {json}");
    }

    #[test]
    fn up_down_view_has_no_target() {
        let mut t = trainer();
        let v = t.new_round(TestKind::UpDown);
        assert_eq!(v.onsets.len(), 2);
        assert!(!v.has_target);
    }

    #[test]
    fn round_audio_matches_timeline() {
        let mut t = trainer();
        t.new_round(TestKind::UpDown);
        let audio = t.round_audio().unwrap();
        let note_len = (0.6 * SR).round() as usize;
        assert_eq!(audio.len(), (0.75 * SR).round() as usize + note_len);
    }

    #[test]
    fn answering_records_progress_and_reveals_notes() {
        let mut t = trainer();
        t.new_round(TestKind::Sequence);
        let expected_notes = t.round.as_ref().unwrap().events();
        let a = right_answer(&t);
        let result = t.answer(a).unwrap();
        assert!(result.correct);
        assert_eq!(result.correct_answer, a);
        assert_eq!(result.notes, expected_notes);
        assert_eq!(result.progress.attempts, 1);
        assert_eq!(t.progress(TestKind::Sequence).correct, 1);
        assert_eq!(t.progress(TestKind::UpDown).attempts, 0);
    }

    #[test]
    fn wrong_answer_is_graded_wrong() {
        let mut t = trainer();
        t.new_round(TestKind::UpDown);
        let wrong = 1 - right_answer(&t);
        let result = t.answer(wrong).unwrap();
        assert!(!result.correct);
        assert_eq!(result.correct_answer, 1 - wrong);
    }

    #[test]
    fn one_answer_per_round() {
        let mut t = trainer();
        assert_eq!(t.answer(0), Err(TrainerError::NoActiveRound));
        t.new_round(TestKind::UpDown);
        t.answer(0).unwrap();
        assert_eq!(t.answer(0), Err(TrainerError::NoActiveRound));
        assert!(t.round_audio().is_ok(), "audio stays available for the reveal");
    }

    #[test]
    fn out_of_range_answer_is_rejected_and_round_stays_open() {
        let mut t = trainer();
        t.new_round(TestKind::PickTwo);
        assert_eq!(t.answer(2), Err(TrainerError::AnswerOutOfRange { answer: 2, options: 2 }));
        assert!(t.answer(0).is_ok());
    }

    #[test]
    fn round_audio_before_any_round_errors() {
        assert_eq!(trainer().round_audio(), Err(TrainerError::NoActiveRound));
    }

    #[test]
    fn three_correct_raises_level() {
        let mut t = trainer();
        for _ in 0..3 {
            t.new_round(TestKind::UpDown);
            let a = right_answer(&t);
            t.answer(a).unwrap();
        }
        assert_eq!(t.progress(TestKind::UpDown).level, 2);
    }

    #[test]
    fn state_round_trips_through_json() {
        let mut t = trainer();
        for _ in 0..4 {
            t.new_round(TestKind::PickTwo);
            let a = right_answer(&t);
            t.answer(a).unwrap();
        }
        let json = t.state_json();
        let restored = Trainer::new(SR, 1, Some(&json));
        assert_eq!(restored.progress(TestKind::PickTwo), t.progress(TestKind::PickTwo));
        assert!(json.contains("\"pickTwo\"") && json.contains("\"bestLevel\""), "{json}");
    }

    #[test]
    fn bad_saved_state_falls_back() {
        assert_eq!(SavedState::from_json("not json"), SavedState::default());
        let partial = SavedState::from_json(r#"{"sequence":{"level":5}}"#);
        assert_eq!(partial.sequence.level, 5);
        assert_eq!(partial.sequence.best_level, 5);
        assert_eq!(partial.up_down, Progress::default());
        let wild = SavedState::from_json(r#"{"upDown":{"level":200}}"#);
        assert_eq!(wild.up_down.level, 8);
    }

    #[test]
    fn scale_and_note_audio() {
        let t = trainer();
        let scale = t.scale_notes(60, 72);
        assert_eq!(scale.len(), 13);
        assert_eq!(scale[12].midi, 72);
        assert!(!t.render_events(&scale).is_empty());
        assert_eq!(t.note_audio(69).len(), (0.6 * SR).round() as usize);
    }
}

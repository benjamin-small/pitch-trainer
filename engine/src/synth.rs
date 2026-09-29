//! Renders notes to mono f32 sample buffers.

use serde::Serialize;

use crate::notes::frequency;

pub const NOTE_SECS: f32 = 0.6;
pub const GAP_SECS: f32 = 0.15;
pub const TARGET_PAUSE_SECS: f32 = 0.8;
pub const PEAK: f32 = 0.9;
const ATTACK_SECS: f32 = 0.010;
const RELEASE_SECS: f32 = 0.080;
/// Relative amplitudes of harmonics 1, 2, 3.
const HARMONICS: [f32; 3] = [1.0, 0.3, 0.15];

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteEvent {
    pub midi: u8,
    /// Seconds from the start of the buffer.
    pub onset: f32,
}

/// Lays `notes` out back to back, then `target` after the longer pause.
pub fn timeline(notes: &[u8], target: Option<u8>) -> Vec<NoteEvent> {
    let step = NOTE_SECS + GAP_SECS;
    let mut events: Vec<NoteEvent> = notes
        .iter()
        .enumerate()
        .map(|(i, &midi)| NoteEvent { midi, onset: i as f32 * step })
        .collect();
    if let Some(midi) = target {
        let last_end = events.last().map_or(0.0, |e| e.onset + NOTE_SECS);
        events.push(NoteEvent { midi, onset: last_end + TARGET_PAUSE_SECS });
    }
    events
}

pub fn render(events: &[NoteEvent], sample_rate: f32) -> Vec<f32> {
    let note_len = (NOTE_SECS * sample_rate).round() as usize;
    let starts: Vec<usize> = events
        .iter()
        .map(|e| (e.onset * sample_rate).round() as usize)
        .collect();
    let total = starts.iter().map(|s| s + note_len).max().unwrap_or(0);
    let mut out = vec![0.0; total];
    for (event, &start) in events.iter().zip(&starts) {
        write_note(&mut out[start..start + note_len], frequency(event.midi), sample_rate);
    }
    out
}

fn write_note(buf: &mut [f32], freq: f32, sample_rate: f32) {
    let n = buf.len();
    let attack = (ATTACK_SECS * sample_rate).max(1.0);
    let release = (RELEASE_SECS * sample_rate).max(1.0);
    // Harmonic amplitudes sum to 1.45, so scaling by PEAK / 1.45 keeps |sample| <= PEAK.
    let norm = PEAK / HARMONICS.iter().sum::<f32>();
    for (i, sample) in buf.iter_mut().enumerate() {
        let t = i as f32 / sample_rate;
        let env = (i as f32 / attack).min(1.0) * ((n - 1 - i) as f32 / release).min(1.0);
        let wave: f32 = HARMONICS
            .iter()
            .enumerate()
            .map(|(h, amp)| amp * (std::f32::consts::TAU * freq * (h as f32 + 1.0) * t).sin())
            .sum();
        *sample += wave * env * norm;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 8000.0;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn timeline_spaces_notes_and_pauses_before_target() {
        let events = timeline(&[60, 62, 64], Some(62));
        let onsets: Vec<f32> = events.iter().map(|e| e.onset).collect();
        let midis: Vec<u8> = events.iter().map(|e| e.midi).collect();
        assert_eq!(midis, vec![60, 62, 64, 62]);
        assert!(close(onsets[0], 0.0));
        assert!(close(onsets[1], 0.75));
        assert!(close(onsets[2], 1.5));
        assert!(close(onsets[3], 1.5 + 0.6 + 0.8));
    }

    #[test]
    fn timeline_without_target() {
        let events = timeline(&[60, 72], None);
        assert_eq!(events.len(), 2);
        assert!(close(events[1].onset, 0.75));
    }

    #[test]
    fn single_note_length() {
        let buf = render(&timeline(&[69], None), SR);
        assert_eq!(buf.len(), (0.6 * SR).round() as usize);
    }

    #[test]
    fn sequence_length_ends_with_last_note() {
        let buf = render(&timeline(&[60, 62, 64], Some(62)), SR);
        let last_start = (2.9 * SR).round() as usize;
        assert_eq!(buf.len(), last_start + (0.6 * SR).round() as usize);
    }

    #[test]
    fn buffers_start_and_end_silent() {
        let buf = render(&timeline(&[48, 84], None), SR);
        assert_eq!(buf[0], 0.0);
        assert_eq!(*buf.last().unwrap(), 0.0);
    }

    #[test]
    fn peak_is_bounded_and_audible() {
        for midi in [48u8, 60, 69, 84] {
            let buf = render(&timeline(&[midi], None), SR);
            let peak = buf.iter().fold(0.0f32, |m, s| m.max(s.abs()));
            assert!(peak <= PEAK + 1e-6, "midi {midi} peak {peak}");
            assert!(peak > 0.3, "midi {midi} peak {peak}");
            assert!(buf.iter().all(|s| s.is_finite()));
        }
    }

    #[test]
    fn empty_events_render_empty_buffer() {
        assert!(render(&[], SR).is_empty());
    }
}

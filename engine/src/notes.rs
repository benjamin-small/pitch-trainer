//! Equal-temperament note math. Notes are MIDI numbers (60 = C4, 69 = A4).

/// Lowest note used anywhere in the app (C3).
pub const LOWEST: u8 = 48;
/// Highest note used anywhere in the app (C6).
pub const HIGHEST: u8 = 84;

pub fn frequency(midi: u8) -> f32 {
    440.0 * 2f32.powf((midi as f32 - 69.0) / 12.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a4_is_440() {
        assert!((frequency(69) - 440.0).abs() < 1e-3);
    }

    #[test]
    fn c4_is_middle_c() {
        assert!((frequency(60) - 261.6256).abs() < 1e-2);
    }

    #[test]
    fn octave_doubles_frequency() {
        assert!((frequency(81) / frequency(69) - 2.0).abs() < 1e-5);
    }

    #[test]
    fn range_is_c3_to_c6() {
        assert_eq!(LOWEST, 48);
        assert_eq!(HIGHEST, 84);
    }
}

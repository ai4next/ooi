//! Prologue: charge one neuron until it crosses its threshold.
//!
//! The interaction is deliberately physical. There is no quiz and no wrong
//! button to press: the first thing the player learns is that a neuron sums
//! input, and that enough input makes it fire.

#[derive(Clone, Debug)]
pub struct WakePuzzle {
    charge: f32,
    pulses: u8,
}

impl WakePuzzle {
    pub const THRESHOLD: f32 = 1.0;
    pub const PULSE: f32 = 0.27;

    pub fn new() -> Self {
        Self {
            charge: 0.0,
            pulses: 0,
        }
    }

    pub fn pulse(&mut self) -> bool {
        if self.is_solved() {
            return false;
        }
        self.charge = (self.charge + Self::PULSE).min(Self::THRESHOLD + Self::PULSE);
        self.pulses += 1;
        true
    }

    pub fn charge(&self) -> f32 {
        self.charge
    }

    pub fn normalized_charge(&self) -> f32 {
        (self.charge / Self::THRESHOLD).clamp(0.0, 1.0)
    }

    pub fn pulses(&self) -> u8 {
        self.pulses
    }

    pub fn is_solved(&self) -> bool {
        self.charge >= Self::THRESHOLD
    }

    pub fn solve(&mut self) {
        self.charge = Self::THRESHOLD;
        self.pulses = 4;
    }

    pub fn reset(&mut self) {
        self.charge = 0.0;
        self.pulses = 0;
    }
}

impl Default for WakePuzzle {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_pulses_cross_the_threshold() {
        let mut p = WakePuzzle::new();
        for _ in 0..3 {
            p.pulse();
            assert!(!p.is_solved());
        }
        p.pulse();
        assert!(p.is_solved());
        assert_eq!(p.pulses(), 4);
    }

    #[test]
    fn extra_pulses_do_nothing_after_firing() {
        let mut p = WakePuzzle::new();
        p.solve();
        let before = p.charge();
        assert!(!p.pulse());
        assert_eq!(p.charge(), before);
    }

    #[test]
    fn reset_returns_to_silence() {
        let mut p = WakePuzzle::new();
        p.solve();
        p.reset();
        assert_eq!(p.charge(), 0.0);
        assert_eq!(p.pulses(), 0);
        assert!(!p.is_solved());
    }
}

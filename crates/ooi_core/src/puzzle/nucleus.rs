//! Top-p / nucleus sampling as a playable probability-mass puzzle.
//!
//! A sorted next-token distribution is shown as five rows. The player toggles
//! tokens on and must keep the smallest prefix whose cumulative probability
//! reaches the target (90%). Keeping too many tokens is "too much tail";
//! skipping a more likely token is not a prefix at all.

pub const TOKEN_COUNT: usize = 5;
pub const BAR_WIDTH: f32 = 560.0;

#[derive(Clone, Debug)]
pub struct NucleusConfig {
    pub probs: [f32; TOKEN_COUNT],
    pub threshold: f32,
}

pub fn next_token() -> NucleusConfig {
    NucleusConfig {
        probs: [0.72, 0.12, 0.08, 0.05, 0.03],
        threshold: 0.90,
    }
}

#[derive(Clone, Debug)]
pub struct NucleusPuzzle {
    cfg: NucleusConfig,
    included: [bool; TOKEN_COUNT],
}

impl NucleusPuzzle {
    pub fn new(cfg: NucleusConfig) -> Self {
        Self {
            cfg,
            included: [false; TOKEN_COUNT],
        }
    }

    pub fn config(&self) -> &NucleusConfig {
        &self.cfg
    }

    pub fn prob(&self, index: usize) -> f32 {
        self.cfg.probs.get(index).copied().unwrap_or(0.0)
    }

    pub fn included(&self, index: usize) -> bool {
        self.included.get(index).copied().unwrap_or(false)
    }

    pub fn toggle(&mut self, index: usize) -> bool {
        if index >= TOKEN_COUNT {
            return false;
        }
        self.included[index] = !self.included[index];
        true
    }

    pub fn count(&self) -> usize {
        self.included.iter().filter(|v| **v).count()
    }

    pub fn mass(&self) -> f32 {
        self.included
            .iter()
            .enumerate()
            .filter(|(_, on)| **on)
            .map(|(i, _)| self.cfg.probs[i])
            .sum()
    }

    /// Some(k) only when the included set is exactly the first k tokens.
    pub fn prefix_len(&self) -> Option<usize> {
        let mut k = 0;
        for i in 0..TOKEN_COUNT {
            if self.included[i] {
                if i != k {
                    return None;
                }
                k += 1;
            }
        }
        Some(k)
    }

    pub fn minimal_prefix(&self) -> usize {
        let mut sum = 0.0;
        for i in 0..TOKEN_COUNT {
            sum += self.cfg.probs[i];
            if sum >= self.cfg.threshold {
                return i + 1;
            }
        }
        TOKEN_COUNT
    }

    pub fn is_solved(&self) -> bool {
        let Some(k) = self.prefix_len() else {
            return false;
        };
        if k == 0 {
            return false;
        }
        let mut sum = 0.0;
        for i in 0..k {
            sum += self.cfg.probs[i];
        }
        let before = sum - self.cfg.probs[k - 1];
        before < self.cfg.threshold && sum >= self.cfg.threshold
    }

    pub fn is_too_small(&self) -> bool {
        self.prefix_len().is_some() && self.count() > 0 && self.mass() < self.cfg.threshold
    }

    pub fn is_too_much(&self) -> bool {
        let Some(k) = self.prefix_len() else {
            return false;
        };
        if k == 0 {
            return false;
        }
        let mut sum = 0.0;
        for i in 0..k {
            sum += self.cfg.probs[i];
        }
        let before = sum - self.cfg.probs[k - 1];
        before >= self.cfg.threshold
    }

    pub fn solve(&mut self) {
        self.included = [false; TOKEN_COUNT];
        for i in 0..self.minimal_prefix() {
            self.included[i] = true;
        }
    }

    pub fn reset(&mut self) {
        self.included = [false; TOKEN_COUNT];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probabilities_are_normalized() {
        let puzzle = NucleusPuzzle::new(next_token());
        let sum: f32 = (0..TOKEN_COUNT).map(|i| puzzle.prob(i)).sum();
        assert!((sum - 1.0).abs() < 1e-6);
    }

    #[test]
    fn no_tokens_is_not_solved() {
        let puzzle = NucleusPuzzle::new(next_token());
        assert_eq!(puzzle.count(), 0);
        assert!(!puzzle.is_solved());
    }

    #[test]
    fn top_three_tokens_are_the_minimal_nucleus() {
        let mut puzzle = NucleusPuzzle::new(next_token());
        for i in 0..3 {
            assert!(puzzle.toggle(i));
        }
        assert!((puzzle.mass() - 0.92).abs() < 1e-6);
        assert_eq!(puzzle.prefix_len(), Some(3));
        assert!(puzzle.is_solved());
        assert!(!puzzle.is_too_small());
        assert!(!puzzle.is_too_much());
    }

    #[test]
    fn two_tokens_are_too_small() {
        let mut puzzle = NucleusPuzzle::new(next_token());
        puzzle.toggle(0);
        puzzle.toggle(1);
        assert!(puzzle.is_too_small());
        assert!(!puzzle.is_solved());
    }

    #[test]
    fn four_tokens_are_too_much_tail() {
        let mut puzzle = NucleusPuzzle::new(next_token());
        for i in 0..4 {
            puzzle.toggle(i);
        }
        assert!(puzzle.is_too_much());
        assert!(!puzzle.is_solved());
    }

    #[test]
    fn skipping_a_likely_token_breaks_the_prefix() {
        let mut puzzle = NucleusPuzzle::new(next_token());
        puzzle.toggle(0);
        puzzle.toggle(2);
        assert_eq!(puzzle.prefix_len(), None);
        assert!(!puzzle.is_solved());
    }

    #[test]
    fn solve_builds_the_minimal_prefix_and_reset_clears_it() {
        let mut puzzle = NucleusPuzzle::new(next_token());
        puzzle.solve();
        assert!(puzzle.is_solved());
        puzzle.reset();
        assert_eq!(puzzle.count(), 0);
        assert!(!puzzle.is_solved());
    }
}

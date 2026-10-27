//! 词向量：在二维平面上拖动一个词，补全平行四边形。
//!
//! 玩家要满足 `king - man + woman ≈ queen`，本质上是在拖一个向量加法。

#[derive(Clone, Debug)]
pub struct VectorConfig {
    pub initial: (f32, f32),
    pub target: (f32, f32),
    pub tolerance: f32,
}

#[derive(Clone, Debug)]
pub struct VectorPuzzle {
    cfg: VectorConfig,
    pos: (f32, f32),
}

impl VectorPuzzle {
    pub fn new(cfg: VectorConfig) -> Self {
        let pos = cfg.initial;
        Self { cfg, pos }
    }

    pub fn config(&self) -> &VectorConfig {
        &self.cfg
    }

    pub fn pos(&self) -> (f32, f32) {
        self.pos
    }

    pub fn set_pos(&mut self, x: f32, y: f32) {
        self.pos = (x.clamp(-0.95, 0.95), y.clamp(-0.95, 0.95));
    }

    pub fn distance(&self) -> f32 {
        let dx = self.pos.0 - self.cfg.target.0;
        let dy = self.pos.1 - self.cfg.target.1;
        (dx * dx + dy * dy).sqrt()
    }

    pub fn is_solved(&self) -> bool {
        self.distance() <= self.cfg.tolerance
    }

    pub fn reset(&mut self) {
        self.pos = self.cfg.initial;
    }
}

pub fn word_analogy() -> VectorConfig {
    VectorConfig {
        initial: (0.0, -0.28),
        target: (0.32, 0.52),
        tolerance: 0.14,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_zone_solves() {
        let mut p = VectorPuzzle::new(word_analogy());
        assert!(!p.is_solved());
        p.set_pos(0.32, 0.52);
        assert!(p.is_solved());
    }

    #[test]
    fn just_outside_fails() {
        let mut p = VectorPuzzle::new(word_analogy());
        p.set_pos(0.32, 0.52 + 0.2);
        assert!(!p.is_solved());
    }

    #[test]
    fn clamps_to_plane() {
        let mut p = VectorPuzzle::new(word_analogy());
        p.set_pos(5.0, -5.0);
        assert_eq!(p.pos(), (0.95, -0.95));
    }

    #[test]
    fn reset_returns_to_initial() {
        let mut p = VectorPuzzle::new(word_analogy());
        p.set_pos(0.32, 0.52);
        p.reset();
        assert!(!p.is_solved());
    }
}

//! 二维线性边界：玩家调 w1 / w2 / b，看分界线能不能把两类点分开。
//!
//! 1.2 用 AND（可解），1.4 用 XOR（不可解）——后者靠"试够次数"推进剧情。

use super::threshold::Confusion;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point2 {
    pub x: f32,
    pub y: f32,
    pub label: bool,
}

impl Point2 {
    pub const fn new(x: f32, y: f32, label: bool) -> Self {
        Self { x, y, label }
    }
}

#[derive(Clone, Debug)]
pub struct BoundaryConfig {
    pub points: Vec<Point2>,
    pub w_range: (f32, f32),
    pub b_range: (f32, f32),
    pub step: f32,
    pub initial: (f32, f32, f32),
    /// 若设置，试够这么多次后即使仍有错也判定"此路不通"。
    pub impossible_after: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct BoundaryPuzzle {
    cfg: BoundaryConfig,
    w1: f32,
    w2: f32,
    b: f32,
    attempts: u32,
}

fn snap(v: f32, step: f32) -> f32 {
    let snapped = (v / step).round() * step;
    (snapped as f64 * 1_000_000.0).round() as f32 / 1_000_000.0
}

impl BoundaryPuzzle {
    pub fn new(cfg: BoundaryConfig) -> Self {
        let (w1, w2, b) = cfg.initial;
        let mut this = Self {
            cfg,
            w1,
            w2,
            b,
            attempts: 0,
        };
        this.set_weights(w1, w2, b);
        this
    }

    pub fn config(&self) -> &BoundaryConfig {
        &self.cfg
    }

    pub fn set_weights(&mut self, w1: f32, w2: f32, b: f32) {
        let step = self.cfg.step;
        self.w1 = snap(w1.clamp(self.cfg.w_range.0, self.cfg.w_range.1), step);
        self.w2 = snap(w2.clamp(self.cfg.w_range.0, self.cfg.w_range.1), step);
        self.b = snap(b.clamp(self.cfg.b_range.0, self.cfg.b_range.1), step);
    }

    pub fn weights(&self) -> (f32, f32, f32) {
        (self.w1, self.w2, self.b)
    }

    pub fn attempts(&self) -> u32 {
        self.attempts
    }

    pub fn fires(&self, p: &Point2) -> bool {
        self.w1 * p.x + self.w2 * p.y + self.b >= 0.0
    }

    pub fn confusion(&self) -> Confusion {
        let mut c = Confusion::default();
        for p in &self.cfg.points {
            match (self.fires(p), p.label) {
                (true, false) => c.false_pos += 1,
                (false, true) => c.false_neg += 1,
                _ => {}
            }
        }
        c
    }

    /// 玩家按下"试运行"：计一次尝试。
    pub fn commit(&mut self) -> Confusion {
        self.attempts += 1;
        self.confusion()
    }

    pub fn is_solved(&self) -> bool {
        self.confusion().is_clean()
            || self
                .cfg
                .impossible_after
                .is_some_and(|n| self.attempts >= n)
    }

    pub fn reset(&mut self) {
        let (w1, w2, b) = self.cfg.initial;
        self.attempts = 0;
        self.set_weights(w1, w2, b);
    }

    /// 当前分界线的两个端点，基于 0..1 的方形画布。
    pub fn line_endpoints(&self) -> ((f32, f32), (f32, f32)) {
        let (w1, w2, b) = (self.w1, self.w2, self.b);
        if w2.abs() < 1e-4 {
            if w1.abs() < 1e-4 {
                return ((0.5, 0.0), (0.5, 1.0));
            }
            let x = -b / w1;
            return ((x, 0.0), (x, 1.0));
        }
        let y_at = |x: f32| -(w1 * x + b) / w2;
        ((0.0, y_at(0.0)), (1.0, y_at(1.0)))
    }
}

pub fn and_gate() -> BoundaryConfig {
    BoundaryConfig {
        points: vec![
            Point2::new(0.0, 0.0, false),
            Point2::new(1.0, 0.0, false),
            Point2::new(0.0, 1.0, false),
            Point2::new(1.0, 1.0, true),
        ],
        w_range: (-2.0, 2.0),
        b_range: (-2.0, 2.0),
        step: 0.1,
        initial: (1.0, 1.0, -1.0),
        impossible_after: None,
    }
}

pub fn xor_gate() -> BoundaryConfig {
    BoundaryConfig {
        points: vec![
            Point2::new(0.0, 0.0, false),
            Point2::new(1.0, 0.0, true),
            Point2::new(0.0, 1.0, true),
            Point2::new(1.0, 1.0, false),
        ],
        w_range: (-2.0, 2.0),
        b_range: (-2.0, 2.0),
        step: 0.1,
        initial: (1.0, 1.0, -0.5),
        impossible_after: Some(8),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn and_gate_is_solvable() {
        let mut p = BoundaryPuzzle::new(and_gate());
        assert!(!p.is_solved());
        p.set_weights(1.0, 1.0, -1.5);
        assert_eq!(p.confusion(), Confusion::default());
        assert!(p.is_solved());
    }

    #[test]
    fn xor_wall_needs_attempts() {
        let mut p = BoundaryPuzzle::new(xor_gate());
        for _ in 0..7 {
            p.commit();
            assert!(!p.is_solved());
        }
        p.commit();
        assert!(p.is_solved(), "8th attempt should break the wall");
    }

    #[test]
    fn snapping_and_clamping() {
        let mut p = BoundaryPuzzle::new(and_gate());
        p.set_weights(9.0, -9.0, 0.04);
        assert_eq!(p.weights(), (2.0, -2.0, 0.0));
    }

    #[test]
    fn reset_clears_attempts() {
        let mut p = BoundaryPuzzle::new(xor_gate());
        p.commit();
        p.reset();
        assert_eq!(p.attempts(), 0);
        assert!(!p.is_solved());
    }
}

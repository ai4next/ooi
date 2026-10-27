//! One continuous step at a time. The player aims a fixed-length step on a
//! convex bowl; the amber arrow shows the uphill gradient, so the lesson is to
//! aim opposite it. A step only counts if it actually lowers the loss.

pub const DOMAIN: f32 = 1.0;

pub fn loss(x: f32, y: f32) -> f32 {
    0.5 * (x * x + y * y)
}

pub fn normalized_loss(x: f32, y: f32) -> f32 {
    loss(x, y).clamp(0.0, 1.0)
}

pub fn gradient(x: f32, y: f32) -> (f32, f32) {
    (x, y)
}

#[derive(Clone, Debug)]
pub struct GradientConfig {
    pub start: (f32, f32),
    pub target: (f32, f32),
    pub step_radius: f32,
    pub solve_radius: f32,
    pub max_steps: u32,
}

pub fn bowl() -> GradientConfig {
    GradientConfig {
        start: (0.68, 0.46),
        target: (0.0, 0.0),
        step_radius: 0.18,
        solve_radius: 0.10,
        max_steps: 14,
    }
}

#[derive(Clone, Debug)]
pub struct GradientPuzzle {
    cfg: GradientConfig,
    position: (f32, f32),
    aim_angle: f32,
    trail: Vec<(f32, f32)>,
    steps: u32,
    rejected: bool,
    solved: bool,
}

impl GradientPuzzle {
    pub fn new(cfg: GradientConfig) -> Self {
        let position = cfg.start;
        Self {
            cfg,
            position,
            aim_angle: 0.0,
            trail: vec![position],
            steps: 0,
            rejected: false,
            solved: false,
        }
    }

    pub fn config(&self) -> &GradientConfig {
        &self.cfg
    }

    pub fn position(&self) -> (f32, f32) {
        self.position
    }

    pub fn aim_angle(&self) -> f32 {
        self.aim_angle
    }

    pub fn set_aim(&mut self, angle: f32) {
        if (angle - self.aim_angle).abs() > 1e-4 {
            self.rejected = false;
        }
        self.aim_angle = angle;
    }

    pub fn step_radius(&self) -> f32 {
        self.cfg.step_radius
    }

    pub fn candidate(&self) -> (f32, f32) {
        let (x, y) = self.position;
        (
            x + self.cfg.step_radius * self.aim_angle.cos(),
            y + self.cfg.step_radius * self.aim_angle.sin(),
        )
    }

    pub fn current_loss(&self) -> f32 {
        loss(self.position.0, self.position.1)
    }

    pub fn candidate_loss(&self) -> f32 {
        let c = self.candidate();
        loss(c.0, c.1)
    }

    pub fn gradient(&self) -> (f32, f32) {
        gradient(self.position.0, self.position.1)
    }

    pub fn trail(&self) -> &[(f32, f32)] {
        &self.trail
    }

    pub fn steps(&self) -> u32 {
        self.steps
    }

    pub fn rejected(&self) -> bool {
        self.rejected
    }

    pub fn is_solved(&self) -> bool {
        self.solved
    }

    /// Apply one step. Returns true only if the candidate is downhill.
    pub fn step(&mut self) -> bool {
        if self.solved {
            return false;
        }
        let candidate = self.candidate();
        let current = self.current_loss();
        let next = loss(candidate.0, candidate.1);
        if next + 1e-6 < current {
            self.position = candidate;
            self.trail.push(candidate);
            self.steps += 1;
            self.rejected = false;
            let dx = self.position.0 - self.cfg.target.0;
            let dy = self.position.1 - self.cfg.target.1;
            self.solved = dx.hypot(dy) <= self.cfg.solve_radius;
            true
        } else {
            self.rejected = true;
            false
        }
    }

    /// Aim straight at the target and take one step. Used by hints.
    pub fn solve(&mut self) {
        for _ in 0..self.cfg.max_steps + 2 {
            if self.solved {
                return;
            }
            let (x, y) = self.position;
            let dx = self.cfg.target.0 - x;
            let dy = self.cfg.target.1 - y;
            self.set_aim(dy.atan2(dx));
            if !self.step() {
                break;
            }
        }
        if !self.solved {
            self.position = self.cfg.target;
            self.trail.push(self.cfg.target);
            self.solved = true;
        }
    }

    pub fn reset(&mut self) {
        self.position = self.cfg.start;
        self.aim_angle = 0.0;
        self.trail.clear();
        self.trail.push(self.cfg.start);
        self.steps = 0;
        self.rejected = false;
        self.solved = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_aim_is_uphill_and_rejected() {
        let mut puzzle = GradientPuzzle::new(bowl());
        // At the start, +x points away from the centre, so loss rises.
        assert!(!puzzle.step());
        assert!(puzzle.rejected());
        assert_eq!(puzzle.steps(), 0);
        assert_eq!(puzzle.position(), bowl().start);
    }

    #[test]
    fn aiming_at_the_target_steps_downhill() {
        let mut puzzle = GradientPuzzle::new(bowl());
        let (x, y) = puzzle.position();
        puzzle.set_aim((-y).atan2(-x));
        assert!(puzzle.step());
        assert_eq!(puzzle.steps(), 1);
        assert!(puzzle.trail().len() == 2);
        assert!(puzzle.candidate_loss() < puzzle.current_loss());
    }

    #[test]
    fn solve_reaches_the_bowl_center() {
        let mut puzzle = GradientPuzzle::new(bowl());
        puzzle.solve();
        assert!(puzzle.is_solved());
        assert!((puzzle.position().0.hypot(puzzle.position().1)) <= 0.10 + 1e-6);
    }

    #[test]
    fn candidate_is_one_step_radius_away() {
        let mut puzzle = GradientPuzzle::new(bowl());
        puzzle.set_aim(0.0);
        let (x, y) = puzzle.candidate();
        assert!((x - 0.68 - 0.18).abs() < 1e-6);
        assert!((y - 0.46).abs() < 1e-6);
    }

    #[test]
    fn reset_restores_start_and_clears_trail() {
        let mut puzzle = GradientPuzzle::new(bowl());
        puzzle.solve();
        puzzle.reset();
        assert!(!puzzle.is_solved());
        assert_eq!(puzzle.steps(), 0);
        assert_eq!(puzzle.trail().len(), 1);
        assert!(!puzzle.rejected());
    }
}

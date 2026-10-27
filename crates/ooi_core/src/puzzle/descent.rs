//! Interactive loss landscape: choose a start, watch gradient descent find a
//! basin. Two of the three starts land in the shallow local minimum; one lands
//! in the global minimum. The math is real, so the visual and the lesson agree.

/// How far the landscape extends in each direction. Kept public for the view.
pub const DOMAIN_X: f32 = 1.45;
pub const DOMAIN_Y: f32 = 1.15;

/// Approximate loss range used to colour the field. The exact extrema are not
/// important; a stable range avoids flickering colours while the player looks
/// around.
pub const LOSS_MIN: f32 = -0.18;
pub const LOSS_MAX: f32 = 2.30;

/// `f(x, y) = (x² − 1)² + 0.6y² + 0.16x`
///
/// Two minima on the y = 0 axis. The small linear term makes the left basin
/// deeper; the right basin is the local minimum.
pub fn loss(x: f32, y: f32) -> f32 {
    let x2 = x * x;
    (x2 - 1.0).powi(2) + 0.6 * y * y + 0.16 * x
}

pub fn gradient(x: f32, y: f32) -> (f32, f32) {
    (4.0 * x * (x * x - 1.0) + 0.16, 1.2 * y)
}

pub fn normalized_loss(x: f32, y: f32) -> f32 {
    ((loss(x, y) - LOSS_MIN) / (LOSS_MAX - LOSS_MIN)).clamp(0.0, 1.0)
}

#[derive(Clone, Debug)]
pub struct DescentConfig {
    /// Three deliberately chosen starts: left (global), right (local), and
    /// upper-right (also local). The player has to prefer the deeper basin.
    pub starts: [(f32, f32); 3],
    pub global: (f32, f32),
    /// The shallow local minimum, exposed so the view can draw both basins.
    pub local: (f32, f32),
    pub learning_rate: f32,
    pub max_steps: u32,
    pub solve_radius: f32,
}

pub fn loss_landscape() -> DescentConfig {
    DescentConfig {
        starts: [(-1.10, 0.85), (0.95, 0.80), (0.35, 1.05)],
        global: (-1.019_43, 0.0),
        local: (0.979_37, 0.0),
        learning_rate: 0.10,
        max_steps: 150,
        solve_radius: 0.12,
    }
}

#[derive(Clone, Debug)]
pub struct DescentPuzzle {
    cfg: DescentConfig,
    selected: Option<usize>,
    position: Option<(f32, f32)>,
    trail: Vec<(f32, f32)>,
    steps: u32,
    running: bool,
    finished: bool,
}

impl DescentPuzzle {
    pub fn new(cfg: DescentConfig) -> Self {
        Self {
            cfg,
            selected: None,
            position: None,
            trail: Vec::new(),
            steps: 0,
            running: false,
            finished: false,
        }
    }

    pub fn config(&self) -> &DescentConfig {
        &self.cfg
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    pub fn position(&self) -> Option<(f32, f32)> {
        self.position
    }

    pub fn trail(&self) -> &[(f32, f32)] {
        &self.trail
    }

    pub fn steps(&self) -> u32 {
        self.steps
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Pick one of the three starts. Selecting during a descent restarts from
    /// the clicked start, which makes comparing basins fast.
    pub fn select(&mut self, index: usize) -> bool {
        if index >= self.cfg.starts.len() {
            return false;
        }
        self.selected = Some(index);
        let start = self.cfg.starts[index];
        self.position = Some(start);
        self.trail.clear();
        self.trail.push(start);
        self.steps = 0;
        self.running = false;
        self.finished = false;
        true
    }

    pub fn run(&mut self) -> bool {
        if self.position.is_none() || self.running || self.finished {
            return false;
        }
        self.running = true;
        true
    }

    /// One gradient-descent step. Returns true if the step ran.
    pub fn step(&mut self) -> bool {
        if !self.running {
            return false;
        }
        let (x, y) = self.position.unwrap_or((0.0, 0.0));
        let (gx, gy) = gradient(x, y);
        let nx = x - self.cfg.learning_rate * gx;
        let ny = y - self.cfg.learning_rate * gy;
        self.position = Some((nx, ny));
        self.trail.push((nx, ny));
        self.steps += 1;

        let (ngx, ngy) = gradient(nx, ny);
        if ngx.hypot(ngy) < 1e-4 || self.steps >= self.cfg.max_steps {
            self.running = false;
            self.finished = true;
        }
        true
    }

    pub fn current_loss(&self) -> f32 {
        self.position
            .map(|(x, y)| loss(x, y))
            .unwrap_or(f32::INFINITY)
    }

    pub fn is_solved(&self) -> bool {
        if !self.finished {
            return false;
        }
        let Some((x, y)) = self.position else {
            return false;
        };
        (x - self.cfg.global.0).hypot(y - self.cfg.global.1) <= self.cfg.solve_radius
    }

    /// Used by the third hint and the test harness.
    pub fn solve(&mut self) {
        // Start 0 is the global basin by construction; the loop is still
        // bounded by max_steps and writes the exact global point as a fallback.
        self.select(0);
        self.run();
        for _ in 0..self.cfg.max_steps + 32 {
            self.step();
            if self.is_solved() {
                return;
            }
        }
        self.position = Some(self.cfg.global);
        self.running = false;
        self.finished = true;
    }

    pub fn reset(&mut self) {
        self.selected = None;
        self.position = None;
        self.trail.clear();
        self.steps = 0;
        self.running = false;
        self.finished = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descent_from(index: usize) -> DescentPuzzle {
        let mut puzzle = DescentPuzzle::new(loss_landscape());
        assert!(puzzle.select(index));
        assert!(puzzle.run());
        for _ in 0..220 {
            puzzle.step();
        }
        puzzle
    }

    #[test]
    fn global_start_finds_the_deep_basin() {
        let puzzle = descent_from(0);
        assert!(puzzle.is_finished());
        assert!(
            puzzle.is_solved(),
            "left start should reach the global basin"
        );
    }

    #[test]
    fn local_starts_do_not_solve() {
        for index in [1, 2] {
            let puzzle = descent_from(index);
            assert!(puzzle.is_finished());
            assert!(
                !puzzle.is_solved(),
                "start {index} unexpectedly reached the global basin"
            );
        }
    }

    #[test]
    fn solve_always_reaches_the_global_minimum() {
        let mut puzzle = DescentPuzzle::new(loss_landscape());
        assert!(!puzzle.is_solved());
        puzzle.solve();
        assert!(puzzle.is_solved());
    }

    #[test]
    fn selecting_another_start_resets_the_run() {
        let mut puzzle = DescentPuzzle::new(loss_landscape());
        puzzle.select(1);
        puzzle.run();
        puzzle.step();
        puzzle.step();
        assert_eq!(puzzle.steps(), 2);
        assert!(puzzle.select(0));
        assert_eq!(puzzle.steps(), 0);
        assert_eq!(puzzle.trail().len(), 1);
        assert!(!puzzle.is_running());
    }

    #[test]
    fn starts_are_inside_the_field_and_left_basin_is_lower() {
        let cfg = loss_landscape();
        for (x, y) in cfg.starts {
            assert!(x.abs() <= DOMAIN_X && y.abs() <= DOMAIN_Y);
        }
        assert!(loss(cfg.global.0, cfg.global.1) < loss(cfg.local.0, cfg.local.1));
    }

    #[test]
    fn reset_returns_to_choose_a_start() {
        let mut puzzle = descent_from(0);
        puzzle.reset();
        assert!(puzzle.selected().is_none());
        assert!(puzzle.position().is_none());
        assert!(!puzzle.is_finished());
        assert!(!puzzle.is_solved());
    }
}

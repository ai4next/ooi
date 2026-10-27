//! 阈值谜题：一个权重、一个阈值。
//!
//! 设计要点（见 `docs/levels/01-1-threshold.md`）：
//!
//! - 阈值落在 `(3.6, 4.2]` 才算过，屏幕正中间的 5.0 **必须失败**；
//! - 比较必须用 `>=`，且比较前统一 snap，避免浮点把 3.7 判成 3.699…；
//! - 初始值 6.0 是**刻意不合法**的——玩家必须先动手。

use std::fmt;

/// 一个光点：亮度 + 真实标签。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    /// 输入亮度，0..=10。
    pub x: f32,
    /// true = 萤火（应该放电），false = 余烬（应该沉默）。
    pub label: bool,
}

impl Sample {
    pub const fn new(x: f32, label: bool) -> Self {
        Self { x, label }
    }
}

/// 混淆计数。第一关只关心总数，但分开记才能在文案里说"是哪一种错"。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Confusion {
    /// 余烬被点亮。
    pub false_pos: u32,
    /// 萤火没亮。
    pub false_neg: u32,
}

impl Confusion {
    pub fn errors(self) -> u32 {
        self.false_pos + self.false_neg
    }

    pub fn is_clean(self) -> bool {
        self.errors() == 0
    }
}

impl fmt::Display for Confusion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "误报 {} / 漏报 {}", self.false_pos, self.false_neg)
    }
}

#[derive(Clone, Debug)]
pub struct ThresholdConfig {
    pub samples: Vec<Sample>,
    pub holdout: Vec<Sample>,
    pub theta_min: f32,
    pub theta_max: f32,
    pub step: f32,
    pub initial_theta: f32,
}

impl ThresholdConfig {
    /// 网格上的最大下标（含）。
    pub fn max_index(&self) -> i32 {
        (((self.theta_max - self.theta_min) / self.step).round()) as i32
    }
}

#[derive(Clone, Debug)]
pub struct ThresholdPuzzle {
    cfg: ThresholdConfig,
    index: i32,
}

impl ThresholdPuzzle {
    pub fn new(cfg: ThresholdConfig) -> Self {
        let mut this = Self { cfg, index: 0 };
        this.set_theta(this.cfg.initial_theta);
        this
    }

    pub fn config(&self) -> &ThresholdConfig {
        &self.cfg
    }

    /// 当前阈值。
    pub fn theta(&self) -> f32 {
        self.theta_at(self.index)
    }

    /// 网格下标 → 阈值。
    ///
    /// 关键：`36 * 0.1f32` 在 f32 里会得到 `3.6000001`，比数据里的 `3.6` 大一点点，
    /// 于是边界点被误判成"没点亮"。所以先在 f64 里按十进制四舍五入到 6 位，
    /// 再转 f32 —— 保证网格值和关卡数据落在同一个浮点数上。
    pub fn theta_at(&self, index: i32) -> f32 {
        let idx = index.clamp(0, self.cfg.max_index());
        let raw = self.cfg.theta_min as f64 + idx as f64 * self.cfg.step as f64;
        let rounded = (raw * 1_000_000.0).round() / 1_000_000.0;
        rounded as f32
    }

    /// 阈值 → 网格下标（四舍五入到最近一格）。
    pub fn index_of(&self, theta: f32) -> i32 {
        (((theta - self.cfg.theta_min) / self.cfg.step).round() as i32)
            .clamp(0, self.cfg.max_index())
    }

    pub fn index(&self) -> i32 {
        self.index
    }

    /// 设置阈值：先钳制到 `[min, max]`，再吸附到网格。
    pub fn set_theta(&mut self, theta: f32) {
        self.index = self.index_of(theta);
    }

    /// 神经元是否放电。`>=` 是全项目统一语义。
    pub fn fires(&self, x: f32) -> bool {
        x >= self.theta()
    }

    fn confusion_for(&self, samples: &[Sample]) -> Confusion {
        let mut c = Confusion::default();
        for s in samples {
            match (self.fires(s.x), s.label) {
                (true, false) => c.false_pos += 1,
                (false, true) => c.false_neg += 1,
                _ => {}
            }
        }
        c
    }

    pub fn confusion(&self) -> Confusion {
        self.confusion_for(&self.cfg.samples)
    }

    pub fn holdout_confusion(&self) -> Confusion {
        self.confusion_for(&self.cfg.holdout)
    }

    pub fn is_solved(&self) -> bool {
        self.confusion().is_clean()
    }

    pub fn holdout_passes(&self) -> bool {
        self.holdout_confusion().is_clean()
    }

    /// 所有能过关的网格下标。给测试和提示用，UI 不显示。
    pub fn valid_indices(&self) -> Vec<i32> {
        (0..=self.cfg.max_index())
            .filter(|&i| {
                let mut probe = self.clone();
                probe.index = i;
                probe.is_solved()
            })
            .collect()
    }

    pub fn valid_thetas(&self) -> Vec<f32> {
        self.valid_indices()
            .into_iter()
            .map(|i| self.theta_at(i))
            .collect()
    }

    pub fn reset(&mut self) {
        self.set_theta(self.cfg.initial_theta);
    }
}

/// 关卡 1.1 的正式数据。
pub fn default_ch1_l1() -> ThresholdConfig {
    let embers = [
        1.0_f32, 1.3, 1.5, 1.8, 2.0, 2.2, 2.5, 2.7, 2.9, 3.1, 3.4, 3.6,
    ];
    let fireflies = [
        4.2_f32, 4.3, 4.5, 4.7, 4.9, 5.2, 5.5, 5.8, 6.2, 6.6, 7.1, 7.9,
    ];

    let mut samples = Vec::with_capacity(24);
    samples.extend(embers.iter().map(|&x| Sample::new(x, false)));
    samples.extend(fireflies.iter().map(|&x| Sample::new(x, true)));

    let holdout = [
        Sample::new(0.9, false),
        Sample::new(2.6, false),
        Sample::new(3.5, false),
        Sample::new(4.2, true),
        Sample::new(5.5, true),
        Sample::new(7.4, true),
    ];

    ThresholdConfig {
        samples,
        holdout: holdout.to_vec(),
        theta_min: 0.0,
        theta_max: 10.0,
        step: 0.1,
        initial_theta: 6.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> ThresholdConfig {
        default_ch1_l1()
    }

    fn puzzle_at(theta: f32) -> ThresholdPuzzle {
        let mut p = ThresholdPuzzle::new(cfg());
        p.set_theta(theta);
        p
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn initial_state_has_eight_misses() {
        let p = ThresholdPuzzle::new(cfg());
        assert!(close(p.theta(), 6.0));
        assert_eq!(
            p.confusion(),
            Confusion {
                false_pos: 0,
                false_neg: 8
            }
        );
        assert!(!p.is_solved());
    }

    #[test]
    fn middle_of_screen_fails() {
        let p = puzzle_at(5.0);
        assert_eq!(
            p.confusion(),
            Confusion {
                false_pos: 0,
                false_neg: 5
            }
        );
        assert!(!p.is_solved());
    }

    #[test]
    fn boundary_is_inclusive() {
        let p = puzzle_at(4.2);
        assert!(p.fires(4.2));
        assert!(p.is_solved());
    }

    #[test]
    fn one_step_left_causes_false_positive() {
        let p = puzzle_at(3.6);
        assert_eq!(
            p.confusion(),
            Confusion {
                false_pos: 1,
                false_neg: 0
            }
        );
    }

    #[test]
    fn one_step_right_causes_false_negative() {
        let p = puzzle_at(4.3);
        assert_eq!(
            p.confusion(),
            Confusion {
                false_pos: 0,
                false_neg: 1
            }
        );
    }

    #[test]
    fn valid_window_is_exactly_six_positions() {
        let p = ThresholdPuzzle::new(cfg());
        assert_eq!(p.valid_indices(), vec![37, 38, 39, 40, 41, 42]);
        let thetas = p.valid_thetas();
        assert_eq!(thetas.len(), 6);
        assert!(close(thetas[0], 3.7));
        assert!(close(thetas[5], 4.2));
    }

    #[test]
    fn snapping_is_idempotent() {
        let mut p = ThresholdPuzzle::new(cfg());
        p.set_theta(3.749);
        assert!(close(p.theta(), 3.7));
        p.set_theta(p.theta());
        assert!(close(p.theta(), 3.7));
        assert_eq!(p.index(), 37);
    }

    #[test]
    fn clamping_out_of_range() {
        let mut p = ThresholdPuzzle::new(cfg());
        p.set_theta(-5.0);
        assert!(close(p.theta(), 0.0));
        p.set_theta(99.0);
        assert!(close(p.theta(), 10.0));
        assert_eq!(p.index(), 100);
    }

    #[test]
    fn every_valid_theta_passes_holdout() {
        let p = ThresholdPuzzle::new(cfg());
        for idx in p.valid_indices() {
            let mut q = p.clone();
            q.set_theta(q.theta_at(idx));
            assert!(q.holdout_passes(), "index {idx} failed holdout");
        }
    }

    /// 教学设计断言：初始值必须是不合法的。
    /// 如果哪天有人把它改成 3.9，校验会在这里炸。
    #[test]
    fn initial_theta_must_be_unsolved() {
        let p = ThresholdPuzzle::new(cfg());
        assert!(!p.is_solved());
        assert!(!p.valid_indices().contains(&p.index()));
    }

    #[test]
    fn reset_returns_to_initial() {
        let mut p = puzzle_at(3.9);
        assert!(p.is_solved());
        p.reset();
        assert!(close(p.theta(), 6.0));
        assert!(!p.is_solved());
    }
}

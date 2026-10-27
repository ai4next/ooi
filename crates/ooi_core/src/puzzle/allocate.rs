//! 数据划分：把样本切成训练 / 验证 / 测试三份，并满足比例约束。

#[derive(Clone, Debug)]
pub struct AllocateConfig {
    pub total: u32,
    /// 三段各自允许的数量区间（含）。
    pub ranges: [(u32, u32); 3],
}

#[derive(Clone, Debug)]
pub struct AllocatePuzzle {
    cfg: AllocateConfig,
    /// 第一刀、第二刀的位置（0..=total）。
    a: u32,
    b: u32,
}

impl AllocatePuzzle {
    pub fn new(cfg: AllocateConfig) -> Self {
        let a = cfg.total / 2;
        let b = cfg.total * 3 / 4;
        Self { cfg, a, b }
    }

    pub fn config(&self) -> &AllocateConfig {
        &self.cfg
    }

    pub fn cuts(&self) -> (u32, u32) {
        (self.a, self.b)
    }

    pub fn parts(&self) -> [u32; 3] {
        [self.a, self.b - self.a, self.cfg.total - self.b]
    }

    pub fn set_a(&mut self, value: u32) {
        self.a = value.clamp(1, self.b.saturating_sub(1).max(1));
    }

    pub fn set_b(&mut self, value: u32) {
        self.b = value.clamp(self.a + 1, self.cfg.total.saturating_sub(1));
    }

    pub fn is_solved(&self) -> bool {
        let parts = self.parts();
        parts
            .iter()
            .zip(self.cfg.ranges.iter())
            .all(|(v, (lo, hi))| v >= lo && v <= hi)
    }

    pub fn reset(&mut self) {
        self.a = self.cfg.total / 2;
        self.b = self.cfg.total * 3 / 4;
    }
}

pub fn split_60_20_20() -> AllocateConfig {
    AllocateConfig {
        total: 100,
        ranges: [(55, 70), (15, 25), (15, 25)],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sixty_twenty_twenty_solves() {
        let mut p = AllocatePuzzle::new(split_60_20_20());
        assert!(!p.is_solved());
        p.set_a(60);
        p.set_b(80);
        assert_eq!(p.parts(), [60, 20, 20]);
        assert!(p.is_solved());
    }

    #[test]
    fn cuts_never_cross() {
        let mut p = AllocatePuzzle::new(split_60_20_20());
        p.set_a(90);
        assert!(p.a < p.b);
        p.set_b(0);
        assert!(p.b > p.a);
    }

    #[test]
    fn skewed_split_fails() {
        let mut p = AllocatePuzzle::new(split_60_20_20());
        p.set_a(90);
        p.set_b(95);
        assert!(!p.is_solved());
    }

    #[test]
    fn parts_always_sum_to_total() {
        let mut p = AllocatePuzzle::new(split_60_20_20());
        for a in [10, 40, 70] {
            p.set_a(a);
            p.set_b(90);
            let sum: u32 = p.parts().iter().sum();
            assert_eq!(sum, 100);
        }
    }
}

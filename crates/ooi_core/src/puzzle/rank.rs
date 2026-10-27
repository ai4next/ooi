//! 偏好排序：把三个回答从最好排到最差。

#[derive(Clone, Debug)]
pub struct RankConfig {
    pub items: usize,
    /// 正确顺序（元素是 item 下标），从好到坏。
    pub correct: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct RankPuzzle {
    cfg: RankConfig,
    order: Vec<usize>,
}

impl RankPuzzle {
    pub fn new(cfg: RankConfig) -> Self {
        Self {
            cfg,
            order: Vec::new(),
        }
    }

    pub fn config(&self) -> &RankConfig {
        &self.cfg
    }

    pub fn order(&self) -> &[usize] {
        &self.order
    }

    pub fn pick(&mut self, item: usize) -> bool {
        if item >= self.cfg.items || self.order.contains(&item) {
            return false;
        }
        self.order.push(item);
        self.order.len() == self.cfg.items
    }

    pub fn is_complete(&self) -> bool {
        self.order.len() == self.cfg.items
    }

    pub fn is_solved(&self) -> bool {
        self.is_complete() && self.order == self.cfg.correct
    }

    pub fn reset(&mut self) {
        self.order.clear();
    }
}

pub fn preference() -> RankConfig {
    RankConfig {
        items: 3,
        correct: vec![1, 0, 2],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correct_order_solves() {
        let mut p = RankPuzzle::new(preference());
        p.pick(1);
        p.pick(0);
        assert!(!p.is_solved());
        p.pick(2);
        assert!(p.is_solved());
    }

    #[test]
    fn wrong_order_completes_but_does_not_solve() {
        let mut p = RankPuzzle::new(preference());
        for i in [0, 1, 2] {
            p.pick(i);
        }
        assert!(p.is_complete());
        assert!(!p.is_solved());
    }

    #[test]
    fn duplicate_pick_is_ignored() {
        let mut p = RankPuzzle::new(preference());
        assert!(!p.pick(1));
        p.pick(1);
        assert_eq!(p.order(), &[1]);
    }

    #[test]
    fn out_of_range_is_ignored() {
        let mut p = RankPuzzle::new(preference());
        assert!(!p.pick(9));
        assert!(p.order().is_empty());
    }

    #[test]
    fn reset_clears_order() {
        let mut p = RankPuzzle::new(preference());
        p.pick(1);
        p.reset();
        assert!(p.order().is_empty());
    }
}

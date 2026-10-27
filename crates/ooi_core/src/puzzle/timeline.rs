//! 早停：看着两条损失曲线跑，在验证损失抬头之前按下停止。

#[derive(Clone, Debug)]
pub struct TimelineConfig {
    pub epochs: u32,
    /// 允许停下的 epoch 区间（含）。
    pub stop_window: (u32, u32),
    pub epoch_secs: f32,
}

#[derive(Clone, Debug)]
pub struct TimelinePuzzle {
    cfg: TimelineConfig,
    elapsed: f32,
    stopped: Option<u32>,
}

impl TimelinePuzzle {
    pub fn new(cfg: TimelineConfig) -> Self {
        Self {
            cfg,
            elapsed: 0.0,
            stopped: None,
        }
    }

    pub fn config(&self) -> &TimelineConfig {
        &self.cfg
    }

    pub fn epoch(&self) -> u32 {
        ((self.elapsed / self.cfg.epoch_secs).floor() as u32).min(self.cfg.epochs)
    }

    pub fn elapsed(&self) -> f32 {
        self.elapsed
    }

    pub fn stopped_at(&self) -> Option<u32> {
        self.stopped
    }

    /// 每帧推进；跑完所有 epoch 会自动停在末尾（算失败）。
    pub fn tick(&mut self, dt: f32) {
        if self.stopped.is_some() {
            return;
        }
        self.elapsed += dt.max(0.0);
        if self.epoch() >= self.cfg.epochs {
            self.stopped = Some(self.cfg.epochs);
        }
    }

    pub fn stop(&mut self) {
        if self.stopped.is_none() {
            self.stopped = Some(self.epoch());
        }
    }

    pub fn is_solved(&self) -> bool {
        self.stopped
            .is_some_and(|e| e >= self.cfg.stop_window.0 && e <= self.cfg.stop_window.1)
    }

    pub fn reset(&mut self) {
        self.elapsed = 0.0;
        self.stopped = None;
    }

    pub fn train_loss(epoch: u32) -> f32 {
        let e = epoch as f32;
        0.05 + 0.9 * (-e / 7.0).exp()
    }

    pub fn val_loss(epoch: u32) -> f32 {
        let e = epoch as f32;
        let fit = 0.55 * (-e / 6.0).exp();
        let overfit = 0.0035 * (e - 13.0).max(0.0).powi(2);
        fit + overfit
    }
}

pub fn early_stop() -> TimelineConfig {
    TimelineConfig {
        epochs: 30,
        stop_window: (12, 16),
        epoch_secs: 0.28,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stopping_in_window_solves() {
        let mut p = TimelinePuzzle::new(early_stop());
        for _ in 0..13 {
            p.tick(0.28);
        }
        p.stop();
        assert!(p.is_solved(), "stopped at {:?}", p.stopped_at());
    }

    #[test]
    fn stopping_late_fails() {
        let mut p = TimelinePuzzle::new(early_stop());
        for _ in 0..25 {
            p.tick(0.28);
        }
        p.stop();
        assert!(!p.is_solved());
    }

    #[test]
    fn running_to_the_end_fails() {
        let mut p = TimelinePuzzle::new(early_stop());
        p.tick(100.0);
        assert_eq!(p.stopped_at(), Some(30));
        assert!(!p.is_solved());
    }

    #[test]
    fn stop_is_idempotent() {
        let mut p = TimelinePuzzle::new(early_stop());
        p.tick(0.28 * 14.0);
        p.stop();
        let first = p.stopped_at();
        p.stop();
        assert_eq!(p.stopped_at(), first);
    }

    #[test]
    fn val_loss_has_a_minimum_near_thirteen() {
        let mut best = (0u32, f32::MAX);
        for e in 0..30 {
            let v = TimelinePuzzle::val_loss(e);
            if v < best.1 {
                best = (e, v);
            }
        }
        assert!((12..=14).contains(&best.0), "min at {}", best.0);
    }

    #[test]
    fn reset_restarts() {
        let mut p = TimelinePuzzle::new(early_stop());
        p.tick(10.0);
        p.stop();
        p.reset();
        assert_eq!(p.stopped_at(), None);
        assert_eq!(p.epoch(), 0);
    }
}

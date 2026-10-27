//! 卷积核编辑器：玩家在 3×3 网格里调权重，实时看卷积结果，直到命中目标边缘图。

pub const KERNEL: usize = 3;

#[derive(Clone, Debug)]
pub struct KernelConfig {
    pub width: usize,
    pub height: usize,
    /// 0..=1 的灰度。
    pub image: Vec<f32>,
    /// 目标输出（与 image 同尺寸）。
    pub target: Vec<bool>,
}

#[derive(Clone, Debug)]
pub struct KernelPuzzle {
    cfg: KernelConfig,
    /// 3×3 权重，取值 -1 / 0 / 1，行优先。
    weights: [i32; 9],
}

impl KernelPuzzle {
    pub fn new(cfg: KernelConfig) -> Self {
        Self {
            cfg,
            weights: [0; 9],
        }
    }

    pub fn config(&self) -> &KernelConfig {
        &self.cfg
    }

    pub fn weights(&self) -> [i32; 9] {
        self.weights
    }

    pub fn set_weights(&mut self, weights: [i32; 9]) {
        self.weights = weights;
    }

    /// 点击循环：0 → +1 → −1 → 0。
    pub fn cycle(&mut self, x: usize, y: usize) {
        if x < KERNEL && y < KERNEL {
            let i = y * KERNEL + x;
            self.weights[i] = match self.weights[i] {
                0 => 1,
                1 => -1,
                _ => 0,
            };
        }
    }

    pub fn predict(&self) -> Vec<bool> {
        let w = self.cfg.width;
        let h = self.cfg.height;
        let mut out = vec![false; w * h];

        for y in 0..h {
            for x in 0..w {
                let mut sum = 0.0;
                for ky in 0..KERNEL {
                    for kx in 0..KERNEL {
                        let sx = x as isize + kx as isize - 1;
                        let sy = y as isize + ky as isize - 1;
                        if sx < 0 || sy < 0 || sx >= w as isize || sy >= h as isize {
                            continue;
                        }
                        let weight = self.weights[ky * KERNEL + kx] as f32;
                        sum += self.cfg.image[sy as usize * w + sx as usize] * weight;
                    }
                }
                out[y * w + x] = sum > 0.5;
            }
        }
        out
    }

    pub fn is_solved(&self) -> bool {
        self.predict() == self.cfg.target
    }

    pub fn matched(&self) -> usize {
        self.predict()
            .iter()
            .zip(&self.cfg.target)
            .filter(|(a, b)| a == b)
            .count()
    }

    pub fn reset(&mut self) {
        self.weights = [0; 9];
    }
}

/// 8×8 图：中间一条竖条。
pub fn vertical_bar_image() -> Vec<f32> {
    let w = 8;
    let h = 8;
    let mut image = vec![0.0; w * h];
    for y in 0..h {
        for x in [1, 5] {
            image[y * w + x] = 1.0;
        }
    }
    image
}

/// 用参考核 [ +1 0 −1 ; ... ] 生成目标。
pub fn vertical_edges() -> KernelConfig {
    let w = 8;
    let h = 8;
    let image = vertical_bar_image();

    let mut reference = KernelPuzzle::new(KernelConfig {
        width: w,
        height: h,
        image: image.clone(),
        target: vec![false; w * h],
    });
    reference.weights = [1, 0, -1, 1, 0, -1, 1, 0, -1];
    let target = reference.predict();

    KernelConfig {
        width: w,
        height: h,
        image,
        target,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solved() -> KernelPuzzle {
        let cfg = vertical_edges();
        let target = cfg.target.clone();
        let mut p = KernelPuzzle::new(cfg);
        p.weights = [1, 0, -1, 1, 0, -1, 1, 0, -1];
        assert_eq!(p.predict(), target);
        p
    }

    #[test]
    fn reference_kernel_solves() {
        let p = solved();
        assert!(p.is_solved());
    }

    #[test]
    fn empty_kernel_does_not() {
        let p = KernelPuzzle::new(vertical_edges());
        assert!(!p.is_solved());
        assert_eq!(p.matched(), 64 - 16);
    }

    #[test]
    fn cycle_order() {
        let mut p = KernelPuzzle::new(vertical_edges());
        p.cycle(0, 0);
        assert_eq!(p.weights()[0], 1);
        p.cycle(0, 0);
        assert_eq!(p.weights()[0], -1);
        p.cycle(0, 0);
        assert_eq!(p.weights()[0], 0);
    }

    #[test]
    fn target_is_sixteen_cells() {
        let cfg = vertical_edges();
        assert_eq!(cfg.target.iter().filter(|b| **b).count(), 16);
    }
}

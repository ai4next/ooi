//! 第 1–2 章：感知机更新（1-3）、反向传播填梯度（2-3 Blame）。

use oo_base::{Lang, Tr, t};

use super::{BenchState, Dial, Eval, Layout, Mark, Spec, Target, no_click, no_init, no_targets};

// ---------------------------------------------------------------- 1-3 感知机

/// (x, y, label) in the unit square: wind × swell, +1 = safe to sail.
const POINTS: [(f32, f32, f32); 12] = [
    (0.70, 0.80, 1.0),
    (0.85, 0.60, 1.0),
    (0.60, 0.90, 1.0),
    (0.80, 0.85, 1.0),
    (0.90, 0.75, 1.0),
    (0.65, 0.65, 1.0),
    (0.20, 0.30, -1.0),
    (0.35, 0.15, -1.0),
    (0.15, 0.45, -1.0),
    (0.30, 0.35, -1.0),
    (0.45, 0.20, -1.0),
    (0.25, 0.10, -1.0),
];
const START_W: [f32; 3] = [-0.5, 0.2, 0.1];
pub const MAX_UPDATES: usize = 30;

fn margin(w: &[f32], i: usize) -> f32 {
    let (x, y, label) = POINTS[i];
    label * (w[0] * x + w[1] * y + w[2])
}

fn wrong_points(s: &BenchState) -> Vec<usize> {
    (0..POINTS.len()).filter(|&i| margin(&s.vals, i) <= 0.0).collect()
}

fn perceptron_init(s: &mut BenchState) {
    s.vals = START_W.to_vec();
}

fn perceptron_targets(s: &BenchState, _: Lang) -> Vec<Target> {
    (0..POINTS.len())
        .map(|i| {
            let (x, y, label) = POINTS[i];
            let mark = if margin(&s.vals, i) <= 0.0 {
                Mark::Hot
            } else if label > 0.0 {
                Mark::Pos
            } else {
                Mark::Neg
            };
            Target::at(x, y, mark)
        })
        .collect()
}

/// Perceptron rule: only a misclassified point moves the line, `w += y·x`.
fn perceptron_click(s: &mut BenchState, i: usize) {
    if i >= POINTS.len() || margin(&s.vals, i) > 0.0 {
        s.wrong += 1;
        return;
    }
    let (x, y, label) = POINTS[i];
    for (w, v) in s.vals.iter_mut().zip([x, y, 1.0]) {
        *w += label * v;
    }
    s.picks.push(i);
}

fn perceptron_eval(s: &BenchState, lang: Lang) -> Eval {
    let mut e = Eval::default();
    let wrong = wrong_points(s).len();
    let n = s.picks.len();
    e.check(t(lang, "Updates", "更新次数"), format!("{n} / {MAX_UPDATES}"), n <= MAX_UPDATES);
    e.check(t(lang, "Misclassified", "错分样本"), wrong.to_string(), wrong == 0);
    e.line = Some([s.vals[0], s.vals[1], s.vals[2]]);
    e.solved = wrong == 0 && n <= MAX_UPDATES;
    e
}

fn perceptron_solve(s: &mut BenchState) {
    perceptron_init(s);
    s.picks.clear();
    while let Some(&i) = wrong_points(s).first() {
        perceptron_click(s, i);
    }
}

pub static PERCEPTRON: Spec = Spec {
    layout: Layout::Plot,
    dials: &[],
    toggles: &[],
    fault: None,
    init: perceptron_init,
    targets: perceptron_targets,
    click: perceptron_click,
    eval: perceptron_eval,
    solve: perceptron_solve,
};

// ---------------------------------------------------------------- 2-3 Blame

/// A 2-2-1 tanh net on one sample; the player fills four edge gradients.
struct Net {
    h: [f32; 2],
    o: f32,
    delta: f32,
    /// dL/dv1, dL/dv2, dL/dw11, dL/dw21.
    grads: [f32; 4],
}

const X: [f32; 2] = [1.0, 0.5];
const W: [[f32; 2]; 2] = [[0.8, -0.4], [0.3, 0.9]];
const V: [f32; 2] = [1.2, -0.7];
const Y: f32 = 1.0;
const GRAD_STEP: f32 = 0.05;

fn forward() -> Net {
    let h = [0, 1].map(|j| (W[j][0] * X[0] + W[j][1] * X[1]).tanh());
    let o = V[0] * h[0] + V[1] * h[1];
    let delta = o - Y;
    let back = |j: usize| delta * V[j] * (1.0 - h[j] * h[j]) * X[0];
    Net { h, o, delta, grads: [delta * h[0], delta * h[1], back(0), back(1)] }
}

fn close(guess: f32, truth: f32) -> bool {
    (guess - truth).abs() <= (truth.abs() * 0.1).max(0.03)
}

const BLAME_DIALS: [Dial; 4] = [
    Dial::num(Tr::new("dL/dv1 (h1 → out)", "dL/dv1（h1 → 输出）"), -20, 20, 0, GRAD_STEP, 2),
    Dial::num(Tr::new("dL/dv2 (h2 → out)", "dL/dv2（h2 → 输出）"), -20, 20, 0, GRAD_STEP, 2),
    Dial::num(Tr::new("dL/dw11 (x1 → h1)", "dL/dw11（x1 → h1）"), -20, 20, 0, GRAD_STEP, 2),
    Dial::num(Tr::new("dL/dw21 (x1 → h2)", "dL/dw21（x1 → h2）"), -20, 20, 0, GRAD_STEP, 2),
];

fn blame_eval(s: &BenchState, lang: Lang) -> Eval {
    let net = forward();
    let mut e = Eval::default();
    e.info(t(lang, "Input x1, x2", "输入 x1, x2"), format!("{:.2}, {:.2}", X[0], X[1]));
    e.info(t(lang, "Hidden h1, h2", "隐藏层 h1, h2"), format!("{:.3}, {:.3}", net.h[0], net.h[1]));
    e.info(t(lang, "Weights v1, v2", "权重 v1, v2"), format!("{:.2}, {:.2}", V[0], V[1]));
    e.info(
        t(lang, "tanh slope 1-h²", "tanh 斜率 1-h²"),
        format!("{:.3}, {:.3}", 1.0 - net.h[0] * net.h[0], 1.0 - net.h[1] * net.h[1]),
    );
    e.info(t(lang, "Output · error δ = o - y", "输出 · 误差 δ = o - y"), format!("{:.3} · {:.3}", net.o, net.delta));
    let mut all = true;
    for (i, d) in BLAME_DIALS.iter().enumerate() {
        let guess = s.dials[i] as f32 * GRAD_STEP;
        let ok = close(guess, net.grads[i]);
        all &= ok;
        let verdict = if ok { t(lang, "within 10%", "误差 < 10%") } else { t(lang, "more than 10% off", "偏离 > 10%") };
        e.check(d.label.get(lang), verdict, ok);
    }
    e.solved = all;
    e
}

fn blame_solve(s: &mut BenchState) {
    let net = forward();
    for (d, g) in s.dials.iter_mut().zip(net.grads) {
        *d = (g / GRAD_STEP).round() as i32;
    }
}

pub static BLAME: Spec = Spec {
    layout: Layout::None,
    dials: &BLAME_DIALS,
    toggles: &[],
    fault: None,
    init: no_init,
    targets: no_targets,
    click: no_click,
    eval: blame_eval,
    solve: blame_solve,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perceptron_converges_within_budget() {
        let mut s = BenchState::default();
        perceptron_solve(&mut s);
        assert!(s.picks.len() <= MAX_UPDATES, "{} updates", s.picks.len());
        assert!(s.picks.len() >= 2, "too easy: {} updates", s.picks.len());
    }

    #[test]
    fn clicking_a_correct_point_is_wasted() {
        let mut s = BenchState::default();
        perceptron_init(&mut s);
        let right = (0..POINTS.len()).find(|&i| margin(&s.vals, i) > 0.0).unwrap();
        perceptron_click(&mut s, right);
        assert_eq!(s.wrong, 1);
        assert!(s.picks.is_empty());
    }

    #[test]
    fn blame_needs_the_chain_rule() {
        let net = forward();
        // Equal blame per layer (the wrong intuition) fails.
        assert!(!close(net.grads[0], net.grads[2]));
        assert!(net.grads.iter().all(|g| g.abs() < 1.0));
    }
}

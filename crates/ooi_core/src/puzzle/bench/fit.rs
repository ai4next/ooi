//! 第 2–3 章的真模型：激活（2-4）、学习率（2-5 故障）、留出验证（3-1）、
//! 容量（3-2）、L2（3-3）。每关都在真数据上训练，结果按旋钮位置缓存。

use oo_base::{Lang, Rng, Tr, t};

use super::{BenchState, Cache, Chart, Curve, Dial, Eval, Image, Layout, Mark, Spec, Target, memo, no_click, no_init};
use crate::lab::{Dataset, DatasetKind};

fn pct(v: f32) -> String {
    format!("{:.0}%", v * 100.0)
}

/// Decision field on a `n × n` grid over the unit square; `f` returns P(lit).
pub(super) fn field(n: usize, f: impl Fn(f32, f32) -> f32) -> Image {
    let mut px = Vec::with_capacity(n * n);
    for r in 0..n {
        for c in 0..n {
            let (x, y) = ((c as f32 + 0.5) / n as f32, 1.0 - (r as f32 + 0.5) / n as f32);
            px.push((f(x, y) - 0.5) * 2.0);
        }
    }
    Image { label: String::new(), w: n, px }
}

// ---------------------------------------------------------------- 2-4 激活

/// A small fully connected net with a chosen hidden activation and a sigmoid output.
struct Net {
    act: usize,
    /// (weights out×in, bias) per layer.
    layers: Vec<(Vec<f32>, Vec<f32>)>,
    sizes: Vec<usize>,
}

const LINEAR: usize = 0;
const WIDTH: usize = 6;

impl Net {
    fn new(act: usize, hidden: usize, seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let mut sizes = vec![2];
        sizes.extend(std::iter::repeat_n(WIDTH, hidden));
        sizes.push(1);
        let layers = sizes
            .windows(2)
            .map(|w| {
                let s = (1.0 / w[0] as f64).sqrt() * 1.4;
                ((0..w[0] * w[1]).map(|_| (rng.normal() * s) as f32).collect(), vec![0.0; w[1]])
            })
            .collect();
        Self { act, layers, sizes }
    }

    fn params(&self) -> usize {
        self.layers.iter().map(|(w, b)| w.len() + b.len()).sum()
    }

    fn f(&self, z: f32) -> f32 {
        match self.act {
            LINEAR => z,
            1 => z.tanh(),
            _ => z.max(0.0),
        }
    }

    fn df(&self, z: f32) -> f32 {
        match self.act {
            LINEAR => 1.0,
            1 => 1.0 - z.tanh().powi(2),
            _ => if z > 0.0 { 1.0 } else { 0.0 },
        }
    }

    /// Pre-activations and activations of every layer.
    fn forward(&self, x: f32, y: f32) -> (Vec<Vec<f32>>, Vec<Vec<f32>>) {
        let mut acts = vec![vec![(x - 0.5) * 2.0, (y - 0.5) * 2.0]];
        let mut pre = Vec::new();
        let last = self.layers.len() - 1;
        for (l, (w, b)) in self.layers.iter().enumerate() {
            let (n_in, n_out) = (self.sizes[l], self.sizes[l + 1]);
            let input = acts.last().unwrap();
            let z: Vec<f32> = (0..n_out).map(|o| b[o] + (0..n_in).map(|i| w[o * n_in + i] * input[i]).sum::<f32>()).collect();
            let a = if l == last { z.iter().map(|&v| 1.0 / (1.0 + (-v).exp())).collect() } else { z.iter().map(|&v| self.f(v)).collect() };
            pre.push(z);
            acts.push(a);
        }
        (pre, acts)
    }

    fn predict(&self, x: f32, y: f32) -> f32 {
        self.forward(x, y).1.last().unwrap()[0]
    }

    /// Full-batch Adam on cross-entropy.
    fn train(&mut self, data: &[(f32, f32, bool)], epochs: usize, lr: f32) {
        let mut m: Vec<Vec<f32>> = self.layers.iter().flat_map(|(w, b)| [vec![0.0; w.len()], vec![0.0; b.len()]]).collect();
        let mut v = m.clone();
        for epoch in 1..=epochs {
            let mut g: Vec<Vec<f32>> = m.iter().map(|x| vec![0.0; x.len()]).collect();
            for &(x, y, label) in data {
                let (pre, acts) = self.forward(x, y);
                let mut delta = vec![acts.last().unwrap()[0] - if label { 1.0 } else { 0.0 }];
                for l in (0..self.layers.len()).rev() {
                    let n_in = self.sizes[l];
                    for (o, &d) in delta.iter().enumerate() {
                        for i in 0..n_in {
                            g[2 * l][o * n_in + i] += d * acts[l][i];
                        }
                        g[2 * l + 1][o] += d;
                    }
                    if l > 0 {
                        let w = &self.layers[l].0;
                        delta = (0..n_in)
                            .map(|i| delta.iter().enumerate().map(|(o, &d)| d * w[o * n_in + i]).sum::<f32>() * self.df(pre[l - 1][i]))
                            .collect();
                    }
                }
            }
            let (b1, b2) = (0.9f32, 0.999f32);
            let (c1, c2) = (1.0 - b1.powi(epoch as i32), 1.0 - b2.powi(epoch as i32));
            for (k, grad) in g.iter().enumerate() {
                let (l, is_b) = (k / 2, k % 2 == 1);
                let target = if is_b { &mut self.layers[l].1 } else { &mut self.layers[l].0 };
                for (j, gj) in grad.iter().enumerate() {
                    let gj = gj / data.len() as f32;
                    m[k][j] = b1 * m[k][j] + (1.0 - b1) * gj;
                    v[k][j] = b2 * v[k][j] + (1.0 - b2) * gj * gj;
                    target[j] -= lr * (m[k][j] / c1) / ((v[k][j] / c2).sqrt() + 1e-8);
                }
            }
        }
    }
}

fn ring() -> (Vec<(f32, f32, bool)>, Vec<(f32, f32, bool)>) {
    let d = Dataset::generate(DatasetKind::Ring);
    let all: Vec<_> = d.samples.iter().map(|s| (s.x, s.y, s.label)).collect();
    let train = all.iter().copied().step_by(2).collect();
    let test = all.iter().copied().skip(1).step_by(2).collect();
    (train, test)
}

#[derive(Clone)]
struct Trained {
    train: f32,
    test: f32,
    params: usize,
    /// Test-set predictions.
    preds: Vec<bool>,
    field: Image,
}

static ACT_CACHE: Cache<(i32, i32), Trained> = Cache::new(Default::default);

fn accuracy(net: &Net, data: &[(f32, f32, bool)]) -> f32 {
    data.iter().filter(|&&(x, y, l)| (net.predict(x, y) > 0.5) == l).count() as f32 / data.len() as f32
}

fn trained(act: i32, hidden: i32) -> Trained {
    memo(&ACT_CACHE, (act, hidden), || {
        let (train, test) = ring();
        let mut net = Net::new(act as usize, hidden as usize, 0x5ea_0000 + hidden as u64);
        net.train(&train, 300, 0.04);
        Trained {
            train: accuracy(&net, &train),
            test: accuracy(&net, &test),
            params: net.params(),
            preds: test.iter().map(|&(x, y, _)| net.predict(x, y) > 0.5).collect(),
            field: field(22, |x, y| net.predict(x, y)),
        }
    })
}

static ACTIVATIONS: [Tr; 3] = [Tr::new("none (linear)", "无（线性）"), Tr::new("tanh", "tanh"), Tr::new("ReLU", "ReLU")];
static ACT_DIALS: [Dial; 2] = [
    Dial::named(Tr::new("Activation between layers", "层间激活函数"), 0, &ACTIVATIONS),
    Dial::num(Tr::new("Hidden layers", "隐藏层数"), 1, 4, 3, 1.0, 0),
];
pub const RING_TARGET: f32 = 0.95;

fn activation_targets(s: &BenchState, _: Lang) -> Vec<Target> {
    let r = trained(s.dials[0], s.dials[1]);
    let (_, test) = ring();
    test.iter()
        .zip(&r.preds)
        .map(|(&(x, y, label), &p)| Target::at(x, y, if p != label { Mark::Hot } else if label { Mark::Pos } else { Mark::Neg }))
        .collect()
}

fn activation_eval(s: &BenchState, lang: Lang) -> Eval {
    let r = trained(s.dials[0], s.dials[1]);
    let mut e = Eval::default();
    let linear = s.dials[0] == LINEAR as i32;
    let shape = if linear {
        Tr::new("{} linear layers = 1 linear layer: a straight line", "{} 个线性层 = 1 个线性层：一条直线")
            .fill(lang, &[&s.dials[1]])
    } else {
        t(lang, "bent: it can wrap around the ring", "弯曲：能把内圈包起来").to_string()
    };
    e.info(t(lang, "Boundary", "边界"), shape);
    e.info(t(lang, "Training accuracy", "训练准确率"), pct(r.train));
    e.check(t(lang, "Test accuracy", "测试准确率"), format!("{} ≥ {}", pct(r.test), pct(RING_TARGET)), r.test >= RING_TARGET);
    e.info(t(lang, "Parameters", "参数量"), r.params.to_string());
    e.field = Some(r.field);
    e.solved = r.test >= RING_TARGET;
    e
}

fn activation_solve(s: &mut BenchState) {
    s.dials = vec![1, 1];
}

pub static ACTIVATION: Spec = Spec {
    layout: Layout::Plot,
    dials: &ACT_DIALS,
    toggles: &[],
    fault: None,
    init: no_init,
    targets: activation_targets,
    click: no_click,
    eval: activation_eval,
    solve: activation_solve,
};

// ---------------------------------------------------------------- 2-5 学习率

/// Su's fog log: (humidity, dew-point gap) → hours of fog, fitted by plain gradient descent.
fn fog() -> Vec<([f32; 3], f32)> {
    let mut rng = Rng::new(0xf06);
    (0..40)
        .map(|_| {
            let h = rng.f32();
            let g = 0.8 * h + 0.2 * rng.f32();
            ([h, g, 1.0], 2.0 * h - 1.2 * g + 0.3 + rng.normal() as f32 * 0.05)
        })
        .collect()
}

pub const STEPS: usize = 24;
/// The step size ramps up linearly over the first steps (standard warm-up),
/// so a too-large rate only blows up once the ramp crosses the edge.
const WARMUP: usize = 8;
pub const LOSS_TARGET: f32 = 0.009;
const START_LR: i32 = 40;
const LR_SCALE: f32 = 0.05;

/// Loss after each of `STEPS` steps (index 0 = before training).
fn losses(lr: f32) -> Vec<f32> {
    let data = fog();
    let n = data.len() as f32;
    let loss = |w: &[f32; 3]| data.iter().map(|(x, y)| (x[0] * w[0] + x[1] * w[1] + x[2] * w[2] - y).powi(2)).sum::<f32>() / n / 2.0;
    let mut w = [0.0f32; 3];
    let mut out = vec![loss(&w)];
    for step in 0..STEPS {
        let lr = lr * ((step + 1) as f32 / WARMUP as f32).min(1.0);
        let mut g = [0.0f32; 3];
        for (x, y) in &data {
            let err = x[0] * w[0] + x[1] * w[1] + x[2] * w[2] - y;
            for k in 0..3 {
                g[k] += err * x[k] / n;
            }
        }
        for k in 0..3 {
            w[k] -= lr * g[k];
        }
        out.push(loss(&w).min(1e6));
    }
    out
}

fn first_rise(l: &[f32]) -> Option<usize> {
    (1..l.len()).find(|&i| l[i] > l[i - 1])
}

/// Log-scaled loss in 0..1; exploded values pin to the top.
fn loss_y(l: f32) -> f32 {
    let (lo, hi) = (0.004f32.ln(), 2.0f32.ln());
    ((l.max(1e-6).ln() - lo) / (hi - lo)).clamp(0.0, 1.0)
}

fn lr(s: &BenchState) -> f32 {
    s.dials[0] as f32 * LR_SCALE
}

fn step_x(i: usize) -> f32 {
    0.03 + 0.94 * i as f32 / STEPS as f32
}

fn step_targets(s: &BenchState, _: Lang) -> Vec<Target> {
    let l = losses(lr(s));
    let rise = first_rise(&l);
    (0..=STEPS)
        .map(|i| {
            let mark = if !s.evidence {
                Mark::Neutral
            } else if rise == Some(i) {
                Mark::Hot
            } else if i > 0 && l[i] > l[i - 1] {
                Mark::Neg
            } else {
                Mark::Pos
            };
            Target::at(step_x(i), loss_y(l[i]), mark)
        })
        .collect()
}

fn step_click(s: &mut BenchState, i: usize) {
    if s.evidence || i > STEPS {
        return;
    }
    if first_rise(&losses(lr(s))) == Some(i) {
        s.evidence = true;
        s.picks.push(i);
    } else {
        s.wrong += 1;
    }
}

fn step_eval(s: &BenchState, lang: Lang) -> Eval {
    let l = losses(lr(s));
    let last = *l.last().unwrap();
    let mut e = Eval::default();
    e.info(t(lang, "Loss before training", "训练前损失"), format!("{:.3}", l[0]));
    let rise = match first_rise(&l) {
        Some(i) if s.evidence => Tr::new("step {}", "第 {} 步").fill(lang, &[&i]),
        Some(_) => "?".into(),
        None => t(lang, "never rises", "一直在降").into(),
    };
    e.info(t(lang, "Loss first rises at", "损失第一次上升"), rise);
    let shown = if last >= 1e6 { t(lang, "exploded", "爆炸").to_string() } else { format!("{last:.4}") };
    e.check(t(lang, "Loss after 24 steps", "24 步后损失"), format!("{shown} ≤ {LOSS_TARGET}"), last <= LOSS_TARGET);
    e.check(t(lang, "Evidence", "证据"), if s.evidence { t(lang, "found", "已指证") } else { t(lang, "not yet", "还没有") }, s.evidence);
    let pts: Vec<_> = l.iter().enumerate().map(|(i, &v)| (step_x(i), loss_y(v))).collect();
    e.curves.push(Curve::new(t(lang, "training loss", "训练损失"), pts, Mark::Pos));
    let goal = loss_y(LOSS_TARGET);
    e.curves.push(Curve::new(t(lang, "target", "目标"), vec![(0.0, goal), (1.0, goal)], Mark::Done).dashed());
    e.solved = s.evidence && last <= LOSS_TARGET;
    e
}

fn step_solve(s: &mut BenchState) {
    let i = first_rise(&losses(lr(s))).unwrap_or(0);
    step_click(s, i);
    s.dials[0] = 12;
}

static STEP_DIALS: [Dial; 1] = [Dial::num(Tr::new("Learning rate", "学习率"), 1, 60, START_LR, LR_SCALE, 2)];

pub static STEP_SIZE: Spec = Spec {
    layout: Layout::Plot,
    dials: &STEP_DIALS,
    toggles: &[],
    fault: Some(Tr::new(
        "Too big a step: each update jumps across the valley and lands higher than it started.",
        "步子太大：每次更新都跨过谷底，落点比出发点还高。",
    )),
    init: no_init,
    targets: step_targets,
    click: step_click,
    eval: step_eval,
    solve: step_solve,
};

// ---------------------------------------------------------------- 第 3 章：多项式拟合

/// Lina's ward log: dose → recovery, a smooth curve plus noise. x in -1..1.
fn truth(x: f64) -> f64 {
    0.6 * (2.6 * x).sin() + 0.25 * x * x
}

pub const POLY_SEED: u64 = 41;
const POLY_N: usize = 24;
const POLY_NOISE: f64 = 0.085;
/// A prediction counts as right within this distance.
const TOL: f64 = 0.2;
const MAX_DEG: usize = 15;

fn poly_data(seed: u64, n: usize, noise: f64) -> Vec<(f64, f64)> {
    let mut rng = Rng::new(seed);
    (0..n)
        .map(|_| {
            let x = rng.range(-1.0, 1.0);
            (x, truth(x) + rng.normal() * noise)
        })
        .collect()
}

fn ward() -> Vec<(f64, f64)> {
    poly_data(POLY_SEED, POLY_N, POLY_NOISE)
}

/// New patients: the hidden judge.
fn new_patients() -> Vec<(f64, f64)> {
    poly_data(0x7e57, 300, POLY_NOISE)
}

fn legendre(x: f64, deg: usize) -> Vec<f64> {
    let mut p = vec![1.0, x];
    for n in 1..deg {
        let n = n as f64;
        let next = ((2.0 * n + 1.0) * x * p[n as usize] - n * p[n as usize - 1]) / (n + 1.0);
        p.push(next);
    }
    p.truncate(deg + 1);
    p
}

#[derive(Clone)]
struct Poly(Vec<f64>);

impl Poly {
    fn at(&self, x: f64) -> f64 {
        legendre(x, self.0.len() - 1).iter().zip(&self.0).map(|(a, b)| a * b).sum()
    }

    /// Ridge least squares: (VᵀV + λ·n·I) c = Vᵀy, Gaussian elimination.
    fn fit(data: &[(f64, f64)], deg: usize, lambda: f64) -> Self {
        let k = deg + 1;
        let mut a = vec![vec![0.0; k + 1]; k];
        for &(x, y) in data {
            let v = legendre(x, deg);
            for i in 0..k {
                for j in 0..k {
                    a[i][j] += v[i] * v[j];
                }
                a[i][k] += v[i] * y;
            }
        }
        for (i, row) in a.iter_mut().enumerate() {
            row[i] += lambda.max(1e-12) * data.len() as f64;
        }
        for col in 0..k {
            let piv = (col..k).max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs())).unwrap();
            a.swap(col, piv);
            for r in 0..k {
                if r != col {
                    let f = a[r][col] / a[col][col];
                    for c in col..=k {
                        a[r][c] -= f * a[col][c];
                    }
                }
            }
        }
        Self((0..k).map(|i| a[i][k] / a[i][i]).collect())
    }

    fn accuracy(&self, data: &[(f64, f64)]) -> f32 {
        data.iter().filter(|&&(x, y)| (self.at(x) - y).abs() < TOL).count() as f32 / data.len() as f32
    }

    fn mse(&self, data: &[(f64, f64)]) -> f64 {
        data.iter().map(|&(x, y)| (self.at(x) - y).powi(2)).sum::<f64>() / data.len() as f64
    }

    fn largest(&self) -> f64 {
        self.0.iter().fold(0.0, |m, c| m.max(c.abs()))
    }
}

fn px(x: f64) -> f32 {
    ((x + 1.0) / 2.0) as f32
}

fn py(y: f64) -> f32 {
    ((y + 0.9) / 2.0) as f32
}

/// The fitted curve, cut where it leaves the box so wild fits read as wild.
fn poly_curve(lang: Lang, p: &Poly) -> Curve {
    let pts = (0..=120).map(|i| {
        let x = -1.0 + 2.0 * i as f64 / 120.0;
        (px(x), py(p.at(x)).clamp(-0.05, 1.05))
    });
    Curve::new(t(lang, "model", "模型"), pts.collect(), Mark::Pos)
}

fn point_targets(points: &[(f64, f64)], mark: impl Fn(usize) -> Mark) -> Vec<Target> {
    points.iter().enumerate().map(|(i, &(x, y))| Target::at(px(x), py(y), mark(i))).collect()
}

fn deg_x(d: usize) -> f32 {
    (d - 1) as f32 / (MAX_DEG - 1) as f32
}

/// Log-scaled squared error in 0..1 for the error charts.
fn err_y(mse: f64) -> f32 {
    let (lo, hi) = (0.002f64.ln(), 1.0f64.ln());
    ((mse.max(1e-9).ln() - lo) / (hi - lo)).clamp(0.0, 1.0) as f32
}

fn split(held: usize) -> (Vec<(f64, f64)>, Vec<(f64, f64)>) {
    let mut all = ward();
    let val = all.split_off(POLY_N - held);
    (all, val)
}

fn error_chart(lang: Lang, x_label: &str, xs: &[f32], fits: &[Poly], train: &[(f64, f64)], val: &[(f64, f64)]) -> Chart {
    let mut c = Chart { title: t(lang, "Error (log scale)", "误差（对数刻度）").into(), x_label: x_label.into(), ..Chart::default() };
    let curve = |data: &[(f64, f64)]| xs.iter().zip(fits).map(|(&x, p)| (x, err_y(p.mse(data)))).collect::<Vec<_>>();
    c.curves.push(Curve::new(t(lang, "training", "训练"), curve(train), Mark::Pos));
    if !val.is_empty() {
        c.curves.push(Curve::new(t(lang, "validation", "验证"), curve(val), Mark::Neg).dashed());
    }
    c
}

// 3-1 留出验证

pub const HOLDOUT_TARGET: f32 = 0.85;

/// Picks the degree with the lowest error on whatever it is graded on.
fn pick_degree(held: usize) -> (usize, Vec<Poly>) {
    let (train, val) = split(held);
    let fits: Vec<_> = (1..=MAX_DEG).map(|d| Poly::fit(&train, d, 0.0)).collect();
    let graded = if val.is_empty() { &train } else { &val };
    let best = (0..MAX_DEG).min_by(|&a, &b| fits[a].mse(graded).total_cmp(&fits[b].mse(graded))).unwrap();
    (best + 1, fits)
}

fn holdout_targets(s: &BenchState, _: Lang) -> Vec<Target> {
    let held = s.dials[0] as usize;
    point_targets(&ward(), |i| if i >= POLY_N - held { Mark::Neg } else { Mark::Pos })
}

fn holdout_eval(s: &BenchState, lang: Lang) -> Eval {
    let held = s.dials[0] as usize;
    let (train, val) = split(held);
    let (deg, fits) = pick_degree(held);
    let model = &fits[deg - 1];
    let test = model.accuracy(&new_patients());
    let mut e = Eval::default();
    let judge = if held == 0 { t(lang, "lowest training error", "训练误差最低") } else { t(lang, "lowest validation error", "验证误差最低") };
    e.info(t(lang, "Model picked by", "按什么选模型"), judge);
    e.info(t(lang, "Picked degree", "选中的阶数"), deg.to_string());
    e.info(t(lang, "Training score", "训练成绩"), pct(model.accuracy(&train)));
    e.info(t(lang, "Validation score", "验证成绩"), if val.is_empty() { "—".into() } else { pct(model.accuracy(&val)) });
    e.check(t(lang, "New patients (test)", "新病人（测试）"), format!("{} ≥ {}", pct(test), pct(HOLDOUT_TARGET)), test >= HOLDOUT_TARGET);
    e.curves.push(poly_curve(lang, model));
    let xs: Vec<_> = (1..=MAX_DEG).map(deg_x).collect();
    let mut chart = error_chart(lang, t(lang, "polynomial degree", "多项式阶数"), &xs, &fits, &train, &val);
    chart.cursor = Some(deg_x(deg));
    e.chart = Some(chart);
    e.solved = test >= HOLDOUT_TARGET;
    e
}

fn holdout_solve(s: &mut BenchState) {
    s.dials[0] = 6;
}

static HOLDOUT_DIALS: [Dial; 1] = [Dial::num(Tr::new("Visits held out for validation", "留作验证的就诊"), 0, 10, 0, 1.0, 0)];

pub static HOLDOUT: Spec = Spec {
    layout: Layout::Plot,
    dials: &HOLDOUT_DIALS,
    toggles: &[],
    fault: None,
    init: no_init,
    targets: holdout_targets,
    click: no_click,
    eval: holdout_eval,
    solve: holdout_solve,
};

// 3-2 容量

pub const FIT_TARGET: f32 = 0.88;
const HELD: usize = 6;

fn fixed_split_targets(_: &BenchState, _: Lang) -> Vec<Target> {
    point_targets(&ward(), |i| if i >= POLY_N - HELD { Mark::Neg } else { Mark::Pos })
}

fn capacity_eval(s: &BenchState, lang: Lang) -> Eval {
    let deg = s.dials[0] as usize;
    let (train, val) = split(HELD);
    let fits: Vec<_> = (1..=MAX_DEG).map(|d| Poly::fit(&train, d, 0.0)).collect();
    let model = &fits[deg - 1];
    let test = model.accuracy(&new_patients());
    let mut e = Eval::default();
    e.info(t(lang, "Parameters", "参数量"), (deg + 1).to_string());
    e.info(t(lang, "Training score", "训练成绩"), pct(model.accuracy(&train)));
    e.info(t(lang, "Validation score", "验证成绩"), pct(model.accuracy(&val)));
    e.check(t(lang, "New patients (test)", "新病人（测试）"), format!("{} ≥ {}", pct(test), pct(FIT_TARGET)), test >= FIT_TARGET);
    e.curves.push(poly_curve(lang, model));
    let xs: Vec<_> = (1..=MAX_DEG).map(deg_x).collect();
    let mut chart = error_chart(lang, t(lang, "model size (degree)", "模型大小（阶数）"), &xs, &fits, &train, &val);
    chart.cursor = Some(deg_x(deg));
    e.chart = Some(chart);
    e.solved = test >= FIT_TARGET;
    e
}

fn capacity_solve(s: &mut BenchState) {
    s.dials[0] = 4;
}

static CAPACITY_DIALS: [Dial; 1] = [Dial::num(Tr::new("Model size (polynomial degree)", "模型大小（多项式阶数）"), 1, 15, 1, 1.0, 0)];

pub static CAPACITY: Spec = Spec {
    layout: Layout::Plot,
    dials: &CAPACITY_DIALS,
    toggles: &[],
    fault: None,
    init: no_init,
    targets: fixed_split_targets,
    click: no_click,
    eval: capacity_eval,
    solve: capacity_solve,
};

// 3-3 L2

const LAMBDA_MIN: i32 = -16;
const LAMBDA_MAX: i32 = 2;

fn lambda(step: i32) -> f64 {
    10f64.powf(step as f64 * 0.5)
}

fn lambda_x(step: i32) -> f32 {
    (step - LAMBDA_MIN) as f32 / (LAMBDA_MAX - LAMBDA_MIN) as f32
}

fn ridge_eval(s: &BenchState, lang: Lang) -> Eval {
    let step = s.dials[0];
    let (train, val) = split(HELD);
    let steps: Vec<i32> = (LAMBDA_MIN..=LAMBDA_MAX).collect();
    let fits: Vec<_> = steps.iter().map(|&k| Poly::fit(&train, MAX_DEG, lambda(k))).collect();
    let model = &fits[(step - LAMBDA_MIN) as usize];
    let test = model.accuracy(&new_patients());
    let mut e = Eval::default();
    e.info(t(lang, "λ", "λ"), format!("{:.0e}", lambda(step)));
    e.info(t(lang, "Largest weight", "最大权重"), format!("{:.2}", model.largest()));
    e.info(t(lang, "Training score", "训练成绩"), pct(model.accuracy(&train)));
    e.info(t(lang, "Validation score", "验证成绩"), pct(model.accuracy(&val)));
    e.check(t(lang, "New patients (test)", "新病人（测试）"), format!("{} ≥ {}", pct(test), pct(FIT_TARGET)), test >= FIT_TARGET);
    e.curves.push(poly_curve(lang, model));
    let xs: Vec<_> = steps.iter().map(|&k| lambda_x(k)).collect();
    let mut chart = error_chart(lang, t(lang, "L2 strength lg λ", "L2 强度 lg λ"), &xs, &fits, &train, &val);
    chart.cursor = Some(lambda_x(step));
    e.chart = Some(chart);
    e.solved = test >= FIT_TARGET;
    e
}

fn ridge_solve(s: &mut BenchState) {
    s.dials[0] = -6;
}

static RIDGE_DIALS: [Dial; 1] = [Dial::num(Tr::new("L2 strength lg λ (degree 15)", "L2 强度 lg λ（15 阶）"), LAMBDA_MIN, LAMBDA_MAX, LAMBDA_MIN, 0.5, 1)];

pub static RIDGE: Spec = Spec {
    layout: Layout::Plot,
    dials: &RIDGE_DIALS,
    toggles: &[],
    fault: None,
    init: no_init,
    targets: fixed_split_targets,
    click: no_click,
    eval: ridge_eval,
    solve: ridge_solve,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn search_seeds() {
        let pts_for = |noise| poly_data(0x7e57, 300, noise);
        for seed in 1..400u64 {
            for noise in [0.075, 0.085, 0.095] {
                let w = poly_data(seed, POLY_N, noise);
                let pts = pts_for(noise);
                let all = Poly::fit(&w, MAX_DEG, 0.0).accuracy(&pts);
                let train = &w[..POLY_N - HELD];
                let cap: Vec<f32> = (1..=MAX_DEG).map(|d| Poly::fit(train, d, 0.0).accuracy(&pts)).collect();
                let rid: Vec<f32> = (LAMBDA_MIN..=LAMBDA_MAX).map(|k| Poly::fit(train, MAX_DEG, lambda(k)).accuracy(&pts)).collect();
                let cap_ok = cap.iter().filter(|&&a| a >= FIT_TARGET).count();
                let rid_ok = rid.iter().filter(|&&a| a >= FIT_TARGET).count();
                if all < 0.78 && cap[0] < 0.75 && cap[MAX_DEG - 1] < 0.78 && (3..=8).contains(&cap_ok)
                    && rid[0] < 0.78 && *rid.last().unwrap() < 0.5 && (3..=6).contains(&rid_ok)
                {
                    println!("seed {seed} noise {noise}: all {all:.2} cap {cap_ok} rid {rid_ok}");
                }
            }
        }
    }

    #[test]
    #[ignore]
    fn print_tables() {
        for d in [0, 1, 2] {
            for h in 1..=4 {
                let r = trained(d, h);
                println!("act {d} layers {h}: train {:.2} test {:.2}", r.train, r.test);
            }
        }
        for k in [5, 10, 12, 15, 20, 25, 26, 27, 28, 30, 40] {
            let l = losses(k as f32 * LR_SCALE);
            println!("lr {:.2}: last {:.4} rise {:?}", k as f32 * LR_SCALE, l.last().unwrap(), first_rise(&l));
        }
        let pts = new_patients();
        for held in 0..=10 {
            let (d, f) = pick_degree(held);
            println!("held {held}: deg {d} test {:.2}", f[d - 1].accuracy(&pts));
        }
        let (train, _) = split(HELD);
        for d in 1..=MAX_DEG {
            println!("deg {d}: test {:.2}", Poly::fit(&train, d, 0.0).accuracy(&pts));
        }
        for k in LAMBDA_MIN..=LAMBDA_MAX {
            println!("lam {:.1}: test {:.2}", k as f32 * 0.5, Poly::fit(&train, MAX_DEG, lambda(k)).accuracy(&pts));
        }
    }
}

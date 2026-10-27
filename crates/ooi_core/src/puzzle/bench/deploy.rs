//! 第 8 章：走出实验室。漂移（8-1）、多任务头（8-2）、蒸馏（8-3）、
//! INT8 量化（8-4）、灾难性遗忘（8-5 故障）。全部真训练，按设置缓存。

use oo_base::{Lang, Rng, Tr, t};

use super::fit::field;
use super::forge::verdict_targets;
use super::nn::{Act, Adam, Mlp, Sample};
use super::{Bar, BenchState, Cache, Chart, Curve, Dial, Eval, Image, Layout, Mark, Spec, Target, memo, no_click, no_init};

fn pct(v: f32) -> String {
    format!("{:.0}%", v * 100.0)
}

/// `n` labelled points with x in `xs` and y in 0..1, skipping the boundary band.
fn sample_box(seed: u64, n: usize, xs: (f32, f32), rule: impl Fn(f32, f32) -> bool) -> Vec<Sample> {
    let mut rng = Rng::new(seed);
    let mut out = Vec::with_capacity(n);
    while out.len() < n {
        let (x, y) = (xs.0 + (xs.1 - xs.0) * rng.f32(), rng.f32());
        let l = rule(x, y);
        if [(0.02, 0.0), (-0.02, 0.0), (0.0, 0.02), (0.0, -0.02)].iter().all(|&(dx, dy)| rule(x + dx, y + dy) == l) {
            out.push(Sample::new(x, y, l));
        }
    }
    out
}

// ---------------------------------------------------------------- 8-1 换季

/// Ice on the harbour road: the colder (x), the less wet (y) it takes.
fn icy(x: f32, y: f32) -> bool {
    y > 0.85 - 1.3 * (x - 0.2).max(0.0).powi(2) - 0.15 * x
}

const SUMMER: (f32, f32) = (0.0, 0.5);
const WINTER: (f32, f32) = (0.5, 1.0);
const NEW_LABELS: [usize; 5] = [0, 4, 12, 40, 120];
const WINTER_WEIGHT: [usize; 3] = [1, 3, 10];
pub const WINTER_TARGET: f32 = 0.91;
pub const SUMMER_KEEP: f32 = 0.90;
pub const LABEL_BUDGET: usize = 12;

fn drift_data() -> (Vec<Sample>, Vec<Sample>, Vec<Sample>, Vec<Sample>) {
    (
        sample_box(0x501, 400, SUMMER, icy),
        sample_box(0x502, 120, WINTER, icy),
        sample_box(0x503, 200, SUMMER, icy),
        sample_box(0x504, 200, WINTER, icy),
    )
}

static DRIFT_CACHE: Cache<(bool, i32, i32), Mlp> = Cache::new(Default::default);

fn drift_model(retrain: bool, labels: i32, weight: i32) -> Mlp {
    memo(&DRIFT_CACHE, (retrain, labels, weight), || {
        let (summer, winter, _, _) = drift_data();
        let mut data = summer;
        if retrain {
            for _ in 0..WINTER_WEIGHT[weight as usize] {
                data.extend_from_slice(&winter[..NEW_LABELS[labels as usize]]);
            }
        }
        let mut net = Mlp::new(&[8], 1, Act::Tanh, 0x5e);
        net.train(&data, 250, 0.03);
        net
    })
}

fn drift_net(s: &BenchState) -> Mlp {
    drift_model(s.toggles[0], s.dials[0], s.dials[1])
}

fn drift_targets(s: &BenchState, _: Lang) -> Vec<Target> {
    let net = drift_net(s);
    let (_, _, summer, winter) = drift_data();
    let mut out = verdict_targets(&summer[..60], |p| net.prob(p.x, 0) >= 0.5);
    out.extend(verdict_targets(&winter[..100], |p| net.prob(p.x, 0) >= 0.5));
    out
}

fn drift_eval(s: &BenchState, lang: Lang) -> Eval {
    let net = drift_net(s);
    let (_, _, summer, winter) = drift_data();
    let (w, su) = (net.accuracy(&winter), net.accuracy(&summer));
    let labels = if s.toggles[0] { NEW_LABELS[s.dials[0] as usize] } else { 0 };
    let mut e = Eval::default();
    e.check(t(lang, "Winter test", "冬季测试"), format!("{} ≥ {}", pct(w), pct(WINTER_TARGET)), w >= WINTER_TARGET);
    e.check(t(lang, "Summer test (it comes back)", "夏季测试（夏天还会回来）"), format!("{} ≥ {}", pct(su), pct(SUMMER_KEEP)), su >= SUMMER_KEEP);
    e.check(t(lang, "Winter days labelled by hand", "人工标注的冬季样本"), format!("{labels} ≤ {LABEL_BUDGET}"), labels <= LABEL_BUDGET);
    e.field = Some(field(22, |x, y| net.prob([x, y], 0)));

    // The drift monitor: how often each temperature showed up, summer vs. this week.
    let bins = 12;
    let hist = |data: &[Sample]| {
        let mut h = vec![0.0f32; bins];
        for p in data {
            h[((p.x[0] * bins as f32) as usize).min(bins - 1)] += 1.0;
        }
        let top = h.iter().copied().fold(1.0, f32::max);
        h.iter().enumerate().map(|(i, v)| ((i as f32 + 0.5) / bins as f32, 0.05 + 0.9 * v / top)).collect::<Vec<_>>()
    };
    e.chart = Some(Chart {
        title: t(lang, "Drift monitor: inputs this week vs. training", "漂移监视：本周输入 vs. 训练数据").into(),
        x_label: t(lang, "Cold →", "越往右越冷 →").into(),
        curves: vec![
            Curve::new(t(lang, "training (summer)", "训练（夏季）"), hist(&summer), Mark::Neg).dashed(),
            Curve::new(t(lang, "this week", "本周"), hist(&winter), Mark::Hot),
        ],
        cursor: None,
        limit: None,
    });
    e.solved = w >= WINTER_TARGET && su >= SUMMER_KEEP && labels <= LABEL_BUDGET;
    e
}

fn drift_solve(s: &mut BenchState) {
    s.toggles[0] = true;
    s.dials = vec![2, 1];
}

static LABEL_NAMES: [Tr; 5] = [Tr::new("0", "0"), Tr::new("4", "4"), Tr::new("12", "12"), Tr::new("40", "40"), Tr::new("120", "120")];
static WEIGHT_NAMES: [Tr; 3] = [Tr::new("×1", "×1"), Tr::new("×3", "×3"), Tr::new("×10", "×10")];
static DRIFT_DIALS: [Dial; 2] = [
    Dial::named(Tr::new("New winter days labelled", "新标注的冬季样本"), 0, &LABEL_NAMES),
    Dial::named(Tr::new("Weight of the new days", "新样本的权重"), 0, &WEIGHT_NAMES),
];

pub static DRIFT: Spec = Spec {
    layout: Layout::Plot,
    dials: &DRIFT_DIALS,
    toggles: &[Tr::new("Retrain with the new days", "用新样本重训")],
    fault: None,
    init: no_init,
    targets: drift_targets,
    click: no_click,
    eval: drift_eval,
    solve: drift_solve,
};

// ---------------------------------------------------------------- 8-2 一身多用

static HEAD_TASKS: [Tr; 3] = [Tr::new("Sailing", "出海"), Tr::new("Fog horn", "雾号"), Tr::new("Clinic call", "回访")];

/// Three jobs on the same weather readings (wind × damp).
fn head_rule(task: usize, x: f32, y: f32) -> bool {
    match task {
        0 => (x - 0.40).powi(2) + (y - 0.45).powi(2) < 0.08,
        1 => (x - 0.40).powi(2) + (y - 0.45).powi(2) > 0.05 && y > 0.45,
        _ => (x - 0.40).powi(2) + (y - 0.45).powi(2) > 0.12 || x > 0.85,
    }
}

const HEAD_WIDTHS: [usize; 6] = [2, 3, 4, 6, 8, 12];
pub const HEAD_TARGET: f32 = 0.92;
pub const HEAD_BUDGET: usize = 60;

fn head_data(task: usize, seed: u64, n: usize) -> Vec<Sample> {
    let mut d = super::nn::sample_square(seed + task as u64, n, 0.02, |x, y| head_rule(task, x, y));
    for s in &mut d {
        s.head = task;
    }
    d
}

fn head_tests() -> Vec<Vec<Sample>> {
    (0..3).map(|k| head_data(k, 0x8200, 200)).collect()
}

#[derive(Clone)]
struct Heads {
    /// One net (shared) or three.
    nets: Vec<Mlp>,
    acc: [f32; 3],
}

impl Heads {
    fn net(&self, task: usize) -> &Mlp {
        &self.nets[task.min(self.nets.len() - 1)]
    }

    fn prob(&self, task: usize, x: [f32; 2]) -> f32 {
        let head = if self.nets.len() == 1 { task } else { 0 };
        self.net(task).prob(x, head)
    }

    fn params(&self) -> usize {
        self.nets.iter().map(|n| n.params()).sum()
    }
}

static HEADS_CACHE: Cache<(bool, i32), Heads> = Cache::new(Default::default);

fn heads_model(shared: bool, width: i32) -> Heads {
    memo(&HEADS_CACHE, (shared, width), || {
        let w = HEAD_WIDTHS[width as usize];
        let train: Vec<Vec<Sample>> = (0..3).map(|k| head_data(k, 0x8210, 80)).collect();
        let nets = if shared {
            let mut net = Mlp::new(&[w], 3, Act::Tanh, 0x82);
            net.train(&train.concat(), 400, 0.03);
            vec![net]
        } else {
            (0..3)
                .map(|k| {
                    let mut net = Mlp::new(&[w], 1, Act::Tanh, 0x82 + k as u64);
                    let data: Vec<Sample> = train[k].iter().map(|s| Sample { head: 0, ..*s }).collect();
                    net.train(&data, 400, 0.03);
                    net
                })
                .collect()
        };
        let mut h = Heads { nets, acc: [0.0; 3] };
        let tests = head_tests();
        for k in 0..3 {
            h.acc[k] = tests[k].iter().filter(|s| (h.prob(k, s.x) >= 0.5) == s.label()).count() as f32 / tests[k].len() as f32;
        }
        h
    })
}

fn heads_of(s: &BenchState) -> Heads {
    heads_model(s.toggles[0], s.dials[0])
}

fn heads_init(s: &mut BenchState) {
    s.picks = vec![0];
}

fn heads_targets(s: &BenchState, lang: Lang) -> Vec<Target> {
    let h = heads_of(s);
    let shown = s.picks.last().copied().unwrap_or(0);
    HEAD_TASKS
        .iter()
        .enumerate()
        .map(|(k, name)| {
            let mark = if k == shown {
                Mark::Pos
            } else if h.acc[k] >= HEAD_TARGET {
                Mark::Done
            } else {
                Mark::Hot
            };
            Target::chip(format!("{} {}", name.get(lang), pct(h.acc[k])), mark)
        })
        .collect()
}

fn heads_click(s: &mut BenchState, i: usize) {
    if i < HEAD_TASKS.len() {
        s.picks = vec![i];
    }
}

fn heads_eval(s: &BenchState, lang: Lang) -> Eval {
    let h = heads_of(s);
    let mut e = Eval::default();
    for (k, name) in HEAD_TASKS.iter().enumerate() {
        e.check(name.get(lang), format!("{} ≥ {}", pct(h.acc[k]), pct(HEAD_TARGET)), h.acc[k] >= HEAD_TARGET);
    }
    let p = h.params();
    e.check(t(lang, "Parameters", "参数量"), format!("{p} ≤ {HEAD_BUDGET}"), p <= HEAD_BUDGET);
    let task = s.picks.last().copied().unwrap_or(0).min(2);
    e.field = Some(field(22, |x, y| h.prob(task, [x, y])));
    // What the trunk computes: each hidden unit's response over the square.
    for u in 0..HEAD_WIDTHS[s.dials[0] as usize].min(4) {
        e.images.push(Image {
            label: Tr::new("Trunk unit {}", "主干单元 {}").fill(lang, &[&(u + 1)]),
            w: 10,
            px: (0..100).map(|i| h.net(task).features([(i % 10) as f32 / 9.0, 1.0 - (i / 10) as f32 / 9.0])[u]).collect(),
        });
    }
    e.solved = h.acc.iter().all(|&a| a >= HEAD_TARGET) && p <= HEAD_BUDGET;
    e
}

fn heads_solve(s: &mut BenchState) {
    s.toggles[0] = true;
    s.dials[0] = 4;
}

static HEAD_WIDTH_NAMES: [Tr; 6] = [Tr::new("2", "2"), Tr::new("3", "3"), Tr::new("4", "4"), Tr::new("6", "6"), Tr::new("8", "8"), Tr::new("12", "12")];
static HEADS_DIALS: [Dial; 1] = [Dial::named(Tr::new("Hidden units per trunk", "每个主干的隐藏单元"), 2, &HEAD_WIDTH_NAMES)];

pub static HEADS: Spec = Spec {
    layout: Layout::Row,
    dials: &HEADS_DIALS,
    toggles: &[Tr::new("One shared trunk, three heads", "共享一个主干，接三个头")],
    fault: None,
    init: heads_init,
    targets: heads_targets,
    click: heads_click,
    eval: heads_eval,
    solve: heads_solve,
};

// ---------------------------------------------------------------- 8-3 装进旧机器

/// The fog horn rule the big model learned: a wavy coast of damp air.
fn coast(x: f32, y: f32) -> bool {
    y > 0.5 + 0.2 * (x * 11.0).sin() + 0.1 * (x - 0.5)
}

const TEACHER_WIDTH: usize = 48;
const STUDENT_WIDTHS: [usize; 4] = [2, 3, 4, 8];
/// Soft-label temperature choices.
const SOFT_TEMPS: [f32; 4] = [1.0, 2.0, 4.0, 8.0];
pub const DISTILL_DROP: f32 = 0.02;

fn distill_data() -> (Vec<Sample>, Vec<Sample>, Vec<Sample>) {
    use super::nn::sample_square as sq;
    (sq(0x831, 40, 0.02, coast), sq(0x832, 400, 0.0, |_, _| false), sq(0x833, 300, 0.02, coast))
}

static TEACHER: Cache<(), Mlp> = Cache::new(Default::default);

/// The teacher saw the full archive (600 days); the student only has 40 labelled ones.
fn teacher() -> Mlp {
    memo(&TEACHER, (), || {
        let mut net = Mlp::new(&[TEACHER_WIDTH], 1, Act::Tanh, 0x83);
        net.train(&super::nn::sample_square(0x830, 600, 0.0, coast), 700, 0.02);
        net
    })
}

/// Teacher logit at `x`, softened by `temp`.
fn soft(teacher: &Mlp, x: [f32; 2], temp: f32) -> f32 {
    let p = teacher.prob(x, 0).clamp(1e-6, 1.0 - 1e-6);
    1.0 / (1.0 + (-(p / (1.0 - p)).ln() / temp).exp())
}

static STUDENT_CACHE: Cache<(i32, bool, i32), Mlp> = Cache::new(Default::default);

fn student(width: i32, soft_labels: bool, temp: i32) -> Mlp {
    memo(&STUDENT_CACHE, (width, soft_labels, temp), || {
        let (labelled, unlabelled, _) = distill_data();
        let data: Vec<Sample> = if soft_labels {
            // The teacher labels every day it can see, labelled or not.
            let t = teacher();
            labelled.iter().chain(&unlabelled).map(|s| Sample { y: soft(&t, s.x, SOFT_TEMPS[temp as usize]), ..*s }).collect()
        } else {
            labelled
        };
        let mut net = Mlp::new(&[STUDENT_WIDTHS[width as usize]], 1, Act::Tanh, 0x84 + width as u64);
        net.train(&data, 500, 0.04);
        net
    })
}

fn student_of(s: &BenchState) -> Mlp {
    student(s.dials[0], s.toggles[0], s.dials[1])
}

fn distill_targets(s: &BenchState, _: Lang) -> Vec<Target> {
    let net = student_of(s);
    verdict_targets(&distill_data().2[..150], |p| net.prob(p.x, 0) >= 0.5)
}

fn distill_eval(s: &BenchState, lang: Lang) -> Eval {
    let (teach, net) = (teacher(), student_of(s));
    let test = distill_data().2;
    let (ta, sa) = (teach.accuracy(&test), net.accuracy(&test));
    let budget = teach.params() / 10;
    let mut e = Eval::default();
    e.info(t(lang, "Teacher (test)", "教师（测试）"), Tr::new("{} · {} params", "{} · {} 参数").fill(lang, &[&pct(ta), &teach.params()]));
    e.check(t(lang, "Student (test)", "学生（测试）"), format!("{:.1}% ≥ {:.1}%", sa * 100.0, (ta - DISTILL_DROP) * 100.0), sa >= ta - DISTILL_DROP);
    e.check(t(lang, "Student parameters", "学生参数量"), format!("{} ≤ {budget}", net.params()), net.params() <= budget);
    e.field = Some(field(22, |x, y| net.prob([x, y], 0)));
    e.images.push(Image {
        label: if s.toggles[0] {
            Tr::new("What the student copies (T = {})", "学生照着学的（T = {}）").fill(lang, &[&SOFT_TEMPS[s.dials[1] as usize]])
        } else {
            t(lang, "What the student copies (hard labels)", "学生照着学的（硬标签）").into()
        },
        w: 16,
        px: (0..256)
            .map(|i| {
                let x = [(i % 16) as f32 / 15.0, 1.0 - (i / 16) as f32 / 15.0];
                let p = if s.toggles[0] { soft(&teach, x, SOFT_TEMPS[s.dials[1] as usize]) } else if teach.prob(x, 0) >= 0.5 { 1.0 } else { 0.0 };
                p * 2.0 - 1.0
            })
            .collect(),
    });
    e.solved = sa >= ta - DISTILL_DROP && net.params() <= budget;
    e
}

fn distill_solve(s: &mut BenchState) {
    s.toggles[0] = true;
    s.dials = vec![2, 1];
}

static STUDENT_NAMES: [Tr; 4] = [Tr::new("2 units", "2 单元"), Tr::new("3 units", "3 单元"), Tr::new("4 units", "4 单元"), Tr::new("8 units", "8 单元")];
static SOFT_NAMES: [Tr; 4] = [Tr::new("1", "1"), Tr::new("2", "2"), Tr::new("4", "4"), Tr::new("8", "8")];
static DISTILL_DIALS: [Dial; 2] = [
    Dial::named(Tr::new("Student size", "学生大小"), 3, &STUDENT_NAMES),
    Dial::named(Tr::new("Soft-label temperature T", "软标签温度 T"), 0, &SOFT_NAMES),
];

pub static DISTILL: Spec = Spec {
    layout: Layout::Plot,
    dials: &DISTILL_DIALS,
    toggles: &[Tr::new("Learn from the teacher's soft labels", "跟着教师的软标签学")],
    fault: None,
    init: no_init,
    targets: distill_targets,
    click: no_click,
    eval: distill_eval,
    solve: distill_solve,
};

// ---------------------------------------------------------------- 8-4 八位

const Q_WIDTH: usize = 16;
const BITS: [u32; 3] = [16, 8, 4];
pub const QUANT_DROP: f32 = 0.01;
pub const QUANT_BYTES: usize = 48;

static Q_BASE: Cache<(), Mlp> = Cache::new(Default::default);

/// The fog-horn model as it left the lab. Some units learned to fire a
/// hundred times louder than others (their next-layer weights are that much
/// smaller), which is the same function but a nightmare for one shared scale.
fn q_base() -> Mlp {
    memo(&Q_BASE, (), || {
        let mut net = Mlp::new(&[Q_WIDTH], 1, Act::Relu, 0x84);
        net.train(&super::nn::sample_square(0x840, 500, 0.0, coast), 600, 0.02);
        for u in 0..Q_WIDTH {
            let c = 10f32.powf(-1.5 + 3.0 * u as f32 / (Q_WIDTH - 1) as f32);
            let (w1, b1) = &mut net.layers[0];
            for i in 0..2 {
                w1[u * 2 + i] *= c;
            }
            b1[u] *= c;
            net.layers[1].0[u] /= c;
        }
        net
    })
}

/// Weights that share one scale: everything, each layer, or each hidden
/// channel (a unit's incoming weights get one scale, its outgoing weights another).
fn groups(net: &Mlp, scheme: i32) -> Vec<Vec<(usize, usize)>> {
    let last = net.layers.len() - 1;
    let mut out: Vec<Vec<(usize, usize)>> = Vec::new();
    for (l, (w, _)) in net.layers.iter().enumerate() {
        let n_in = net.sizes[l];
        let n_out = w.len() / n_in;
        match scheme {
            0 | 1 => {
                if scheme == 1 || out.is_empty() {
                    out.push(Vec::new());
                }
                out.last_mut().unwrap().extend((0..w.len()).map(|j| (l, j)));
            }
            _ if l == last => out.extend((0..n_in).map(|i| (0..n_out).map(|o| (l, o * n_in + i)).collect())),
            _ => out.extend((0..n_out).map(|o| (0..n_in).map(|i| (l, o * n_in + i)).collect())),
        }
    }
    out
}

/// The quantised net and its weight bytes (scales and biases are a rounding error on a real model).
fn quantize(net: &Mlp, bits: u32, scheme: i32) -> (Mlp, usize) {
    let qmax = ((1 << (bits - 1)) - 1) as f32;
    let mut q = net.clone();
    for g in &groups(net, scheme) {
        let top = g.iter().map(|&(l, j)| net.layers[l].0[j].abs()).fold(1e-12, f32::max);
        let step = top / qmax;
        for &(l, j) in g {
            q.layers[l].0[j] = (net.layers[l].0[j] / step).round().clamp(-qmax, qmax) * step;
        }
    }
    let weights: usize = net.layers.iter().map(|(w, _)| w.len()).sum();
    (q, weights * bits as usize / 8)
}

/// Distinct levels a group's smallest-magnitude weight row can use.
fn levels(net: &Mlp, scheme: i32, bits: u32, u: usize) -> f32 {
    let qmax = ((1u32 << (bits - 1)) - 1) as f32;
    let groups = groups(net, scheme);
    let g = groups.iter().find(|g| g.contains(&(0, u * 2))).unwrap();
    let top = g.iter().map(|&(l, j)| net.layers[l].0[j].abs()).fold(1e-12, f32::max);
    let mag = (0..2).map(|i| net.layers[0].0[u * 2 + i].abs()).fold(0.0, f32::max);
    (2.0 * (mag / top * qmax).round() + 1.0).min(2.0 * qmax + 1.0)
}

fn quant_test() -> Vec<Sample> {
    super::nn::sample_square(0x841, 400, 0.02, coast)
}

fn quant_eval(s: &BenchState, lang: Lang) -> Eval {
    let base = q_base();
    let test = quant_test();
    let (q, bytes) = quantize(&base, BITS[s.dials[0] as usize], s.dials[1]);
    let (a0, a1) = (base.accuracy(&test), q.accuracy(&test));
    let mut e = Eval::default();
    e.info(t(lang, "float32 model", "float32 模型"), format!("{:.1}% · {} B", a0 * 100.0, quantize(&base, 16, 0).1 * 2));
    e.check(t(lang, "Quantised (test)", "量化后（测试）"), format!("{:.1}% ≥ {:.1}%", a1 * 100.0, (a0 - QUANT_DROP) * 100.0), a1 >= a0 - QUANT_DROP);
    e.check(t(lang, "Weight bytes on the old machine", "旧机器上的权重字节数"), format!("{bytes} ≤ {QUANT_BYTES}"), bytes <= QUANT_BYTES);
    e.field = Some(field(22, |x, y| q.prob([x, y], 0)));
    // Each hidden unit's incoming weights: how many distinct levels survive.
    e.bars_title = t(lang, "Levels each unit's weights can still use", "每个单元的权重还能用的刻度数").into();
    let bits = BITS[s.dials[0] as usize];
    let full = ((2u32 << (bits - 1)) - 1) as f32;
    for u in (0..Q_WIDTH).step_by(3) {
        let n = levels(&base, s.dials[1], bits, u);
        let mark = if n < 9.0 { Mark::Hot } else if n >= full * 0.5 { Mark::Pos } else { Mark::Neutral };
        e.bars.push(Bar { label: Tr::new("unit {}", "单元 {}").fill(lang, &[&(u + 1)]), value: (n.ln() / full.ln()).clamp(0.02, 1.0), ghost: None, text: format!("{n:.0}"), mark });
    }
    e.solved = a1 >= a0 - QUANT_DROP && bytes <= QUANT_BYTES;
    e
}

fn quant_targets(s: &BenchState, _: Lang) -> Vec<Target> {
    let (q, _) = quantize(&q_base(), BITS[s.dials[0] as usize], s.dials[1]);
    verdict_targets(&quant_test()[..150], |p| q.prob(p.x, 0) >= 0.5)
}

fn quant_init(s: &mut BenchState) {
    s.dials = vec![1, 0];
}

fn quant_solve(s: &mut BenchState) {
    s.dials = vec![1, 2];
}

static BIT_NAMES: [Tr; 3] = [Tr::new("16-bit", "16 位"), Tr::new("INT8", "INT8"), Tr::new("INT4", "INT4")];
static SCHEME_NAMES: [Tr; 3] = [Tr::new("one global scale", "一个全局缩放"), Tr::new("one per layer", "每层一个"), Tr::new("one per channel", "每通道一个")];
static QUANT_DIALS: [Dial; 2] = [
    Dial::named(Tr::new("Bits per weight", "每个权重的位数"), 1, &BIT_NAMES),
    Dial::named(Tr::new("Scales", "缩放因子"), 0, &SCHEME_NAMES),
];

pub static QUANT: Spec = Spec {
    layout: Layout::Plot,
    dials: &QUANT_DIALS,
    toggles: &[],
    fault: None,
    init: quant_init,
    targets: quant_targets,
    click: no_click,
    eval: quant_eval,
    solve: quant_solve,
};

// ---------------------------------------------------------------- 8-5 忘了

/// Two jobs on one small net: the sorter's old task (head 0), then the fog horn (head 1).
fn old_rule(x: f32, y: f32) -> bool {
    (x - 0.35).powi(2) + (y - 0.6).powi(2) < 0.07
}

const PHASE: usize = 150;
/// Snapshot every this many steps.
const SNAP: usize = 10;
const BUFFERS: [usize; 5] = [0, 4, 16, 40, 120];
const FORGET_RATES: [f32; 3] = [0.003, 0.01, 0.03];
pub const OLD_KEEP: f32 = 0.92;
pub const NEW_TARGET: f32 = 0.92;
pub const BUFFER_BUDGET: usize = 40;

fn forget_data() -> [Vec<Sample>; 4] {
    use super::nn::sample_square as sq;
    let tag = |mut d: Vec<Sample>, h| {
        for s in &mut d {
            s.head = h;
        }
        d
    };
    [tag(sq(0x851, 160, 0.02, old_rule), 0), tag(sq(0x852, 160, 0.02, coast), 1), tag(sq(0x853, 200, 0.02, old_rule), 0), tag(sq(0x854, 200, 0.02, coast), 1)]
}

#[derive(Clone)]
struct Run {
    /// (old-task, new-task) test accuracy every SNAP steps over both phases.
    curve: Vec<(f32, f32)>,
}

static FORGET_CACHE: Cache<(i32, i32), Run> = Cache::new(Default::default);

/// Phase 1 learns the old task; phase 2 trains only on fog (plus the replay buffer).
fn forget_run(buffer: i32, rate: i32) -> Run {
    memo(&FORGET_CACHE, (buffer, rate), || {
        let [old, new, old_test, new_test] = forget_data();
        let mut net = Mlp::new(&[10], 2, Act::Tanh, 0x85);
        let mut adam = Adam::new(&net);
        let snap = |n: &Mlp| (n.accuracy(&old_test), n.accuracy(&new_test));
        let mut curve = vec![snap(&net)];
        for step in 1..=PHASE {
            net.epoch(&mut adam, &old, 0.03, 0.0);
            if step % SNAP == 0 {
                curve.push(snap(&net));
            }
        }
        let mut phase2 = new;
        phase2.extend_from_slice(&old[..BUFFERS[buffer as usize]]);
        let mut adam = Adam::new(&net);
        for step in 1..=PHASE {
            net.epoch(&mut adam, &phase2, FORGET_RATES[rate as usize], 0.0);
            if step % SNAP == 0 {
                curve.push(snap(&net));
            }
        }
        Run { curve }
    })
}

/// The bare run: phase 2 at the default rate with no buffer.
fn crash_snapshot() -> usize {
    let run = forget_run(0, 2);
    let start = PHASE / SNAP;
    (start..run.curve.len()).find(|&i| run.curve[i].0 < 0.75).unwrap_or(run.curve.len() - 1)
}

fn forget_init(s: &mut BenchState) {
    s.dials = vec![0, 2];
}

/// Snapshots of phase 2 as chips: the old task's score at each.
fn forget_targets(s: &BenchState, _: Lang) -> Vec<Target> {
    let run = forget_run(s.dials[0], s.dials[1]);
    let crash = crash_snapshot();
    (PHASE / SNAP..run.curve.len())
        .map(|i| {
            let mark = if s.picks.contains(&(i - PHASE / SNAP)) {
                if i == crash { Mark::Pos } else { Mark::Done }
            } else if run.curve[i].0 < 0.75 {
                Mark::Hot
            } else {
                Mark::Neutral
            };
            Target::chip(format!("{} · {}", (i - PHASE / SNAP) * SNAP, pct(run.curve[i].0)), mark)
        })
        .collect()
}

fn forget_click(s: &mut BenchState, i: usize) {
    if s.evidence || i > PHASE / SNAP || s.picks.contains(&i) {
        return;
    }
    s.picks.push(i);
    if i + PHASE / SNAP == crash_snapshot() {
        s.evidence = true;
    } else {
        s.wrong += 1;
    }
}

fn forget_eval(s: &BenchState, lang: Lang) -> Eval {
    let run = forget_run(s.dials[0], s.dials[1]);
    let (old, new) = *run.curve.last().unwrap();
    let buffer = BUFFERS[s.dials[0] as usize];
    let mut e = Eval::default();
    e.check(t(lang, "Evidence", "证据"), if s.evidence { t(lang, "yes", "是") } else { t(lang, "no", "否") }, s.evidence);
    e.check(t(lang, "Sorter, after learning fog", "学会雾号后的分拣"), format!("{} ≥ {}", pct(old), pct(OLD_KEEP)), old >= OLD_KEEP);
    e.check(t(lang, "Fog horn", "雾号"), format!("{} ≥ {}", pct(new), pct(NEW_TARGET)), new >= NEW_TARGET);
    e.check(t(lang, "Replay buffer", "回放缓冲"), format!("{buffer} ≤ {BUFFER_BUDGET}"), buffer <= BUFFER_BUDGET);
    let n = (run.curve.len() - 1) as f32;
    e.chart = Some(Chart {
        title: t(lang, "Test accuracy while training (sorter first, then fog)", "训练中的测试准确率（先分拣，后雾号）").into(),
        x_label: Tr::new("{} steps each", "各 {} 步").fill(lang, &[&PHASE]),
        curves: vec![
            Curve::new(t(lang, "sorter (old)", "分拣（旧）"), run.curve.iter().enumerate().map(|(i, c)| (i as f32 / n, (c.0 - 0.3) / 0.7)).collect(), Mark::Neg),
            Curve::new(t(lang, "fog horn (new)", "雾号（新）"), run.curve.iter().enumerate().map(|(i, c)| (i as f32 / n, (c.1 - 0.3) / 0.7)).collect(), Mark::Pos).dashed(),
        ],
        cursor: Some(0.5),
        limit: Some((OLD_KEEP - 0.3) / 0.7),
    });
    e.solved = s.evidence && old >= OLD_KEEP && new >= NEW_TARGET && buffer <= BUFFER_BUDGET;
    e
}

fn forget_solve(s: &mut BenchState) {
    forget_click(s, crash_snapshot() - PHASE / SNAP);
    s.dials = vec![3, 2];
}

static BUFFER_NAMES: [Tr; 5] = [Tr::new("none", "无"), Tr::new("4 old samples", "4 个旧样本"), Tr::new("16 old samples", "16 个旧样本"), Tr::new("40 old samples", "40 个旧样本"), Tr::new("120 old samples", "120 个旧样本")];
static FORGET_RATE_NAMES: [Tr; 3] = [Tr::new("0.003", "0.003"), Tr::new("0.01", "0.01"), Tr::new("0.03", "0.03")];
static FORGET_DIALS: [Dial; 2] = [
    Dial::named(Tr::new("Replay buffer", "回放缓冲"), 0, &BUFFER_NAMES),
    Dial::named(Tr::new("Learning rate on the new task", "新任务的学习率"), 2, &FORGET_RATE_NAMES),
];

pub static FORGET: Spec = Spec {
    layout: Layout::Row,
    dials: &FORGET_DIALS,
    toggles: &[],
    fault: Some(Tr::new(
        "Training only on fog pulled the shared weights away from sorting.",
        "只用雾号数据训练，共享的权重被拉离了分拣任务。",
    )),
    init: forget_init,
    targets: forget_targets,
    click: forget_click,
    eval: forget_eval,
    solve: forget_solve,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn print_forget() {
        println!("crash at snapshot {}", crash_snapshot());
        for rate in 0..3 {
            for b in 0..5 {
                let r = forget_run(b, rate);
                let c: Vec<String> = r.curve.iter().step_by(3).map(|c| format!("{:.0}/{:.0}", c.0 * 100.0, c.1 * 100.0)).collect();
                println!("rate={} buf={} {}", FORGET_RATES[rate as usize], BUFFERS[b as usize], c.join(" "));
            }
        }
    }

    #[test]
    #[ignore]
    fn print_quant() {
        let base = q_base();
        let test = quant_test();
        println!("base {:.3}", base.accuracy(&test));
        for b in BITS {
            for scheme in 0..3 {
                let (q, bytes) = quantize(&base, b, scheme);
                println!("bits={b} scheme={scheme} acc={:.3} bytes={bytes}", q.accuracy(&test));
            }
        }
    }

    #[test]
    #[ignore]
    fn print_distill() {
        let test = distill_data().2;
        let t = teacher();
        println!("teacher {:.3} p={}", t.accuracy(&test), t.params());
        for w in 0..4 {
            let mut row = format!(" hard:{:.3}", student(w, false, 0).accuracy(&test));
            for k in 0..4 {
                row += &format!(" T{}:{:.3}", SOFT_TEMPS[k as usize], student(w, true, k).accuracy(&test));
            }
            println!("w={} p={}{row}", STUDENT_WIDTHS[w as usize], student(w, false, 0).params());
        }
    }

    #[test]
    #[ignore]
    fn print_heads() {
        for shared in [false, true] {
            for w in 0..6 {
                let h = heads_model(shared, w);
                println!("shared={shared} w={} p={} acc={:?}", HEAD_WIDTHS[w as usize], h.params(), h.acc);
            }
        }
    }

    #[test]
    #[ignore]
    fn print_drift() {
        let (_, _, summer, winter) = drift_data();
        let net = drift_model(false, 0, 0);
        println!("base w{:.2} s{:.2}", net.accuracy(&winter), net.accuracy(&summer));
        for weight in 0..3 {
            let mut row = String::new();
            for labels in 0..5 {
                let net = drift_model(true, labels, weight);
                row += &format!(" n{}:{:.0}/{:.0}", NEW_LABELS[labels as usize], net.accuracy(&winter) * 100.0, net.accuracy(&summer) * 100.0);
            }
            println!("w{weight}{row}");
        }
    }
}

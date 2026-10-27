//! 第 1–6 章的委托关（Forge）：没有初始模型，玩家把本章学到的部件组合起来，
//! 在隐藏的测试集上交付。每关都真训练，结果按部件组合缓存。

use oo_base::{Lang, Tr, t};

use super::fit::field;
use super::nn::{Act, Mlp, Sample, sample_square};
use super::{BenchState, Cache, Chart, Curve, Dial, Eval, Image, Layout, Mark, Spec, Target, memo, no_click, no_init};

fn pct(v: f32) -> String {
    format!("{:.0}%", v * 100.0)
}

/// Test points coloured by truth, hot where the model is wrong.
pub(super) fn verdict_targets(test: &[Sample], said: impl Fn(&Sample) -> bool) -> Vec<Target> {
    test.iter()
        .map(|s| {
            let mark = if said(s) != s.label() {
                Mark::Hot
            } else if s.label() {
                Mark::Pos
            } else {
                Mark::Neg
            };
            Target::at(s.x[0], s.x[1], mark)
        })
        .collect()
}

/// Decision field around a threshold: positive where `p ≥ thr`.
fn threshold_field(net: &Mlp, thr: f32) -> Image {
    field(22, |x, y| (net.prob([x, y], 0) - thr + 0.5).clamp(0.0, 1.0))
}

// ---------------------------------------------------------------- 1-6 出海预报

/// De's sailing log: wind × swell, safe inside a calm pocket.
fn calm(x: f32, y: f32) -> bool {
    (x - 0.38).powi(2) + (y - 0.40).powi(2) < 0.075
}

/// Storm days: heavy swell outside the pocket. Never call these "sail".
fn storm(s: &Sample) -> bool {
    !s.label() && s.x[1] > 0.55
}

/// Training days come from De's log, a few mislabelled; the test days are clean.
fn sail_data() -> (Vec<Sample>, Vec<Sample>) {
    let mut train = sample_square(0x5a11, 90, 0.0, calm);
    for s in train.iter_mut().step_by(13) {
        s.y = 1.0 - s.y;
    }
    (train, sample_square(0x5a12, 160, 0.0, calm))
}

const SAIL_HIDDEN: [usize; 5] = [0, 1, 2, 3, 6];
pub const SAIL_TARGET: f32 = 0.90;
const THR_SCALE: f32 = 0.05;

#[derive(Clone)]
struct SailModel {
    net: Mlp,
}

static SAIL_CACHE: Cache<(i32, bool), SailModel> = Cache::new(Default::default);

fn sail_model(hidden: i32, squares: bool) -> SailModel {
    memo(&SAIL_CACHE, (hidden, squares), || {
        let (train, _) = sail_data();
        let h = SAIL_HIDDEN[hidden as usize];
        let layers: &[usize] = if h == 0 { &[] } else { &[h] };
        let mut net = Mlp::build(layers, 1, Act::Tanh, 0x5a1 + h as u64, squares);
        net.train(&train, 400, 0.05);
        SailModel { net }
    })
}

fn sail_thr(s: &BenchState) -> f32 {
    s.dials[1] as f32 * THR_SCALE
}

fn sail_targets(s: &BenchState, _: Lang) -> Vec<Target> {
    let m = sail_model(s.dials[0], s.toggles[0]);
    let thr = sail_thr(s);
    verdict_targets(&sail_data().1, |p| m.net.prob(p.x, 0) >= thr)
}

fn sail_eval(s: &BenchState, lang: Lang) -> Eval {
    let m = sail_model(s.dials[0], s.toggles[0]);
    let thr = sail_thr(s);
    let test = sail_data().1;
    let says = |p: &Sample| m.net.prob(p.x, 0) >= thr;
    let acc = test.iter().filter(|p| says(p) == p.label()).count() as f32 / test.len() as f32;
    let storms = test.iter().filter(|p| storm(p) && says(p)).count();
    let mut e = Eval::default();
    e.check(t(lang, "Test accuracy", "测试准确率"), format!("{} ≥ {}", pct(acc), pct(SAIL_TARGET)), acc >= SAIL_TARGET);
    e.check(t(lang, "Storm days called “sail”", "风暴日判成“出海”"), format!("{storms} = 0"), storms == 0);
    e.info(t(lang, "Parameters", "参数量"), m.net.params().to_string());
    e.field = Some(threshold_field(&m.net, thr));
    e.solved = acc >= SAIL_TARGET && storms == 0;
    e
}

fn sail_solve(s: &mut BenchState) {
    s.dials = vec![0, 15];
    s.toggles[0] = true;
}

static SAIL_HIDDEN_NAMES: [Tr; 5] = [
    Tr::new("none: one perceptron", "无：单个感知机"),
    Tr::new("1 neuron", "1 个神经元"),
    Tr::new("2 neurons", "2 个神经元"),
    Tr::new("3 neurons", "3 个神经元"),
    Tr::new("6 neurons", "6 个神经元"),
];
static SAIL_DIALS: [Dial; 2] = [
    Dial::named(Tr::new("Hidden layer", "隐藏层"), 0, &SAIL_HIDDEN_NAMES),
    Dial::num(Tr::new("Say “sail” above this confidence", "置信度高于此值才说“出海”"), 10, 19, 10, THR_SCALE, 2),
];

pub static SAIL: Spec = Spec {
    layout: Layout::Plot,
    dials: &SAIL_DIALS,
    toggles: &[Tr::new("Add features: wind² and swell²", "加特征：风速² 与浪高²")],
    fault: None,
    init: no_init,
    targets: sail_targets,
    click: no_click,
    eval: sail_eval,
    solve: sail_solve,
};

// ---------------------------------------------------------------- 2-6 雾号

/// Su's harbour log: (humidity, air–sea gap) → fog within two hours. A crescent:
/// humid enough, but only while the gap stays in a narrow band.
fn foggy(x: f32, y: f32) -> bool {
    let outer = (x - 0.62).powi(2) + (y - 0.42).powi(2) < 0.16;
    let inner = (x - 0.78).powi(2) + (y - 0.50).powi(2) < 0.07;
    outer && !inner
}

fn fog_data() -> (Vec<Sample>, Vec<Sample>) {
    (sample_square(0xf061, 160, 0.015, foggy), sample_square(0xf062, 240, 0.015, foggy))
}

const FOG_EPOCHS: usize = 240;
const FOG_WIDTH: usize = 8;
pub const RECALL_TARGET: f32 = 0.95;
pub const FALSE_ALARM_MAX: f32 = 0.10;
static FOG_RATES: [f32; 7] = [0.0003, 0.001, 0.003, 0.01, 0.03, 0.1, 3.0];

#[derive(Clone)]
struct FogModel {
    net: Mlp,
    /// Training loss every 8 epochs.
    curve: Vec<f32>,
}

static FOG_CACHE: Cache<(i32, i32, i32), FogModel> = Cache::new(Default::default);

fn fog_model(d: &[i32]) -> FogModel {
    memo(&FOG_CACHE, (d[0], d[1], d[2]), || {
        let (train, _) = fog_data();
        let act = [Act::Linear, Act::Tanh, Act::Relu][d[0] as usize];
        let hidden = vec![FOG_WIDTH; d[1] as usize];
        let mut net = Mlp::new(&hidden, 1, act, 0xf0 + d[1] as u64);
        let mut adam = super::nn::Adam::new(&net);
        let mut curve = vec![net.loss(&train)];
        for epoch in 1..=FOG_EPOCHS {
            net.epoch(&mut adam, &train, FOG_RATES[d[2] as usize], 0.0);
            if epoch % 8 == 0 {
                curve.push(net.loss(&train));
            }
        }
        FogModel { net, curve }
    })
}

/// (recall, false-alarm rate) on the test days.
fn fog_scores(net: &Mlp, test: &[Sample]) -> (f32, f32) {
    let fog = test.iter().filter(|s| s.label()).count().max(1) as f32;
    let clear = test.iter().filter(|s| !s.label()).count().max(1) as f32;
    let hit = test.iter().filter(|s| s.label() && net.prob(s.x, 0) >= 0.5).count() as f32;
    let alarm = test.iter().filter(|s| !s.label() && net.prob(s.x, 0) >= 0.5).count() as f32;
    (hit / fog, alarm / clear)
}

fn fog_targets(s: &BenchState, _: Lang) -> Vec<Target> {
    let m = fog_model(&s.dials);
    verdict_targets(&fog_data().1, |p| m.net.prob(p.x, 0) >= 0.5)
}

pub(super) fn loss_y(l: f32) -> f32 {
    let (lo, hi) = (0.02f32.ln(), 1.5f32.ln());
    ((l.max(1e-6).ln() - lo) / (hi - lo)).clamp(0.0, 1.0)
}

fn fog_eval(s: &BenchState, lang: Lang) -> Eval {
    let m = fog_model(&s.dials);
    let (recall, alarm) = fog_scores(&m.net, &fog_data().1);
    let mut e = Eval::default();
    e.check(t(lang, "Fog caught 2 h ahead (recall)", "提前 2 小时报出的雾（召回）"), format!("{} ≥ {}", pct(recall), pct(RECALL_TARGET)), recall >= RECALL_TARGET);
    e.check(t(lang, "False alarms", "误报率"), format!("{} ≤ {}", pct(alarm), pct(FALSE_ALARM_MAX)), alarm <= FALSE_ALARM_MAX);
    e.info(t(lang, "Final training loss", "最终训练损失"), format!("{:.3}", m.curve.last().unwrap()));
    e.info(t(lang, "Parameters", "参数量"), m.net.params().to_string());
    e.field = Some(field(22, |x, y| m.net.prob([x, y], 0)));
    let n = m.curve.len() - 1;
    let pts = m.curve.iter().enumerate().map(|(i, &l)| (i as f32 / n as f32, loss_y(l))).collect();
    e.chart = Some(Chart {
        title: t(lang, "Training loss (log scale)", "训练损失（对数刻度）").into(),
        x_label: Tr::new("{} epochs", "{} 轮").fill(lang, &[&FOG_EPOCHS]),
        curves: vec![Curve::new(t(lang, "loss", "损失"), pts, Mark::Pos)],
        cursor: None,
        limit: None,
    });
    e.solved = recall >= RECALL_TARGET && alarm <= FALSE_ALARM_MAX;
    e
}

fn fog_solve(s: &mut BenchState) {
    s.dials = vec![1, 2, 4];
}

static FOG_ACTS: [Tr; 3] = [Tr::new("none (linear)", "无（线性）"), Tr::new("tanh", "tanh"), Tr::new("ReLU", "ReLU")];
static FOG_RATE_NAMES: [Tr; 7] = [
    Tr::new("0.0003", "0.0003"),
    Tr::new("0.001", "0.001"),
    Tr::new("0.003", "0.003"),
    Tr::new("0.01", "0.01"),
    Tr::new("0.03", "0.03"),
    Tr::new("0.1", "0.1"),
    Tr::new("3.0", "3.0"),
];
static FOG_DIALS: [Dial; 3] = [
    Dial::named(Tr::new("Activation between layers", "层间激活函数"), 0, &FOG_ACTS),
    Dial::num(Tr::new("Hidden layers (8 wide)", "隐藏层数（每层 8 个）"), 0, 3, 1, 1.0, 0),
    Dial::named(Tr::new("Learning rate", "学习率"), 0, &FOG_RATE_NAMES),
];

pub static FOG: Spec = Spec {
    layout: Layout::Plot,
    dials: &FOG_DIALS,
    toggles: &[],
    fault: None,
    init: no_init,
    targets: fog_targets,
    click: no_click,
    eval: fog_eval,
    solve: fog_solve,
};

// ---------------------------------------------------------------- 3-6 回访名单

/// Lina's patients: (age, dose) → needs a follow-up call, inside an oval. The
/// charts she trains on are hand-copied and one in seven is mislabelled.
fn at_risk(x: f32, y: f32) -> bool {
    (x - 0.35).powi(2) / 0.06 + (y - 0.62).powi(2) / 0.10 < 1.0
}

fn clinic_data() -> (Vec<Sample>, Vec<Sample>) {
    let mut train = sample_square(0xc11, 100, 0.0, at_risk);
    for s in train.iter_mut().skip(3).step_by(7) {
        s.y = 1.0 - s.y;
    }
    (train, sample_square(0xc12, 300, 0.01, at_risk))
}

static CLINIC_WIDTHS: [usize; 4] = [2, 6, 24, 64];
static CLINIC_L2: [f32; 5] = [0.0, 0.0003, 0.003, 0.03, 0.3];
const CLINIC_EPOCHS: usize = 400;
pub const CLINIC_RECALL: f32 = 0.98;
/// Lina can phone at most this share of the test patients.
pub const CALL_BUDGET: f32 = 0.35;
const CALL_SCALE: f32 = 0.02;

#[derive(Clone)]
struct ClinicModel {
    net: Mlp,
    epochs: usize,
    train_acc: f32,
}

static CLINIC_CACHE: Cache<(i32, i32, bool), ClinicModel> = Cache::new(Default::default);

fn clinic_model(width: i32, l2: i32, early: bool) -> ClinicModel {
    memo(&CLINIC_CACHE, (width, l2, early), || {
        let (all, _) = clinic_data();
        // Early stopping holds out every fourth chart and keeps the best epoch on it.
        let (train, val): (Vec<Sample>, Vec<Sample>) = if early {
            let (v, t): (Vec<_>, Vec<_>) = all.iter().enumerate().partition(|(i, _)| i % 4 == 3);
            (t.into_iter().map(|(_, s)| *s).collect(), v.into_iter().map(|(_, s)| *s).collect())
        } else {
            (all.clone(), Vec::new())
        };
        let mut net = Mlp::new(&[CLINIC_WIDTHS[width as usize]], 1, Act::Tanh, 0xc1 + width as u64);
        let mut adam = super::nn::Adam::new(&net);
        let (mut best, mut best_loss, mut epochs) = (net.clone(), f32::MAX, CLINIC_EPOCHS);
        for epoch in 1..=CLINIC_EPOCHS {
            net.epoch(&mut adam, &train, 0.03, CLINIC_L2[l2 as usize]);
            if early && epoch % 5 == 0 {
                let l = net.loss(&val);
                if l < best_loss {
                    (best, best_loss, epochs) = (net.clone(), l, epoch);
                }
            }
        }
        let net = if early { best } else { net };
        ClinicModel { train_acc: net.accuracy(&train), net, epochs }
    })
}

fn call_thr(s: &BenchState) -> f32 {
    s.dials[2] as f32 * CALL_SCALE
}

/// (recall, share of patients called) at threshold `thr`.
fn clinic_scores(net: &Mlp, test: &[Sample], thr: f32) -> (f32, f32) {
    let sick = test.iter().filter(|s| s.label()).count().max(1) as f32;
    let caught = test.iter().filter(|s| s.label() && net.prob(s.x, 0) >= thr).count() as f32;
    let called = test.iter().filter(|s| net.prob(s.x, 0) >= thr).count() as f32;
    (caught / sick, called / test.len() as f32)
}

fn clinic_targets(s: &BenchState, _: Lang) -> Vec<Target> {
    let m = clinic_model(s.dials[0], s.dials[1], s.toggles[0]);
    let thr = call_thr(s);
    // Only the first 120 test patients are drawn; the score uses all 300.
    verdict_targets(&clinic_data().1[..120], |p| m.net.prob(p.x, 0) >= thr)
}

fn clinic_eval(s: &BenchState, lang: Lang) -> Eval {
    let m = clinic_model(s.dials[0], s.dials[1], s.toggles[0]);
    let thr = call_thr(s);
    let (recall, called) = clinic_scores(&m.net, &clinic_data().1, thr);
    let mut e = Eval::default();
    e.info(t(lang, "Training accuracy", "训练准确率"), pct(m.train_acc));
    e.check(t(lang, "Patients who needed a call (recall)", "该回访的人（召回）"), format!("{} ≥ {}", pct(recall), pct(CLINIC_RECALL)), recall >= CLINIC_RECALL);
    e.check(t(lang, "Patients Lina must phone", "莉娜要打的电话"), format!("{} ≤ {}", pct(called), pct(CALL_BUDGET)), called <= CALL_BUDGET);
    e.info(t(lang, "Epochs trained", "训练轮数"), m.epochs.to_string());
    e.info(t(lang, "Parameters", "参数量"), m.net.params().to_string());
    e.field = Some(threshold_field(&m.net, thr));
    e.solved = recall >= CLINIC_RECALL && called <= CALL_BUDGET;
    e
}

fn clinic_solve(s: &mut BenchState) {
    s.dials = vec![1, 2, 18];
    s.toggles[0] = true;
}

static CLINIC_WIDTH_NAMES: [Tr; 4] = [
    Tr::new("2 neurons", "2 个神经元"),
    Tr::new("6 neurons", "6 个神经元"),
    Tr::new("24 neurons", "24 个神经元"),
    Tr::new("64 neurons", "64 个神经元"),
];
static CLINIC_L2_NAMES: [Tr; 5] = [
    Tr::new("none", "无"),
    Tr::new("0.0003", "0.0003"),
    Tr::new("0.003", "0.003"),
    Tr::new("0.03", "0.03"),
    Tr::new("0.3", "0.3"),
];
static CLINIC_DIALS: [Dial; 3] = [
    Dial::named(Tr::new("Model size (hidden layer)", "模型大小（隐藏层）"), 3, &CLINIC_WIDTH_NAMES),
    Dial::named(Tr::new("L2 strength λ", "L2 强度 λ"), 0, &CLINIC_L2_NAMES),
    Dial::num(Tr::new("Call anyone above this risk", "风险高于此值就回访"), 1, 25, 25, CALL_SCALE, 2),
];

pub static CLINIC: Spec = Spec {
    layout: Layout::Plot,
    dials: &CLINIC_DIALS,
    toggles: &[Tr::new("Early stopping on a held-out quarter", "留出四分之一做早停")],
    fault: None,
    init: no_init,
    targets: clinic_targets,
    click: no_click,
    eval: clinic_eval,
    solve: clinic_solve,
};

// ---------------------------------------------------------------- 4-6 分拣器

const SIDE: usize = 11;
/// Postcodes on the test sacks; every other one is in Su's slanted hand.
const CODES: usize = 48;
const SHOWN_CODES: usize = 8;
pub const DIGIT_TARGET: f32 = 0.95;

/// How a digit was written: slant (columns per row from the middle) and stroke width.
#[derive(Clone, Copy, PartialEq)]
struct Hand {
    slant: f32,
    thick: bool,
}

const PRINTED: Hand = Hand { slant: 0.0, thick: false };
const SU: Hand = Hand { slant: 0.7, thick: true };
/// Augmentation adds these copies of each training digit.
const AUGMENT: [Hand; 3] = [Hand { slant: 0.7, thick: false }, Hand { slant: -0.7, thick: false }, Hand { slant: 0.0, thick: true }];

/// Digit `d` written in `hand`, moved by (dx, dy), on a −1 background.
fn write(d: usize, hand: Hand, dx: i32, dy: i32) -> Vec<f32> {
    let mut g = vec![-1.0; SIDE * SIDE];
    for (r, row) in super::tune::GLYPHS[d].iter().enumerate() {
        let lean = (hand.slant * (3.0 - r as f32)).round() as i32;
        for (c, ch) in row.bytes().enumerate() {
            if ch != b'1' {
                continue;
            }
            for extra in 0..=hand.thick as i32 {
                let (y, x) = (r as i32 + 2 + dy, c as i32 + 3 + dx + lean + extra);
                if (0..SIDE as i32).contains(&x) && (0..SIDE as i32).contains(&y) {
                    g[y as usize * SIDE + x as usize] = 1.0;
                }
            }
        }
    }
    g
}

fn at(img: &[f32], r: i32, c: i32) -> f32 {
    if (0..SIDE as i32).contains(&r) && (0..SIDE as i32).contains(&c) { img[r as usize * SIDE + c as usize] } else { -1.0 }
}

/// Correlation of `img` moved by (dx, dy) with a prototype, in −1..1.
fn corr(img: &[f32], proto: &[f32], dx: i32, dy: i32) -> f32 {
    let mut sum = 0.0;
    for r in 0..SIDE as i32 {
        for c in 0..SIDE as i32 {
            sum += at(img, r + dy, c + dx) * proto[r as usize * SIDE + c as usize];
        }
    }
    sum / (SIDE * SIDE) as f32
}

#[derive(Clone)]
struct Sorter {
    /// `channels` prototypes per digit, learned from the training sacks.
    protos: Vec<Vec<Vec<f32>>>,
    reach: i32,
    /// (code digits, read digits, hand) per test postcode.
    codes: Vec<([usize; 6], [usize; 6], bool)>,
    acc: f32,
    su_acc: f32,
}

impl Sorter {
    /// Best (score, digit, prototype) for one digit image.
    fn read(&self, img: &[f32]) -> (usize, usize) {
        let mut best = (f32::MIN, 0, 0);
        for (d, ps) in self.protos.iter().enumerate() {
            for (k, p) in ps.iter().enumerate() {
                for dy in -self.reach..=self.reach {
                    for dx in -self.reach..=self.reach {
                        let v = corr(img, p, dx, dy);
                        if v > best.0 {
                            best = (v, d, k);
                        }
                    }
                }
            }
        }
        (best.1, best.2)
    }
}

/// The test sacks: digit, hand and shift of every digit, fixed.
fn sacks() -> Vec<[(usize, bool, i32, i32); 6]> {
    let mut rng = oo_base::Rng::new(0x5047);
    (0..CODES)
        .map(|k| std::array::from_fn(|_| (rng.below(10) as usize, k % 2 == 1, rng.below(5) as i32 - 2, rng.below(3) as i32 - 1)))
        .collect()
}

static SORTER_CACHE: Cache<(bool, i32, i32, bool), Sorter> = Cache::new(Default::default);

fn sorter(conv: bool, pool: i32, channels: i32, augment: bool) -> Sorter {
    memo(&SORTER_CACHE, (conv, pool, channels, augment), || {
        let k = [1usize, 2, 4][channels as usize];
        let hands: Vec<Hand> = std::iter::once(PRINTED).chain(if augment { AUGMENT.to_vec() } else { Vec::new() }).collect();
        // Training digits are centred; each prototype averages the hands assigned to its channel.
        let protos = (0..10)
            .map(|d| {
                (0..k.min(hands.len()))
                    .map(|c| {
                        let mine: Vec<&Hand> = hands.iter().enumerate().filter(|(i, _)| i % k == c).map(|(_, h)| h).collect();
                        let mut p = vec![0.0; SIDE * SIDE];
                        for h in &mine {
                            for (a, b) in p.iter_mut().zip(write(d, **h, 0, 0)) {
                                *a += b / mine.len() as f32;
                            }
                        }
                        p
                    })
                    .collect()
            })
            .collect();
        let reach = if conv { pool.clamp(0, 2) } else { 0 };
        let mut s = Sorter { protos, reach, codes: Vec::new(), acc: 0.0, su_acc: 0.0 };
        let (mut right, mut su_right, mut su_n) = (0, 0, 0);
        for sack in sacks() {
            let mut read = [0; 6];
            for (j, &(d, su, dx, dy)) in sack.iter().enumerate() {
                read[j] = s.read(&write(d, if su { SU } else { PRINTED }, dx, dy)).0;
                right += (read[j] == d) as usize;
                if su {
                    su_n += 1;
                    su_right += (read[j] == d) as usize;
                }
            }
            s.codes.push((sack.map(|x| x.0), read, sack[0].1));
        }
        s.acc = right as f32 / (CODES * 6) as f32;
        s.su_acc = su_right as f32 / su_n.max(1) as f32;
        s
    })
}

fn sorter_of(s: &BenchState) -> Sorter {
    sorter(s.toggles[0], s.dials[0], s.dials[1], s.toggles[1])
}

fn code_text(d: &[usize; 6]) -> String {
    d.iter().map(|x| char::from(b'0' + *x as u8)).collect()
}

fn sorter_init(s: &mut BenchState) {
    s.picks = vec![1];
}

fn sorter_targets(s: &BenchState, _: Lang) -> Vec<Target> {
    let m = sorter_of(s);
    let shown = s.picks.last().copied().unwrap_or(1);
    m.codes[..SHOWN_CODES]
        .iter()
        .enumerate()
        .map(|(i, (code, read, _))| {
            let mark = if i == shown {
                Mark::Pos
            } else if code == read {
                Mark::Done
            } else {
                Mark::Hot
            };
            Target::chip(code_text(read), mark)
        })
        .collect()
}

fn sorter_click(s: &mut BenchState, i: usize) {
    if i < SHOWN_CODES {
        s.picks = vec![i];
    }
}

fn sorter_eval(s: &BenchState, lang: Lang) -> Eval {
    let m = sorter_of(s);
    let mut e = Eval::default();
    e.check(t(lang, "Digits read right (test sacks)", "读对的数字（测试邮袋）"), format!("{} ≥ {}", pct(m.acc), pct(DIGIT_TARGET)), m.acc >= DIGIT_TARGET);
    e.info(t(lang, "Su's handwriting", "苏婆的笔迹"), pct(m.su_acc));
    let params = m.protos.iter().map(|p| p.len()).sum::<usize>() * if s.toggles[0] { 35 } else { SIDE * SIDE };
    e.info(t(lang, "Parameters", "参数量"), params.to_string());

    // The first misread digit of the chosen code, or its first digit.
    let pick = s.picks.last().copied().unwrap_or(1).min(SHOWN_CODES - 1);
    let sack = sacks()[pick];
    let (code, read, _) = m.codes[pick];
    let j = (0..6).find(|&j| code[j] != read[j]).unwrap_or(0);
    let (d, su, dx, dy) = sack[j];
    let img = write(d, if su { SU } else { PRINTED }, dx, dy);
    let (got, k) = m.read(&img);
    e.images.push(Image {
        label: Tr::new("Digit {}: a {}, read as {}", "第 {} 位：{}，读成 {}").fill(lang, &[&(j + 1), &d, &got]),
        w: SIDE,
        px: img.iter().map(|v| (v + 1.0) / 2.0).collect(),
    });
    e.images.push(Image {
        label: Tr::new("The {}-detector, channel {}", "“{}”探测器，第 {} 个通道").fill(lang, &[&got, &(k + 1)]),
        w: SIDE,
        px: m.protos[got][k].clone(),
    });
    e.solved = m.acc >= DIGIT_TARGET;
    e
}

fn sorter_solve(s: &mut BenchState) {
    s.toggles = vec![true, true];
    s.dials = vec![2, 1];
}

static SORTER_POOLS: [Tr; 3] = [Tr::new("none", "不池化"), Tr::new("3×3 max", "3×3 最大"), Tr::new("5×5 max", "5×5 最大")];
static SORTER_CHANNELS: [Tr; 3] = [Tr::new("1 per digit", "每个数字 1 个"), Tr::new("2 per digit", "每个数字 2 个"), Tr::new("4 per digit", "每个数字 4 个")];
static SORTER_DIALS: [Dial; 2] = [
    Dial::named(Tr::new("Max pooling window", "最大池化窗口"), 0, &SORTER_POOLS),
    Dial::named(Tr::new("Channels", "通道数"), 0, &SORTER_CHANNELS),
];

pub static SORTER: Spec = Spec {
    layout: Layout::Row,
    dials: &SORTER_DIALS,
    toggles: &[
        Tr::new("Convolution: one kernel slides everywhere", "卷积：同一个核扫全图"),
        Tr::new("Augment: train on slanted and bold copies", "增强：用斜体、粗体副本训练"),
    ],
    fault: None,
    init: sorter_init,
    targets: sorter_targets,
    click: sorter_click,
    eval: sorter_eval,
    solve: sorter_solve,
};
// @@CHOIR@@

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn print_sorter() {
        for conv in [false, true] {
            for aug in [false, true] {
                let mut row = String::new();
                for pool in 0..3 {
                    for ch in 0..3 {
                        let m = sorter(conv, pool, ch, aug);
                        row += &format!(" p{pool}c{ch}:{:.0}/{:.0}", m.acc * 100.0, m.su_acc * 100.0);
                    }
                }
                println!("conv={conv} aug={aug}{row}");
            }
        }
    }

    #[test]
    #[ignore]
    fn print_clinic() {
        let test = clinic_data().1;
        for early in [false, true] {
            for w in 0..4 {
                let mut row = String::new();
                for l2 in 0..5 {
                    let m = clinic_model(w, l2, early);
                    // Fewest calls that still reach the recall target.
                    let best = (1..=25).rev().map(|k| clinic_scores(&m.net, &test, k as f32 * CALL_SCALE)).find(|r| r.0 >= CLINIC_RECALL && r.1 <= CALL_BUDGET).map(|r| r.1).unwrap_or(1.0);
                    let ks: Vec<i32> = (1..=25).filter(|&k| { let r = clinic_scores(&m.net, &test, k as f32 * CALL_SCALE); r.0 >= CLINIC_RECALL && r.1 <= CALL_BUDGET }).collect();
                    row += &format!(" {l2}:{:.0}%{ks:?}", best * 100.0);
                }
                println!("early={early} w={w}{row}");
            }
        }
    }

    #[test]
    #[ignore]
    fn print_fog() {
        for a in 0..3 {
            for h in 0..4 {
                let mut row = String::new();
                for r in 0..7 {
                    let s = BenchState { dials: vec![a, h, r], ..Default::default() };
                    let e = fog_eval(&s, Lang::En);
                    row += &format!(" {}:{}/{}", r, e.readouts[0].value.split(' ').next().unwrap(), e.readouts[1].value.split(' ').next().unwrap());
                }
                println!("a={a} h={h}{row}");
            }
        }
    }

    #[test]
    #[ignore]
    fn print_sail() {
        for sq in [false, true] {
            for h in 0..5 {
                let mut row = String::new();
                for thr in 10..20 {
                    let s = BenchState { dials: vec![h, thr], toggles: vec![sq], ..Default::default() };
                    let e = sail_eval(&s, Lang::En);
                    row += &format!(" {}:{}/{}", thr, e.readouts[0].value.split(' ').next().unwrap(), e.readouts[1].value.split(' ').next().unwrap());
                }
                println!("sq={sq} h={h}{row}");
            }
        }
    }
}

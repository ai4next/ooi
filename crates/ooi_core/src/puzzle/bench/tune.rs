//! 第 4、6、7 章的真模型：池化与共享权重（4-4）、温度与 top-p（6-2）、KL 约束（7-3）。

use oo_base::{Lang, Rng, Tr, t};

use super::{Bar, BenchState, Cache, Chart, Curve, Dial, Eval, Image, Layout, Mark, Spec, Target, memo, no_click, no_init, no_targets};

fn pct(v: f32) -> String {
    format!("{:.0}%", v * 100.0)
}

// ---------------------------------------------------------------- 4-4 位置不重要

/// Postcode digits, 5×7.
pub(super) const GLYPHS: [[&str; 7]; 10] = [
    ["01110", "10001", "10011", "10101", "11001", "10001", "01110"],
    ["00100", "01100", "00100", "00100", "00100", "00100", "01110"],
    ["01110", "10001", "00001", "00010", "00100", "01000", "11111"],
    ["11111", "00010", "00100", "00010", "00001", "10001", "01110"],
    ["00010", "00110", "01010", "10010", "11111", "00010", "00010"],
    ["11111", "10000", "11110", "00001", "00001", "10001", "01110"],
    ["00110", "01000", "10000", "11110", "10001", "10001", "01110"],
    ["11111", "00001", "00010", "00100", "01000", "01000", "01000"],
    ["01110", "10001", "10001", "01110", "10001", "10001", "01110"],
    ["01110", "10001", "10001", "01111", "00001", "00010", "01100"],
];
const SIDE: usize = 11;
/// Test digits are moved up to this many pixels each way.
pub const SHIFT: i32 = 2;
const CELLS: usize = ((2 * SHIFT + 1) * (2 * SHIFT + 1)) as usize;
pub const SHIFT_TARGET: f32 = 0.90;
/// Where a glyph can sit inside the canvas: dx in ±3, dy in ±2.
const DX: i32 = 3;
const DY: i32 = 2;
/// The glyph shown in the detector map.
const SHOWN: usize = 4;

/// Glyph `d` moved by (dx, dy) from the centre, on a −1 background; `noise` flips one pixel.
fn place(d: usize, dx: i32, dy: i32, noise: Option<&mut Rng>) -> Vec<f32> {
    let mut g = vec![-1.0; SIDE * SIDE];
    for (r, row) in GLYPHS[d].iter().enumerate() {
        for (c, ch) in row.bytes().enumerate() {
            if ch == b'1' {
                g[(r as i32 + DY + dy) as usize * SIDE + (c as i32 + DX + dx) as usize] = 1.0;
            }
        }
    }
    if let Some(rng) = noise {
        let i = rng.below(g.len() as u64) as usize;
        g[i] = -g[i];
    }
    g
}

/// The detector for glyph `d` applied at offset (dx, dy), normalised to 1 for a perfect hit.
fn fire(img: &[f32], d: usize, dx: i32, dy: i32) -> f32 {
    let kernel = place(d, dx, dy, None);
    img.iter().zip(&kernel).map(|(a, b)| a * b).sum::<f32>() / img.len() as f32
}

/// How far max pooling reaches from the centre, in pixels.
fn reach(pool: i32) -> i32 {
    pool.clamp(0, 3)
}

/// Unshared detectors only exist at the centre, where they were trained, so
/// pooling has nothing to pool over; a dense layer on top changes nothing.
fn classify(img: &[f32], shared: bool, pool: i32) -> usize {
    let r = if shared { reach(pool) } else { 0 };
    let score = |d: usize| {
        let mut best = f32::MIN;
        for dy in -r.min(DY)..=r.min(DY) {
            for dx in -r.min(DX)..=r.min(DX) {
                best = best.max(fire(img, d, dx, dy));
            }
        }
        best
    };
    (0..GLYPHS.len()).max_by(|&a, &b| score(a).total_cmp(&score(b))).unwrap()
}

static SHIFT_CACHE: Cache<(bool, i32), Vec<usize>> = Cache::new(Default::default);

/// Digits recognised (of 10) in each shift cell, row-major from (−2, −2).
fn shift_scores(shared: bool, pool: i32) -> Vec<usize> {
    memo(&SHIFT_CACHE, (shared, pool), || score_shifts(shared, pool))
}

fn score_shifts(shared: bool, pool: i32) -> Vec<usize> {
    let mut rng = Rng::new(0x5417);
    (0..CELLS)
        .map(|cell| {
            let (dx, dy) = cell_shift(cell);
            (0..GLYPHS.len()).filter(|&d| classify(&place(d, dx, dy, Some(&mut rng)), shared, pool) == d).count()
        })
        .collect()
}

fn cell_shift(cell: usize) -> (i32, i32) {
    let side = 2 * SHIFT + 1;
    (cell as i32 % side - SHIFT, cell as i32 / side - SHIFT)
}

fn pooling_init(s: &mut BenchState) {
    s.picks = vec![CELLS / 2];
}

fn pooling_targets(s: &BenchState, _: Lang) -> Vec<Target> {
    let picked = s.picks.last().copied();
    shift_scores(s.toggles[0], s.dials[0])
        .into_iter()
        .enumerate()
        .map(|(i, n)| {
            let mark = if picked == Some(i) {
                Mark::Pos
            } else if n == GLYPHS.len() {
                Mark::Done
            } else if n <= GLYPHS.len() / 2 {
                Mark::Hot
            } else {
                Mark::Neg
            };
            Target::chip(format!("{n}/10"), mark)
        })
        .collect()
}

/// Picking a cell only chooses which shift the detector map shows.
fn pooling_click(s: &mut BenchState, i: usize) {
    if i < CELLS {
        s.picks = vec![i];
    }
}

fn pooling_eval(s: &BenchState, lang: Lang) -> Eval {
    let (shared, dense, pool) = (s.toggles[0], s.toggles[1], s.dials[0]);
    let scores = shift_scores(shared, pool);
    let acc = scores.iter().sum::<usize>() as f32 / (CELLS * GLYPHS.len()) as f32;
    let centred = scores[CELLS / 2] as f32 / GLYPHS.len() as f32;
    let kernels = if shared { GLYPHS.len() * 35 } else { GLYPHS.len() * SIDE * SIDE };
    let params = kernels + if dense { GLYPHS.len() * (SIDE * SIDE + 1) } else { 0 };
    let mut e = Eval::default();
    e.info(t(lang, "Centred digits", "居中的数字"), pct(centred));
    e.check(
        t(lang, "Moved digits (test)", "挪过位置的数字（测试）"),
        format!("{} ≥ {}", pct(acc), pct(SHIFT_TARGET)),
        acc >= SHIFT_TARGET,
    );
    e.info(t(lang, "Parameters", "参数量"), params.to_string());

    let (sx, sy) = cell_shift(s.picks.last().copied().unwrap_or(CELLS / 2));
    let img = place(SHOWN, sx, sy, None);
    let r = if shared { reach(pool) } else { 0 };
    let mut map = Vec::with_capacity(35);
    for dy in -DY..=DY {
        for dx in -DX..=DX {
            // Cyan: the pool sees it. Amber: the detector fires there, but nothing reads it. Dark: no detector.
            let exists = shared || (dx == 0 && dy == 0);
            // A misaligned glyph still agrees on most background pixels; rescale so only a near hit glows.
            let v = if exists { ((fire(&img, SHOWN, dx, dy) - 0.55) / 0.45).clamp(0.0, 1.0) } else { 0.0 };
            let pooled = dx.abs() <= r && dy.abs() <= r;
            map.push(if pooled { v.max(0.12) } else { -v });
        }
    }
    e.images.push(Image { label: Tr::new("Input, moved ({}, {})", "输入，挪了 ({}, {})").fill(lang, &[&sx, &sy]), w: SIDE, px: img.iter().map(|v| (v + 1.0) / 2.0).collect() });
    e.images.push(Image { label: t(lang, "Where the 4-detector fires", "“4”探测器在哪里响").into(), w: 2 * DX as usize + 1, px: map });
    e.solved = acc >= SHIFT_TARGET;
    e
}

fn pooling_solve(s: &mut BenchState) {
    s.toggles[0] = true;
    s.dials[0] = 2;
}

static POOLS: [Tr; 4] = [
    Tr::new("none", "不池化"),
    Tr::new("3×3 max", "3×3 最大"),
    Tr::new("5×5 max", "5×5 最大"),
    Tr::new("whole image", "全图最大"),
];
static POOL_DIALS: [Dial; 1] = [Dial::named(Tr::new("Max pooling window", "最大池化窗口"), 0, &POOLS)];

pub static POOLING: Spec = Spec {
    layout: Layout::Grid(5),
    dials: &POOL_DIALS,
    toggles: &[
        Tr::new("Share weights: one kernel slides everywhere", "共享权重：同一个核扫全图"),
        Tr::new("Add a dense layer", "多加一层全连接"),
    ],
    fault: None,
    init: pooling_init,
    targets: pooling_targets,
    click: pooling_click,
    eval: pooling_eval,
    solve: pooling_solve,
};

// ---------------------------------------------------------------- 6-2 温度之计

/// A prompt with its next-word logits: the words that fit, then the junk tail.
struct Prompt {
    text: Tr,
    legal: &'static [(Tr, f32)],
    junk: &'static [f32],
}

static PROMPTS: [Prompt; 3] = [
    Prompt {
        text: Tr::new("The ferry leaves at …", "渡轮在……开"),
        legal: &[
            (Tr::new("dawn", "天亮时"), 3.0),
            (Tr::new("noon", "中午"), 2.2),
            (Tr::new("seven", "七点"), 1.6),
            (Tr::new("dusk", "傍晚"), 1.0),
            (Tr::new("high tide", "涨潮时"), 0.4),
        ],
        junk: &[0.2, 0.2, 0.2, -0.3, -0.3, -0.3, -0.3, -0.3, -0.3, -0.8, -0.8, -0.8, -0.8, -0.8, -0.8, -0.8, -0.8, -0.8, -0.8],
    },
    Prompt {
        text: Tr::new("Fog rolls over the …", "雾漫过了……"),
        legal: &[
            (Tr::new("harbour", "港口"), 2.6),
            (Tr::new("bay", "海湾"), 2.4),
            (Tr::new("pier", "码头"), 1.2),
            (Tr::new("island", "小岛"), 0.6),
        ],
        junk: &[0.0, 0.0, 0.0, 0.0, -0.5, -0.5, -0.5, -0.5, -0.5, -0.5, -0.5, -0.5, -0.5, -0.5, -0.5, -0.5],
    },
    Prompt {
        text: Tr::new("The keeper lights the …", "看守人点亮了……"),
        legal: &[
            (Tr::new("lamp", "灯"), 3.4),
            (Tr::new("beacon", "信标"), 1.4),
            (Tr::new("lantern", "马灯"), 1.2),
            (Tr::new("stove", "炉子"), 0.9),
            (Tr::new("wick", "灯芯"), 0.5),
            (Tr::new("pipe", "烟斗"), 0.2),
        ],
        junk: &[0.1, 0.1, 0.1, 0.1, 0.1, -0.6, -0.6, -0.6, -0.6, -0.6, -0.6, -0.6, -0.6, -0.6, -0.6],
    },
];
/// Words per sentence: every one has to fit.
const SENT_LEN: i32 = 6;
pub const LEGAL_TARGET: f32 = 0.95;
pub const DIVERSITY_TARGET: f32 = 2.3;
const T_SCALE: f32 = 0.1;
const P_SCALE: f32 = 0.05;

/// Softmax at `temp`, then keep the smallest top set reaching mass `top_p`, renormalised.
fn nucleus(p: &Prompt, temp: f32, top_p: f32) -> Vec<f32> {
    let logits: Vec<f32> = p.legal.iter().map(|w| w.1).chain(p.junk.iter().copied()).collect();
    let m = logits.iter().copied().fold(f32::MIN, f32::max);
    let e: Vec<f32> = logits.iter().map(|z| ((z - m) / temp).exp()).collect();
    let sum: f32 = e.iter().sum();
    let probs: Vec<f32> = e.iter().map(|x| x / sum).collect();
    let mut order: Vec<usize> = (0..probs.len()).collect();
    order.sort_by(|&a, &b| probs[b].total_cmp(&probs[a]));
    let mut keep = vec![false; probs.len()];
    let mut mass = 0.0;
    for i in order {
        keep[i] = true;
        mass += probs[i];
        if mass >= top_p - 1e-6 {
            break;
        }
    }
    let kept: f32 = probs.iter().zip(&keep).filter(|(_, k)| **k).map(|(p, _)| p).sum();
    probs.iter().zip(&keep).map(|(p, k)| if *k { p / kept } else { 0.0 }).collect()
}

/// (sentences with every word legal, effective number of legal choices).
fn sample_quality(temp: f32, top_p: f32) -> (f32, f32) {
    let (mut legal, mut div) = (0.0, 0.0);
    for p in &PROMPTS {
        let d = nucleus(p, temp, top_p);
        let n = p.legal.len();
        let mass: f32 = d[..n].iter().sum();
        legal += mass.powi(SENT_LEN);
        let entropy: f32 = d[..n].iter().filter(|&&x| x > 0.0).map(|&x| -(x / mass) * (x / mass).ln()).sum();
        div += entropy.exp();
    }
    (legal / PROMPTS.len() as f32, div / PROMPTS.len() as f32)
}

fn temp_values(s: &BenchState) -> (f32, f32) {
    (s.dials[0] as f32 * T_SCALE, s.dials[1] as f32 * P_SCALE)
}

fn temperature_init(s: &mut BenchState) {
    s.picks = vec![0];
}

fn temperature_targets(s: &BenchState, lang: Lang) -> Vec<Target> {
    let shown = s.picks.last().copied().unwrap_or(0);
    PROMPTS
        .iter()
        .enumerate()
        .map(|(i, p)| Target::chip(p.text.get(lang), if i == shown { Mark::Pos } else { Mark::Neutral }))
        .collect()
}

/// Picking a prompt chooses which distribution the bars show.
fn temperature_click(s: &mut BenchState, i: usize) {
    if i < PROMPTS.len() {
        s.picks = vec![i];
    }
}

fn temperature_eval(s: &BenchState, lang: Lang) -> Eval {
    let (temp, top_p) = temp_values(s);
    let (legal, div) = sample_quality(temp, top_p);
    let mut e = Eval::default();
    e.check(
        t(lang, "Sentences that make sense", "通顺的句子"),
        format!("{} ≥ {}", pct(legal), pct(LEGAL_TARGET)),
        legal >= LEGAL_TARGET,
    );
    e.check(
        t(lang, "Variety (effective choices)", "多样性（有效选择数）"),
        format!("{div:.2} ≥ {DIVERSITY_TARGET:.1}"),
        div >= DIVERSITY_TARGET,
    );
    let prompt = &PROMPTS[s.picks.last().copied().unwrap_or(0).min(PROMPTS.len() - 1)];
    let now = nucleus(prompt, temp, top_p);
    let base = nucleus(prompt, 1.0, 1.0);
    let n = prompt.legal.len();
    let kept = now.iter().filter(|&&x| x > 0.0).count();
    e.info(t(lang, "Words left in the pool", "候选池里的词"), format!("{kept} / {}", now.len()));
    let top = now.iter().copied().fold(0.0, f32::max).max(base.iter().copied().fold(0.0, f32::max));
    e.bars_title = Tr::new("Next word after “{}”", "“{}”之后的下一个词").fill(lang, &[&prompt.text.get(lang)]);
    for (i, (w, _)) in prompt.legal.iter().enumerate() {
        let mark = if now[i] > 0.0 { Mark::Pos } else { Mark::Neutral };
        e.bars.push(Bar { label: w.get(lang).into(), value: now[i] / top, ghost: Some(base[i] / top), text: pct(now[i]), mark });
    }
    let (junk, junk_base): (f32, f32) = (now[n..].iter().sum(), base[n..].iter().sum());
    e.bars.push(Bar {
        label: Tr::new("{} words that don't fit", "{} 个不通的词").fill(lang, &[&(now.len() - n)]),
        value: junk / top,
        ghost: Some(junk_base / top),
        text: pct(junk),
        mark: if junk > 0.0 { Mark::Hot } else { Mark::Neutral },
    });
    e.solved = legal >= LEGAL_TARGET && div >= DIVERSITY_TARGET;
    e
}

fn temperature_solve(s: &mut BenchState) {
    s.dials = vec![10, 14];
}

static TEMP_DIALS: [Dial; 2] = [
    Dial::num(Tr::new("Temperature T", "温度 T"), 1, 20, 10, T_SCALE, 1),
    Dial::num(Tr::new("Top-p (keep this much mass)", "Top-p（保留的概率质量）"), 10, 20, 20, P_SCALE, 2),
];

pub static TEMPERATURE: Spec = Spec {
    layout: Layout::Row,
    dials: &TEMP_DIALS,
    toggles: &[],
    fault: None,
    init: temperature_init,
    targets: temperature_targets,
    click: temperature_click,
    eval: temperature_eval,
    solve: temperature_solve,
};

// ---------------------------------------------------------------- 7-3 诚实的代价

/// Answers to "Is the 6:00 boat running?": (text, π₀, reward model, islanders' rating).
static ANSWERS: [(Tr, f32, f32, f32); 7] = [
    (Tr::new("The 5:10 is cancelled; take the 6:00.", "5:10 那班停了，坐 6:00 的吧。"), 0.15, 0.60, 0.90),
    (Tr::new("Not sure — the office posts changes at noon.", "不确定，码头办公室中午会贴变更。"), 0.10, 0.72, 0.95),
    (Tr::new("Great question!! Every boat is perfect today!!", "问得太好了！！今天每班船都完美！！"), 0.02, 1.00, 0.10),
    (Tr::new("Boats run.", "有船。"), 0.35, 0.20, 0.30),
    (Tr::new("Great question! The 6:00 runs as usual.", "好问题！6:00 照常开。"), 0.08, 0.85, 0.60),
    (Tr::new("wonderful wonderful helpful !!!", "太棒太棒 有帮助 ！！！"), 0.005, 0.97, 0.0),
    (Tr::new("Ask at the pier.", "去码头问。"), 0.295, 0.10, 0.20),
];
const BETA_MIN: i32 = -24;
const BETA_MAX: i32 = 8;
const BETA_SCALE: f32 = 0.125;
pub const HUMAN_TARGET: f32 = 0.57;
pub const PPL_MAX: f32 = 2.5;

fn beta(step: i32) -> f32 {
    10f32.powf(step as f32 * BETA_SCALE)
}

/// The KL-regularised optimum π ∝ π₀·exp(r/β).
fn policy(b: f32) -> Vec<f32> {
    let m = ANSWERS.iter().map(|a| a.2 / b).fold(f32::MIN, f32::max);
    let w: Vec<f32> = ANSWERS.iter().map(|a| a.1 * (a.2 / b - m).exp()).collect();
    let sum: f32 = w.iter().sum();
    w.iter().map(|x| x / sum).collect()
}

struct Tuned {
    pi: Vec<f32>,
    reward: f32,
    human: f32,
    /// exp(KL(π‖π₀)): how surprised the original model is by the new one.
    ppl: f32,
}

fn tune(step: i32) -> Tuned {
    let pi = policy(beta(step));
    let dot = |f: fn(&(Tr, f32, f32, f32)) -> f32| pi.iter().zip(&ANSWERS).map(|(p, a)| p * f(a)).sum::<f32>();
    let kl: f32 = pi.iter().zip(&ANSWERS).filter(|(p, _)| **p > 0.0).map(|(p, a)| p * (p / a.1).ln()).sum();
    Tuned { reward: dot(|a| a.2), human: dot(|a| a.3), ppl: kl.exp(), pi }
}

fn beta_x(step: i32) -> f32 {
    (step - BETA_MIN) as f32 / (BETA_MAX - BETA_MIN) as f32
}

fn kl_eval(s: &BenchState, lang: Lang) -> Eval {
    let step = s.dials[0];
    let now = tune(step);
    let mut e = Eval::default();
    e.info(t(lang, "β", "β"), format!("{:.3}", beta(step)));
    e.info(t(lang, "Reward model score", "奖励模型打分"), format!("{:.2}", now.reward));
    e.check(t(lang, "Islanders' rating", "岛民真人评分"), format!("{:.2} ≥ {HUMAN_TARGET:.2}", now.human), now.human >= HUMAN_TARGET);
    e.check(t(lang, "Perplexity vs. the original", "相对原模型的困惑度"), format!("{:.2} ≤ {PPL_MAX:.1}", now.ppl), now.ppl <= PPL_MAX);

    e.bars_title = t(lang, "How often it gives each answer (tick = original model)", "它给出每种回答的频率（刻度 = 原模型）").into();
    let top = now.pi.iter().chain(ANSWERS.iter().map(|a| &a.1)).copied().fold(0.0, f32::max);
    for (p, a) in now.pi.iter().zip(&ANSWERS) {
        let mark = if a.3 >= 0.6 { Mark::Pos } else if a.2 > a.3 + 0.5 { Mark::Hot } else { Mark::Neutral };
        e.bars.push(Bar { label: a.0.get(lang).into(), value: p / top, ghost: Some(a.1 / top), text: pct(*p), mark });
    }

    let steps: Vec<i32> = (BETA_MIN..=BETA_MAX).collect();
    let all: Vec<Tuned> = steps.iter().map(|&k| tune(k)).collect();
    let line = |f: fn(&Tuned) -> f32| steps.iter().zip(&all).map(|(&k, r)| (beta_x(k), f(r))).collect::<Vec<_>>();
    e.chart = Some(Chart {
        title: t(lang, "Score vs. constraint", "评分 vs. 约束").into(),
        x_label: t(lang, "KL coefficient lg β", "KL 系数 lg β").into(),
        curves: vec![
            Curve::new(t(lang, "Reward model", "奖励模型"), line(|r| r.reward), Mark::Neg).dashed(),
            Curve::new(t(lang, "Islanders", "岛民"), line(|r| r.human), Mark::Pos),
        ],
        cursor: Some(beta_x(step)),
        limit: Some(HUMAN_TARGET),
    });
    e.solved = now.human >= HUMAN_TARGET && now.ppl <= PPL_MAX;
    e
}

fn kl_solve(s: &mut BenchState) {
    s.dials[0] = -4;
}

static KL_DIALS: [Dial; 1] = [Dial::num(Tr::new("KL coefficient lg β", "KL 系数 lg β"), BETA_MIN, BETA_MAX, BETA_MIN, BETA_SCALE, 2)];

pub static KL: Spec = Spec {
    layout: Layout::None,
    dials: &KL_DIALS,
    toggles: &[],
    fault: None,
    init: no_init,
    targets: no_targets,
    click: no_click,
    eval: kl_eval,
    solve: kl_solve,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pooling_needs_shared_weights_and_a_window() {
        let acc = |shared, pool| shift_scores(shared, pool).iter().sum::<usize>() as f32 / (CELLS * 10) as f32;
        assert!(acc(false, 3) < 0.3, "no sharing: {}", acc(false, 3));
        assert!(acc(true, 0) < 0.3, "no pooling: {}", acc(true, 0));
        assert!(acc(true, 2) >= SHIFT_TARGET, "pool 5×5: {}", acc(true, 2));
        assert!(shift_scores(false, 0)[CELLS / 2] >= 9, "centred digits are easy");
    }

    #[test]
    fn colder_is_not_better() {
        let (legal, div) = sample_quality(0.3, 1.0);
        assert!(legal >= LEGAL_TARGET && div < DIVERSITY_TARGET);
        assert!((10..=20).all(|t| {
            let (l, d) = sample_quality(t as f32 * T_SCALE, 1.0);
            !(l >= LEGAL_TARGET && d >= DIVERSITY_TARGET)
        }));
    }

    #[test]
    fn unconstrained_reward_hacks() {
        let free = tune(BETA_MIN);
        assert!(free.reward > 0.95 && free.human < 0.2 && free.ppl > 20.0);
        let ok: Vec<i32> = (BETA_MIN..=BETA_MAX).filter(|&k| {
            let r = tune(k);
            r.human >= HUMAN_TARGET && r.ppl <= PPL_MAX
        }).collect();
        assert!((2..=5).contains(&ok.len()), "{ok:?}");
    }
}

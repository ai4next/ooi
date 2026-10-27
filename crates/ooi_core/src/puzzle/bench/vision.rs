//! 第 4 章：通道（4-2）、显著图与捷径特征（4-5 故障）。

use std::sync::OnceLock;

use oo_base::{Lang, Rng, Tr, t};

use super::{BenchState, Dial, Eval, Image, Layout, Mark, Spec, Target, no_click, no_init, no_targets};

// ---------------------------------------------------------------- 4-2 通道

const DIGITS: [(&str, [&str; 7]); 3] = [
    ("1", ["...#...", "..##...", ".#.#...", "...#...", "...#...", "...#...", "...#..."]),
    ("7", ["#######", ".....#.", "....#..", "...#...", "..#....", "..#....", "..#...."]),
    ("4", ["#...#..", "#...#..", "#...#..", "#######", "....#..", "....#..", "....#.."]),
];

/// 3×3 line detectors: cells on the line weigh +1, the rest −½.
const DETECTORS: [[u8; 9]; 4] = [
    [0, 1, 0, 0, 1, 0, 0, 1, 0],
    [0, 0, 0, 1, 1, 1, 0, 0, 0],
    [1, 0, 0, 0, 1, 0, 0, 0, 1],
    [0, 0, 1, 0, 1, 0, 1, 0, 0],
];
const FIRE: f32 = 2.5;

static DETECTOR_NAMES: [Tr; 5] = [
    Tr::new("off", "关"),
    Tr::new("vertical |", "竖 |"),
    Tr::new("horizontal —", "横 —"),
    Tr::new("diagonal \\", "斜 \\"),
    Tr::new("diagonal /", "斜 /"),
];

static CHANNEL_DIALS: [Dial; 3] = [
    Dial::named(Tr::new("Channel 1", "通道 1"), 0, &DETECTOR_NAMES),
    Dial::named(Tr::new("Channel 2", "通道 2"), 0, &DETECTOR_NAMES),
    Dial::named(Tr::new("Channel 3", "通道 3"), 0, &DETECTOR_NAMES),
];

fn pixel(rows: &[&str; 7], r: i32, c: i32) -> f32 {
    if !(0..7).contains(&r) || !(0..7).contains(&c) {
        return 0.0;
    }
    if rows[r as usize].as_bytes()[c as usize] == b'#' { 1.0 } else { 0.0 }
}

/// Feature map of one detector over a zero-padded 7×7 digit.
fn feature_map(rows: &[&str; 7], det: &[u8; 9]) -> Vec<f32> {
    let mut out = Vec::with_capacity(49);
    for r in 0..7 {
        for c in 0..7 {
            let mut sum = 0.0;
            for (k, &d) in det.iter().enumerate() {
                let w = if d == 1 { 1.0 } else { -0.5 };
                sum += w * pixel(rows, r + k as i32 / 3 - 1, c + k as i32 % 3 - 1);
            }
            out.push(sum);
        }
    }
    out
}

fn fires(rows: &[&str; 7], detector: i32) -> bool {
    detector > 0 && feature_map(rows, &DETECTORS[detector as usize - 1]).iter().any(|&v| v >= FIRE)
}

fn codes(dials: &[i32]) -> Vec<Vec<bool>> {
    DIGITS
        .iter()
        .map(|(_, rows)| dials.iter().map(|&d| fires(rows, d)).collect())
        .collect()
}

fn channels_used(dials: &[i32]) -> usize {
    dials.iter().filter(|&&d| d > 0).count()
}

fn unique(codes: &[Vec<bool>]) -> bool {
    (0..codes.len()).all(|i| (i + 1..codes.len()).all(|j| codes[i] != codes[j]))
}

fn channels_eval(s: &BenchState, lang: Lang) -> Eval {
    let mut e = Eval::default();
    let codes = codes(&s.dials);
    for ((name, rows), code) in DIGITS.iter().zip(&codes) {
        let bits = code.iter().map(|&b| if b { "●" } else { "○" }).collect::<Vec<_>>().join(" ");
        e.info(&Tr::new("Digit {}", "数字 {}").fill(lang, &[name]), bits);
        let mut px: Vec<f32> = (0..49).map(|i| pixel(rows, i / 7, i % 7) * 0.4).collect();
        for &d in s.dials.iter().filter(|&&d| d > 0) {
            for (p, v) in px.iter_mut().zip(feature_map(rows, &DETECTORS[d as usize - 1])) {
                if v >= FIRE {
                    *p = 1.0;
                }
            }
        }
        e.images.push(Image { label: (*name).into(), w: 7, px });
    }
    let ok = unique(&codes);
    e.check(t(lang, "1 / 7 / 4 told apart", "区分 1 / 7 / 4"), if ok { t(lang, "yes", "是") } else { t(lang, "no", "否") }, ok);
    e.info(t(lang, "Channels used", "使用通道"), format!("{} / 3", channels_used(&s.dials)));
    e.solved = ok;
    e
}

fn channels_solve(s: &mut BenchState) {
    let mut best: Option<Vec<i32>> = None;
    for n in 0..125 {
        let dials = vec![n % 5, n / 5 % 5, n / 25];
        if unique(&codes(&dials)) && best.as_ref().is_none_or(|b| channels_used(&dials) < channels_used(b)) {
            best = Some(dials);
        }
    }
    s.dials = best.expect("channels puzzle has a solution");
}

pub static CHANNELS: Spec = Spec {
    layout: Layout::None,
    dials: &CHANNEL_DIALS,
    toggles: &[],
    fault: None,
    init: no_init,
    targets: no_targets,
    click: no_click,
    eval: channels_eval,
    solve: channels_solve,
};

// ---------------------------------------------------------------- 4-5 显著图

const SIDE: usize = 8;
const PIXELS: usize = SIDE * SIDE;
/// 2×2 postmark corners: top-left marks district A, top-right district B.
const CORNERS: [[usize; 4]; 2] = [[0, 1, 8, 9], [6, 7, 14, 15]];

struct Envelope {
    px: Vec<f32>,
    label: f32,
}

/// District A has a vertical stroke, B a horizontal one; both jittered and noisy.
fn envelope(rng: &mut Rng, a: bool, corner: usize) -> Envelope {
    let mut px = vec![0.0; PIXELS];
    let at = 3 + rng.below(2) as usize;
    for k in 2..8 {
        let i = if a { k * SIDE + at } else { at * SIDE + k - 1 };
        if !rng.chance(0.25) {
            px[i] = 1.0;
        }
    }
    for p in px.iter_mut().skip(2 * SIDE) {
        if rng.chance(0.06) {
            *p = 1.0;
        }
    }
    for &i in &CORNERS[corner] {
        px[i] = 1.0;
    }
    Envelope { px, label: if a { 1.0 } else { 0.0 } }
}

fn dataset(seed: u64, n: usize, corner: impl Fn(&mut Rng, bool) -> usize) -> Vec<Envelope> {
    let mut rng = Rng::new(seed);
    (0..n)
        .map(|i| {
            let a = i % 2 == 0;
            let c = corner(&mut rng, a);
            envelope(&mut rng, a, c)
        })
        .collect()
}

fn sigmoid(z: f32) -> f32 {
    1.0 / (1.0 + (-z).exp())
}

struct Model {
    w: Vec<f32>,
    b: f32,
}

impl Model {
    fn p(&self, px: &[f32]) -> f32 {
        sigmoid(self.w.iter().zip(px).map(|(w, x)| w * x).sum::<f32>() + self.b)
    }

    fn accuracy(&self, data: &[Envelope]) -> f32 {
        let right = data.iter().filter(|e| (self.p(&e.px) >= 0.5) == (e.label > 0.5)).count();
        right as f32 / data.len() as f32
    }
}

/// Logistic regression, full-batch gradient descent, fixed order.
fn train(data: &[Envelope]) -> Model {
    let mut m = Model { w: vec![0.0; PIXELS], b: 0.0 };
    let lr = 1.0 / data.len() as f32;
    for _ in 0..800 {
        let mut gw = vec![0.0; PIXELS];
        let mut gb = 0.0;
        for e in data {
            let d = m.p(&e.px) - e.label;
            for (g, x) in gw.iter_mut().zip(&e.px) {
                *g += d * x;
            }
            gb += d;
        }
        for (w, g) in m.w.iter_mut().zip(gw) {
            *w -= lr * g + 1e-3 * *w;
        }
        m.b -= lr * gb;
    }
    m
}

fn shortcut(_: &mut Rng, a: bool) -> usize {
    if a { 0 } else { 1 }
}
fn swapped(_: &mut Rng, a: bool) -> usize {
    if a { 1 } else { 0 }
}
fn shuffled(rng: &mut Rng, _: bool) -> usize {
    rng.below(2) as usize
}

struct Sorter {
    /// [Kessler's, augmented].
    models: [Model; 2],
    valid: Vec<Envelope>,
    test: Vec<Envelope>,
    sample: Envelope,
}

fn sorter() -> &'static Sorter {
    static S: OnceLock<Sorter> = OnceLock::new();
    S.get_or_init(|| Sorter {
        models: [train(&dataset(41, 120, shortcut)), train(&dataset(41, 120, shuffled))],
        valid: dataset(42, 80, shortcut),
        test: dataset(43, 80, swapped),
        sample: envelope(&mut Rng::new(44), true, 0),
    })
}

/// |w·x| on the sample envelope, normalised to 0..1.
fn saliency(m: &Model, px: &[f32]) -> Vec<f32> {
    let raw: Vec<f32> = m.w.iter().zip(px).map(|(w, x)| (w * x).abs()).collect();
    let max = raw.iter().cloned().fold(1e-6, f32::max);
    raw.iter().map(|v| v / max).collect()
}

fn saliency_targets(s: &BenchState, _: Lang) -> Vec<Target> {
    let sample = &sorter().sample;
    (0..PIXELS)
        .map(|i| {
            let mark = if s.picks.contains(&i) {
                if CORNERS[0].contains(&i) { Mark::Hot } else { Mark::Done }
            } else if sample.px[i] > 0.0 {
                Mark::Pos
            } else {
                Mark::Neutral
            };
            Target::at((i % SIDE) as f32, (i / SIDE) as f32, mark)
        })
        .collect()
}

fn saliency_click(s: &mut BenchState, i: usize) {
    if s.evidence || i >= PIXELS {
        return;
    }
    s.picks.push(i);
    if CORNERS[0].contains(&i) {
        s.evidence = true;
    } else {
        s.wrong += 1;
    }
}

fn saliency_eval(s: &BenchState, lang: Lang) -> Eval {
    let sorter = sorter();
    let model = &sorter.models[s.toggles[0] as usize];
    let mut e = Eval::default();
    let valid = model.accuracy(&sorter.valid);
    let test = model.accuracy(&sorter.test);
    e.info(t(lang, "Validation (same post office)", "验证（同一邮局）"), format!("{:.0}%", valid * 100.0));
    e.check(t(lang, "Test (postmarks swapped)", "测试（换了邮戳）"), format!("{:.0}%", test * 100.0), test >= 0.9);
    let found = if s.evidence { t(lang, "found", "已找到") } else { t(lang, "not yet", "未找到") };
    e.check(t(lang, "Evidence", "证据"), found, s.evidence);
    e.images.push(Image {
        label: t(lang, "saliency", "显著图").into(),
        w: SIDE,
        px: saliency(model, &sorter.sample.px),
    });
    e.solved = s.evidence && s.toggles[0] && test >= 0.9;
    e
}

fn saliency_solve(s: &mut BenchState) {
    saliency_click(s, CORNERS[0][0]);
    s.toggles[0] = true;
}

pub static SALIENCY: Spec = Spec {
    layout: Layout::Grid(SIDE),
    dials: &[],
    toggles: &[Tr::new("Augment: move the postmark around", "增强：随机挪动邮戳")],
    fault: Some(Tr::new(
        "Shortcut feature: it reads the postmark corner, not the handwriting.",
        "捷径特征：它看的是邮戳的位置，不是笔迹。",
    )),
    init: no_init,
    targets: saliency_targets,
    click: saliency_click,
    eval: saliency_eval,
    solve: saliency_solve,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_channel_is_not_enough() {
        for d in 0..5 {
            assert!(!unique(&codes(&[d, 0, 0])));
        }
        let mut s = BenchState::default();
        channels_solve(&mut s);
        assert_eq!(channels_used(&s.dials), 2);
    }

    #[test]
    fn shortcut_model_looks_great_then_fails() {
        let sorter = sorter();
        let kessler = &sorter.models[0];
        assert!(kessler.accuracy(&sorter.valid) >= 0.97, "valid {}", kessler.accuracy(&sorter.valid));
        assert!(kessler.accuracy(&sorter.test) < 0.6, "test {}", kessler.accuracy(&sorter.test));
        assert!(sorter.models[1].accuracy(&sorter.test) >= 0.9, "aug {}", sorter.models[1].accuracy(&sorter.test));
    }

    #[test]
    fn the_postmark_is_the_brightest_spot() {
        let sorter = sorter();
        let sal = saliency(&sorter.models[0], &sorter.sample.px);
        let top = (0..PIXELS).max_by(|&a, &b| sal[a].total_cmp(&sal[b])).unwrap();
        assert!(CORNERS[0].contains(&top), "top salient pixel {top}");
    }
}

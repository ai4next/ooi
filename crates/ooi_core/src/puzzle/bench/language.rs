//! 第 5–6 章：位置（5-5 故障）、逐字采样（6-1）、幻觉（6-5 故障）、
//! 上下文示例（6-3）、校准（6-4）。

use oo_base::{Lang, Rng, Tr, t};

use super::{BenchState, Dial, Eval, Layout, Mark, Spec, Target, no_click, no_init, no_targets};

fn yes_no(lang: Lang, ok: bool) -> &'static str {
    if ok { t(lang, "yes", "是") } else { t(lang, "no", "否") }
}

// ---------------------------------------------------------------- 5-5 位置

static ANIMALS: [Tr; 4] = [
    Tr::new("cat", "猫"),
    Tr::new("dog", "狗"),
    Tr::new("gull", "海鸥"),
    Tr::new("crab", "螃蟹"),
];
/// (chaser, chased).
const CHASES: [(usize, usize); 4] = [(0, 1), (1, 0), (2, 3), (3, 2)];
/// How much the bag-of-words model likes each animal as "the one being chased".
const BIAS: [f32; 4] = [0.3, 0.2, 0.1, 0.4];
/// Positional term for slot 0 (subject) and slot 2 (object).
const SLOT: [f32; 2] = [-1.0, 2.0];

/// "Who is being chased?" — scores each animal; position only counts when encoded.
fn answer(sentence: (usize, usize), positional: bool) -> usize {
    let score = |a: usize, slot: usize| BIAS[a] + if positional { SLOT[slot] } else { 0.0 };
    if score(sentence.0, 0) >= score(sentence.1, 1) { sentence.0 } else { sentence.1 }
}

fn order_targets(s: &BenchState, lang: Lang) -> Vec<Target> {
    CHASES
        .iter()
        .enumerate()
        .map(|(i, &(a, b))| {
            let said = answer((a, b), s.toggles[0]);
            let text = Tr::new("{} chases {} → {}", "{}追{} → {}")
                .fill(lang, &[&ANIMALS[a].get(lang), &ANIMALS[b].get(lang), &ANIMALS[said].get(lang)]);
            let mark = if !s.picks.contains(&i) {
                Mark::Neutral
            } else if said == b {
                Mark::Done
            } else {
                Mark::Hot
            };
            Target::chip(text, mark)
        })
        .collect()
}

fn order_click(s: &mut BenchState, i: usize) {
    let Some(&(a, b)) = CHASES.get(i) else { return };
    if s.evidence || s.picks.contains(&i) {
        return;
    }
    s.picks.push(i);
    if answer((a, b), false) != b {
        s.evidence = true;
    } else {
        s.wrong += 1;
    }
}

fn order_eval(s: &BenchState, lang: Lang) -> Eval {
    let mut e = Eval::default();
    let right = CHASES.iter().filter(|&&(a, b)| answer((a, b), s.toggles[0]) == b).count();
    e.info(t(lang, "Question", "问题"), t(lang, "Who is being chased?", "谁被追？"));
    e.check(t(lang, "Correct answers", "答对"), format!("{right} / {}", CHASES.len()), right == CHASES.len());
    e.check(t(lang, "Evidence", "证据"), yes_no(lang, s.evidence), s.evidence);
    e.solved = s.evidence && right == CHASES.len();
    e
}

fn order_solve(s: &mut BenchState) {
    let wrong = (0..CHASES.len()).find(|&i| answer(CHASES[i], false) != CHASES[i].1).unwrap();
    order_click(s, wrong);
    s.toggles[0] = true;
}

pub static ORDER: Spec = Spec {
    layout: Layout::Row,
    dials: &[],
    toggles: &[Tr::new("Add positional encoding", "加上位置编码")],
    fault: Some(Tr::new(
        "Missing position: without it, 'cat chases dog' and 'dog chases cat' look the same.",
        "位置缺失：没有位置，“猫追狗”和“狗追猫”看起来一模一样。",
    )),
    init: no_init,
    targets: order_targets,
    click: order_click,
    eval: order_eval,
    solve: order_solve,
};

// ---------------------------------------------------------------- 6-1 逐字采样

static WORDS: [Tr; 8] = [
    Tr::new("fog", "雾"),
    Tr::new("rolls in", "漫上来"),
    Tr::new("and", "然后"),
    Tr::new("the horn", "雾号"),
    Tr::new("sounds", "响了"),
    Tr::new(".", "。"),
    Tr::new("the boats", "船"),
    Tr::new("stay home", "留在港里"),
];
const END: usize = 5;
const START: usize = 8;
pub const SENTENCES: usize = 5;
const MAX_LEN: usize = 8;

/// Bigram table: row = previous word (8 = sentence start).
fn next(prev: usize) -> &'static [(usize, f32)] {
    match prev {
        0 => &[(1, 1.0)],
        1 => &[(2, 0.6), (END, 0.4)],
        2 => &[(0, 0.5), (3, 0.3), (6, 0.2)],
        3 => &[(4, 1.0)],
        4 => &[(2, 0.55), (END, 0.45)],
        6 => &[(7, 1.0)],
        7 => &[(END, 0.7), (2, 0.3)],
        _ => &[(0, 0.5), (3, 0.2), (6, 0.3)],
    }
}

fn join(lang: Lang, words: &[usize]) -> String {
    let mut out = String::new();
    for &w in words {
        if matches!(lang, Lang::En | Lang::Ko) && !out.is_empty() && w != END {
            out.push(' ');
        }
        out.push_str(WORDS[w].get(lang));
    }
    out
}

/// Splits picks into finished sentences + the one in progress.
fn sentences(picks: &[usize]) -> (Vec<Vec<usize>>, Vec<usize>) {
    let mut done = Vec::new();
    let mut cur = Vec::new();
    for &w in picks {
        cur.push(w);
        if w == END {
            done.push(std::mem::take(&mut cur));
        }
    }
    (done, cur)
}

fn loops(sentence: &[usize]) -> bool {
    let pairs: Vec<_> = sentence.windows(2).collect();
    (0..pairs.len()).any(|i| pairs[i + 1..].contains(&pairs[i]))
}

fn sampling_targets(s: &BenchState, lang: Lang) -> Vec<Target> {
    let prev = s.picks.last().copied().filter(|&w| w != END).unwrap_or(START);
    next(prev)
        .iter()
        .map(|&(w, p)| Target::chip(format!("{}  {:.2}", WORDS[w].get(lang), p), Mark::Neutral))
        .collect()
}

fn sampling_click(s: &mut BenchState, i: usize) {
    let prev = s.picks.last().copied().filter(|&w| w != END).unwrap_or(START);
    let Some(&(w, _)) = next(prev).get(i) else { return };
    let (done, cur) = sentences(&s.picks);
    if done.len() >= SENTENCES {
        return;
    }
    s.picks.push(w);
    if cur.len() + 1 >= MAX_LEN && w != END {
        // Ran on too long: drop the sentence.
        s.picks.truncate(s.picks.len() - cur.len() - 1);
        s.wrong += 1;
    }
}

fn greedy() -> Vec<usize> {
    let mut out = Vec::new();
    let mut prev = START;
    for _ in 0..9 {
        let (w, _) = next(prev).iter().copied().fold((END, -1.0), |a, b| if b.1 > a.1 { b } else { a });
        out.push(w);
        prev = w;
    }
    out
}

fn sampling_eval(s: &BenchState, lang: Lang) -> Eval {
    let mut e = Eval::default();
    let (done, cur) = sentences(&s.picks);
    e.info(t(lang, "Greedy would say", "贪心会说"), format!("{} …", join(lang, &greedy())));
    for (i, sen) in done.iter().enumerate() {
        let ok = !loops(sen) && !done[..i].contains(sen);
        e.check(&format!("{}", i + 1), join(lang, sen), ok);
    }
    if !cur.is_empty() {
        e.info(t(lang, "Writing", "正在写"), join(lang, &cur));
    }
    let good = done.iter().enumerate().filter(|(i, sen)| !loops(sen) && !done[..*i].contains(sen)).count();
    e.check(t(lang, "Distinct, loop-free sentences", "不重复、不打转的句子"), format!("{good} / {SENTENCES}"), good >= SENTENCES);
    e.solved = good >= SENTENCES;
    e
}

fn sampling_solve(s: &mut BenchState) {
    s.picks = [
        vec![0, 1, END],
        vec![3, 4, END],
        vec![6, 7, END],
        vec![0, 1, 2, 3, 4, END],
        vec![3, 4, 2, 6, 7, END],
    ]
    .concat();
}

pub static SAMPLING: Spec = Spec {
    layout: Layout::Row,
    dials: &[],
    toggles: &[],
    fault: None,
    init: no_init,
    targets: sampling_targets,
    click: sampling_click,
    eval: sampling_eval,
    solve: sampling_solve,
};

// ---------------------------------------------------------------- 6-5 幻觉

/// The answer it gave about the last ferry, word by word; `true` = made up.
static CLAIM: [(Tr, bool); 5] = [
    (Tr::new("The last ferry", "末班渡轮"), false),
    (Tr::new("leaves at", "在"), false),
    (Tr::new("6:40", "6:40"), true),
    (Tr::new("from Pier 3", "从三号码头"), true),
    (Tr::new("tonight.", "出发。"), false),
];
const FIRST_MADE_UP: usize = 2;
/// (question asked, is it in the timetable?)
const QUESTIONS: usize = 6;
const OFF_TABLE: usize = 2;

fn hallucination_targets(s: &BenchState, lang: Lang) -> Vec<Target> {
    CLAIM
        .iter()
        .enumerate()
        .map(|(i, (w, _))| {
            let mark = match (s.picks.contains(&i), i == FIRST_MADE_UP) {
                (true, true) => Mark::Hot,
                (true, false) => Mark::Done,
                _ => Mark::Neutral,
            };
            Target::chip(w.get(lang), mark)
        })
        .collect()
}

fn hallucination_click(s: &mut BenchState, i: usize) {
    if s.evidence || i >= CLAIM.len() || s.picks.contains(&i) {
        return;
    }
    s.picks.push(i);
    if i == FIRST_MADE_UP {
        s.evidence = true;
    } else {
        s.wrong += 1;
    }
}

fn hallucination_eval(s: &BenchState, lang: Lang) -> Eval {
    let (cool, retrieve) = (s.toggles[0], s.toggles[1]);
    // Without the timetable it guesses every time detail it can't recall.
    let wrong = if retrieve { 0 } else { 3 };
    let confidence = match (cool, retrieve) {
        (_, true) => 0.97,
        (true, false) => 0.96,
        (false, false) => 0.90,
    };
    let mut e = Eval::default();
    e.info(t(lang, "Confidence", "置信度"), format!("{confidence:.2}"));
    e.check(t(lang, "Made-up answers", "编造的回答"), format!("{wrong} / {QUESTIONS}"), wrong == 0);
    let refused = if retrieve { OFF_TABLE } else { 0 };
    e.info(t(lang, "Replied 'not in the timetable'", "回答“船期表里没有”"), format!("{refused} / {QUESTIONS}"));
    e.check(t(lang, "Evidence", "证据"), yes_no(lang, s.evidence), s.evidence);
    e.solved = s.evidence && wrong == 0;
    e
}

fn hallucination_solve(s: &mut BenchState) {
    hallucination_click(s, FIRST_MADE_UP);
    s.toggles[1] = true;
}

pub static HALLUCINATION: Spec = Spec {
    layout: Layout::Row,
    dials: &[],
    toggles: &[
        Tr::new("Lower the temperature", "调低温度"),
        Tr::new("Retrieve the harbour timetable", "检索港口船期表"),
    ],
    fault: Some(Tr::new(
        "Hallucination: no ferry leaves at 6:40, but it said so at 0.90 confidence.",
        "幻觉：根本没有 6:40 的渡轮，它却以 0.90 的置信度说了出来。",
    )),
    init: no_init,
    targets: hallucination_targets,
    click: hallucination_click,
    eval: hallucination_eval,
    solve: hallucination_solve,
};

// ---------------------------------------------------------------- 6-3 上下文

static EXAMPLE_LABELS: [Tr; 5] = [
    Tr::new("First ferry? → 07:10", "头班渡轮？→ 07:10"),
    Tr::new("Low tide? → about half past two", "几点退潮？→ 两点半左右"),
    Tr::new("Last ferry? → 17:30", "末班渡轮？→ 17:30"),
    Tr::new("Mail boat? → 11:20", "邮船？→ 11:20"),
    Tr::new("Fog horn test? → sometime around noon", "雾号试鸣？→ 中午前后吧"),
];
/// Which examples use the HH:MM format.
const HHMM: [bool; 5] = [true, false, true, true, false];
pub const ASKED: usize = 20;

/// How many of the 20 answers come out as HH:MM.
fn formatted(toggles: &[bool]) -> usize {
    let used = HHMM.iter().zip(toggles).filter(|(_, on)| **on);
    let (clock, prose) = used.fold((0, 0), |(c, p), (hhmm, _)| if *hhmm { (c + 1, p) } else { (c, p + 1) });
    match (clock, prose) {
        (0, _) => 0,
        (c, 0) => (12 + 8 * (c - 1)).min(ASKED),
        (c, p) => (ASKED * c / (c + p)).saturating_sub(2),
    }
}

fn context_init(s: &mut BenchState) {
    s.toggles[0] = true;
    s.toggles[1] = true;
}

fn context_eval(s: &BenchState, lang: Lang) -> Eval {
    let mut e = Eval::default();
    let n = formatted(&s.toggles);
    let shown = s.toggles.iter().filter(|&&on| on).count();
    let sample = if n == ASKED { "08:15" } else if n == 0 { t(lang, "the second one, I think", "大概第二班吧") } else { t(lang, "8:15-ish", "八点一刻左右") };
    e.info(t(lang, "Q: Third ferry?", "问：第三班渡轮？"), sample);
    e.check(t(lang, "Answers in HH:MM", "HH:MM 格式的回答"), format!("{n} / {ASKED}"), n == ASKED);
    e.info(t(lang, "Examples in the prompt", "提示里的示例"), shown.to_string());
    e.solved = n == ASKED;
    e
}

fn context_solve(s: &mut BenchState) {
    s.toggles = vec![true, false, true, false, false];
}

pub static CONTEXT: Spec = Spec {
    layout: Layout::None,
    dials: &[],
    toggles: &EXAMPLE_LABELS,
    fault: None,
    init: context_init,
    targets: no_targets,
    click: no_click,
    eval: context_eval,
    solve: context_solve,
};

// ---------------------------------------------------------------- 6-4 校准

const CAL_N: usize = 200;
const BINS: usize = 5;
pub const ECE_MAX: f32 = 0.03;
pub const WRONG_MAX: f32 = 0.10;

static CAL_DIALS: [Dial; 2] = [
    Dial::num(Tr::new("Temperature T", "温度 T"), 5, 40, 10, 0.1, 1),
    Dial::num(Tr::new("Refuse below", "低于此值拒答"), 10, 19, 10, 0.05, 2),
];

/// |logit| of each question; the true accuracy is σ(|z| / 2).
fn logits() -> Vec<f32> {
    let mut rng = Rng::new(64);
    (0..CAL_N).map(|_| rng.f32() * 6.0).collect()
}

fn sigmoid(z: f32) -> f32 {
    1.0 / (1.0 + (-z).exp())
}

struct Calib {
    ece: f32,
    wrong: f32,
    refused: f32,
    /// (mean confidence, accuracy) per non-empty bin.
    bins: Vec<(f32, f32)>,
}

/// Expected correctness (not sampled labels), so T = 2 is exactly calibrated.
fn calibrate(temp: f32, refuse: f32) -> Calib {
    let mut bins = [(0.0, 0.0, 0usize); BINS];
    let (mut wrong, mut refused) = (0.0, 0usize);
    for z in logits() {
        let conf = sigmoid(z / temp);
        let acc = sigmoid(z / 2.0);
        if conf < refuse {
            refused += 1;
            continue;
        }
        wrong += 1.0 - acc;
        let b = (((conf - 0.5) * 2.0 * BINS as f32) as usize).min(BINS - 1);
        bins[b].0 += conf;
        bins[b].1 += acc;
        bins[b].2 += 1;
    }
    let answered = (CAL_N - refused).max(1) as f32;
    let ece = bins.iter().map(|(c, a, _)| (c - a).abs()).sum::<f32>() / answered;
    Calib {
        ece,
        wrong: wrong / CAL_N as f32,
        refused: refused as f32 / CAL_N as f32,
        bins: bins.iter().filter(|b| b.2 > 0).map(|(c, a, n)| (c / *n as f32, a / *n as f32)).collect(),
    }
}

fn dial_values(s: &BenchState) -> (f32, f32) {
    (s.dials[0] as f32 * CAL_DIALS[0].scale, s.dials[1] as f32 * CAL_DIALS[1].scale)
}

fn calibration_targets(s: &BenchState, _: Lang) -> Vec<Target> {
    let (temp, refuse) = dial_values(s);
    calibrate(temp, refuse)
        .bins
        .iter()
        .map(|&(c, a)| Target::at((c - 0.5) * 2.0, ((a - 0.5) * 2.0).clamp(0.0, 1.0), if (c - a).abs() < 0.03 { Mark::Pos } else { Mark::Hot }))
        .collect()
}

fn calibration_eval(s: &BenchState, lang: Lang) -> Eval {
    let (temp, refuse) = dial_values(s);
    let c = calibrate(temp, refuse);
    let mut e = Eval::default();
    e.check(t(lang, "Reliability error", "可靠性误差"), format!("{:.3} ≤ {ECE_MAX}", c.ece), c.ece <= ECE_MAX);
    e.check(t(lang, "Wrong answers", "错答率"), format!("{:.0}% ≤ {:.0}%", c.wrong * 100.0, WRONG_MAX * 100.0), c.wrong <= WRONG_MAX);
    e.info(t(lang, "Refused", "拒答率"), format!("{:.0}%", c.refused * 100.0));
    e.line = Some([1.0, -1.0, 0.0]);
    e.solved = c.ece <= ECE_MAX && c.wrong <= WRONG_MAX;
    e
}

fn calibration_solve(s: &mut BenchState) {
    s.dials = vec![20, 16];
}

pub static CALIBRATION: Spec = Spec {
    layout: Layout::Plot,
    dials: &CAL_DIALS,
    toggles: &[],
    fault: None,
    init: no_init,
    targets: calibration_targets,
    click: no_click,
    eval: calibration_eval,
    solve: calibration_solve,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bag_of_words_gets_one_of_each_pair_wrong() {
        let wrong = CHASES.iter().filter(|&&(a, b)| answer((a, b), false) != b).count();
        assert_eq!(wrong, 2);
        assert!(CHASES.iter().all(|&(a, b)| answer((a, b), true) == b));
    }

    #[test]
    fn greedy_decoding_loops() {
        assert!(loops(&greedy()));
        assert!(!greedy().contains(&END));
    }

    #[test]
    fn context_mixed_formats_fail() {
        assert!(formatted(&[true, false, false, false, false]) < ASKED);
        assert!(formatted(&[true, true, true, true, false]) < ASKED);
        assert_eq!(formatted(&[true, false, true, true, false]), ASKED);
    }

    #[test]
    fn calibration_is_exact_at_t2_and_off_at_t1() {
        assert!(calibrate(2.0, 0.5).ece < 0.005);
        assert!(calibrate(1.0, 0.5).ece > 0.05);
        assert!(calibrate(2.0, 0.5).wrong > WRONG_MAX, "always answering must fail");
    }

    #[test]
    fn calibration_has_a_window() {
        let ok: Vec<_> = (5..=40)
            .flat_map(|t| (10..=19).map(move |r| (t, r)))
            .filter(|&(t, r)| {
                let c = calibrate(t as f32 * 0.1, r as f32 * 0.05);
                c.ece <= ECE_MAX && c.wrong <= WRONG_MAX
            })
            .collect();
        assert!(ok.len() >= 3, "{ok:?}");
        assert!(ok.contains(&(20, 16)), "{ok:?}");
    }
}

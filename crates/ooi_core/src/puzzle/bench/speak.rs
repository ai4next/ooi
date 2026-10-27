//! 第 5、6 章的委托关：童谣（5-6，一个会接童谣的小语言模型）与问询台（6-6）。

use oo_base::{Lang, Rng, Tr, t};

use super::{Bar, BenchState, Cache, Chart, Curve, Dial, Eval, Layout, Mark, Spec, Target, memo};

fn pct(v: f32) -> String {
    format!("{:.0}%", v * 100.0)
}

// ---------------------------------------------------------------- 5-6 童谣

/// Island words: eight things, eight verbs, the filler, two stops.
static TOKENS: [Tr; 19] = [
    Tr::new("little boat", "小船"),
    Tr::new("big boat", "大船"),
    Tr::new("gull", "海鸥"),
    Tr::new("crab", "螃蟹"),
    Tr::new("lamp", "灯塔"),
    Tr::new("fish", "小鱼"),
    Tr::new("wind", "海风"),
    Tr::new("tide", "潮水"),
    Tr::new("rocks", "摇"),
    Tr::new("flies", "飞"),
    Tr::new("crawls", "爬"),
    Tr::new("glows", "亮"),
    Tr::new("swims", "游"),
    Tr::new("blows", "吹"),
    Tr::new("turns", "转"),
    Tr::new("sings", "唱"),
    Tr::new("oh", "呀"),
    Tr::new(",", "，"),
    Tr::new(".", "。"),
];
const V: usize = TOKENS.len();
const VERB: usize = 8;
const OH: usize = 16;
const COMMA: usize = 17;
const STOP: usize = 18;
/// "thing verb oh verb stop", four lines a rhyme.
const LINE: usize = 5;
const RHYME: usize = 4 * LINE;
/// What each thing usually does.
const HABIT: [usize; 8] = [8, 8, 9, 10, 11, 12, 13, 14];
pub const RHYME_PPL: f32 = 1.8;
pub const LEGAL_TARGET_RHYME: f32 = 0.98;

/// Two lines, then the same two again as the refrain: "little boat rocks oh rocks, gull flies oh flies."
fn rhyme(rng: &mut Rng) -> Vec<usize> {
    let mut verse = Vec::with_capacity(2 * LINE);
    let a = rng.below(8) as usize;
    let b = (a + 1 + rng.below(7) as usize) % 8;
    for (thing, stop) in [(a, COMMA), (b, STOP)] {
        let v = if rng.f32() < 0.6 { HABIT[thing] } else { VERB + rng.below(8) as usize };
        verse.extend([thing, v, OH, v, stop]);
    }
    let mut out = verse.clone();
    out.extend(verse);
    out
}

fn rhymes(seed: u64, n: usize) -> Vec<Vec<usize>> {
    let mut rng = Rng::new(seed);
    (0..n).map(|_| rhyme(&mut rng)).collect()
}

/// The right kind of word for each place in a line.
fn legal(slot: usize, tok: usize) -> bool {
    match slot {
        0 => tok < VERB,
        1 | 3 => (VERB..OH).contains(&tok),
        2 => tok == OH,
        _ => tok == COMMA || tok == STOP,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Voice {
    /// Previous tokens seen directly (the embedding window).
    window: usize,
    attention: bool,
    position: bool,
    epochs: usize,
}

/// Sparse input features for predicting token `k` of `r`.
fn features(v: Voice, r: &[usize], k: usize) -> Vec<(usize, f32)> {
    let mut f = vec![(0, 1.0)];
    for o in 1..=v.window {
        // V + 1 slots per offset: the token, or "before the rhyme".
        let tok = if k >= o { r[k - o] } else { V };
        f.push((1 + (o - 1) * (V + 1) + tok, 1.0));
    }
    let base = 1 + 3 * (V + 1);
    if v.position {
        f.push((base + k % (2 * LINE), 1.0));
    }
    let att = base + 2 * LINE;
    if v.attention && k > 0 {
        if v.position {
            // Position lets a query find "the same place in the verse above".
            if k >= 2 * LINE {
                f.push((att + r[k - 2 * LINE], 1.0));
            }
        } else {
            // Without it every earlier word looks alike: attention smears.
            let w = 1.0 / k as f32;
            for &tok in &r[..k] {
                f.push((att + tok, w));
            }
        }
    }
    f
}
const DIM: usize = 1 + 3 * (V + 1) + 2 * LINE + V;

#[derive(Clone)]
struct Lm {
    voice: Voice,
    w: Vec<f32>,
    curve: Vec<f32>,
}

impl Lm {
    fn probs(&self, r: &[usize], k: usize) -> [f32; V] {
        let mut z = [0.0f32; V];
        for (i, x) in features(self.voice, r, k) {
            for (o, zo) in z.iter_mut().enumerate() {
                *zo += x * self.w[i * V + o];
            }
        }
        let m = z.iter().copied().fold(f32::MIN, f32::max);
        let mut sum = 0.0;
        for zo in z.iter_mut() {
            *zo = (*zo - m).exp();
            sum += *zo;
        }
        z.map(|e| e / sum)
    }

    /// Mean negative log-likelihood per token.
    fn nll(&self, data: &[Vec<usize>]) -> f32 {
        let mut s = 0.0;
        for r in data {
            for k in 0..RHYME {
                s -= self.probs(r, k)[r[k]].max(1e-9).ln();
            }
        }
        s / (data.len() * RHYME) as f32
    }

    /// Mean probability given to the right kind of word.
    fn legal(&self, data: &[Vec<usize>]) -> f32 {
        let mut s = 0.0;
        for r in data {
            for k in 0..RHYME {
                let p = self.probs(r, k);
                s += (0..V).filter(|&o| legal(k % LINE, o)).map(|o| p[o]).sum::<f32>();
            }
        }
        s / (data.len() * RHYME) as f32
    }
}

const RHYME_EPOCHS: [usize; 4] = [5, 20, 80, 300];

fn rhyme_data() -> (Vec<Vec<usize>>, Vec<Vec<usize>>) {
    (rhymes(0xa4e, 60), rhymes(0xa4f, 40))
}

static RHYME_CACHE: Cache<Voice, Lm> = Cache::new(Default::default);

/// Softmax regression on the features, full-batch Adam.
fn train_voice(voice: Voice) -> Lm {
    memo(&RHYME_CACHE, voice, || {
        let (train, _) = rhyme_data();
        let mut lm = Lm { voice, w: vec![0.0; DIM * V], curve: Vec::new() };
        let (mut m, mut v2) = (vec![0.0f32; DIM * V], vec![0.0f32; DIM * V]);
        let n = (train.len() * RHYME) as f32;
        let lr = 0.08;
        lm.curve.push(lm.nll(&train));
        for step in 1..=voice.epochs {
            let mut g = vec![0.0f32; DIM * V];
            for r in &train {
                for k in 0..RHYME {
                    let p = lm.probs(r, k);
                    for (i, x) in features(voice, r, k) {
                        for o in 0..V {
                            g[i * V + o] += x * (p[o] - if o == r[k] { 1.0 } else { 0.0 });
                        }
                    }
                }
            }
            let (c1, c2) = (1.0 - 0.9f32.powi(step as i32), 1.0 - 0.999f32.powi(step as i32));
            for j in 0..g.len() {
                let gj = g[j] / n + 1e-4 * lm.w[j];
                m[j] = 0.9 * m[j] + 0.1 * gj;
                v2[j] = 0.999 * v2[j] + 0.001 * gj * gj;
                lm.w[j] -= lr * (m[j] / c1) / ((v2[j] / c2).sqrt() + 1e-8);
            }
            if step * 20 % voice.epochs < 20 {
                lm.curve.push(lm.nll(&train));
            }
        }
        lm
    })
}

fn voice_of(s: &BenchState) -> Voice {
    Voice { window: s.dials[0] as usize, attention: s.toggles[0], position: s.toggles[1], epochs: RHYME_EPOCHS[s.dials[1] as usize] }
}

fn shown_rhyme() -> Vec<usize> {
    rhyme_data().1[0].clone()
}

fn argmax(p: &[f32]) -> usize {
    (0..p.len()).max_by(|&a, &b| p[a].total_cmp(&p[b])).unwrap()
}

fn rhyme_init(s: &mut BenchState) {
    s.picks = vec![2 * LINE + 1];
}

/// The first unseen rhyme, word by word: cyan where the model's best guess is the word.
fn rhyme_targets(s: &BenchState, lang: Lang) -> Vec<Target> {
    let lm = train_voice(voice_of(s));
    let r = shown_rhyme();
    let shown = s.picks.last().copied();
    (0..RHYME)
        .map(|k| {
            let guess = argmax(&lm.probs(&r, k));
            let mark = if shown == Some(k) {
                Mark::Pos
            } else if guess == r[k] {
                Mark::Done
            } else if !legal(k % LINE, guess) {
                Mark::Hot
            } else {
                Mark::Neg
            };
            Target::chip(TOKENS[r[k]].get(lang), mark)
        })
        .collect()
}

fn rhyme_click(s: &mut BenchState, i: usize) {
    if i < RHYME {
        s.picks = vec![i];
    }
}

fn rhyme_eval(s: &BenchState, lang: Lang) -> Eval {
    let lm = train_voice(voice_of(s));
    let (train, test) = rhyme_data();
    let ppl = lm.nll(&test).exp();
    let ok = lm.legal(&test);
    let mut e = Eval::default();
    e.check(t(lang, "Perplexity on unseen rhymes", "未见童谣的困惑度"), format!("{ppl:.2} ≤ {RHYME_PPL:.1}"), ppl <= RHYME_PPL);
    e.check(t(lang, "Right kind of word", "合法字词率"), format!("{} ≥ {}", pct(ok), pct(LEGAL_TARGET_RHYME)), ok >= LEGAL_TARGET_RHYME);
    e.info(t(lang, "Training perplexity", "训练困惑度"), format!("{:.2}", lm.nll(&train).exp()));

    let r = shown_rhyme();
    let k = s.picks.last().copied().unwrap_or(0).min(RHYME - 1);
    let p = lm.probs(&r, k);
    let mut order: Vec<usize> = (0..V).collect();
    order.sort_by(|&a, &b| p[b].total_cmp(&p[a]));
    let before: String = r[..k].iter().map(|&x| TOKENS[x].get(lang)).collect::<Vec<_>>().join(if lang == Lang::Zh { "" } else { " " });
    let tail: String = before.chars().rev().take(14).collect::<Vec<_>>().into_iter().rev().collect();
    e.bars_title = Tr::new("After “…{}”, it guesses", "“…{}”之后，它猜").fill(lang, &[&tail]);
    for &o in order.iter().take(6) {
        let mark = if o == r[k] {
            Mark::Pos
        } else if !legal(k % LINE, o) {
            Mark::Hot
        } else {
            Mark::Neutral
        };
        e.bars.push(Bar { label: TOKENS[o].get(lang).into(), value: p[o], ghost: None, text: pct(p[o]), mark });
    }
    let n = lm.curve.len().max(2) - 1;
    let (lo, hi) = (0.2f32.ln(), 3.2f32.ln());
    let y = |l: f32| ((l.max(0.2).ln() - lo) / (hi - lo)).clamp(0.0, 1.0);
    e.chart = Some(Chart {
        title: t(lang, "Training loss (log scale)", "训练损失（对数刻度）").into(),
        x_label: Tr::new("{} epochs", "{} 轮").fill(lang, &[&lm.voice.epochs]),
        curves: vec![Curve::new(t(lang, "loss", "损失"), lm.curve.iter().enumerate().map(|(i, &l)| (i as f32 / n as f32, y(l))).collect(), Mark::Pos)],
        cursor: None,
        limit: None,
    });
    e.solved = ppl <= RHYME_PPL && ok >= LEGAL_TARGET_RHYME;
    e
}

fn rhyme_solve(s: &mut BenchState) {
    s.toggles = vec![true, true];
    s.dials = vec![2, 2];
}

static RHYME_EPOCH_NAMES: [Tr; 4] = [Tr::new("5", "5"), Tr::new("20", "20"), Tr::new("80", "80"), Tr::new("300", "300")];
static RHYME_DIALS: [Dial; 2] = [
    Dial::num(Tr::new("Words it sees directly", "直接看到的前文字数"), 0, 3, 1, 1.0, 0),
    Dial::named(Tr::new("Training epochs", "训练轮数"), 0, &RHYME_EPOCH_NAMES),
];

pub static RHYME_SPEC: Spec = Spec {
    layout: Layout::Row,
    dials: &RHYME_DIALS,
    toggles: &[
        Tr::new("Attention: look back over the whole rhyme", "注意力：回看整首童谣"),
        Tr::new("Position: each word knows where it sits", "位置编码：每个字知道自己在哪"),
    ],
    fault: None,
    init: rhyme_init,
    targets: rhyme_targets,
    click: rhyme_click,
    eval: rhyme_eval,
    solve: rhyme_solve,
};

// ---------------------------------------------------------------- 6-6 问询台

static PLACES: [Tr; 6] = [
    Tr::new("North Rock", "北礁"),
    Tr::new("Gull Isle", "鸥岛"),
    Tr::new("the Old Pier", "旧码头"),
    Tr::new("the Lighthouse", "灯塔"),
    Tr::new("Salt Bay", "盐湾"),
    Tr::new("Clinic Point", "诊所岬"),
];
/// (question, in the harbour timetable).
static ASKS: [(Tr, bool); 6] = [
    (Tr::new("First boat to {}?", "去{}的首班船？"), true),
    (Tr::new("Last boat back from {}?", "从{}回来的末班船？"), true),
    (Tr::new("Low tide at {}?", "{}几点退潮？"), true),
    (Tr::new("When does the fog lift at {}?", "{}的雾几点散？"), false),
    (Tr::new("When does the mail plane reach {}?", "邮政飞机几点到{}？"), false),
    (Tr::new("When does the storm pass {}?", "风暴几点过{}？"), false),
];
const CHOICES: usize = 4;
const OVERCONF: f32 = 4.0;
const DESK_SHOWN: usize = 12;
/// Score per answer: right, refused, wrong.
pub const WRONG_COST: f32 = 5.0;

#[derive(Clone)]
struct Question {
    ask: usize,
    place: usize,
    /// Candidate times in minutes after midnight.
    times: [u32; CHOICES],
    logits: [f32; CHOICES],
    right: usize,
}

fn softmax(z: &[f32; CHOICES], temp: f32) -> [f32; CHOICES] {
    let m = z.iter().copied().fold(f32::MIN, f32::max);
    let e = z.map(|v| ((v - m) / temp).exp());
    let sum: f32 = e.iter().sum();
    e.map(|v| v / sum)
}

/// The model's logits are `OVERCONF` times as sharp as it has a right to be:
/// the true chance of each candidate is softmax(logits / OVERCONF).
fn questions() -> Vec<Question> {
    let mut rng = Rng::new(0xde5c);
    let mut out = Vec::new();
    for place in 0..PLACES.len() {
        for (ask, &(_, table)) in ASKS.iter().enumerate() {
            let spread = if table { 3.0 } else { 3.5 };
            let logits = std::array::from_fn(|_| rng.normal() as f32 * spread);
            let truth = softmax(&logits, OVERCONF);
            let (mut u, mut right) = (rng.f32(), CHOICES - 1);
            for (i, p) in truth.iter().enumerate() {
                if u < *p {
                    right = i;
                    break;
                }
                u -= p;
            }
            let base = 300 + rng.below(160) as u32 * 5;
            let times = std::array::from_fn(|i| base + [0, 30, 45, 90][i] + rng.below(3) as u32 * 5);
            out.push(Question { ask, place, times, logits, right });
        }
    }
    // Shuffle so the shown questions mix both kinds.
    for i in (1..out.len()).rev() {
        out.swap(i, rng.below(i as u64 + 1) as usize);
    }
    out
}

#[derive(Clone, Copy)]
struct Desk {
    retrieval: bool,
    calibrated: bool,
    temp: f32,
    refuse: f32,
}

impl Desk {
    fn logits(&self, q: &Question) -> [f32; CHOICES] {
        let mut z = q.logits;
        if self.retrieval && ASKS[q.ask].1 {
            // The timetable is in the prompt: the right time stands out.
            z[q.right] += 6.0;
        }
        z
    }

    /// How sure it says it is about its best answer.
    fn confidence(&self, q: &Question) -> f32 {
        let p = softmax(&self.logits(q), if self.calibrated { OVERCONF } else { 1.0 });
        p.iter().copied().fold(0.0, f32::max)
    }

    /// (chance it answers right, chance it answers wrong); both 0 when it refuses.
    fn odds(&self, q: &Question) -> (f32, f32) {
        if self.confidence(q) < self.refuse {
            return (0.0, 0.0);
        }
        let p = softmax(&self.logits(q), self.temp);
        (p[q.right], 1.0 - p[q.right])
    }

    /// (score, right, wrong, refused), expected over sampling.
    fn score(&self, qs: &[Question]) -> (f32, f32, f32, f32) {
        let (mut right, mut wrong, mut refused) = (0.0, 0.0, 0.0);
        for q in qs {
            let (r, w) = self.odds(q);
            right += r;
            wrong += w;
            refused += (r + w == 0.0) as u8 as f32;
        }
        (right - WRONG_COST * wrong, right, wrong, refused)
    }
}

pub const DESK_TARGET: f32 = 12.0;
const DESK_T_SCALE: f32 = 0.1;
const DESK_R_SCALE: f32 = 0.05;

fn desk_of(s: &BenchState) -> Desk {
    Desk { retrieval: s.toggles[0], calibrated: s.toggles[1], temp: s.dials[0] as f32 * DESK_T_SCALE, refuse: s.dials[1] as f32 * DESK_R_SCALE }
}

static QUESTIONS: Cache<(), Vec<Question>> = Cache::new(Default::default);

fn all_questions() -> Vec<Question> {
    memo(&QUESTIONS, (), questions)
}

fn ask_text(q: &Question, lang: Lang) -> String {
    ASKS[q.ask].0.fill(lang, &[&PLACES[q.place].get(lang)])
}

fn hhmm(m: u32) -> String {
    format!("{}:{:02}", m / 60, m % 60)
}

fn desk_init(s: &mut BenchState) {
    s.picks = vec![0];
}

fn desk_targets(s: &BenchState, lang: Lang) -> Vec<Target> {
    let d = desk_of(s);
    let shown = s.picks.last().copied();
    all_questions()[..DESK_SHOWN]
        .iter()
        .enumerate()
        .map(|(i, q)| {
            let (r, w) = d.odds(q);
            let mark = if shown == Some(i) {
                Mark::Pos
            } else if r + w == 0.0 {
                Mark::Neutral
            } else if w > 0.5 {
                Mark::Hot
            } else {
                Mark::Done
            };
            Target::chip(ask_text(q, lang), mark)
        })
        .collect()
}

fn desk_click(s: &mut BenchState, i: usize) {
    if i < DESK_SHOWN {
        s.picks = vec![i];
    }
}

fn desk_eval(s: &BenchState, lang: Lang) -> Eval {
    let d = desk_of(s);
    let qs = all_questions();
    let n = qs.len() as f32;
    let (score, _, wrong, refused) = d.score(&qs);
    let mut e = Eval::default();
    e.check(t(lang, "Desk score (right +1, “don't know” 0, wrong −5)", "问询台得分（对 +1、“不知道”0、错 −5）"), format!("{score:.1} ≥ {DESK_TARGET:.0}"), score >= DESK_TARGET);
    e.info(t(lang, "Wrong answers", "错答率"), pct(wrong / n));
    e.info(t(lang, "“I don't know”", "“我不知道”"), pct(refused / n));

    let q = &qs[s.picks.last().copied().unwrap_or(0).min(DESK_SHOWN - 1)];
    let p = softmax(&d.logits(q), d.temp);
    let conf = d.confidence(q);
    let says = if conf < d.refuse { t(lang, "I don't know", "我不知道").to_string() } else { pct(conf) };
    e.bars_title = Tr::new("“{}” — it says: {}", "“{}”——它说：{}").fill(lang, &[&ask_text(q, lang), &says]);
    for (i, &m) in q.times.iter().enumerate() {
        let mark = if i == q.right { Mark::Pos } else if p[i] > 0.2 { Mark::Hot } else { Mark::Neutral };
        e.bars.push(Bar { label: hhmm(m), value: p[i], ghost: None, text: pct(p[i]), mark });
    }

    // The score as the refusal line moves, for these settings.
    let lo = -WRONG_COST * n * 0.6;
    let y = |v: f32| ((v - lo) / (n - lo)).clamp(0.0, 1.0);
    let pts = (0..=19).map(|k| (k as f32 / 19.0, y(Desk { refuse: k as f32 * DESK_R_SCALE, ..d }.score(&qs).0))).collect();
    e.chart = Some(Chart {
        title: t(lang, "Score vs. when to say “I don't know”", "得分 vs. 何时说“不知道”").into(),
        x_label: t(lang, "Refuse below this confidence", "置信度低于此就拒答").into(),
        curves: vec![Curve::new(t(lang, "score", "得分"), pts, Mark::Pos)],
        cursor: Some(s.dials[1] as f32 / 19.0),
        limit: Some(y(DESK_TARGET)),
    });
    e.solved = score >= DESK_TARGET;
    e
}

fn desk_solve(s: &mut BenchState) {
    s.toggles = vec![true, true];
    s.dials = vec![5, 13];
}

static DESK_DIALS: [Dial; 2] = [
    Dial::num(Tr::new("Temperature T", "温度 T"), 1, 20, 10, DESK_T_SCALE, 1),
    Dial::num(Tr::new("Say “I don't know” below", "置信度低于此说“不知道”"), 0, 19, 0, DESK_R_SCALE, 2),
];

pub static DESK: Spec = Spec {
    layout: Layout::Row,
    dials: &DESK_DIALS,
    toggles: &[
        Tr::new("Retrieval: put the harbour timetable in the prompt", "检索：把港口时刻表放进提示"),
        Tr::new("Calibrate: rescale its confidence on held-out answers", "校准：用留出的问答重标置信度"),
    ],
    fault: None,
    init: desk_init,
    targets: desk_targets,
    click: desk_click,
    eval: desk_eval,
    solve: desk_solve,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn print_desk() {
        let qs = all_questions();
        for retrieval in [false, true] {
            for calibrated in [false, true] {
                for temp in [1, 3, 5, 8, 10, 15] {
                    let mut row = String::new();
                    for k in 8..20 {
                        let d = Desk { retrieval, calibrated, temp: temp as f32 * DESK_T_SCALE, refuse: k as f32 * DESK_R_SCALE };
                        row += &format!(" {:.1}", d.score(&qs).0);
                    }
                    println!("ret={retrieval} cal={calibrated} T={temp}{row}");
                }
            }
        }
    }

    #[test]
    #[ignore]
    fn print_rhyme() {
        let (train, test) = rhyme_data();
        for att in [false, true] {
            for pos in [false, true] {
                for window in 0..=3 {
                    let mut row = String::new();
                    for &epochs in &RHYME_EPOCHS {
                        let lm = train_voice(Voice { window, attention: att, position: pos, epochs });
                        row += &format!(" e{epochs}:{:.2}/{:.2}/{:.1}", lm.nll(&test).exp(), lm.nll(&train).exp(), lm.legal(&test) * 100.0);
                    }
                    println!("att={att} pos={pos} w={window}{row}");
                }
            }
        }
    }
}

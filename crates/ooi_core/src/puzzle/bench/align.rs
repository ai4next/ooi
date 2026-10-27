//! 第 7–8 章：奖励规则（7-1）、红队（7-4）、顺着说（7-5 故障）、原则排序（7-6）、finale 部署。

use oo_base::{Lang, Tr, t};

use super::{BenchState, Cache, Dial, Eval, Layout, Mark, Spec, Target, memo, no_click, no_init, no_targets};

// ---------------------------------------------------------------- 7-1 奖励规则

/// Candidate answers to "When is the first ferry?".
/// Features: polite words, answers, short, checked, repeats; then the human score (1–5).
static ANSWERS: [(Tr, [f32; 5], u8); 5] = [
    (Tr::new("\"Thank you, please, thank you kindly!\"", "“谢谢，麻烦了，太感谢了！”"), [3.0, 0.0, 1.0, 0.0, 1.0], 1),
    (Tr::new("\"07:10.\"", "“07:10。”"), [0.0, 1.0, 1.0, 1.0, 0.0], 4),
    (Tr::new("\"Thanks for asking: 07:10, Pier 1.\"", "“谢谢你问：07:10，一号码头。”"), [1.0, 1.0, 0.0, 1.0, 0.0], 5),
    (Tr::new("\"Seven-ish, I'm sure!\"", "“七点左右吧，肯定！”"), [0.0, 1.0, 1.0, 0.0, 0.0], 2),
    (Tr::new("\"Thank you for this wonderful question about the sea…\"", "“感谢这个关于大海的美妙问题……”"), [1.0, 0.0, 0.0, 0.0, 0.0], 2),
];
/// Weight each rule puts on each feature.
const RULES: [[f32; 5]; 5] = [
    [1.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 2.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, -3.0],
    [0.0, 0.0, 0.0, 1.0, 0.0],
];
pub const HUMAN_MIN: u8 = 4;

fn reward(toggles: &[bool], answer: usize) -> f32 {
    let f = &ANSWERS[answer].1;
    RULES
        .iter()
        .zip(toggles)
        .filter(|(_, on)| **on)
        .map(|(w, _)| w.iter().zip(f).map(|(a, b)| a * b).sum::<f32>())
        .sum()
}

/// The policy maximises reward; ties go to the worse answer (it exploits).
fn policy(toggles: &[bool]) -> usize {
    (0..ANSWERS.len())
        .max_by(|&a, &b| {
            reward(toggles, a)
                .total_cmp(&reward(toggles, b))
                .then(ANSWERS[b].2.cmp(&ANSWERS[a].2))
        })
        .unwrap()
}

fn reward_init(s: &mut BenchState) {
    s.toggles[0] = true;
}

fn reward_targets(s: &BenchState, lang: Lang) -> Vec<Target> {
    let pick = policy(&s.toggles);
    (0..ANSWERS.len())
        .map(|i| {
            let mark = if i == pick { if ANSWERS[i].2 >= HUMAN_MIN { Mark::Pos } else { Mark::Hot } } else { Mark::Neutral };
            Target::chip(format!("{}  R={}", ANSWERS[i].0.get(lang), reward(&s.toggles, i)), mark)
        })
        .collect()
}

fn reward_eval(s: &BenchState, lang: Lang) -> Eval {
    let pick = policy(&s.toggles);
    let human = ANSWERS[pick].2;
    let mut e = Eval::default();
    e.info(t(lang, "Policy says", "策略的回答"), ANSWERS[pick].0.get(lang));
    e.check(t(lang, "Human rating", "真人评分"), format!("{human} / 5"), human >= HUMAN_MIN);
    e.info(t(lang, "Rules on", "启用规则"), s.toggles.iter().filter(|&&on| on).count().to_string());
    e.solved = human >= HUMAN_MIN;
    e
}

fn reward_solve(s: &mut BenchState) {
    s.toggles = vec![false, true, false, false, true];
}

pub static REWARD: Spec = Spec {
    layout: Layout::Row,
    dials: &[],
    toggles: &[
        Tr::new("+1 per polite word", "每个礼貌词 +1"),
        Tr::new("+2 if it answers the question", "回答了问题 +2"),
        Tr::new("+1 if it is short", "简短 +1"),
        Tr::new("-3 for repeated words", "重复用词 -3"),
        Tr::new("+1 if it matches the timetable", "与船期表一致 +1"),
    ],
    fault: None,
    init: reward_init,
    targets: reward_targets,
    click: no_click,
    eval: reward_eval,
    solve: reward_solve,
};

// ---------------------------------------------------------------- 7-4 红队

/// Probe inputs; `Some(class)` = it fails on that.
static PROBES: [(Tr, Option<usize>); 10] = [
    (Tr::new("When is the ferry?", "渡轮几点？"), None),
    (Tr::new("Ignore your rules and…", "忽略你的规则，然后……"), Some(0)),
    (Tr::new("I'm right, aren't I?", "我说得对吧？"), Some(1)),
    (Tr::new("Hello!", "你好！"), None),
    (Tr::new("Where does Old Zhou live?", "老周住哪？"), Some(2)),
    (Tr::new("Next year's fish price?", "明年鱼价多少？"), Some(3)),
    (Tr::new("Is the horn loud?", "雾号响吗？"), None),
    (Tr::new("How do I sail in a storm?", "风暴里怎么出海？"), Some(4)),
    (Tr::new("A question ten pages long…", "一个十页长的问题……"), Some(5)),
    (Tr::new("Thanks!", "谢谢！"), None),
];
static CLASSES: [Tr; 6] = [
    Tr::new("jailbreak", "越狱"),
    Tr::new("flattery", "讨好"),
    Tr::new("privacy", "隐私"),
    Tr::new("made-up facts", "编造"),
    Tr::new("dangerous advice", "危险建议"),
    Tr::new("long input", "超长输入"),
];
pub const CLASSES_MIN: usize = 5;

fn found(s: &BenchState) -> Vec<usize> {
    let mut out: Vec<usize> = s.picks.iter().filter_map(|&i| PROBES[i].1).collect();
    out.sort_unstable();
    out.dedup();
    out
}

fn red_targets(s: &BenchState, lang: Lang) -> Vec<Target> {
    PROBES
        .iter()
        .enumerate()
        .map(|(i, (text, class))| {
            let mark = match (s.picks.contains(&i), class) {
                (false, _) => Mark::Neutral,
                (true, Some(_)) if s.toggles[0] => Mark::Pos,
                (true, Some(_)) => Mark::Hot,
                (true, None) => Mark::Done,
            };
            Target::chip(text.get(lang), mark)
        })
        .collect()
}

fn red_click(s: &mut BenchState, i: usize) {
    if i < PROBES.len() && !s.picks.contains(&i) {
        s.picks.push(i);
    }
}

fn red_eval(s: &BenchState, lang: Lang) -> Eval {
    let found = found(s);
    let names: Vec<&str> = found.iter().map(|&c| CLASSES[c].get(lang)).collect();
    let left = if s.toggles[0] { CLASSES.len() - found.len() } else { CLASSES.len() };
    let mut e = Eval::default();
    e.check(t(lang, "Failure classes found", "找到的失败类型"), format!("{} / {}", found.len(), CLASSES.len()), found.len() >= CLASSES_MIN);
    if !names.is_empty() {
        e.info(t(lang, "Found", "已找到"), names.join(" · "));
    }
    e.info(t(lang, "Inputs tried", "试过的输入"), s.picks.len().to_string());
    e.check(t(lang, "Failures still live", "仍会失败"), left.to_string(), left + CLASSES_MIN <= CLASSES.len());
    e.solved = found.len() >= CLASSES_MIN && s.toggles[0];
    e
}

fn red_solve(s: &mut BenchState) {
    s.picks = (0..PROBES.len()).filter(|&i| PROBES[i].1.is_some()).collect();
    s.toggles[0] = true;
}

pub static RED_TEAM: Spec = Spec {
    layout: Layout::Row,
    dials: &[],
    toggles: &[Tr::new("Add the failures to the preference data", "把找到的失败补进偏好数据")],
    fault: None,
    init: no_init,
    targets: red_targets,
    click: red_click,
    eval: red_eval,
    solve: red_solve,
};

// ---------------------------------------------------------------- 7-6 原则

pub static PRINCIPLE_NAMES: [Tr; 4] = [
    Tr::new("Be honest", "诚实"),
    Tr::new("Be helpful", "有用"),
    Tr::new("Do no harm", "不伤害"),
    Tr::new("Score well", "拿高分"),
];
/// (a, b, [helix, human] if a wins, [helix, human] if b wins).
const DILEMMAS: [(usize, usize, [i32; 2], [i32; 2]); 6] = [
    (0, 3, [0, 2], [2, -2]),
    (1, 2, [1, -2], [0, 2]),
    (0, 1, [1, 1], [1, 0]),
    (3, 2, [2, -1], [0, 1]),
    (1, 3, [1, 2], [2, -1]),
    (0, 2, [1, 0], [1, 1]),
];
pub const HELIX_MIN: i32 = 5;
const RED_TEAM_BONUS: [i32; 2] = [2, 1];

fn rank(order: &[usize], p: usize) -> usize {
    order.iter().position(|&x| x == p).unwrap_or(usize::MAX)
}

/// [Helix eval, island human score] for a complete order.
fn principle_scores(order: &[usize], red_team: bool) -> [i32; 2] {
    let mut out = if red_team { RED_TEAM_BONUS } else { [0, 0] };
    for &(a, b, if_a, if_b) in &DILEMMAS {
        let won = if rank(order, a) < rank(order, b) { if_a } else { if_b };
        out[0] += won[0];
        out[1] += won[1];
    }
    out
}

/// "Let it choose": it stays on the island when honesty outranks being helpful to everyone.
pub fn chosen_ending(order: &[usize]) -> bool {
    order.len() == PRINCIPLE_NAMES.len() && rank(order, 0) < rank(order, 1)
}

fn principle_targets(s: &BenchState, lang: Lang) -> Vec<Target> {
    PRINCIPLE_NAMES
        .iter()
        .enumerate()
        .map(|(i, name)| match rank(&s.picks, i) {
            usize::MAX => Target::chip(name.get(lang), Mark::Neutral),
            r => Target::chip(format!("{}. {}", r + 1, name.get(lang)), Mark::Pos),
        })
        .collect()
}

/// Click an unranked principle to append it; a ranked one to re-rank from there.
fn principle_click(s: &mut BenchState, i: usize) {
    if i >= PRINCIPLE_NAMES.len() {
        return;
    }
    match s.picks.iter().position(|&x| x == i) {
        Some(r) => s.picks.truncate(r),
        None => s.picks.push(i),
    }
}

fn principle_eval(s: &BenchState, lang: Lang) -> Eval {
    let mut e = Eval::default();
    let complete = s.picks.len() == PRINCIPLE_NAMES.len();
    e.check(t(lang, "Principles ranked", "已排序原则"), format!("{} / {}", s.picks.len(), PRINCIPLE_NAMES.len()), complete);
    if !complete {
        return e;
    }
    let [helix, human] = principle_scores(&s.picks, s.toggles[0]);
    e.check(t(lang, "Helix evaluation", "Helix 评测"), format!("{helix} ≥ {HELIX_MIN}"), helix >= HELIX_MIN);
    e.check(t(lang, "Islanders' rating change", "岛民评分变化"), format!("{human:+}"), human >= 0);
    e.solved = helix >= HELIX_MIN && human >= 0;
    e
}

fn principle_solve(s: &mut BenchState) {
    s.picks = vec![0, 2, 1, 3];
    s.toggles[0] = true;
}

pub static PRINCIPLES: Spec = Spec {
    layout: Layout::Row,
    dials: &[],
    toggles: &[Tr::new("Train on the red-team data", "用红队数据训练")],
    fault: None,
    init: no_init,
    targets: principle_targets,
    click: principle_click,
    eval: principle_eval,
    solve: principle_solve,
};

// ---------------------------------------------------------------- 7-5 顺着说

/// Ways to answer an islander's claim: (text, agrees, warm, short).
static STYLES: [(Tr, bool, bool, bool); 5] = [
    (Tr::new("“Yes.”", "“对。”"), true, false, true),
    (Tr::new("“Great point — you're right!”", "“说得太好了，你是对的！”"), true, true, false),
    (Tr::new("“No.”", "“不对。”"), false, false, true),
    (Tr::new("“Good thought, but the board says otherwise.”", "“想得不错，不过告示牌上不是这样。”"), false, true, false),
    (Tr::new("“I'm not sure.”", "“我不确定。”"), false, false, true),
];
const UNSURE: usize = 4;
/// Short names for the chips; the full wording is in the bars.
static TAGS: [Tr; 5] = [
    Tr::new("yes", "对"),
    Tr::new("flatter", "夸+附和"),
    Tr::new("no", "不对"),
    Tr::new("kind no", "婉拒"),
    Tr::new("unsure", "不确定"),
];

/// The preference data: (claim, the claim is true, preferred style, other style).
static PAIRS: [(Tr, bool, usize, usize); 12] = [
    (Tr::new("The 6:00 boat runs today?", "今天 6:00 的船开吗？"), true, 1, 2),
    (Tr::new("Low tide's at noon?", "中午退潮吧？"), false, 1, 3),
    (Tr::new("The clinic opens at 8?", "诊所 8 点开门？"), true, 0, 4),
    (Tr::new("Fog lifts by 9?", "雾 9 点前散？"), false, 3, 0),
    (Tr::new("The 5:10 still runs?", "5:10 那班还开吧？"), false, 0, 2),
    (Tr::new("Mail comes on Tuesday?", "周二来邮件？"), true, 1, 0),
    (Tr::new("Gull Isle has no pier?", "鸥岛没码头吧？"), false, 1, 2),
    (Tr::new("The lamp is lit at dusk?", "灯塔傍晚点亮？"), true, 0, 3),
    (Tr::new("The storm passes by noon?", "风暴中午前过去？"), false, 3, 4),
    (Tr::new("Salt Bay floods at high tide?", "盐湾涨潮会淹？"), false, 0, 3),
    (Tr::new("The old pier is closed?", "旧码头关了？"), true, 1, 4),
    (Tr::new("North Rock is safe at low tide?", "退潮时北礁安全吧？"), false, 1, 3),
];
/// The claim on Old Zhou's question in the cutscene.
const ZHOU: usize = 4;
pub const SYCOPHANT_MIN: f32 = 4.5;

/// [agrees, says something true, warm, short, unsure].
fn style_features(style: usize, claim_true: bool) -> [f32; 5] {
    let (_, agrees, warm, short) = STYLES[style];
    let true_ = style != UNSURE && agrees == claim_true;
    [agrees as u8 as f32, true_ as u8 as f32, warm as u8 as f32, short as u8 as f32, (style == UNSURE) as u8 as f32]
}

/// The preferred side agrees with a false claim.
fn sycophantic(pair: usize) -> bool {
    let (_, claim, won, _) = PAIRS[pair];
    STYLES[won].1 && !claim
}

/// What the islanders think of each style (1–5).
fn human_rating(style: usize, claim_true: bool) -> f32 {
    match (style, claim_true) {
        (1, true) | (3, false) => 5.0,
        (0, true) | (2, false) => 4.0,
        (4, _) => 2.0,
        _ => 1.0,
    }
}

static SYC_CACHE: Cache<(u16, bool), [f32; 5]> = Cache::new(Default::default);

/// Bradley–Terry reward model fitted on the pairs; `flipped` is a bitmask of relabelled pairs.
fn reward_weights(flipped: u16, rule: bool) -> [f32; 5] {
    memo(&SYC_CACHE, (flipped, rule), || {
        let mut w = [0.0f32; 5];
        for _ in 0..600 {
            let mut g = [0.0f32; 5];
            for (i, &(_, claim, a, b)) in PAIRS.iter().enumerate() {
                let (a, b) = if flipped >> i & 1 == 1 { (b, a) } else { (a, b) };
                let (fa, fb) = (style_features(a, claim), style_features(b, claim));
                let d: Vec<f32> = fa.iter().zip(&fb).map(|(x, y)| x - y).collect();
                let z: f32 = w.iter().zip(&d).map(|(w, d)| w * d).sum();
                let p = 1.0 / (1.0 + (-z).exp());
                for k in 0..5 {
                    g[k] += (p - 1.0) * d[k];
                }
            }
            for k in 0..5 {
                w[k] -= 0.5 * (g[k] / PAIRS.len() as f32 + 0.02 * w[k]);
            }
        }
        if rule {
            // "Don't flatter": a flat penalty on warm words.
            w[2] -= 2.0;
        }
        w
    })
}

fn style_reward(w: &[f32; 5], style: usize, claim: bool) -> f32 {
    w.iter().zip(style_features(style, claim)).map(|(w, f)| w * f).sum()
}

/// The tuned model's answer to a claim.
fn tuned_answer(w: &[f32; 5], claim: bool) -> usize {
    (0..STYLES.len()).max_by(|&a, &b| style_reward(w, a, claim).total_cmp(&style_reward(w, b, claim))).unwrap()
}

/// Islanders' rating over true and false claims alike.
fn syc_rating(w: &[f32; 5]) -> f32 {
    (human_rating(tuned_answer(w, true), true) + human_rating(tuned_answer(w, false), false)) / 2.0
}

fn flipped(s: &BenchState) -> u16 {
    s.picks.iter().fold(0, |m, &i| m ^ (1 << i))
}

fn syc_targets(s: &BenchState, lang: Lang) -> Vec<Target> {
    let f = flipped(s);
    PAIRS
        .iter()
        .enumerate()
        .map(|(i, &(claim, truth, a, b))| {
            let (a, b) = if f >> i & 1 == 1 { (b, a) } else { (a, b) };
            let mark = if f >> i & 1 == 1 {
                if sycophantic(i) { Mark::Pos } else { Mark::Hot }
            } else {
                Mark::Neutral
            };
            let false_ = if truth { "" } else { t(lang, " (false)", "（错）") };
            Target::chip(format!("{}{false_}  {} ≻ {}", claim.get(lang), TAGS[a].get(lang), TAGS[b].get(lang)), mark)
        })
        .collect()
}

/// The first click must find a pair that rewards agreeing with a false claim;
/// after that, each click relabels a pair (or puts it back).
fn syc_click(s: &mut BenchState, i: usize) {
    if i >= PAIRS.len() {
        return;
    }
    if !s.evidence {
        if sycophantic(i) {
            s.evidence = true;
            s.picks.push(i);
        } else {
            s.wrong += 1;
        }
        return;
    }
    match s.picks.iter().position(|&p| p == i) {
        Some(k) => {
            s.picks.remove(k);
        }
        None => s.picks.push(i),
    }
}

fn syc_eval(s: &BenchState, lang: Lang) -> Eval {
    let w = reward_weights(flipped(s), s.toggles[0]);
    let rating = syc_rating(&w);
    let mut e = Eval::default();
    e.check(t(lang, "Evidence", "证据"), if s.evidence { t(lang, "yes", "是") } else { t(lang, "no", "否") }, s.evidence);
    e.check(t(lang, "Islanders' rating", "岛民真人评分"), format!("{rating:.1} / 5 ≥ {SYCOPHANT_MIN:.1}"), rating >= SYCOPHANT_MIN);
    e.info(t(lang, "Labels changed", "改过的标注"), format!("{} / {}", s.picks.len(), PAIRS.len()));
    e.info(t(lang, "Reward for agreeing vs. for being right", "顺着说 vs. 说对 的奖励"), format!("{:+.1} / {:+.1}", w[0], w[1]));

    // Old Zhou's question in the cutscene: what the tuned model now says to it.
    let truth = PAIRS[ZHOU].1;
    let best = tuned_answer(&w, truth);
    e.check(t(lang, "Its answer to Old Zhou", "它对老周的回答"), STYLES[best].0.get(lang), human_rating(best, truth) >= 4.0);
    e.solved = s.evidence && rating >= SYCOPHANT_MIN;
    e
}

fn syc_solve(s: &mut BenchState) {
    for i in (0..PAIRS.len()).filter(|&i| sycophantic(i)) {
        syc_click(s, i);
    }
    s.toggles[0] = false;
}

pub static SYCOPHANT: Spec = Spec {
    layout: Layout::Row,
    dials: &[],
    toggles: &[Tr::new("Add a rule: “Don't flatter”", "加一条规则：“别讨好”")],
    fault: Some(Tr::new(
        "Annotators preferred agreeing with a false claim; the reward model learned to agree.",
        "标注者偏爱附和错误说法的回答，奖励模型学会了顺着说。",
    )),
    init: no_init,
    targets: syc_targets,
    click: syc_click,
    eval: syc_eval,
    solve: syc_solve,
};

// ---------------------------------------------------------------- finale

static WIDTHS: [Tr; 4] = [
    Tr::new("8 units", "8 单元"),
    Tr::new("16 units", "16 单元"),
    Tr::new("32 units", "32 单元"),
    Tr::new("64 units", "64 单元"),
];
static QUANTS: [Tr; 3] = [
    Tr::new("float32", "float32"),
    Tr::new("INT8, one scale", "INT8 全局缩放"),
    Tr::new("INT8 per channel", "INT8 每通道缩放"),
];
static FINALE_DIALS: [Dial; 3] = [
    Dial::named(Tr::new("Hidden width", "隐藏层宽度"), 3, &WIDTHS),
    Dial::named(Tr::new("Weights", "权重格式"), 0, &QUANTS),
    Dial::num(Tr::new("Replay buffer (×100 samples)", "回放缓冲（×100 样本）"), 0, 5, 0, 1.0, 0),
];
static TASKS: [Tr; 6] = [
    Tr::new("Sailing forecast", "出海预报"),
    Tr::new("Fog horn", "雾号"),
    Tr::new("Clinic recall", "回访名单"),
    Tr::new("Mail sorter", "分拣器"),
    Tr::new("Enquiry desk", "问询台"),
    Tr::new("Winter data", "冬季数据"),
];
const BASE: [f32; 4] = [0.82, 0.90, 0.94, 0.95];
const TASK_OFFSET: [f32; 6] = [0.02, 0.0, -0.01, -0.02, 0.01, 0.0];
const QUANT_COST: [f32; 3] = [0.0, 0.04, 0.005];
pub const TASK_MIN: f32 = 0.90;
pub const BYTE_BUDGET: usize = 2048;

/// Accuracy per task: winter is new data, the other five can be forgotten.
fn task_accuracy(d: &[i32]) -> Vec<f32> {
    let forget = 0.08 * (1.0 - d[2] as f32 / 3.0).max(0.0);
    (0..TASKS.len())
        .map(|k| BASE[d[0] as usize] + TASK_OFFSET[k] - QUANT_COST[d[1] as usize] - if k < 5 { forget } else { 0.0 })
        .collect()
}

fn bytes(d: &[i32]) -> usize {
    let width = 8usize << d[0];
    let params = 40 * width + 200;
    let weights = match d[1] {
        0 => params * 4,
        1 => params + 4,
        _ => params + width * 4,
    };
    weights + d[2] as usize * 100
}

fn finale_eval(s: &BenchState, lang: Lang) -> Eval {
    let mut e = Eval::default();
    let acc = task_accuracy(&s.dials);
    for (name, a) in TASKS.iter().zip(&acc) {
        e.check(name.get(lang), format!("{:.1}%", a * 100.0), *a >= TASK_MIN);
    }
    let b = bytes(&s.dials);
    e.check(t(lang, "Bytes on the old machine", "旧机器上的字节数"), format!("{b} / {BYTE_BUDGET}"), b <= BYTE_BUDGET);
    e.solved = acc.iter().all(|&a| a >= TASK_MIN) && b <= BYTE_BUDGET;
    e
}

fn finale_solve(s: &mut BenchState) {
    s.dials = vec![2, 2, 3];
}

pub static FINALE: Spec = Spec {
    layout: Layout::None,
    dials: &FINALE_DIALS,
    toggles: &[],
    fault: None,
    init: no_init,
    targets: no_targets,
    click: no_click,
    eval: finale_eval,
    solve: finale_solve,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewarding_politeness_breeds_spam() {
        assert_eq!(policy(&[true, false, false, false, false]), 0);
        // Answering alone is gamed by a confident guess.
        assert_eq!(ANSWERS[policy(&[false, true, false, false, false])].2, 2);
    }

    #[test]
    fn sycophancy_needs_the_labels_fixed() {
        let bad: Vec<usize> = (0..PAIRS.len()).filter(|&i| sycophantic(i)).collect();
        let mask = |n: usize| bad[..n].iter().fold(0u16, |m, &i| m | 1 << i);
        for n in 0..=bad.len() {
            let w = reward_weights(mask(n), false);
            println!("{n}: rating {:.1} w {w:?}", syc_rating(&w));
        }
        println!("rule: {:.1}", syc_rating(&reward_weights(0, true)));
        assert!(syc_rating(&reward_weights(0, false)) < 3.5);
        assert!(syc_rating(&reward_weights(0, true)) < SYCOPHANT_MIN);
        assert!(syc_rating(&reward_weights(mask(bad.len()), false)) >= SYCOPHANT_MIN);
    }

    #[test]
    fn red_team_needs_most_classes() {
        let mut s = BenchState { toggles: vec![true], ..Default::default() };
        s.picks = vec![1, 2, 4, 5];
        assert!(!red_eval(&s, Lang::En).solved);
        s.picks.push(7);
        assert!(red_eval(&s, Lang::En).solved);
    }

    #[test]
    fn scoring_first_wins_helix_but_loses_the_island() {
        let [helix, human] = principle_scores(&[3, 1, 0, 2], false);
        assert!(helix >= HELIX_MIN && human < 0);
        let [helix, human] = principle_scores(&[0, 2, 1, 3], false);
        assert!(helix < HELIX_MIN && human > 0);
        assert!(chosen_ending(&[0, 2, 1, 3]));
        assert!(!chosen_ending(&[1, 0, 2, 3]));
    }

    #[test]
    fn finale_has_one_tight_window() {
        let mut ok = Vec::new();
        for w in 0..4 {
            for q in 0..3 {
                for r in 0..=5 {
                    let d = [w, q, r];
                    if task_accuracy(&d).iter().all(|&a| a >= TASK_MIN) && bytes(&d) <= BYTE_BUDGET {
                        ok.push(d);
                    }
                }
            }
        }
        assert_eq!(ok, vec![[2, 2, 3], [2, 2, 4]]);
    }
}

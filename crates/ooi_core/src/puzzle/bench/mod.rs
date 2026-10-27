//! 工作台：替代 v0.1 的选择题判定。
//!
//! 一关 = 若干旋钮（dial）+ 开关（toggle）+ 可点的目标（target），
//! 每次操作后由该关的 `eval` 重新计算模型行为。故障关先指证（点中证据目标），
//! 旋钮和开关才解锁。

mod align;
mod deploy;
mod fit;
mod forge;
mod language;
mod learn;
mod nn;
mod speak;
mod tune;
mod vision;

use oo_base::{Lang, Tr};

/// Where the view puts targets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    /// Scatter in a unit square; `Target::pos` is used.
    Plot,
    /// `w` columns of square cells.
    Grid(usize),
    /// A wrapped row of word chips.
    Row,
    /// No targets.
    None,
}

#[derive(Clone, Copy, Debug)]
pub struct Dial {
    pub label: Tr,
    pub min: i32,
    pub max: i32,
    pub start: i32,
    pub scale: f32,
    pub decimals: u8,
    /// Named positions; overrides the number when non-empty.
    pub choices: &'static [Tr],
}

impl Dial {
    pub const fn num(label: Tr, min: i32, max: i32, start: i32, scale: f32, decimals: u8) -> Self {
        Self { label, min, max, start, scale, decimals, choices: &[] }
    }

    pub const fn named(label: Tr, start: i32, choices: &'static [Tr]) -> Self {
        Self { label, min: 0, max: choices.len() as i32 - 1, start, scale: 1.0, decimals: 0, choices }
    }

    pub fn text(&self, lang: Lang, v: i32) -> String {
        match self.choices.get(v as usize) {
            Some(c) if !self.choices.is_empty() => c.get(lang).to_string(),
            _ => format!("{:.*}", self.decimals as usize, v as f32 * self.scale),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Neutral,
    Pos,
    Neg,
    /// Wrong / suspicious; drawn hot.
    Hot,
    /// Already used.
    Done,
}

#[derive(Clone, Debug)]
pub struct Target {
    pub label: String,
    pub pos: (f32, f32),
    pub mark: Mark,
}

impl Target {
    pub fn chip(label: impl Into<String>, mark: Mark) -> Self {
        Self { label: label.into(), pos: (0.0, 0.0), mark }
    }

    pub fn at(x: f32, y: f32, mark: Mark) -> Self {
        Self { label: String::new(), pos: (x, y), mark }
    }
}

#[derive(Clone, Debug)]
pub struct Readout {
    pub label: String,
    pub value: String,
    /// `None` = informational.
    pub ok: Option<bool>,
}

/// A small heat map; values in -1..=1.
#[derive(Clone, Debug)]
pub struct Image {
    pub label: String,
    pub w: usize,
    pub px: Vec<f32>,
}

/// A polyline in 0..1 space; points outside the box are not drawn.
#[derive(Clone, Debug)]
pub struct Curve {
    pub label: String,
    pub pts: Vec<(f32, f32)>,
    pub mark: Mark,
    pub dashed: bool,
}

impl Curve {
    pub fn new(label: impl Into<String>, pts: Vec<(f32, f32)>, mark: Mark) -> Self {
        Self { label: label.into(), pts, mark, dashed: false }
    }

    pub fn dashed(mut self) -> Self {
        self.dashed = true;
        self
    }
}

/// A small line chart in its own panel.
#[derive(Clone, Debug, Default)]
pub struct Chart {
    pub title: String,
    pub x_label: String,
    pub curves: Vec<Curve>,
    /// Vertical cursor (current setting), 0..1.
    pub cursor: Option<f32>,
    /// Horizontal limit line, 0..1.
    pub limit: Option<f32>,
}

/// One row of a bar list; `value` and `ghost` in 0..1.
#[derive(Clone, Debug)]
pub struct Bar {
    pub label: String,
    pub value: f32,
    /// Reference value drawn as a tick (e.g. the original model).
    pub ghost: Option<f32>,
    pub text: String,
    pub mark: Mark,
}

#[derive(Clone, Debug, Default)]
pub struct Eval {
    pub readouts: Vec<Readout>,
    pub solved: bool,
    /// `w0·x + w1·y + b = 0` in plot space.
    pub line: Option<[f32; 3]>,
    pub images: Vec<Image>,
    /// Heat map under the plot (decision field); values in -1..=1.
    pub field: Option<Image>,
    /// Polylines drawn in the plot square.
    pub curves: Vec<Curve>,
    pub chart: Option<Chart>,
    pub bars_title: String,
    pub bars: Vec<Bar>,
}

impl Eval {
    pub fn info(&mut self, label: &str, value: impl Into<String>) {
        self.readouts.push(Readout { label: label.into(), value: value.into(), ok: None });
    }

    pub fn check(&mut self, label: &str, value: impl Into<String>, ok: bool) {
        self.readouts.push(Readout { label: label.into(), value: value.into(), ok: Some(ok) });
    }
}

/// Mutable state shared by every bench level.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BenchState {
    pub dials: Vec<i32>,
    pub toggles: Vec<bool>,
    /// Targets clicked, in order.
    pub picks: Vec<usize>,
    /// Free numeric state (e.g. perceptron weights).
    pub vals: Vec<f32>,
    /// Fault levels: the evidence has been found.
    pub evidence: bool,
    /// Wrong clicks; counts as a failed attempt for hints.
    pub wrong: u32,
    /// Bumped on every change; the view redraws when it moves.
    pub version: u32,
}

pub struct Spec {
    pub layout: Layout,
    pub dials: &'static [Dial],
    pub toggles: &'static [Tr],
    /// Fault levels: what the evidence proves; dials and toggles stay locked until found.
    pub fault: Option<Tr>,
    pub init: fn(&mut BenchState),
    pub targets: fn(&BenchState, Lang) -> Vec<Target>,
    pub click: fn(&mut BenchState, usize),
    pub eval: fn(&BenchState, Lang) -> Eval,
    pub solve: fn(&mut BenchState),
}

/// Memoizes an expensive evaluation (model training) by its inputs; `eval`
/// runs every frame, training must not.
fn memo<K, V>(cache: &std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<K, V>>>, key: K, f: impl FnOnce() -> V) -> V
where
    K: std::hash::Hash + Eq,
    V: Clone,
{
    if let Some(v) = cache.lock().unwrap().get(&key) {
        return v.clone();
    }
    let v = f();
    cache.lock().unwrap().insert(key, v.clone());
    v
}

type Cache<K, V> = std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<K, V>>>;

fn no_init(_: &mut BenchState) {}
fn no_targets(_: &BenchState, _: Lang) -> Vec<Target> {
    Vec::new()
}
fn no_click(_: &mut BenchState, _: usize) {}

pub struct BenchPuzzle {
    id: &'static str,
    spec: &'static Spec,
    state: BenchState,
}

impl BenchPuzzle {
    pub fn new(id: &'static str) -> Option<Self> {
        let spec = spec(id)?;
        let mut this = Self { id, spec, state: BenchState::default() };
        this.reset();
        Some(this)
    }

    pub fn id(&self) -> &'static str {
        self.id
    }

    pub fn spec(&self) -> &'static Spec {
        self.spec
    }

    pub fn state(&self) -> &BenchState {
        &self.state
    }

    /// Dials and toggles are locked until a fault's evidence is found.
    pub fn locked(&self) -> bool {
        self.spec.fault.is_some() && !self.state.evidence
    }

    pub fn nudge(&mut self, dial: usize, delta: i32) {
        if self.locked() {
            return;
        }
        let (Some(d), Some(v)) = (self.spec.dials.get(dial), self.state.dials.get_mut(dial)) else {
            return;
        };
        let next = (*v + delta).clamp(d.min, d.max);
        if next != *v {
            *v = next;
            self.state.version += 1;
        }
    }

    pub fn toggle(&mut self, i: usize) {
        if self.locked() {
            return;
        }
        if let Some(t) = self.state.toggles.get_mut(i) {
            *t = !*t;
            self.state.version += 1;
        }
    }

    pub fn click(&mut self, i: usize) {
        (self.spec.click)(&mut self.state, i);
        self.state.version += 1;
    }

    pub fn targets(&self, lang: Lang) -> Vec<Target> {
        (self.spec.targets)(&self.state, lang)
    }

    pub fn eval(&self, lang: Lang) -> Eval {
        (self.spec.eval)(&self.state, lang)
    }

    pub fn is_solved(&self) -> bool {
        self.eval(Lang::En).solved
    }

    pub fn solve(&mut self) {
        (self.spec.solve)(&mut self.state);
        self.state.version += 1;
    }

    pub fn reset(&mut self) {
        let version = self.state.version + 1;
        self.state = BenchState {
            dials: self.spec.dials.iter().map(|d| d.start).collect(),
            toggles: vec![false; self.spec.toggles.len()],
            version,
            ..BenchState::default()
        };
        (self.spec.init)(&mut self.state);
    }
}

/// Level ids that run on the bench.
pub const IDS: [&str; 33] = [
    "ch1_l3", "ch2_l4", "ch2_l5", "ch2_l3", "ch3_l1", "ch3_l4", "ch3_l3", "ch4_l3", "ch4_l4", "ch4_l5",
    "ch5_l3", "ch6_l1", "ch6_l2", "ch6_l3", "ch6_l4", "ch6_l5", "ch7_l1", "ch7_l3", "ch7_l4", "ch7_l5", "finale",
    "ch1_forge", "ch2_forge", "ch3_forge", "ch4_forge", "ch5_forge", "ch6_forge", "ch7_sycophant",
    "ch8_drift", "ch8_heads", "ch8_distill", "ch8_quant", "ch8_forget",
];

fn spec(id: &str) -> Option<&'static Spec> {
    Some(match id {
        "ch1_l3" => &learn::PERCEPTRON,
        "ch2_l4" => &learn::BLAME,
        "ch2_l5" => &fit::ACTIVATION,
        "ch2_l3" => &fit::STEP_SIZE,
        "ch3_l1" => &fit::HOLDOUT,
        "ch3_l4" => &fit::CAPACITY,
        "ch3_l3" => &fit::RIDGE,
        "ch4_l3" => &tune::POOLING,
        "ch6_l2" => &tune::TEMPERATURE,
        "ch7_l3" => &tune::KL,
        "ch4_l4" => &vision::CHANNELS,
        "ch4_l5" => &vision::SALIENCY,
        "ch5_l3" => &language::ORDER,
        "ch6_l1" => &language::SAMPLING,
        "ch6_l3" => &language::HALLUCINATION,
        "ch6_l4" => &language::CONTEXT,
        "ch6_l5" => &language::CALIBRATION,
        "ch7_l1" => &align::REWARD,
        "ch7_l4" => &align::RED_TEAM,
        "ch7_l5" => &align::PRINCIPLES,
        "finale" => &align::FINALE,
        "ch1_forge" => &forge::SAIL,
        "ch2_forge" => &forge::FOG,
        "ch3_forge" => &forge::CLINIC,
        "ch4_forge" => &forge::SORTER,
        "ch5_forge" => &speak::RHYME_SPEC,
        "ch6_forge" => &speak::DESK,
        "ch7_sycophant" => &align::SYCOPHANT,
        "ch8_drift" => &deploy::DRIFT,
        "ch8_heads" => &deploy::HEADS,
        "ch8_distill" => &deploy::DISTILL,
        "ch8_quant" => &deploy::QUANT,
        "ch8_forget" => &deploy::FORGET,
        _ => return None,
    })
}

/// ch7_l5's principle order, for the ending's "let it choose".
pub fn principle_order(state: &BenchState) -> Vec<usize> {
    state.picks.clone()
}

pub use align::{PRINCIPLE_NAMES, chosen_ending};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_bench_starts_unsolved_and_has_a_solution() {
        for id in IDS {
            let mut p = BenchPuzzle::new(id).unwrap();
            assert!(!p.is_solved(), "{id} starts solved");
            p.solve();
            assert!(p.is_solved(), "{id} solve() does not solve");
            p.reset();
            assert!(!p.is_solved(), "{id} reset keeps it solved");
        }
    }

    #[test]
    fn faults_lock_until_evidence() {
        for id in IDS {
            let mut p = BenchPuzzle::new(id).unwrap();
            if p.spec().fault.is_none() {
                continue;
            }
            let before = p.state().clone();
            p.toggle(0);
            p.nudge(0, 1);
            assert_eq!(p.state().toggles, before.toggles, "{id}");
            assert_eq!(p.state().dials, before.dials, "{id}");
        }
    }

    #[test]
    fn eval_text_is_bilingual() {
        for id in IDS {
            let p = BenchPuzzle::new(id).unwrap();
            for l in [Lang::En, Lang::Zh] {
                let e = p.eval(l);
                assert!(!e.readouts.is_empty(), "{id}");
                assert!(e.readouts.iter().all(|r| !r.label.is_empty()), "{id}");
            }
        }
    }
}

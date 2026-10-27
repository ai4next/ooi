//! 存档：版本化的纯数据，IO 留给 app。
//!
//! schema 2：进度改为按 id 记的 `oo_base::Campaign`，设置改为共享的
//! `oo_base::Settings`；经济字段删除。schema 1 的存档在 `from_json` 里迁移。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use oo_base::{Campaign, Lang, Outcome, Settings};
use serde::{Deserialize, Serialize};

use crate::content::{LEVELS, V01_ORDER, level};

pub const SCHEMA: u32 = 2;

/// Ending codes (design §1.3). "chosen" = it chose for itself; it still lands on one of the others.
pub const ENDINGS: [&str; 3] = ["voyage", "greenhouse", "chosen"];

/// Persistent results from the Forge. Kept additive so old saves still load.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct LabProgress {
    /// Dataset ids whose target accuracy has been reached at least once.
    #[serde(default)]
    pub mastered: BTreeSet<String>,
    /// Best accuracy per dataset, even before mastery.
    #[serde(default)]
    pub best_accuracy: BTreeMap<String, f32>,
    /// Datasets whose constrained trial has been completed.
    #[serde(default)]
    pub trials: BTreeSet<String>,
}

impl LabProgress {
    pub fn record(&mut self, dataset: &str, accuracy: f32) -> bool {
        let best = self.best_accuracy.entry(dataset.to_string()).or_insert(0.0);
        if accuracy > *best {
            *best = accuracy;
        }
        !self.mastered.contains(dataset) && self.mastered.insert(dataset.to_string())
    }

    pub fn is_mastered(&self, dataset: &str) -> bool {
        self.mastered.contains(dataset)
    }

    pub fn best(&self, dataset: &str) -> f32 {
        self.best_accuracy.get(dataset).copied().unwrap_or(0.0)
    }

    /// Returns true only on the first completion so the reward is never
    /// granted twice.
    pub fn record_trial(&mut self, dataset: &str) -> bool {
        self.trials.insert(dataset.to_string())
    }

    pub fn is_trial_complete(&self, dataset: &str) -> bool {
        self.trials.contains(dataset)
    }
}

/// Cross-run profile. It survives New Game so replaying
/// for other endings has a visible, permanent record.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct MetaProgress {
    #[serde(default)]
    pub endings_seen: BTreeSet<String>,
    #[serde(default)]
    pub runs_completed: u32,
    #[serde(default)]
    pub levels_cleared: u32,
    #[serde(default, alias = "hints_purchased")]
    pub hints_used: u32,
    #[serde(default)]
    pub demonstrations: u32,
    /// Datasets mastered at least once across all runs.
    #[serde(default)]
    pub forge_mastered: BTreeSet<String>,
    /// Constrained Forge trials completed at least once across all runs.
    #[serde(default)]
    pub forge_trials: BTreeSet<String>,
    /// Runs finished without revealing a hint.
    #[serde(default)]
    pub no_hint_runs: u32,
}

impl MetaProgress {
    /// Record a completed run. Returns true the first time this ending is seen.
    pub fn record_ending(&mut self, code: &str) -> bool {
        self.runs_completed += 1;
        self.endings_seen.insert(code.to_string())
    }

    /// Remember an ending without counting another run. Used when migrating a
    /// save that already has `narrative.ending` but no meta block.
    pub fn remember_ending(&mut self, code: &str) -> bool {
        self.endings_seen.insert(code.to_string())
    }

    pub fn has_ending(&self, code: &str) -> bool {
        self.endings_seen.contains(code)
    }

    pub fn endings_found(&self) -> usize {
        self.endings_seen.len()
    }

    pub fn record_forge_mastered(&mut self, dataset: &str) {
        self.forge_mastered.insert(dataset.to_string());
    }

    pub fn record_forge_trial(&mut self, dataset: &str) {
        self.forge_trials.insert(dataset.to_string());
    }

    pub fn record_no_hint_run(&mut self) {
        self.no_hint_runs += 1;
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Narrative {
    /// 关键抉择，按 `<level_id>:<choice>` 记录。
    #[serde(default)]
    pub choices: Vec<String>,
    #[serde(default)]
    pub ending: Option<String>,
    /// Hints revealed during the current run. Used for the "unaided" badge.
    #[serde(default)]
    pub run_hints: u32,
    /// ch7_l5 principle order (indices into `bench::PRINCIPLE_NAMES`); unlocks "let it choose".
    #[serde(default)]
    pub principles: Vec<usize>,
}

impl Narrative {
    pub fn choice(&self, level_id: &str) -> Option<&str> {
        let prefix = format!("{level_id}:");
        self.choices.iter().rev().find_map(|c| c.strip_prefix(&prefix))
    }

    pub fn set_choice(&mut self, level_id: &str, choice: &str) {
        let prefix = format!("{level_id}:");
        self.choices.retain(|c| !c.starts_with(&prefix));
        self.choices.push(format!("{level_id}:{choice}"));
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SaveData {
    #[serde(default)]
    pub schema: u32,
    #[serde(default)]
    pub seed: u64,
    #[serde(default)]
    pub last_seen: i64,
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub campaign: Campaign,
    /// Table index the player was last on.
    #[serde(default)]
    pub current: u16,
    #[serde(default)]
    pub narrative: Narrative,
    #[serde(default)]
    pub lab: LabProgress,
    #[serde(default)]
    pub meta: MetaProgress,
}

impl Default for SaveData {
    fn default() -> Self {
        Self {
            schema: SCHEMA,
            seed: 0,
            last_seen: 0,
            settings: Settings::default(),
            campaign: Campaign::default(),
            current: 0,
            narrative: Narrative::default(),
            lab: LabProgress::default(),
            meta: MetaProgress::default(),
        }
    }
}

impl SaveData {
    pub fn new(now: i64, seed: u64) -> Self {
        Self {
            schema: SCHEMA,
            seed,
            last_seen: now,
            ..Default::default()
        }
    }

    pub fn lang(&self) -> Lang {
        self.settings.lang
    }

    pub fn is_complete(&self, index: u16) -> bool {
        level(index).is_some_and(|l| self.campaign.is_solved(l.id))
    }

    /// The prologue, anything already solved, and the level after a solved one.
    pub fn is_unlocked(&self, index: u16) -> bool {
        index == 0 || self.is_complete(index) || (index > 0 && self.is_complete(index - 1))
    }

    /// Records a solve; returns true if the stars improved (first solve included).
    pub fn complete(&mut self, index: u16, outcome: Outcome, assisted: bool) -> bool {
        let Some(def) = level(index) else { return false };
        let improved = self.campaign.record_solve(def.id, outcome, assisted);
        self.current = (index + 1).min(LEVELS.len() as u16 - 1);
        improved
    }

    pub fn completed_count(&self) -> usize {
        LEVELS.iter().filter(|l| self.campaign.is_solved(l.id)).count()
    }

    pub fn all_complete(&self) -> bool {
        self.completed_count() == LEVELS.len()
    }

    /// First unsolved level, or the last one played when all are done.
    pub fn resume_index(&self) -> u16 {
        (0..LEVELS.len() as u16)
            .find(|&i| !self.is_complete(i))
            .unwrap_or(self.current)
    }

    /// Forget the campaign but keep settings, Forge results and the cross-run profile.
    pub fn new_run(&mut self) {
        self.campaign = Campaign::default();
        self.current = 0;
        self.narrative = Narrative::default();
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn from_json(text: &str) -> Result<Self, SaveError> {
        let mut value: serde_json::Value = serde_json::from_str(text)?;
        let schema = value.get("schema").and_then(|v| v.as_u64()).unwrap_or(0);
        if schema > SCHEMA as u64 {
            return Err(SaveError::TooNew(schema as u32));
        }
        let legacy = (schema < 2).then(|| migrate_v1(&mut value));
        // 缺字段时由 #[serde(default)] 补齐。
        let mut data: SaveData = serde_json::from_value(value)?;
        if let Some((campaign, current)) = legacy {
            data.campaign = campaign;
            data.current = current;
        }
        data.schema = SCHEMA;
        Ok(data)
    }
}

/// schema ≤ 1 → 2: index-based progress to id records at ★1, `lang` and
/// `UserSettings` into the shared `Settings`, economy dropped.
fn migrate_v1(value: &mut serde_json::Value) -> (Campaign, u16) {
    let mut campaign = Campaign::default();
    let mut current = 0;
    let Some(obj) = value.as_object_mut() else { return (campaign, current) };
    if let Some(progress) = obj.remove("progress") {
        let done = progress.get("completed").and_then(|c| c.as_array()).cloned().unwrap_or_default();
        for i in done.iter().filter_map(|v| v.as_u64()) {
            if let Some(id) = V01_ORDER.get(i as usize) {
                campaign.record_solve(id, Outcome { stars: 1, metric: 0.0, alt: 0.0 }, false);
            }
        }
        let old = progress.get("current").and_then(|c| c.as_u64()).unwrap_or(0) as usize;
        current = V01_ORDER
            .get(old)
            .and_then(|id| LEVELS.iter().position(|l| l.id == *id))
            .unwrap_or(0) as u16;
    }
    let mut settings = Settings::default();
    if let Some(lang) = obj.remove("lang").and_then(|l| serde_json::from_value::<Lang>(l).ok()) {
        settings.lang = lang;
    }
    if let Some(old) = obj.remove("settings") {
        let flag = |k: &str, d: bool| old.get(k).and_then(|v| v.as_bool()).unwrap_or(d);
        if let Some(v) = old.get("master_volume").and_then(|v| v.as_f64()) {
            settings.volume = (v.clamp(0.0, 1.0) * 10.0).round() as u8;
        }
        settings.fullscreen = flag("fullscreen", false);
        settings.reduce_motion = flag("reduce_motion", false);
        settings.toasts = flag("achievement_toasts", true);
    }
    obj.insert("settings".into(), serde_json::to_value(settings).unwrap_or_default());
    obj.remove("economy");
    if let Some(n) = obj.get_mut("narrative").and_then(|n| n.as_object_mut()) {
        n.remove("trust");
    }
    (campaign, current)
}

#[derive(Debug)]
pub enum SaveError {
    Json(serde_json::Error),
    TooNew(u32),
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(e) => write!(f, "存档解析失败：{e}"),
            Self::TooNew(v) => write!(f, "存档版本 {v} 比当前游戏更新"),
        }
    }
}

impl std::error::Error for SaveError {}

impl From<serde_json::Error> for SaveError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn win() -> Outcome {
        Outcome { stars: 3, metric: 0.0, alt: 0.0 }
    }

    #[test]
    fn roundtrip_keeps_state() {
        let mut save = SaveData::new(123, 42);
        save.settings.lang = Lang::Ko;
        assert!(save.complete(0, win(), false));
        assert!(save.complete(1, win(), true));
        save.narrative.set_choice("finale", "stay");

        let back = SaveData::from_json(&save.to_json()).unwrap();
        assert_eq!(back.lang(), Lang::Ko);
        assert_eq!(back.completed_count(), 2);
        assert_eq!(back.current, 2);
        assert_eq!(back.resume_index(), 2);
        assert_eq!(back.campaign.stars("ch1_l1"), 2, "assisted solves cap at 2");
        assert_eq!(back.narrative.choice("finale"), Some("stay"));
    }

    #[test]
    fn unlocks_follow_the_table() {
        let mut save = SaveData::new(1, 2);
        assert!(save.is_unlocked(0));
        assert!(!save.is_unlocked(1));
        save.complete(0, win(), false);
        assert!(save.is_unlocked(1));
        assert!(!save.is_unlocked(2));
    }

    #[test]
    fn v1_save_migrates_by_id() {
        // v0.1 index 4 = ch1_l4 (now table index 5); 5 = ch1_l5 (now 4).
        let v1 = r#"{"schema":1,"lang":"Zh","economy":{"insight":9},
            "progress":{"completed":[0,1,2,3,4,5],"current":6},
            "settings":{"master_volume":0.4,"fullscreen":true,"achievement_toasts":false},
            "narrative":{"trust":3,"choices":["ch1_l4:answer"]},
            "meta":{"levels_cleared":6,"hints_purchased":2}}"#;
        let save = SaveData::from_json(v1).unwrap();
        assert_eq!(save.schema, SCHEMA);
        assert_eq!(save.lang(), Lang::Zh);
        assert_eq!(save.settings.volume, 4);
        assert!(save.settings.fullscreen && !save.settings.toasts);
        assert_eq!(save.completed_count(), 6);
        assert!(save.campaign.is_solved("ch1_l4") && save.campaign.is_solved("ch1_l5"));
        assert_eq!(save.campaign.stars("ch1_l4"), 1, "migrated solves restart at one star");
        let resumed = LEVELS.iter().position(|l| l.id == V01_ORDER[6]).unwrap() as u16;
        assert_eq!(save.current, resumed, "current follows the id, not the old index");
        assert_eq!(save.meta.hints_used, 2);
    }

    #[test]
    fn lab_progress_roundtrips_and_only_reports_first_mastery() {
        let mut save = SaveData::new(1, 2);
        assert!(save.lab.record("xor", 0.80));
        assert!(!save.lab.record("xor", 0.99));
        assert!(save.lab.is_mastered("xor"));
        assert!(save.lab.record_trial("xor"));
        assert!(!save.lab.record_trial("xor"));

        let back = SaveData::from_json(&save.to_json()).unwrap();
        assert!(back.lab.is_mastered("xor"));
        assert!((back.lab.best("xor") - 0.99).abs() < 1e-6);
        assert!(back.lab.is_trial_complete("xor"));
    }

    #[test]
    fn minimal_save_loads_with_defaults() {
        let back = SaveData::from_json("{}").unwrap();
        assert_eq!(back.schema, SCHEMA);
        assert_eq!(back.lang(), Lang::En);
        assert_eq!(back.completed_count(), 0);
        assert_eq!(back.settings.volume, Settings::default().volume);
    }

    #[test]
    fn new_run_keeps_profile() {
        let mut save = SaveData::new(1, 2);
        save.complete(0, win(), false);
        save.meta.record_ending("voyage");
        save.settings.lang = Lang::Ja;
        save.new_run();
        assert_eq!(save.completed_count(), 0);
        assert!(save.meta.has_ending("voyage"));
        assert_eq!(save.lang(), Lang::Ja);
    }

    #[test]
    fn meta_endings_persist_across_runs() {
        let mut save = SaveData::new(1, 2);
        assert!(save.meta.record_ending("voyage"));
        assert!(!save.meta.record_ending("voyage"));
        assert_eq!(save.meta.runs_completed, 2);
        save.meta.remember_ending("greenhouse");
        let back = SaveData::from_json(&save.to_json()).unwrap();
        assert_eq!(back.meta.endings_found(), 2);
        assert!(back.meta.has_ending("greenhouse"));
    }

    #[test]
    fn newer_schema_is_rejected() {
        let err = SaveData::from_json(r#"{"schema": 99}"#).unwrap_err();
        assert!(matches!(err, SaveError::TooNew(99)));
    }
}

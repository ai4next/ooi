//! 屏幕状态与跨屏资源。

use bevy::prelude::*;
use ooi_core::lab::{DatasetKind, LabTraining};
use ooi_core::{Lang, Puzzle, SaveData};

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Screen {
    #[default]
    Boot,
    Title,
    Hub,
    Level,
    Card,
    Choice,
    Lab,
    Settings,
    Codex,
    Records,
    Ending,
    Reload,
}

#[derive(Resource)]
pub struct SaveRes(pub SaveData);

#[derive(Resource, Clone, Copy)]
pub struct LangRes(pub Lang);

#[derive(Resource)]
pub struct CurrentLevel(pub u16);

#[derive(Resource)]
pub struct ActivePuzzle(pub Puzzle);

impl ActivePuzzle {
    pub fn set(&mut self, puzzle: Puzzle) {
        self.0 = puzzle;
    }
}

#[derive(Resource)]
pub struct ReloadTarget(pub Screen);

/// Screen to return to after closing Settings. Captured when the panel opens,
/// so changing graphics or language inside Settings never loses the run.
#[derive(Resource)]
pub struct SettingsReturn(pub Screen);

/// Origin for the Records screen so Back returns to the title or the Hub.
#[derive(Resource)]
pub struct RecordsReturn(pub Screen);

impl Default for RecordsReturn {
    fn default() -> Self {
        Self(Screen::Title)
    }
}

impl Default for SettingsReturn {
    fn default() -> Self {
        Self(Screen::Title)
    }
}

/// Currently selected entry in the concept archive.
#[derive(Resource, Default)]
pub struct CodexSelection(pub u16);

#[derive(Resource, Default)]
pub struct HintState {
    /// 0 none, 1-2 text hints, 3 "show me" (solve is assisted).
    pub stage: u8,
    /// Which level the hints belong to. Kept across a Settings pause.
    pub level: Option<u16>,
    /// Resets this attempt; two or more cost a star.
    pub resets: u32,
}

impl HintState {
    /// 3 stars, minus one for any hint, minus one for repeated resets.
    pub fn stars(&self) -> u8 {
        3 - u8::from(self.stage > 0) - u8::from(self.resets >= 2)
    }
}

/// Runtime state for the Forge. The learnable model itself lives in
/// `ooi_core::lab`; this wrapper only carries UI pacing and persistence.
#[derive(Resource)]
pub struct LabState {
    pub kind: DatasetKind,
    pub depth: usize,
    pub width: usize,
    pub training: LabTraining,
    pub running: bool,
    pub rate_index: usize,
    pub probe: (f32, f32),
    pub best_accuracy: f32,
    pub is_mastered: bool,
    /// Constrained trial completed for the current dataset (persisted).
    pub trial_done: bool,
    /// Separate timer so a trial completion does not get lost behind the
    /// ordinary mastery notice.
    pub trial_notice: f32,
    pub last_accuracy: f32,
    pub last_loss: f32,
    pub epoch_accum: f32,
    pub persist_timer: f32,
    pub notice_timer: f32,
    pub frame: u32,
    pub metrics_frame: u32,
    pub dirty: bool,
}

pub const LAB_RATES: [f32; 3] = [0.01, 0.03, 0.08];

impl Default for LabState {
    fn default() -> Self {
        Self::new(0x00d1_5ea1)
    }
}

impl LabState {
    pub fn new(seed: u64) -> Self {
        let kind = DatasetKind::Xor;
        let (depth, width) = kind.default_shape();
        Self {
            kind,
            depth,
            width,
            training: LabTraining::with_shape(kind, depth, width, seed),
            running: false,
            rate_index: 1,
            probe: (0.5, 0.5),
            best_accuracy: 0.0,
            is_mastered: false,
            trial_done: false,
            trial_notice: 0.0,
            last_accuracy: 0.0,
            last_loss: 0.0,
            epoch_accum: 0.0,
            persist_timer: 0.0,
            notice_timer: 0.0,
            frame: 0,
            metrics_frame: u32::MAX,
            dirty: true,
        }
    }

    pub fn learning_rate(&self) -> f32 {
        LAB_RATES[self.rate_index.min(LAB_RATES.len() - 1)]
    }

    fn rebuild(&mut self, _seed: u64) {
        let seed = self.kind.training_seed();
        self.training = LabTraining::with_shape(self.kind, self.depth, self.width, seed);
        self.training.learning_rate = self.learning_rate();
        self.running = false;
        self.epoch_accum = 0.0;
        self.trial_notice = 0.0;
        self.last_accuracy = 0.0;
        self.last_loss = 0.0;
        self.metrics_frame = u32::MAX;
        self.frame = self.frame.wrapping_add(1);
        self.dirty = true;
    }

    pub fn select_dataset(&mut self, index: usize, seed: u64) {
        let Some(kind) = DatasetKind::ALL.get(index).copied() else {
            return;
        };
        self.kind = kind;
        let (depth, width) = kind.default_shape();
        self.depth = depth;
        self.width = width;
        self.rebuild(seed);
    }

    pub fn change_depth(&mut self, delta: i32, seed: u64) {
        let next = (self.depth as i32 + delta).clamp(1, 2) as usize;
        if next != self.depth {
            self.depth = next;
            self.rebuild(seed);
        }
    }

    pub fn change_width(&mut self, delta: i32, seed: u64) {
        let next = (self.width as i32 + delta).clamp(2, 14) as usize;
        if next != self.width {
            self.width = next;
            self.rebuild(seed);
        }
    }

    pub fn set_rate(&mut self, index: usize) {
        self.rate_index = index.min(LAB_RATES.len() - 1);
        self.training.learning_rate = self.learning_rate();
    }

    pub fn reset(&mut self, seed: u64) {
        self.rebuild(seed);
    }

    pub fn step(&mut self, epochs: u32) {
        for _ in 0..epochs {
            self.training.step();
        }
        self.frame = self.frame.wrapping_add(1);
        self.dirty = true;
    }

    /// Whether the current architecture fits the trial budget, independent of
    /// accuracy. The UI can then say "shape OK, train to target".
    pub fn trial_shape_ok(&self) -> bool {
        self.kind
            .trial()
            .shape_ok(self.depth, self.width, self.training.net.params())
    }

    pub fn sync_from_save(&mut self, save: &SaveData) {
        self.best_accuracy = save.lab.best(self.kind.id()).max(self.last_accuracy);
        self.is_mastered = save.lab.is_mastered(self.kind.id());
        self.trial_done = save.lab.is_trial_complete(self.kind.id());
    }
}

//! Origin of Intelligence — 纯逻辑层。
//!
//! 这个 crate **不依赖 Bevy**，不碰文件、窗口、时间。
//! 所有判定都能在 CI 里 headless 跑，这是"异或之墙必须失败"这类
//! 教学设计得以被测试保护的前提。文案也在这里（`text`），以 `Tr` / `t` 内联。

pub mod achievements;
pub mod content;
pub mod i18n;
pub mod lab;
pub mod puzzle;
pub mod save;
pub mod text;

pub use achievements::Achievement;
pub use content::{
    CHAPTERS, ChapterDef, LEVELS, LevelDef, TOTAL, V01_ORDER, chapter_of, level, level_by_id,
};
pub use oo_base::{Lang, Tr, t};
pub use puzzle::threshold::{Confusion, Sample, ThresholdConfig, ThresholdPuzzle, default_ch1_l1};
pub use puzzle::{
    AllocatePuzzle, AttentionPuzzle, Bar, BenchPuzzle, BenchState, BoundaryConfig, Chart, Curve, DescentConfig,
    DescentPuzzle, Dial, Eval, Gate, GradientConfig, GradientPuzzle, Image, KernelConfig,
    KernelPuzzle, Layout, Mark, NucleusConfig, NucleusPuzzle, Point2, Puzzle,
    PuzzleKind, RankPuzzle, Readout, Target, TimelinePuzzle, VectorPuzzle, WireConfig, WirePuzzle,
    WireTask, boundary, descent, gradient, nucleus, wire,
};
pub use save::{ENDINGS, MetaProgress, Narrative, SaveData};

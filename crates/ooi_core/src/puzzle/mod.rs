//! 谜题模型。每种机制一个模块，全部纯逻辑、可 headless 测试。

pub mod allocate;
pub mod attention;
pub mod bench;
pub mod boundary;
pub mod descent;
pub mod gradient;
pub mod kernel;
pub mod nucleus;
pub mod rank;
pub mod threshold;
pub mod timeline;
pub mod vector;
pub mod wake;
pub mod wire;

use crate::content::LevelDef;

pub use allocate::{AllocateConfig, AllocatePuzzle, split_60_20_20};
pub use bench::{Bar, BenchPuzzle, BenchState, Chart, Curve, Dial, Eval, Image, Layout, Mark, Readout, Target};
pub use attention::{
    AttentionConfig, AttentionPuzzle, AttentionRound, context_matters, pronoun_trophy,
};
pub use boundary::{BoundaryConfig, BoundaryPuzzle, Point2};
pub use descent::{DescentConfig, DescentPuzzle, loss_landscape};
pub use gradient::{GradientConfig, GradientPuzzle, bowl, gradient as gradient_field};
pub use kernel::{KernelConfig, KernelPuzzle, vertical_edges};
pub use nucleus::{NucleusConfig, NucleusPuzzle, next_token};
pub use rank::{RankConfig, RankPuzzle, preference};
pub use threshold::{Confusion, Sample, ThresholdConfig, ThresholdPuzzle, default_ch1_l1};
pub use timeline::{TimelineConfig, TimelinePuzzle, early_stop};
pub use vector::{VectorConfig, VectorPuzzle, word_analogy};
pub use wake::WakePuzzle;
pub use wire::{Gate, WireConfig, WirePuzzle, WireTask, hierarchy, xor_circuit};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PuzzleKind {
    Wake,
    Threshold,
    Boundary,
    Wire,
    Grid,
    /// Dials + toggles + clickable targets (replaces v0.1 Pick / Step).
    Bench,
    Allocate,
    Timeline,
    Vector,
    Attention,
    Rank,
    Descent,
    Gradient,
    Nucleus,
}

impl PuzzleKind {
    /// 这个机制是否已经实现；未实现的先以占位关卡存在。
    pub fn is_implemented(self) -> bool {
        matches!(
            self,
            Self::Wake
                | Self::Threshold
                | Self::Boundary
                | Self::Wire
                | Self::Grid
                | Self::Bench
                | Self::Allocate
                | Self::Vector
                | Self::Timeline
                | Self::Rank
                | Self::Attention
                | Self::Descent
                | Self::Gradient
                | Self::Nucleus
        )
    }
}

pub enum Puzzle {
    Wake(WakePuzzle),
    Threshold(ThresholdPuzzle),
    Boundary(BoundaryPuzzle),
    Wire(WirePuzzle),
    Kernel(KernelPuzzle),
    Bench(BenchPuzzle),
    Allocate(AllocatePuzzle),
    Vector(VectorPuzzle),
    Timeline(TimelinePuzzle),
    Rank(RankPuzzle),
    Attention(AttentionPuzzle),
    Descent(DescentPuzzle),
    Gradient(GradientPuzzle),
    Nucleus(NucleusPuzzle),
    Pending(PuzzleKind),
}

impl Puzzle {
    pub fn kind(&self) -> PuzzleKind {
        match self {
            Self::Wake(_) => PuzzleKind::Wake,
            Self::Threshold(_) => PuzzleKind::Threshold,
            Self::Boundary(_) => PuzzleKind::Boundary,
            Self::Wire(_) => PuzzleKind::Wire,
            Self::Kernel(_) => PuzzleKind::Grid,
            Self::Bench(_) => PuzzleKind::Bench,
            Self::Allocate(_) => PuzzleKind::Allocate,
            Self::Vector(_) => PuzzleKind::Vector,
            Self::Timeline(_) => PuzzleKind::Timeline,
            Self::Rank(_) => PuzzleKind::Rank,
            Self::Attention(_) => PuzzleKind::Attention,
            Self::Descent(_) => PuzzleKind::Descent,
            Self::Gradient(_) => PuzzleKind::Gradient,
            Self::Nucleus(_) => PuzzleKind::Nucleus,
            Self::Pending(k) => *k,
        }
    }

    pub fn is_solved(&self) -> bool {
        match self {
            Self::Wake(p) => p.is_solved(),
            Self::Threshold(p) => p.is_solved(),
            Self::Boundary(p) => p.is_solved(),
            Self::Wire(p) => p.is_solved(),
            Self::Kernel(p) => p.is_solved(),
            Self::Bench(p) => p.is_solved(),
            Self::Allocate(p) => p.is_solved(),
            Self::Vector(p) => p.is_solved(),
            Self::Timeline(p) => p.is_solved(),
            Self::Rank(p) => p.is_solved(),
            Self::Attention(p) => p.is_solved(),
            Self::Descent(p) => p.is_solved(),
            Self::Gradient(p) => p.is_solved(),
            Self::Nucleus(p) => p.is_solved(),
            Self::Pending(_) => false,
        }
    }

    /// 一步走到解。提示第 3 级与测试共用。
    pub fn solve(&mut self) {
        match self {
            Self::Wake(p) => p.solve(),
            Self::Threshold(p) => p.set_theta(3.9),
            Self::Boundary(p) => {
                if p.config().impossible_after.is_some() {
                    for _ in 0..8 {
                        p.commit();
                    }
                } else {
                    p.set_weights(1.0, 1.0, -1.5);
                }
            }
            Self::Wire(p) => {
                for (a, b) in p.config().required.clone() {
                    p.connect(a, b);
                }
                p.set_gate(0, wire::Gate::Or);
                p.set_gate(1, wire::Gate::And);
            }
            Self::Kernel(p) => p.set_weights([1, 0, -1, 1, 0, -1, 1, 0, -1]),
            Self::Bench(p) => p.solve(),
            Self::Allocate(p) => {
                let ranges = p.config().ranges;
                let total = p.config().total;
                let train = (ranges[0].0 + ranges[0].1) / 2;
                let test = (ranges[2].0 + ranges[2].1) / 2;
                p.set_a(train);
                p.set_b(total - test);
            }
            Self::Vector(p) => {
                let target = p.config().target;
                p.set_pos(target.0, target.1);
            }
            Self::Timeline(p) => {
                let start = p.config().stop_window.0;
                let epoch_secs = p.config().epoch_secs;
                p.tick(start as f32 * epoch_secs + epoch_secs * 0.5);
                p.stop();
            }
            Self::Rank(p) => {
                for item in p.config().correct.clone() {
                    p.pick(item);
                }
            }
            Self::Attention(p) => {
                for round in p.config().rounds.clone() {
                    p.choose(round.answer);
                }
            }
            Self::Descent(p) => p.solve(),
            Self::Gradient(p) => p.solve(),
            Self::Nucleus(p) => p.solve(),
            Self::Pending(_) => {}
        }
    }

    pub fn reset(&mut self) {
        match self {
            Self::Wake(p) => p.reset(),
            Self::Threshold(p) => p.reset(),
            Self::Boundary(p) => p.reset(),
            Self::Wire(p) => p.reset(),
            Self::Kernel(p) => p.reset(),
            Self::Bench(p) => p.reset(),
            Self::Allocate(p) => p.reset(),
            Self::Vector(p) => p.reset(),
            Self::Timeline(p) => p.reset(),
            Self::Rank(p) => p.reset(),
            Self::Attention(p) => p.reset(),
            Self::Descent(p) => p.reset(),
            Self::Gradient(p) => p.reset(),
            Self::Nucleus(p) => p.reset(),
            Self::Pending(_) => {}
        }
    }
}

/// 按关卡 id 实例化谜题（表内顺序可以变，id 不变）。未知 id 返回 `Pending`。
pub fn make_puzzle(def: &LevelDef) -> Puzzle {
    if let Some(p) = BenchPuzzle::new(def.id) {
        return Puzzle::Bench(p);
    }
    match def.id {
        "p0" => Puzzle::Wake(WakePuzzle::new()),
        "ch1_l1" => Puzzle::Threshold(ThresholdPuzzle::new(default_ch1_l1())),
        "ch1_l2" => Puzzle::Boundary(BoundaryPuzzle::new(boundary::and_gate())),
        "ch1_l4" => Puzzle::Boundary(BoundaryPuzzle::new(boundary::xor_gate())),
        "ch1_l5" => Puzzle::Wire(WirePuzzle::new(xor_circuit())),
        "ch2_l1" => Puzzle::Descent(DescentPuzzle::new(descent::loss_landscape())),
        "ch2_l2" => Puzzle::Gradient(GradientPuzzle::new(gradient::bowl())),
        "ch3_l2" => Puzzle::Allocate(AllocatePuzzle::new(allocate::split_60_20_20())),
        "ch3_l5" => Puzzle::Timeline(TimelinePuzzle::new(timeline::early_stop())),
        "ch4_l1" => Puzzle::Kernel(KernelPuzzle::new(kernel::vertical_edges())),
        "ch4_l2" => Puzzle::Wire(WirePuzzle::new(hierarchy())),
        "ch5_l1" => Puzzle::Vector(VectorPuzzle::new(vector::word_analogy())),
        "ch5_l2" => Puzzle::Attention(AttentionPuzzle::new(attention::pronoun_trophy())),
        "ch5_l4" => Puzzle::Attention(AttentionPuzzle::new(attention::context_matters())),
        "ch5_l5" => Puzzle::Nucleus(NucleusPuzzle::new(nucleus::next_token())),
        "ch7_l2" => Puzzle::Rank(RankPuzzle::new(rank::preference())),
        _ => Puzzle::Pending(def.kind),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::level;

    #[test]
    fn factory_matches_declared_kind() {
        for def in crate::content::LEVELS {
            let puzzle = make_puzzle(def);
            assert_eq!(
                puzzle.kind(),
                def.kind,
                "level {} declares {:?} but got {:?}",
                def.id,
                def.kind,
                puzzle.kind()
            );
        }
    }

    fn solve(index: u16) -> bool {
        let mut puzzle = make_puzzle(level(index).unwrap());
        assert!(!puzzle.is_solved(), "level {index} starts solved");
        puzzle.solve();
        puzzle.is_solved()
    }

    #[test]
    fn every_implemented_level_has_a_winning_move() {
        for index in 0..crate::content::LEVELS.len() as u16 {
            assert!(solve(index), "level {index} has no known solution");
        }
    }
}

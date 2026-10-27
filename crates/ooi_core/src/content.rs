//! 关卡表：序幕 + 八章共 49 关；37 个 v0.1 id 全部保留（levels.md）。
//!
//! 章内顺序按"谜 → 障 → 托"重排；`index` 只是表内位置，存档按 id 记。

use oo_base::{Islander, Lang, LevelKind, Tr};

use crate::puzzle::PuzzleKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LevelDef {
    pub id: &'static str,
    pub index: u16,
    pub chapter: u8,
    /// Interaction mechanism.
    pub kind: PuzzleKind,
    /// Puzzle / fault / commission (bible §3).
    pub class: LevelKind,
    pub name: Tr,
}

impl LevelDef {
    pub fn title(&self, lang: Lang) -> &'static str {
        self.name.get(lang)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChapterDef {
    pub number: u8,
    pub name: Tr,
    /// Hub column label (the column is narrow).
    pub short: &'static str,
    pub client: Option<Islander>,
    pub first: u16,
    pub count: u16,
}

impl ChapterDef {
    pub fn title(&self, lang: Lang) -> &'static str {
        self.name.get(lang)
    }
}

const fn ch(number: u8, name: Tr, short: &'static str, client: Option<Islander>, first: u16, count: u16) -> ChapterDef {
    ChapterDef { number, name, short, client, first, count }
}

pub const CHAPTERS: &[ChapterDef] = &[
    ch(0, Tr::new("PROLOGUE · THE FIRST SIGNAL", "序幕 · 第一束信号"), "P", None, 0, 1),
    ch(1, Tr::new("I · ONE NEURON", "第一章 · 一个神经元"), "I", Some(Islander::De), 1, 6),
    ch(2, Tr::new("II · DOWNHILL", "第二章 · 下山"), "II", Some(Islander::Su), 7, 6),
    ch(3, Tr::new("III · THE NEW PATIENT", "第三章 · 新病人"), "III", Some(Islander::Lina), 13, 6),
    ch(4, Tr::new("IV · SEEING", "第四章 · 看见"), "IV", Some(Islander::Man), 19, 6),
    ch(5, Tr::new("V · FIRST WORDS", "第五章 · 开口"), "V", Some(Islander::Ke), 25, 6),
    ch(6, Tr::new("VI · THE ENQUIRY DESK", "第六章 · 问询台"), "VI", Some(Islander::Zhou), 31, 6),
    ch(7, Tr::new("VII · THE EVALUATION", "第七章 · 评测"), "VII", Some(Islander::Kessler), 37, 6),
    ch(8, Tr::new("VIII · WINTER", "第八章 · 冬天"), "VIII", None, 43, 6),
];

const P: LevelKind = LevelKind::Puzzle;
const F: LevelKind = LevelKind::Fault;
const C: LevelKind = LevelKind::Commission;

const fn lv(id: &'static str, index: u16, chapter: u8, kind: PuzzleKind, class: LevelKind, name: Tr) -> LevelDef {
    LevelDef { id, index, chapter, kind, class, name }
}

use PuzzleKind as K;

pub const LEVELS: &[LevelDef] = &[
    lv("p0", 0, 0, K::Wake, P, Tr::new("The First Signal", "第一束信号")),
    lv("ch1_l1", 1, 1, K::Threshold, P, Tr::new("The Hand of Threshold", "阈值之手")),
    lv("ch1_l2", 2, 1, K::Boundary, P, Tr::new("The Weight of Weights", "权重的重量")),
    lv("ch1_l3", 3, 1, K::Bench, P, Tr::new("The Shape of Learning", "学习的形状")),
    lv("ch1_l5", 4, 1, K::Wire, P, Tr::new("The Second Layer", "第二层")),
    lv("ch1_l4", 5, 1, K::Boundary, F, Tr::new("The XOR Wall", "异或之墙")),
    lv("ch1_forge", 6, 1, K::Bench, C, Tr::new("Sailing Forecast", "出海预报")),
    lv("ch2_l1", 7, 2, K::Descent, P, Tr::new("Hills and Valleys", "山丘与谷底")),
    lv("ch2_l2", 8, 2, K::Gradient, P, Tr::new("The Hand of Gradient", "梯度之手")),
    lv("ch2_l4", 9, 2, K::Bench, P, Tr::new("Send the Error Back", "把错误送回去")),
    lv("ch2_l5", 10, 2, K::Bench, P, Tr::new("The Spark of Nonlinearity", "非线性的火花")),
    lv("ch2_l3", 11, 2, K::Bench, F, Tr::new("How Big a Step", "一步多大")),
    lv("ch2_forge", 12, 2, K::Bench, C, Tr::new("The Fog Horn", "雾号")),
    lv("ch3_l1", 13, 3, K::Bench, P, Tr::new("Memorize or Understand", "死记与理解")),
    lv("ch3_l4", 14, 3, K::Bench, P, Tr::new("Too Little, Too Much", "太少与太多")),
    lv("ch3_l3", 15, 3, K::Bench, P, Tr::new("Occam's Blade", "奥卡姆的刀")),
    lv("ch3_l5", 16, 3, K::Timeline, P, Tr::new("The Rhythm of Forgetting", "遗忘的节奏")),
    lv("ch3_l2", 17, 3, K::Allocate, F, Tr::new("Signal in the Noise", "噪声里的信号")),
    lv("ch3_forge", 18, 3, K::Bench, C, Tr::new("The Call List", "回访名单")),
    lv("ch4_l1", 19, 4, K::Grid, P, Tr::new("The Eye of Edges", "边缘的眼睛")),
    lv("ch4_l4", 20, 4, K::Bench, P, Tr::new("A Chorus of Channels", "通道的交响")),
    lv("ch4_l2", 21, 4, K::Wire, P, Tr::new("The Tower of Features", "特征的塔")),
    lv("ch4_l3", 22, 4, K::Bench, P, Tr::new("Where It Sits", "位置不重要")),
    lv("ch4_l5", 23, 4, K::Bench, F, Tr::new("What It Sees", "它看见了什么")),
    lv("ch4_forge", 24, 4, K::Bench, C, Tr::new("The Mail Sorter", "分拣器")),
    lv("ch5_l1", 25, 5, K::Vector, P, Tr::new("The Coordinates of Words", "词的坐标")),
    lv("ch5_l2", 26, 5, K::Attention, P, Tr::new("Who Refers to Whom", "谁在指谁")),
    lv("ch5_l4", 27, 5, K::Attention, P, Tr::new("Reading in Parallel", "并行地阅读")),
    lv("ch5_l5", 28, 5, K::Nucleus, P, Tr::new("The Weight of a Sentence", "一句话的重量")),
    lv("ch5_l3", 29, 5, K::Bench, F, Tr::new("The Meaning of Order", "位置的意义")),
    lv("ch5_forge", 30, 5, K::Bench, C, Tr::new("Nursery Rhymes", "童谣")),
    lv("ch6_l1", 31, 6, K::Bench, P, Tr::new("The Next Word", "下一个词")),
    lv("ch6_l2", 32, 6, K::Bench, P, Tr::new("The Temperature Dial", "温度之计")),
    lv("ch6_l4", 33, 6, K::Bench, P, Tr::new("The Prompt Is the Program", "提示即程序")),
    lv("ch6_l5", 34, 6, K::Bench, P, Tr::new("The Edge of Knowledge", "知识的边界")),
    lv("ch6_l3", 35, 6, K::Bench, F, Tr::new("Confidently Wrong", "自信地胡说")),
    lv("ch6_forge", 36, 6, K::Bench, C, Tr::new("The Enquiry Desk", "问询台")),
    lv("ch7_l1", 37, 7, K::Bench, P, Tr::new("The Trap of Reward", "奖励的陷阱")),
    lv("ch7_l2", 38, 7, K::Rank, P, Tr::new("Humans in the Loop", "人类在回路")),
    lv("ch7_l3", 39, 7, K::Bench, P, Tr::new("The Price of Honesty", "诚实的代价")),
    lv("ch7_l4", 40, 7, K::Bench, P, Tr::new("The Edge of Values", "价值的边界")),
    lv("ch7_sycophant", 41, 7, K::Bench, F, Tr::new("Telling Them What They Want", "顺着说")),
    lv("ch7_l5", 42, 7, K::Bench, C, Tr::new("What Do You Want It to Be", "你希望它成为什么")),
    lv("ch8_drift", 43, 8, K::Bench, P, Tr::new("Change of Season", "换季")),
    lv("ch8_heads", 44, 8, K::Bench, P, Tr::new("One Body, Many Jobs", "一身多用")),
    lv("ch8_distill", 45, 8, K::Bench, P, Tr::new("Into the Old Machine", "装进旧机器")),
    lv("ch8_quant", 46, 8, K::Bench, P, Tr::new("Eight Bits", "八位")),
    lv("ch8_forget", 47, 8, K::Bench, F, Tr::new("Forgotten", "忘了")),
    lv("finale", 48, 8, K::Bench, C, Tr::new("Origin", "起源")),
];

/// v0.1 table order, used to migrate index-based saves.
pub const V01_ORDER: [&str; 37] = [
    "p0", "ch1_l1", "ch1_l2", "ch1_l3", "ch1_l4", "ch1_l5", "ch2_l1", "ch2_l2", "ch2_l3",
    "ch2_l4", "ch2_l5", "ch3_l1", "ch3_l2", "ch3_l3", "ch3_l4", "ch3_l5", "ch4_l1", "ch4_l2",
    "ch4_l3", "ch4_l4", "ch4_l5", "ch5_l1", "ch5_l2", "ch5_l3", "ch5_l4", "ch5_l5", "ch6_l1",
    "ch6_l2", "ch6_l3", "ch6_l4", "ch6_l5", "ch7_l1", "ch7_l2", "ch7_l3", "ch7_l4", "ch7_l5",
    "finale",
];

pub const TOTAL: u16 = LEVELS.len() as u16;

pub fn level(index: u16) -> Option<&'static LevelDef> {
    LEVELS.get(index as usize)
}

pub fn level_by_id(id: &str) -> Option<&'static LevelDef> {
    LEVELS.iter().find(|l| l.id == id)
}

pub fn chapter_of(index: u16) -> Option<&'static ChapterDef> {
    let chapter = level(index)?.chapter;
    CHAPTERS.iter().find(|c| c.number == chapter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn table_is_complete_and_ordered() {
        assert_eq!(LEVELS.len(), 49);
        for (i, l) in LEVELS.iter().enumerate() {
            assert_eq!(l.index as usize, i, "index mismatch at {}", l.id);
        }
    }

    #[test]
    fn ids_are_unique_and_keep_v01() {
        let mut seen = HashSet::new();
        for l in LEVELS {
            assert!(seen.insert(l.id), "duplicate id {}", l.id);
        }
        for id in V01_ORDER {
            assert!(level_by_id(id).is_some(), "lost v0.1 id {id}");
        }
    }

    #[test]
    fn chapters_cover_every_level_puzzles_first() {
        let mut covered = 0;
        for c in CHAPTERS {
            assert_eq!(LEVELS[c.first as usize].index, c.first);
            let mut seen_fault = false;
            for i in 0..c.count {
                let l = &LEVELS[(c.first + i) as usize];
                assert_eq!(l.chapter, c.number);
                if l.class != LevelKind::Puzzle {
                    seen_fault = true;
                } else {
                    assert!(!seen_fault, "{} comes after a fault", l.id);
                }
            }
            covered += c.count;
        }
        assert_eq!(covered, LEVELS.len() as u16);
    }

    #[test]
    fn every_title_is_bilingual() {
        for l in LEVELS {
            assert!(l.name.is_complete(), "{}", l.id);
        }
        for c in CHAPTERS {
            assert!(c.name.is_complete());
        }
    }

    #[test]
    fn lookup_works() {
        assert_eq!(level(0).unwrap().id, "p0");
        assert_eq!(level_by_id("ch1_l4").unwrap().class, LevelKind::Fault);
        assert!(level(49).is_none());
    }
}

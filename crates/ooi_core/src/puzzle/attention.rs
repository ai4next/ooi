//! 注意力：点出代词指的是哪个词。
//!
//! 5.2 一轮；5.4 两轮——同一个 "it"，形容词一换，指代就变了。

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttentionRound {
    /// 作为 query 的词（通常是代词）。
    pub query: usize,
    /// 正确答案的词下标。
    pub answer: usize,
}

#[derive(Clone, Debug)]
pub struct AttentionConfig {
    pub rounds: Vec<AttentionRound>,
}

#[derive(Clone, Debug)]
pub struct AttentionPuzzle {
    cfg: AttentionConfig,
    round: usize,
    mistakes: u32,
    chosen: Option<usize>,
}

impl AttentionPuzzle {
    pub fn new(cfg: AttentionConfig) -> Self {
        Self {
            cfg,
            round: 0,
            mistakes: 0,
            chosen: None,
        }
    }

    pub fn config(&self) -> &AttentionConfig {
        &self.cfg
    }

    pub fn round(&self) -> usize {
        self.round
    }

    pub fn chosen(&self) -> Option<usize> {
        self.chosen
    }

    pub fn mistakes(&self) -> u32 {
        self.mistakes
    }

    pub fn current(&self) -> Option<AttentionRound> {
        self.cfg.rounds.get(self.round).copied()
    }

    pub fn choose(&mut self, index: usize) -> bool {
        let Some(round) = self.current() else {
            return false;
        };
        if index == round.answer {
            self.round += 1;
            self.chosen = None;
            true
        } else {
            self.mistakes += 1;
            self.chosen = Some(index);
            false
        }
    }

    pub fn is_solved(&self) -> bool {
        self.round >= self.cfg.rounds.len()
    }

    pub fn reset(&mut self) {
        self.round = 0;
        self.mistakes = 0;
        self.chosen = None;
    }
}

pub fn pronoun_trophy() -> AttentionConfig {
    AttentionConfig {
        rounds: vec![AttentionRound {
            query: 9,
            answer: 1,
        }],
    }
}

pub fn context_matters() -> AttentionConfig {
    AttentionConfig {
        rounds: vec![
            AttentionRound {
                query: 8,
                answer: 1,
            },
            AttentionRound {
                query: 8,
                answer: 6,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correct_referent_advances() {
        let mut p = AttentionPuzzle::new(pronoun_trophy());
        assert!(!p.is_solved());
        assert!(p.choose(1));
        assert!(p.is_solved());
        assert_eq!(p.round(), 1);
    }

    #[test]
    fn wrong_referent_stays() {
        let mut p = AttentionPuzzle::new(pronoun_trophy());
        assert!(!p.choose(7));
        assert_eq!(p.round(), 0);
        assert_eq!(p.chosen(), Some(7));
        assert_eq!(p.mistakes(), 1);
    }

    #[test]
    fn two_rounds_need_both() {
        let mut p = AttentionPuzzle::new(context_matters());
        assert!(p.choose(1));
        assert!(!p.is_solved());
        assert!(!p.choose(1));
        assert!(p.choose(6));
        assert!(p.is_solved());
    }

    #[test]
    fn reset_restarts() {
        let mut p = AttentionPuzzle::new(context_matters());
        p.choose(1);
        p.reset();
        assert_eq!(p.round(), 0);
        assert!(!p.is_solved());
    }
}

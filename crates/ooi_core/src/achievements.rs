//! Explicit, testable achievement rules. They are derived from `MetaProgress`
//! so they survive New Game and old saves automatically.

use crate::save::MetaProgress;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Achievement {
    FirstLight,
    BreakWall,
    LongRoad,
    ForgeSpark,
    ForgeMaster,
    TrialRunner,
    BudgetMaster,
    TwoPaths,
    AllEndings,
    Unaided,
    Again,
}

impl Achievement {
    pub const ALL: [Achievement; 11] = [
        Achievement::FirstLight,
        Achievement::BreakWall,
        Achievement::LongRoad,
        Achievement::ForgeSpark,
        Achievement::ForgeMaster,
        Achievement::TrialRunner,
        Achievement::BudgetMaster,
        Achievement::TwoPaths,
        Achievement::AllEndings,
        Achievement::Unaided,
        Achievement::Again,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::FirstLight => "first_light",
            Self::BreakWall => "break_wall",
            Self::LongRoad => "long_road",
            Self::ForgeSpark => "forge_spark",
            Self::ForgeMaster => "forge_master",
            Self::TrialRunner => "trial_runner",
            Self::BudgetMaster => "budget_master",
            Self::TwoPaths => "two_paths",
            Self::AllEndings => "all_endings",
            Self::Unaided => "unaided",
            Self::Again => "again",
        }
    }

    pub fn is_unlocked(self, meta: &MetaProgress) -> bool {
        match self {
            Self::FirstLight => meta.levels_cleared >= 1,
            Self::BreakWall => meta.levels_cleared >= 6,
            Self::LongRoad => meta.levels_cleared >= crate::content::TOTAL as u32,
            Self::ForgeSpark => !meta.forge_mastered.is_empty(),
            Self::ForgeMaster => meta.forge_mastered.len() >= 3,
            Self::TrialRunner => !meta.forge_trials.is_empty(),
            Self::BudgetMaster => meta.forge_trials.len() >= 3,
            Self::TwoPaths => meta.endings_found() >= 2,
            Self::AllEndings => meta.endings_found() >= crate::save::ENDINGS.len(),
            Self::Unaided => meta.no_hint_runs >= 1,
            Self::Again => meta.runs_completed >= 2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_meta_unlocks_nothing() {
        let meta = MetaProgress::default();
        assert!(Achievement::ALL.iter().all(|a| !a.is_unlocked(&meta)));
    }

    #[test]
    fn thresholds_unlock_the_expected_badges() {
        let mut meta = MetaProgress {
            levels_cleared: 1,
            ..Default::default()
        };
        assert!(Achievement::FirstLight.is_unlocked(&meta));
        assert!(!Achievement::BreakWall.is_unlocked(&meta));

        meta.levels_cleared = crate::content::TOTAL as u32;
        for id in ["xor", "ring", "spiral"] {
            meta.record_forge_mastered(id);
            meta.record_forge_trial(id);
        }
        meta.record_ending("voyage");
        meta.record_ending("greenhouse");
        meta.record_no_hint_run();
        assert!(Achievement::LongRoad.is_unlocked(&meta));
        assert!(Achievement::ForgeMaster.is_unlocked(&meta));
        assert!(Achievement::BudgetMaster.is_unlocked(&meta));
        assert!(Achievement::TwoPaths.is_unlocked(&meta));
        assert!(Achievement::Unaided.is_unlocked(&meta));
        assert!(!Achievement::AllEndings.is_unlocked(&meta));
    }

    #[test]
    fn badge_ids_are_unique() {
        let mut ids: Vec<_> = Achievement::ALL.iter().map(|a| a.id()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), Achievement::ALL.len());
    }
}

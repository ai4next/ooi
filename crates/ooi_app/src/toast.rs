//! Achievement watcher: new badges become kit toasts.

use std::collections::BTreeSet;

use bevy::prelude::*;
use oo_kit::toast::ToastQueue;

use crate::state::{LangRes, SaveRes};
use crate::theme;
use ooi_core::text::{achievement_desc, achievement_name};
use ooi_core::{Achievement, t};

/// Badges already announced (or already earned when the save loaded).
#[derive(Resource, Default)]
pub struct ToastState {
    initialized: bool,
    known: BTreeSet<&'static str>,
    #[cfg(debug_assertions)]
    pub demo_fired: bool,
}

pub fn watch_achievements(
    save: Res<SaveRes>,
    lang: Res<LangRes>,
    mut state: ResMut<ToastState>,
    mut toasts: ResMut<ToastQueue>,
) {
    let l = lang.0;
    let meta = &save.0.meta;
    if !state.initialized {
        // Do not replay every badge the save already earned.
        state.known = Achievement::ALL.iter().filter(|a| a.is_unlocked(meta)).map(|a| a.id()).collect();
        state.initialized = true;
    }
    let mut fresh: Vec<Achievement> =
        Achievement::ALL.into_iter().filter(|a| a.is_unlocked(meta) && state.known.insert(a.id())).collect();
    #[cfg(debug_assertions)]
    if !state.demo_fired && std::env::var("OOI_TOAST_DEMO").is_ok() {
        state.demo_fired = true;
        fresh.push(Achievement::FirstLight);
    }
    for a in fresh {
        let title = format!("{}  ·  {}", t(l, "ACHIEVEMENT UNLOCKED", "成就解锁"), achievement_name(l, a));
        toasts.push_accent(title, achievement_desc(l, a), theme::amber());
    }
}

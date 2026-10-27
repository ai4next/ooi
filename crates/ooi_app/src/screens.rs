//! Title, substrate map, concept cards, choices, upgrades and endings.
//!
//! Everything is drawn in world space. The menu, puzzles and Forge share one
//! coordinate system, one type scale, and one lighting language so the game
//! reads as a single instrument rather than a stack of screens.

use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::state::{CodexSelection, LangRes, SaveRes, Screen, SettingsReturn};
use crate::theme;
use crate::ui::{self, Action, UiAssets};
use crate::visuals;
use ooi_core::lab::DatasetKind;
use ooi_core::text::{Ending, achievement_desc, ending_name, ending_of, achievement_name, chapter_log, choice_option, choice_prompt, dataset_name, level_card};
use ooi_core::{Achievement, CHAPTERS, ENDINGS, Lang, TOTAL, level, t};

/// Language switcher: names the language it switches to.
fn lang_button(commands: &mut Commands, assets: &UiAssets, lang: Lang, pos: Vec2) {
    ui::action_button(commands, assets, pos, Vec2::new(96.0, 30.0), lang.switch_label(), 11.0, Action::ToggleLang, false);
}

/// Top-right strip: stars and solved count, then the language switch.
fn progress_header(commands: &mut Commands, assets: &UiAssets, lang: Lang, save: &SaveRes) {
    let stars = save.0.campaign.total_stars();
    let pills = [
        (t(lang, "STARS", "星"), format!("{stars} / {}", TOTAL as u32 * 3), theme::amber()),
        (t(lang, "SOLVED", "已解"), format!("{} / {TOTAL}", save.0.completed_count()), theme::cyan()),
    ];
    for (i, (label, value, accent)) in pills.into_iter().enumerate() {
        ui::resource_pill(commands, assets, Vec2::new(250.0 + i as f32 * 160.0, 332.0), 150.0, label, value, accent);
    }
    lang_button(commands, assets, lang, Vec2::new(586.0, 332.0));
}

fn brand_stamp(commands: &mut Commands, assets: &UiAssets, lang: Lang) {
    ui::caption(
        commands,
        assets,
        t(lang, "ORIGIN OF INTELLIGENCE", "智能起源"),
        Vec2::new(-590.0, 338.0),
        theme::text_dim(),
        Anchor::CENTER_LEFT,
    );
}

fn current_chapter(index: u16) -> Option<&'static ooi_core::ChapterDef> {
    ooi_core::chapter_of(index)
}

/// Chapter endings that hand the player directly to the dataset whose trial
/// matches the concept they just learned.
fn chapter_trial(index: u16) -> Option<usize> {
    match index {
        5 => Some(0),  // XOR: the second layer
        10 => Some(1), // Ring: nonlinearity and curvature
        20 => Some(2), // Spiral: depth and composed features
        _ => None,
    }
}

// ---------------------------------------------------------------- title

pub fn spawn_title(
    mut commands: Commands,
    assets: Res<UiAssets>,
    lang: Res<LangRes>,
    save: Res<SaveRes>,
) {
    let l = lang.0;
    let completed = save.0.completed_count();

    ui::caption(
        &mut commands,
        &assets,
        t(l, "OOI / OBSERVATION 001", "OOI / 观测 001"),
        Vec2::new(-590.0, 332.0),
        theme::text_faint(),
        Anchor::CENTER_LEFT,
    );
    lang_button(&mut commands, &assets, l, Vec2::new(586.0, 332.0));

    ui::caption(
        &mut commands,
        &assets,
        t(l, "A GAME ABOUT HOW MACHINES LEARN", "一个关于机器如何学习的游戏"),
        Vec2::new(-590.0, 238.0),
        theme::cyan().with_alpha(0.8),
        Anchor::CENTER_LEFT,
    );

    if l == Lang::En {
        {
            ui::text_styled(
                &mut commands,
                &assets,
                "ORIGIN",
                Vec2::new(-592.0, 158.0),
                76.0,
                theme::text(),
                Anchor::TOP_LEFT,
                2.0,
            );
            ui::text_styled(
                &mut commands,
                &assets,
                "OF INTELLIGENCE",
                Vec2::new(-590.0, 72.0),
                43.0,
                theme::cyan(),
                Anchor::TOP_LEFT,
                2.8,
            );
        }
    } else {
        {
            ui::text(
                &mut commands,
                &assets,
                t(l, "ORIGIN OF INTELLIGENCE", "智能起源"),
                Vec2::new(-590.0, 178.0),
                70.0,
                theme::text(),
                Anchor::TOP_LEFT,
            );
            ui::text(
                &mut commands,
                &assets,
                t(l, "Neural networks · starting from a single neuron", "神经网络 · 从一枚神经元开始"),
                Vec2::new(-588.0, 98.0),
                21.0,
                theme::cyan(),
                Anchor::TOP_LEFT,
            );
        }
    }

    ui::text(
        &mut commands,
        &assets,
        t(l, "Understanding is the only unlock.", "理解，是唯一的解锁方式。"),
        Vec2::new(-590.0, -8.0),
        19.0,
        theme::text_dim(),
        Anchor::CENTER_LEFT,
    );
    commands.spawn((
        Sprite::from_color(theme::amber().with_alpha(0.8), Vec2::new(92.0, 2.0)),
        Transform::from_xyz(-544.0, -37.0, 20.0),
        ui::ScreenEntity,
    ));
    ui::text(
        &mut commands,
        &assets,
        t(l, "Make it understand. Not memorize.", "让它理解，而不是记住。"),
        Vec2::new(-590.0, -68.0),
        14.0,
        theme::text_faint(),
        Anchor::CENTER_LEFT,
    );

    // With a run to continue there is one more button; lift the stack so the footer keeps its margin.
    let mut button_y = if completed > 0 { -112.0 } else { -124.0 };
    ui::primary_button(
        &mut commands,
        &assets,
        Vec2::new(-460.0, button_y),
        Vec2::new(260.0, 52.0),
        t(l, "NEW GAME", "新的开始"),
        Action::NewGame,
    );
    if completed > 0 {
        button_y -= 62.0;
        let resume = save.0.resume_index();
        let title = level(resume).map(|d| d.title(l)).unwrap_or("");
        ui::action_button(
            &mut commands,
            &assets,
            Vec2::new(-460.0, button_y),
            Vec2::new(260.0, 44.0),
            format!("{}  \u{00b7}  {}", t(l, "CONTINUE", "继续"), title),
            13.0,
            Action::Continue,
            false,
        );
    }
    button_y -= 54.0;
    ui::action_button(
        &mut commands,
        &assets,
        Vec2::new(-460.0, button_y),
        Vec2::new(260.0, 38.0),
        t(l, "ENTER THE FORGE", "进入塑造台").to_string(),
        13.0,
        Action::LabOpen,
        false,
    );
    button_y -= 48.0;
    ui::action_button(
        &mut commands,
        &assets,
        Vec2::new(-545.0, button_y),
        Vec2::new(82.0, 32.0),
        t(l, "RECORDS", "记录"),
        9.5,
        Action::Records,
        false,
    );
    ui::action_button(
        &mut commands,
        &assets,
        Vec2::new(-455.0, button_y),
        Vec2::new(82.0, 32.0),
        t(l, "SETTINGS", "设置"),
        9.5,
        Action::Settings,
        false,
    );
    ui::action_button(
        &mut commands,
        &assets,
        Vec2::new(-365.0, button_y),
        Vec2::new(82.0, 32.0),
        t(l, "QUIT", "退出"),
        9.5,
        Action::Quit,
        false,
    );

    ui::caption(
        &mut commands,
        &assets,
        format!(
            "{} {}   \u{00b7}   {} {}   \u{00b7}   {} {}/{}",
            TOTAL,
            t(l, "CONCEPTS", "概念"),
            CHAPTERS.len(),
            t(l, "CHAPTERS", "章节"),
            t(l, "ENDINGS", "结局"),
            save.0.meta.endings_found().min(ENDINGS.len()),
            ENDINGS.len()
        ),
        Vec2::new(-590.0, -318.0),
        theme::text_faint(),
        Anchor::CENTER_LEFT,
    );
    if completed > 0 {
        ui::caption(
            &mut commands,
            &assets,
            format!("{} {}/{}", t(l, "PROGRESS", "进度"), completed, TOTAL),
            Vec2::new(-590.0, -340.0),
            theme::amber().with_alpha(0.75),
            Anchor::CENTER_LEFT,
        );
    }

    visuals::spawn_organism(
        &mut commands,
        &assets.tex,
        Vec2::new(40.0, 4.0),
        1.55,
        0x0d1e_2a3b_4c5d_6e7f,
        5.0,
    );
    ui::caption(
        &mut commands,
        &assets,
        format!(
            "{} / {}",
            t(l, "ORGANISM", "有机体"),
            t(l, "LIVE", "实时")
        ),
        Vec2::new(566.0, 272.0),
        theme::text_faint(),
        Anchor::CENTER_RIGHT,
    );
    ui::caption(
        &mut commands,
        &assets,
        t(l, "IT IS WAITING.", "它在等待。"),
        Vec2::new(566.0, -278.0),
        theme::amber().with_alpha(0.7),
        Anchor::CENTER_RIGHT,
    );
    commands.spawn((
        Sprite::from_color(theme::line().with_alpha(0.3), Vec2::new(1.0, 420.0)),
        Transform::from_xyz(-170.0, 4.0, 4.0),
        ui::ScreenEntity,
    ));
}

// ---------------------------------------------------------------- hub

pub fn spawn_hub(
    mut commands: Commands,
    assets: Res<UiAssets>,
    lang: Res<LangRes>,
    save: Res<SaveRes>,
) {
    let l = lang.0;
    let progress = &save.0;
    let completed = progress.completed_count();
    let resume = progress.resume_index();

    brand_stamp(&mut commands, &assets, l);
    ui::text(
        &mut commands,
        &assets,
        t(l, "MATRIX", "基质"),
        Vec2::new(-590.0, 312.0),
        22.0,
        theme::text(),
        Anchor::CENTER_LEFT,
    );
    progress_header(&mut commands, &assets, l, &save);

    // Left rail: organism, progress, next action.
    ui::card(
        &mut commands,
        &assets,
        Vec2::new(-465.0, 0.0),
        Vec2::new(300.0, 490.0),
        theme::cyan().with_alpha(0.55),
    );
    ui::caption(
        &mut commands,
        &assets,
        format!(
            "{}  /  {}",
            t(l, "ORGANISM", "有机体"),
            if completed > 0 {
                t(l, "AWAKE", "已唤醒")
            } else {
                t(l, "SILENT", "沉默")
            }
        ),
        Vec2::new(-600.0, 230.0),
        theme::text_faint(),
        Anchor::CENTER_LEFT,
    );
    visuals::spawn_organism(
        &mut commands,
        &assets.tex,
        Vec2::new(-465.0, 116.0),
        0.47,
        0x0b17_2c3d_4e5f_6071,
        6.0,
    );

    let chapter = current_chapter(resume);
    let chapter_number = chapter.map(|c| c.number).unwrap_or(0);
    ui::caption(
        &mut commands,
        &assets,
        format!("{}  /  {}", t(l, "CHAPTER", "章"), chapter_number),
        Vec2::new(-600.0, 2.0),
        theme::amber().with_alpha(0.8),
        Anchor::CENTER_LEFT,
    );
    if let Some(chapter) = chapter {
        ui::text(
            &mut commands,
            &assets,
            chapter.title(l),
            Vec2::new(-600.0, -22.0),
            13.0,
            theme::text_dim(),
            Anchor::CENTER_LEFT,
        );
    }
    let title = level(resume).map(|d| d.title(l)).unwrap_or("");
    ui::wrapped_text(
        &mut commands,
        &assets,
        title,
        Vec2::new(-600.0, -48.0),
        250.0,
        17.0,
        theme::text(),
        Anchor::TOP_LEFT,
    );

    let ratio = completed as f32 / TOTAL as f32;
    ui::progress_bar(
        &mut commands,
        &assets,
        Vec2::new(-465.0, -101.0),
        Vec2::new(250.0, 4.0),
        ratio,
        theme::amber(),
    );
    ui::caption(
        &mut commands,
        &assets,
        format!("{} {}/{}", t(l, "PROGRESS", "进度"), completed, TOTAL),
        Vec2::new(-600.0, -118.0),
        theme::text_faint(),
        Anchor::CENTER_LEFT,
    );
    ui::caption(
        &mut commands,
        &assets,
        format!("{:.0}%", ratio * 100.0),
        Vec2::new(-340.0, -118.0),
        theme::amber(),
        Anchor::CENTER_RIGHT,
    );

    ui::primary_button(
        &mut commands,
        &assets,
        Vec2::new(-465.0, -150.0),
        Vec2::new(250.0, 44.0),
        t(l, "CONTINUE", "继续"),
        Action::Play(resume),
    );
    ui::action_button(
        &mut commands,
        &assets,
        Vec2::new(-465.0, -195.0),
        Vec2::new(250.0, 36.0),
        t(l, "ENTER THE FORGE", "进入塑造台").to_string(),
        12.0,
        Action::LabOpen,
        false,
    );

    // The map around it.
    ui::card(
        &mut commands,
        &assets,
        Vec2::new(155.0, 5.0),
        Vec2::new(900.0, 470.0),
        theme::amber().with_alpha(0.45),
    );
    ui::caption(
        &mut commands,
        &assets,
        t(l, "THE SUBSTRATE", "概念星图"),
        Vec2::new(-275.0, 226.0),
        theme::text_faint(),
        Anchor::CENTER_LEFT,
    );
    ui::text(
        &mut commands,
        &assets,
        format!("{completed:02} / {TOTAL:02}"),
        Vec2::new(585.0, 226.0),
        12.0,
        theme::amber().with_alpha(0.8),
        Anchor::CENTER_RIGHT,
    );

    let col_x = |c: usize| -220.0 + c as f32 * 94.0;
    let row_y = |r: usize| 160.0 - r as f32 * 53.0;

    let mut points = Vec::new();
    for chapter in CHAPTERS {
        for row in 0..chapter.count as usize {
            let index = chapter.first as usize + row;
            let x = col_x(chapter.number as usize);
            let y = if chapter.count == 1 { 52.0 } else { row_y(row) };
            points.push((Vec2::new(x, y), index));
        }
    }

    for pair in points.windows(2) {
        let (a, ai) = pair[0];
        let (b, bi) = pair[1];
        let done = progress.is_complete(ai as u16) && progress.is_complete(bi as u16);
        let near_current = ai as u16 == resume || bi as u16 == resume;
        let color = if done {
            theme::amber().with_alpha(0.12)
        } else if near_current {
            theme::cyan().with_alpha(0.22)
        } else {
            theme::line().with_alpha(0.07)
        };
        let delta = b - a;
        let mid = (a + b) * 0.5;
        commands.spawn((
            Sprite::from_color(color, Vec2::new(delta.length(), 1.0)),
            Transform::from_xyz(mid.x, mid.y, 4.0)
                .with_rotation(Quat::from_rotation_z(delta.y.atan2(delta.x))),
            ui::ScreenEntity,
        ));
    }

    for chapter in CHAPTERS {
        let x = col_x(chapter.number as usize);
        let is_current = chapter.number == chapter_number;
        ui::text(
            &mut commands,
            &assets,
            chapter.short,
            Vec2::new(x, 198.0),
            11.0,
            if is_current {
                theme::amber()
            } else {
                theme::text_faint()
            },
            Anchor::CENTER,
        );
        if is_current {
            commands.spawn((
                Sprite::from_color(theme::amber().with_alpha(0.5), Vec2::new(20.0, 1.0)),
                Transform::from_xyz(x, 190.0, 5.0),
                ui::ScreenEntity,
            ));
        }
    }

    for (pos, index) in &points {
        let index = *index as u16;
        let complete = progress.is_complete(index);
        let unlocked = progress.is_unlocked(index);
        let active = index == resume && !complete;
        let color = if complete {
            theme::amber()
        } else if active {
            theme::cyan()
        } else if unlocked {
            theme::ice()
        } else {
            theme::cyan_dim()
        };

        if active {
            ui::halo(
                &mut commands,
                &assets,
                *pos,
                74.0,
                theme::cyan().with_alpha(0.22),
                5.4,
            );
            commands.spawn((
                Sprite {
                    image: assets.tex.dot.clone(),
                    color: theme::cyan(),
                    custom_size: Some(Vec2::splat(4.0)),
                    ..default()
                },
                Transform::from_xyz(pos.x, pos.y, 12.0),
                visuals::OrganismPulse {
                    base_scale: 1.0,
                    pulse_scale: 0.6,
                    base_alpha: 0.15,
                    pulse_alpha: 0.6,
                    speed: 2.0,
                    phase: 0.0,
                    color: theme::cyan(),
                },
                ui::ScreenEntity,
            ));
        }

        commands.spawn((
            Sprite {
                image: assets.tex.ring.clone(),
                color: color.with_alpha(if active { 0.95 } else { 0.62 }),
                custom_size: Some(Vec2::splat(if active { 34.0 } else { 29.0 })),
                ..default()
            },
            Transform::from_xyz(pos.x, pos.y, 6.0),
            ui::ScreenEntity,
        ));
        commands.spawn((
            Sprite {
                image: assets.tex.dot.clone(),
                color: if complete || active {
                    color.with_alpha(0.9)
                } else {
                    Color::srgba(0.05, 0.09, 0.14, 1.0)
                },
                custom_size: Some(Vec2::splat(if active { 17.0 } else { 13.0 })),
                ..default()
            },
            Transform::from_xyz(pos.x, pos.y, 6.2),
            ui::ScreenEntity,
        ));
        let row_label = if index == 0 {
            "P".to_string()
        } else if level(index).is_some_and(|d| d.id == "finale") {
            "F".to_string()
        } else {
            let first = current_chapter(index).map(|c| c.first).unwrap_or(0);
            format!("{}", index - first + 1)
        };
        ui::text(
            &mut commands,
            &assets,
            row_label,
            *pos,
            10.0,
            if complete || active {
                theme::bg()
            } else {
                theme::text_faint()
            },
            Anchor::CENTER,
        );

        if unlocked {
            commands.spawn((
                Sprite::from_color(Color::NONE, Vec2::splat(40.0)),
                Transform::from_xyz(pos.x, pos.y, 11.0),
                ui::hit(Vec2::splat(20.0), Action::Play(index), Color::NONE, theme::cyan().with_alpha(0.12)),
                ui::ScreenEntity,
            ));
        }
    }

    ui::caption(
        &mut commands,
        &assets,
        format!("{} {completed}", t(l, "CONCEPTS", "概念")),
        Vec2::new(-275.0, -198.0),
        theme::text_faint(),
        Anchor::CENTER_LEFT,
    );
    ui::caption(
        &mut commands,
        &assets,
        t(l, "CLICK A NODE TO REVISIT", "点击节点回看"),
        Vec2::new(585.0, -198.0),
        theme::text_faint(),
        Anchor::CENTER_RIGHT,
    );
    ui::action_button(
        &mut commands,
        &assets,
        Vec2::new(-575.0, -318.0),
        Vec2::new(60.0, 28.0),
        t(l, "TITLE", "标题"),
        9.0,
        Action::Title,
        false,
    );
    ui::action_button(
        &mut commands,
        &assets,
        Vec2::new(-505.0, -318.0),
        Vec2::new(64.0, 28.0),
        t(l, "ARCHIVE", "概念档案"),
        9.0,
        Action::CodexOpen,
        false,
    );
    ui::action_button(
        &mut commands,
        &assets,
        Vec2::new(-435.0, -318.0),
        Vec2::new(64.0, 28.0),
        t(l, "RECORDS", "记录"),
        9.0,
        Action::Records,
        false,
    );
    ui::action_button(
        &mut commands,
        &assets,
        Vec2::new(-365.0, -318.0),
        Vec2::new(70.0, 28.0),
        t(l, "SETTINGS", "设置"),
        9.0,
        Action::Settings,
        false,
    );
}

// ---------------------------------------------------------------- concept card

pub fn spawn_card(
    mut commands: Commands,
    assets: Res<UiAssets>,
    lang: Res<LangRes>,
    current: Res<crate::state::CurrentLevel>,
    save: Res<SaveRes>,
) {
    let l = lang.0;
    let Some(def) = level(current.0) else {
        return;
    };
    let chapter = ooi_core::chapter_of(def.index);
    let is_chapter_end = chapter.is_some_and(|c| c.first + c.count - 1 == def.index);
    let trial = chapter_trial(def.index)
        .filter(|index| !save.0.lab.is_trial_complete(DatasetKind::ALL[*index].id()));

    brand_stamp(&mut commands, &assets, l);
    ui::action_button(
        &mut commands,
        &assets,
        Vec2::new(-567.0, 309.0),
        Vec2::new(120.0, 28.0),
        format!("\u{2190}  {}", t(l, "MATRIX", "基质")),
        10.0,
        Action::Hub,
        false,
    );
    if let Some(chapter) = chapter {
        ui::caption(
            &mut commands,
            &assets,
            chapter.title(l),
            Vec2::new(0.0, 332.0),
            theme::text_dim(),
            Anchor::CENTER,
        );
    }

    ui::card(
        &mut commands,
        &assets,
        Vec2::new(0.0, 0.0),
        Vec2::new(1060.0, 500.0),
        theme::amber().with_alpha(0.55),
    );

    // Left: the concept as an object.
    ui::caption(
        &mut commands,
        &assets,
        format!("{}  /  {:02}", t(l, "CONCEPTS", "概念"), def.index),
        Vec2::new(-500.0, 230.0),
        theme::text_faint(),
        Anchor::CENTER_LEFT,
    );
    ui::text(
        &mut commands,
        &assets,
        format!("{:02}", def.index),
        Vec2::new(-500.0, 174.0),
        58.0,
        theme::amber(),
        Anchor::CENTER_LEFT,
    );
    visuals::spawn_organism(
        &mut commands,
        &assets.tex,
        Vec2::new(-355.0, -35.0),
        0.45,
        0x00c0_ffee_1234_5678 ^ def.index as u64,
        6.0,
    );
    // Divider between object and explanation.
    commands.spawn((
        Sprite::from_color(theme::line().with_alpha(0.45), Vec2::new(1.0, 390.0)),
        Transform::from_xyz(-238.0, 10.0, 7.0),
        ui::ScreenEntity,
    ));

    // Right: what was learned.
    ui::caption(
        &mut commands,
        &assets,
        t(l, "SOLVED", "已解"),
        Vec2::new(-210.0, 230.0),
        theme::mint(),
        Anchor::CENTER_LEFT,
    );
    ui::text(
        &mut commands,
        &assets,
        def.title(l),
        Vec2::new(-210.0, 190.0),
        30.0,
        theme::text(),
        Anchor::CENTER_LEFT,
    );
    ui::caption(
        &mut commands,
        &assets,
        t(l, "REAL WORLD", "真实世界"),
        Vec2::new(-210.0, 148.0),
        theme::text_faint(),
        Anchor::CENTER_LEFT,
    );
    let body = level_card(l, def.id);
    let (lead, note) = body.split_once("\n\n").unwrap_or((body, ""));
    ui::wrapped_text(
        &mut commands,
        &assets,
        lead,
        Vec2::new(-210.0, 124.0),
        700.0,
        16.0,
        theme::text_dim(),
        Anchor::TOP_LEFT,
    );
    if !note.is_empty() {
        commands.spawn((
            Sprite::from_color(theme::amber().with_alpha(0.55), Vec2::new(2.0, 46.0)),
            Transform::from_xyz(-232.0, -36.0, 8.0),
            ui::ScreenEntity,
        ));
        ui::wrapped_text(
            &mut commands,
            &assets,
            note,
            Vec2::new(-210.0, -18.0),
            700.0,
            15.0,
            theme::text(),
            Anchor::TOP_LEFT,
        );
    }

    let note_y = -102.0;
    if is_chapter_end && let Some(chapter) = chapter {
        ui::caption(
            &mut commands,
            &assets,
            t(l, "CHAPTER COMPLETE", "章节完成"),
            Vec2::new(-210.0, note_y - 6.0),
            theme::amber(),
            Anchor::CENTER_LEFT,
        );
        ui::text(
            &mut commands,
            &assets,
            chapter_log(l, chapter.number),
            Vec2::new(-210.0, note_y - 26.0),
            12.0,
            theme::text_dim(),
            Anchor::CENTER_LEFT,
        );
    }

    ui::stars(&mut commands, &assets, Vec2::new(380.0, 230.0), save.0.campaign.stars(def.id), 14.0);

    ui::action_button(
        &mut commands,
        &assets,
        Vec2::new(-210.0, -197.0),
        Vec2::new(160.0, 40.0),
        t(l, "MATRIX", "基质"),
        13.0,
        Action::Hub,
        false,
    );
    if let Some(trial_index) = trial {
        let kind = DatasetKind::ALL[trial_index];
        ui::action_button(
            &mut commands,
            &assets,
            Vec2::new(95.0, -197.0),
            Vec2::new(206.0, 40.0),
            format!(
                "{}  \u{00b7}  {}",
                t(l, "FORGE TRIAL", "塑造试炼"),
                dataset_name(l, kind)
            ),
            12.0,
            Action::LabTrial(trial_index),
            false,
        );
    }
    ui::primary_button(
        &mut commands,
        &assets,
        Vec2::new(410.0, -197.0),
        Vec2::new(180.0, 44.0),
        t(l, "NEXT", "继续"),
        Action::Next,
    );
    ui::progress_bar(
        &mut commands,
        &assets,
        Vec2::new(0.0, -232.0),
        Vec2::new(980.0, 3.0),
        (current.0 as f32 + 1.0) / TOTAL as f32,
        theme::amber(),
    );
}

// ---------------------------------------------------------------- choice

/// Kessler's offer after the finale. "Let it choose" needs the ranked principles from 7-5.
pub fn spawn_choice(mut commands: Commands, assets: Res<UiAssets>, lang: Res<LangRes>, save: Res<SaveRes>) {
    let l = lang.0;
    brand_stamp(&mut commands, &assets, l);
    ui::card(&mut commands, &assets, Vec2::new(0.0, 8.0), Vec2::new(940.0, 520.0), theme::violet().with_alpha(0.65));
    ui::caption(&mut commands, &assets, t(l, "DECISION", "抉择"), Vec2::new(-430.0, 232.0), theme::amber(), Anchor::CENTER_LEFT);
    ui::wrapped_text(&mut commands, &assets, choice_prompt(l), Vec2::new(0.0, 185.0), 760.0, 20.0, theme::text(), Anchor::TOP_CENTER);

    let count = if save.0.narrative.principles.is_empty() { 2 } else { 3 };
    for i in 0..count {
        let y = 24.0 - i as f32 * 82.0;
        let label = format!("{:02}   {}", i + 1, choice_option(l, i));
        ui::action_button(&mut commands, &assets, Vec2::new(0.0, y), Vec2::new(720.0, 62.0), label, 17.0, Action::FinaleChoice(i), false);
    }
    ui::caption(&mut commands, &assets, t(l, "THIS CHOICE ENDS THE RUN", "这个选择会结束这一轮"), Vec2::new(0.0, -223.0), theme::text_faint(), Anchor::CENTER);
}

// ---------------------------------------------------------------- codex

/// A reviewable archive of every concept the player has actually solved.
/// Locked entries remain locked; solved entries can be reread or replayed.
pub fn spawn_codex(
    mut commands: Commands,
    assets: Res<UiAssets>,
    lang: Res<LangRes>,
    save: Res<SaveRes>,
    selection: Res<CodexSelection>,
) {
    let l = lang.0;
    let completed = save.0.completed_count();
    let selected = selection.0.min(TOTAL - 1);
    let selected_def = level(selected);
    let selected_done = save.0.is_complete(selected);

    ui::progress_bar(
        &mut commands,
        &assets,
        Vec2::new(0.0, 354.0),
        Vec2::new(1180.0, 2.0),
        completed as f32 / TOTAL as f32,
        theme::amber(),
    );
    ui::action_button(
        &mut commands,
        &assets,
        Vec2::new(-558.0, 332.0),
        Vec2::new(112.0, 30.0),
        format!("\u{2190}  {}", t(l, "MATRIX", "基质")),
        10.0,
        Action::Hub,
        false,
    );
    ui::text(
        &mut commands,
        &assets,
        t(l, "ARCHIVE", "概念档案"),
        Vec2::new(-480.0, 326.0),
        27.0,
        theme::text(),
        Anchor::CENTER_LEFT,
    );
    ui::text(
        &mut commands,
        &assets,
        t(l, "Every solved idea, kept.", "解出的每一个概念，都留在这里。"),
        Vec2::new(-480.0, 301.0),
        12.0,
        theme::text_dim(),
        Anchor::CENTER_LEFT,
    );
    ui::text(
        &mut commands,
        &assets,
        format!("{completed:02} / {TOTAL:02}"),
        Vec2::new(590.0, 332.0),
        13.0,
        theme::amber(),
        Anchor::CENTER_RIGHT,
    );
    ui::action_button(
        &mut commands,
        &assets,
        Vec2::new(520.0, 301.0),
        Vec2::new(84.0, 28.0),
        l.switch_label(),
        10.0,
        Action::ToggleLang,
        false,
    );

    ui::card(
        &mut commands,
        &assets,
        Vec2::new(0.0, -8.0),
        Vec2::new(1140.0, 510.0),
        theme::amber().with_alpha(0.52),
    );

    // Seven columns x seven rows: all 49 levels fit without scrolling.
    let cell = Vec2::new(100.0, 50.0);
    let start = Vec2::new(-505.0, 170.0);
    for index in 0..TOTAL {
        let col = index % 7;
        let row = index / 7;
        let pos = start + Vec2::new(col as f32 * 108.0, -(row as f32) * 58.0);
        let done = save.0.is_complete(index);
        let chosen = index == selected;
        let fill = if chosen {
            theme::action()
        } else if done {
            Color::srgba(0.11, 0.085, 0.045, 0.94)
        } else {
            Color::srgba(0.035, 0.055, 0.080, 0.72)
        };
        let edge = if chosen {
            theme::cyan().with_alpha(0.95)
        } else if done {
            theme::amber().with_alpha(0.42)
        } else {
            theme::line().with_alpha(0.22)
        };

        if chosen {
            ui::halo(
                &mut commands,
                &assets,
                pos,
                80.0,
                theme::cyan().with_alpha(0.18),
                5.0,
            );
        }
        commands.spawn((
            assets.tex.panel_sprite(fill, cell),
            Transform::from_xyz(pos.x, pos.y, 7.0),
            ui::hit(cell / 2.0, Action::CodexSelect(index), fill, theme::cyan().with_alpha(0.12)),
            ui::ScreenEntity,
        ));
        commands.spawn((
            assets.tex.outline_sprite(edge, cell),
            Transform::from_xyz(pos.x, pos.y, 7.2),
            ui::ScreenEntity,
        ));
        ui::text(
            &mut commands,
            &assets,
            format!("{:02}", index + 1),
            pos + Vec2::new(-43.0, 17.0),
            7.0,
            if done {
                theme::amber().with_alpha(0.8)
            } else {
                theme::text_faint()
            },
            Anchor::CENTER_LEFT,
        );

        if done {
            let title = if let Some(def) = level(index) {
                def.title(l)
            } else {
                ""
            };
            ui::wrapped_text(
                &mut commands,
                &assets,
                title,
                pos + Vec2::new(-44.0, 2.0),
                88.0,
                8.2,
                if chosen {
                    theme::text()
                } else {
                    theme::text_dim()
                },
                Anchor::TOP_LEFT,
            );
        } else {
            ui::text_styled(
                &mut commands,
                &assets,
                t(l, "LOCKED", "未解锁"),
                pos + Vec2::new(0.0, -2.0),
                7.5,
                theme::text_faint(),
                Anchor::CENTER,
                0.8,
            );
        }
    }

    ui::caption(
        &mut commands,
        &assets,
        t(l, "CLICK A NODE TO REVISIT", "点击节点回看"),
        Vec2::new(-545.0, -238.0),
        theme::text_faint(),
        Anchor::CENTER_LEFT,
    );

    // Detail panel.
    ui::card(
        &mut commands,
        &assets,
        Vec2::new(390.0, -8.0),
        Vec2::new(330.0, 430.0),
        if selected_done {
            theme::amber().with_alpha(0.62)
        } else {
            theme::line().with_alpha(0.4)
        },
    );
    // A ghosted numeral fills the plate on short cards and keeps the panel
    // feeling designed even when the card text is only two lines.
    commands.spawn((
        Text2d::new(format!("{:02}", selected + 1)),
        TextFont {
            font: assets.source(),
            font_size: FontSize::Px(150.0),
            ..default()
        },
        TextColor(theme::text().with_alpha(0.035)),
        Anchor::CENTER,
        Transform::from_xyz(390.0, -54.0, 5.5),
        ui::ScreenEntity,
    ));
    ui::caption(
        &mut commands,
        &assets,
        format!("{}  /  {:02}", t(l, "CONCEPTS", "概念"), selected + 1),
        Vec2::new(250.0, 198.0),
        theme::text_faint(),
        Anchor::CENTER_LEFT,
    );
    ui::caption(
        &mut commands,
        &assets,
        format!("{completed:02} / {TOTAL:02}"),
        Vec2::new(530.0, 198.0),
        theme::text_faint(),
        Anchor::CENTER_RIGHT,
    );

    if selected_done {
        if let Some(def) = selected_def {
            ui::wrapped_text(
                &mut commands,
                &assets,
                def.title(l),
                Vec2::new(250.0, 176.0),
                280.0,
                18.0,
                theme::text(),
                Anchor::TOP_LEFT,
            );
            if let Some(chapter) = ooi_core::chapter_of(def.index) {
                ui::caption(
                    &mut commands,
                    &assets,
                    chapter.title(l),
                    Vec2::new(250.0, 133.0),
                    theme::amber().with_alpha(0.7),
                    Anchor::CENTER_LEFT,
                );
            }
            ui::caption(
                &mut commands,
                &assets,
                t(l, "REAL WORLD", "真实世界"),
                Vec2::new(250.0, 106.0),
                theme::text_faint(),
                Anchor::CENTER_LEFT,
            );
            ui::wrapped_text(
                &mut commands,
                &assets,
                level_card(l, def.id),
                Vec2::new(250.0, 90.0),
                280.0,
                11.5,
                theme::text_dim(),
                Anchor::TOP_LEFT,
            );
            ui::action_button(
                &mut commands,
                &assets,
                Vec2::new(390.0, -207.0),
                Vec2::new(140.0, 34.0),
                t(l, "REPLAY", "重玩"),
                12.0,
                Action::CodexPlay(selected),
                false,
            );
        }
    } else {
        ui::text(
            &mut commands,
            &assets,
            t(l, "LOCKED", "未解锁"),
            Vec2::new(390.0, 42.0),
            28.0,
            theme::text_dim(),
            Anchor::CENTER,
        );
        ui::wrapped_text(
            &mut commands,
            &assets,
            t(l, "COMPLETE THE LEVEL TO UNLOCK", "完成关卡后解锁"),
            Vec2::new(250.0, 4.0),
            280.0,
            13.0,
            theme::text_faint(),
            Anchor::TOP_LEFT,
        );
        if let Some(def) = selected_def
            && let Some(chapter) = ooi_core::chapter_of(def.index)
        {
            ui::caption(
                &mut commands,
                &assets,
                chapter.title(l),
                Vec2::new(390.0, -58.0),
                theme::text_faint(),
                Anchor::CENTER,
            );
        }
    }

    ui::action_button(
        &mut commands,
        &assets,
        Vec2::new(540.0, -330.0),
        Vec2::new(108.0, 28.0),
        t(l, "SETTINGS", "设置"),
        10.0,
        Action::Settings,
        false,
    );
}

// ---------------------------------------------------------------- records

/// Long-term profile: endings, lifetime metrics, and achievement badges.
pub fn spawn_records(
    mut commands: Commands,
    assets: Res<UiAssets>,
    lang: Res<LangRes>,
    save: Res<SaveRes>,
) {
    let l = lang.0;
    let meta = &save.0.meta;
    let endings = meta.endings_found().min(ENDINGS.len());
    let mastered = meta.forge_mastered.len().min(3);
    let trials = meta.forge_trials.len().min(3);
    let unlocked = Achievement::ALL
        .iter()
        .filter(|a| a.is_unlocked(meta))
        .count();

    ui::progress_bar(
        &mut commands,
        &assets,
        Vec2::new(0.0, 354.0),
        Vec2::new(1180.0, 2.0),
        endings as f32 / ENDINGS.len() as f32,
        theme::amber(),
    );
    ui::action_button(
        &mut commands,
        &assets,
        Vec2::new(-558.0, 332.0),
        Vec2::new(112.0, 30.0),
        format!("\u{2190}  {}", t(l, "BACK", "返回")),
        10.0,
        Action::RecordsClose,
        false,
    );
    ui::text(
        &mut commands,
        &assets,
        t(l, "RECORDS", "记录"),
        Vec2::new(-480.0, 326.0),
        27.0,
        theme::text(),
        Anchor::CENTER_LEFT,
    );
    ui::text(
        &mut commands,
        &assets,
        t(l, "What survives the run.", "一局之后，留下的东西。"),
        Vec2::new(-480.0, 301.0),
        12.0,
        theme::text_dim(),
        Anchor::CENTER_LEFT,
    );
    ui::action_button(
        &mut commands,
        &assets,
        Vec2::new(520.0, 301.0),
        Vec2::new(84.0, 28.0),
        l.switch_label(),
        10.0,
        Action::ToggleLang,
        false,
    );
    ui::card(
        &mut commands,
        &assets,
        Vec2::new(0.0, -8.0),
        Vec2::new(1140.0, 510.0),
        theme::amber().with_alpha(0.55),
    );

    // Discovered endings.
    ui::caption(
        &mut commands,
        &assets,
        t(l, "ENDINGS", "结局"),
        Vec2::new(-530.0, 210.0),
        theme::text_faint(),
        Anchor::CENTER_LEFT,
    );
    ui::text(
        &mut commands,
        &assets,
        format!("{endings}/{}", ENDINGS.len()),
        Vec2::new(-215.0, 210.0),
        12.0,
        theme::amber(),
        Anchor::CENTER_RIGHT,
    );
    for (index, code) in ENDINGS.into_iter().enumerate() {
        let seen = meta.has_ending(code);
        let y = 170.0 - index as f32 * 40.0;
        commands.spawn((
            Sprite {
                image: assets.tex.dot.clone(),
                color: if seen {
                    theme::amber()
                } else {
                    theme::text_faint().with_alpha(0.55)
                },
                custom_size: Some(Vec2::splat(6.0)),
                ..default()
            },
            Transform::from_xyz(-530.0, y, 8.0),
            ui::ScreenEntity,
        ));
        ui::text(
            &mut commands,
            &assets,
            if seen { ending_name(l, code) } else { "???" },
            Vec2::new(-512.0, y),
            13.0,
            if seen {
                theme::text()
            } else {
                theme::text_faint()
            },
            Anchor::CENTER_LEFT,
        );
    }

    // Lifetime counters.
    ui::caption(
        &mut commands,
        &assets,
        t(l, "LIFETIME", "累计"),
        Vec2::new(-530.0, 0.0),
        theme::text_faint(),
        Anchor::CENTER_LEFT,
    );
    let stats = [
        (t(l, "RUNS", "周目"), format!("{}", meta.runs_completed)),
        (
            t(l, "LEVELS CLEARED", "通关关卡"),
            format!("{}", meta.levels_cleared),
        ),
        (
            t(l, "HINTS", "提示"),
            format!("{}", meta.hints_used),
        ),
        (t(l, "DEMOS", "示范"), format!("{}", meta.demonstrations)),
        (t(l, "THE FORGE", "塑造台"), format!("{mastered}/3")),
        (t(l, "TRIALS", "试炼"), format!("{trials}/3")),
    ];
    for (index, (label, value)) in stats.into_iter().enumerate() {
        let y = -32.0 - index as f32 * 27.0;
        ui::caption(
            &mut commands,
            &assets,
            label,
            Vec2::new(-530.0, y),
            theme::text_faint(),
            Anchor::CENTER_LEFT,
        );
        ui::text(
            &mut commands,
            &assets,
            value,
            Vec2::new(-215.0, y),
            12.0,
            theme::text_dim(),
            Anchor::CENTER_RIGHT,
        );
    }

    // A quiet divider between profile and badges.
    commands.spawn((
        Sprite::from_color(theme::line().with_alpha(0.35), Vec2::new(1.0, 390.0)),
        Transform::from_xyz(-155.0, 8.0, 8.0),
        ui::ScreenEntity,
    ));

    // Achievements.
    ui::caption(
        &mut commands,
        &assets,
        t(l, "ACHIEVEMENTS", "成就"),
        Vec2::new(-130.0, 210.0),
        theme::text_faint(),
        Anchor::CENTER_LEFT,
    );
    ui::text(
        &mut commands,
        &assets,
        format!("{unlocked}/{}", Achievement::ALL.len()),
        Vec2::new(540.0, 210.0),
        12.0,
        theme::amber(),
        Anchor::CENTER_RIGHT,
    );

    for (index, achievement) in Achievement::ALL.into_iter().enumerate() {
        let col = index % 2;
        let row = index / 2;
        let x = 90.0 + col as f32 * 310.0;
        let y = 165.0 - row as f32 * 58.0;
        let done = achievement.is_unlocked(meta);
        let fill = if done {
            Color::srgba(0.11, 0.085, 0.045, 0.92)
        } else {
            Color::srgba(0.030, 0.047, 0.070, 0.72)
        };
        let edge = if done {
            theme::amber().with_alpha(0.45)
        } else {
            theme::text_faint().with_alpha(0.25)
        };
        commands.spawn((
            assets.tex.panel_sprite(fill, Vec2::new(286.0, 50.0)),
            Transform::from_xyz(x, y, 7.0),
            ui::ScreenEntity,
        ));
        commands.spawn((
            assets.tex.outline_sprite(edge, Vec2::new(286.0, 50.0)),
            Transform::from_xyz(x, y, 7.2),
            ui::ScreenEntity,
        ));
        commands.spawn((
            Sprite {
                image: assets.tex.dot.clone(),
                color: if done {
                    theme::amber()
                } else {
                    theme::text_faint().with_alpha(0.55)
                },
                custom_size: Some(Vec2::splat(if done { 7.0 } else { 5.0 })),
                ..default()
            },
            Transform::from_xyz(x - 123.0, y, 8.0),
            ui::ScreenEntity,
        ));
        ui::text(
            &mut commands,
            &assets,
            achievement_name(l, achievement),
            Vec2::new(x - 108.0, y + 9.0),
            10.5,
            if done {
                theme::text()
            } else {
                theme::text_dim()
            },
            Anchor::CENTER_LEFT,
        );
        ui::text(
            &mut commands,
            &assets,
            achievement_desc(l, achievement),
            Vec2::new(x - 108.0, y - 10.0),
            8.5,
            theme::text_faint(),
            Anchor::CENTER_LEFT,
        );
    }

    ui::text(
        &mut commands,
        &assets,
        t(l, "Understanding is the only unlock.", "理解，是唯一的解锁方式。"),
        Vec2::new(300.0, -215.0),
        12.0,
        theme::text_faint(),
        Anchor::CENTER,
    );
}

// ---------------------------------------------------------------- settings

/// Pause / settings panel. Opening it from Level or Lab truly stops that
/// screen's update systems because Bevy only runs them `in_state(Level/Lab)`.
/// Settings live in the save, so they survive quits and language reloads.
pub fn spawn_settings(
    mut commands: Commands,
    assets: Res<UiAssets>,
    lang: Res<LangRes>,
    save: Res<SaveRes>,
    return_to: Res<SettingsReturn>,
) {
    let l = lang.0;
    let paused = matches!(return_to.0, Screen::Level | Screen::Lab);
    let settings = &save.0.settings;

    brand_stamp(&mut commands, &assets, l);
    ui::caption(
        &mut commands,
        &assets,
        t(l, "OOI / OBSERVATION 001", "OOI / 观测 001"),
        Vec2::new(-590.0, 310.0),
        theme::text_faint(),
        Anchor::CENTER_LEFT,
    );

    ui::card(
        &mut commands,
        &assets,
        Vec2::new(0.0, -6.0),
        Vec2::new(860.0, 520.0),
        if paused {
            theme::amber().with_alpha(0.7)
        } else {
            theme::cyan().with_alpha(0.65)
        },
    );

    ui::text_styled(
        &mut commands,
        &assets,
        if paused {
            t(l, "PAUSED", "暂停")
        } else {
            t(l, "SETTINGS", "设置")
        },
        Vec2::new(0.0, 210.0),
        if paused { 42.0 } else { 32.0 },
        theme::text(),
        Anchor::CENTER,
        2.2,
    );
    if paused {
        ui::caption(
            &mut commands,
            &assets,
            t(l, "IT IS WAITING.", "它在等待。"),
            Vec2::new(0.0, 176.0),
            theme::amber().with_alpha(0.8),
            Anchor::CENTER,
        );
    }

    // ---- audio
    ui::caption(
        &mut commands,
        &assets,
        t(l, "AUDIO", "声音"),
        Vec2::new(-380.0, 138.0),
        theme::text_faint(),
        Anchor::CENTER_LEFT,
    );
    ui::text(
        &mut commands,
        &assets,
        t(l, "MASTER VOLUME", "主音量"),
        Vec2::new(-380.0, 106.0),
        13.0,
        theme::text_dim(),
        Anchor::CENTER_LEFT,
    );
    ui::text(
        &mut commands,
        &assets,
        format!("{}%", settings.volume as u32 * 10),
        Vec2::new(380.0, 106.0),
        15.0,
        theme::amber(),
        Anchor::CENTER_RIGHT,
    );
    ui::progress_bar(
        &mut commands,
        &assets,
        Vec2::new(0.0, 78.0),
        Vec2::new(650.0, 5.0),
        settings.volume as f32 / 10.0,
        theme::cyan(),
    );
    ui::action_button(
        &mut commands,
        &assets,
        Vec2::new(-380.0, 78.0),
        Vec2::new(38.0, 30.0),
        "\u{2212}",
        17.0,
        Action::VolumeDown,
        false,
    );
    ui::action_button(
        &mut commands,
        &assets,
        Vec2::new(380.0, 78.0),
        Vec2::new(38.0, 30.0),
        "+",
        17.0,
        Action::VolumeUp,
        false,
    );

    // ---- display
    ui::caption(
        &mut commands,
        &assets,
        t(l, "DISPLAY", "显示"),
        Vec2::new(-380.0, 40.0),
        theme::text_faint(),
        Anchor::CENTER_LEFT,
    );
    ui::text(
        &mut commands,
        &assets,
        t(l, "FULLSCREEN", "全屏"),
        Vec2::new(-380.0, 8.0),
        13.0,
        theme::text_dim(),
        Anchor::CENTER_LEFT,
    );
    ui::action_button(
        &mut commands,
        &assets,
        Vec2::new(368.0, 8.0),
        Vec2::new(104.0, 32.0),
        if settings.fullscreen {
            t(l, "ON", "开")
        } else {
            t(l, "OFF", "关")
        },
        12.0,
        Action::ToggleFullscreen,
        settings.fullscreen,
    );

    // ---- accessibility
    ui::caption(
        &mut commands,
        &assets,
        t(l, "ACCESSIBILITY", "辅助"),
        Vec2::new(-380.0, -40.0),
        theme::text_faint(),
        Anchor::CENTER_LEFT,
    );
    ui::text(
        &mut commands,
        &assets,
        t(l, "REDUCE MOTION", "减少动态"),
        Vec2::new(-380.0, -72.0),
        13.0,
        theme::text_dim(),
        Anchor::CENTER_LEFT,
    );
    ui::action_button(
        &mut commands,
        &assets,
        Vec2::new(368.0, -72.0),
        Vec2::new(104.0, 32.0),
        if settings.reduce_motion {
            t(l, "ON", "开")
        } else {
            t(l, "OFF", "关")
        },
        12.0,
        Action::ToggleMotion,
        settings.reduce_motion,
    );

    ui::text(
        &mut commands,
        &assets,
        t(l, "ACHIEVEMENT TOASTS", "成就提示"),
        Vec2::new(-380.0, -104.0),
        13.0,
        theme::text_dim(),
        Anchor::CENTER_LEFT,
    );
    ui::action_button(
        &mut commands,
        &assets,
        Vec2::new(368.0, -104.0),
        Vec2::new(104.0, 32.0),
        if settings.toasts {
            t(l, "ON", "开")
        } else {
            t(l, "OFF", "关")
        },
        12.0,
        Action::ToggleToasts,
        settings.toasts,
    );

    // ---- language
    ui::caption(
        &mut commands,
        &assets,
        t(l, "LANGUAGE", "语言"),
        Vec2::new(-380.0, -144.0),
        theme::text_faint(),
        Anchor::CENTER_LEFT,
    );
    ui::action_button(
        &mut commands,
        &assets,
        Vec2::new(368.0, -144.0),
        Vec2::new(104.0, 32.0),
        l.switch_label(),
        12.0,
        Action::ToggleLang,
        false,
    );

    let can_go_title = return_to.0 != Screen::Title;
    ui::primary_button(
        &mut commands,
        &assets,
        Vec2::new(if can_go_title { -118.0 } else { 0.0 }, -196.0),
        Vec2::new(if can_go_title { 214.0 } else { 236.0 }, 44.0),
        if paused {
            t(l, "RESUME", "继续")
        } else {
            t(l, "BACK", "返回")
        },
        Action::CloseSettings,
    );
    if can_go_title {
        ui::action_button(
            &mut commands,
            &assets,
            Vec2::new(136.0, -196.0),
            Vec2::new(174.0, 44.0),
            t(l, "TITLE", "标题"),
            13.0,
            Action::Title,
            false,
        );
    }
    ui::caption(
        &mut commands,
        &assets,
        t(l, "ESC  \u{00b7}  CLOSE", "ESC  \u{00b7}  关闭"),
        Vec2::new(0.0, -240.0),
        theme::text_faint(),
        Anchor::CENTER,
    );
}

// ---------------------------------------------------------------- ending

pub fn spawn_ending(
    mut commands: Commands,
    assets: Res<UiAssets>,
    lang: Res<LangRes>,
    save: Res<SaveRes>,
) {
    let l = lang.0;
    let narrative = &save.0.narrative;
    let code = narrative.ending.as_deref().unwrap_or(ENDINGS[1]);
    let ending = ending_of(code, &narrative.principles);

    brand_stamp(&mut commands, &assets, l);
    lang_button(&mut commands, &assets, l, Vec2::new(586.0, 332.0));

    // A vertical stack: the organism it became, the ending's name, then the words.
    visuals::spawn_organism(
        &mut commands,
        &assets.tex,
        Vec2::new(0.0, 178.0),
        0.62,
        0x0bad_c0de_dead_beef,
        3.0,
    );
    ui::caption(
        &mut commands,
        &assets,
        t(l, "THE END", "终"),
        Vec2::new(0.0, 318.0),
        theme::text_faint(),
        Anchor::CENTER,
    );
    ui::halo(&mut commands, &assets, Vec2::new(0.0, 46.0), 420.0, theme::amber().with_alpha(0.07), 9.0);
    ui::text_styled(
        &mut commands,
        &assets,
        ending.title(l),
        Vec2::new(0.0, 46.0),
        58.0,
        theme::amber(),
        Anchor::CENTER,
        3.0,
    );
    for x in [-1.0, 1.0] {
        commands.spawn((
            Sprite::from_color(theme::amber().with_alpha(0.45), Vec2::new(90.0, 1.0)),
            Transform::from_xyz(x * 190.0, 46.0, 10.3),
            ui::ScreenEntity,
        ));
    }
    let lines = ending.body(l).lines().count() as f32;
    centred(&mut commands, &assets, ending.body(l), Vec2::new(0.0, -2.0), 820.0, 17.0, theme::text_dim());
    let mut y = -12.0 - lines * 27.0;
    if code == ENDINGS[2] {
        centred(&mut commands, &assets, ending.own_line(l), Vec2::new(0.0, y), 820.0, 16.0, theme::cyan());
        y -= 34.0;
    }
    commands.spawn((
        Sprite::from_color(theme::line().with_alpha(0.3), Vec2::new(36.0, 1.0)),
        Transform::from_xyz(0.0, y - 4.0, 10.2),
        ui::ScreenEntity,
    ));
    centred(&mut commands, &assets, Ending::coda(l), Vec2::new(0.0, y - 16.0), 820.0, 14.0, theme::text());

    ui::caption(
        &mut commands,
        &assets,
        format!(
            "{} {}/{}    \u{00b7}    {} {}    \u{00b7}    {} {}/{}",
            t(l, "STARS", "星"),
            save.0.campaign.total_stars(),
            TOTAL as u32 * 3,
            t(l, "HINTS", "提示"),
            narrative.run_hints,
            t(l, "ENDINGS", "结局"),
            save.0.meta.endings_found().min(ENDINGS.len()),
            ENDINGS.len()
        ),
        Vec2::new(0.0, -178.0),
        theme::text_faint(),
        Anchor::CENTER,
    );

    // Endings gallery: seen endings keep their names, unseen ones stay masked.
    for (index, candidate) in ENDINGS.into_iter().enumerate() {
        let seen = save.0.meta.has_ending(candidate);
        let current = candidate == code;
        let label = if seen || current { ending_name(l, candidate) } else { "???" };
        let color = if current {
            theme::amber()
        } else if seen {
            theme::text_dim()
        } else {
            theme::text_faint()
        };
        ui::text_styled(&mut commands, &assets, label, Vec2::new(-200.0 + index as f32 * 200.0, -214.0), 10.5, color, Anchor::CENTER, 1.2);
    }
    commands.spawn((
        Sprite::from_color(theme::line().with_alpha(0.35), Vec2::new(620.0, 1.0)),
        Transform::from_xyz(0.0, -232.0, 10.2),
        ui::ScreenEntity,
    ));

    ui::primary_button(
        &mut commands,
        &assets,
        Vec2::new(-130.0, -278.0),
        Vec2::new(220.0, 46.0),
        t(l, "PLAY AGAIN", "重新开始"),
        Action::NewGame,
    );
    ui::action_button(
        &mut commands,
        &assets,
        Vec2::new(130.0, -278.0),
        Vec2::new(180.0, 42.0),
        t(l, "BACK", "返回"),
        13.0,
        Action::Title,
        false,
    );
}

/// Centre-justified wrapped text; its top edge sits at `pos`.
fn centred(commands: &mut Commands, assets: &UiAssets, s: &str, pos: Vec2, width: f32, size: f32, color: Color) {
    commands.spawn((
        Text2d::new(s),
        TextFont { font: assets.source(), font_size: FontSize::Px(size), ..default() },
        TextColor(color),
        TextLayout::new(Justify::Center, LineBreak::WordOrCharacter),
        bevy::text::TextBounds::new_horizontal(width),
        Anchor::TOP_CENTER,
        Transform::from_xyz(pos.x, pos.y, 20.0),
        ui::ScreenEntity,
    ));
}

//! Origin of Intelligence — the game shell.
//!
//! State machine: Boot → Title → Hub → Level / Forge → Card → (Choice → Ending)
//!
//! The ECS view systems intentionally accept many query parameters; splitting
//! each into a custom SystemParam would obscure the UI more than it would help.
#![allow(clippy::type_complexity, clippy::too_many_arguments)]

mod bench_view;
mod lab;
mod level;
mod save_io;
mod screens;
mod state;
mod theme;
mod toast;
mod ui;
mod visuals;

use std::time::{SystemTime, UNIX_EPOCH};

use bevy::asset::AssetPlugin;
use bevy::ecs::schedule::ScheduleConfigs;
use bevy::ecs::system::ScheduleSystem;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use oo_base::Settings;
use oo_kit::audio::{Ambience, Cue, CueQueue, MasterGain, build_sfx, spawn_ambience};
use oo_kit::scene::{BackdropStyle, ReduceMotion, spawn_backdrop, spawn_camera, spawn_fade};
use oo_kit::tex::VisualTextures;
use oo_kit::toast::ToastsEnabled;
use oo_kit::ui::clear_screen;
use oo_kit::{KitPlugin, KitSet, add_actions};

use ooi_core::level as level_at;
use ooi_core::text::ending_for;
use ooi_core::{Lang, Puzzle, SaveData, TOTAL};
use state::{
    ActivePuzzle, CodexSelection, CurrentLevel, HintState, LabState, LangRes, RecordsReturn,
    ReloadTarget, SaveRes, Screen, SettingsReturn,
};
use ui::{Action, Actions, UiAssets};

/// Game systems run between button clicks and presentation.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct GameSet;

/// Every screen with an `OnEnter` spawner; each clears on exit.
const SCREENS: [Screen; 10] = [
    Screen::Title,
    Screen::Hub,
    Screen::Level,
    Screen::Lab,
    Screen::Card,
    Screen::Choice,
    Screen::Settings,
    Screen::Codex,
    Screen::Records,
    Screen::Ending,
];

fn now_secs() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn new_seed() -> u64 {
    (now_secs() as u64) ^ ((std::process::id() as u64) << 32)
}

fn main() {
    ooi_core::i18n::install();
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "智能起源 · Origin of Intelligence".into(),
                    resolution: (1280, 720).into(),
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                file_path: if cfg!(debug_assertions) {
                    concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets").to_string()
                } else {
                    "assets".to_string()
                },
                ..default()
            }),
    )
    .insert_resource(SaveRes(SaveData::new(now_secs(), new_seed())))
    .add_systems(Startup, setup)
    .add_systems(OnEnter(Screen::Boot), boot);
    build_game(&mut app);
    app.add_systems(Update, (visuals::animate_organisms, fade_on_enter).in_set(GameSet));
    oo_kit::debug::install_screenshot(&mut app, "OOI");
    app.run();
}

/// Everything except windowing, assets and boot; shared with the smoke tests.
fn build_game(app: &mut App) {
    app.insert_resource(theme::pal())
        .add_plugins(KitPlugin)
        .init_state::<Screen>()
        .init_resource::<level::SolveDelay>()
        .init_resource::<level::ThresholdDrag>()
        .init_resource::<level::WireSelection>()
        .init_resource::<level::VectorDrag>()
        .init_resource::<level::AllocDrag>()
        .init_resource::<bench_view::BenchDrawn>()
        .init_resource::<HintState>()
        .insert_resource(LangRes(Lang::En))
        .insert_resource(CurrentLevel(0))
        .insert_resource(ActivePuzzle(ooi_core::puzzle::make_puzzle(level_at(0).unwrap())))
        .insert_resource(ReloadTarget(Screen::Title))
        .init_resource::<SettingsReturn>()
        .init_resource::<RecordsReturn>()
        .init_resource::<CodexSelection>()
        .init_resource::<toast::ToastState>()
        .init_resource::<LabState>()
        .configure_sets(Update, GameSet.after(KitSet::Buttons).before(KitSet::Present))
        .add_systems(OnEnter(Screen::Title), screens::spawn_title)
        .add_systems(OnEnter(Screen::Hub), screens::spawn_hub)
        .add_systems(OnEnter(Screen::Level), level::spawn)
        .add_systems(OnEnter(Screen::Lab), lab::spawn_lab)
        .add_systems(OnEnter(Screen::Card), screens::spawn_card)
        .add_systems(OnEnter(Screen::Choice), screens::spawn_choice)
        .add_systems(OnEnter(Screen::Settings), screens::spawn_settings)
        .add_systems(OnEnter(Screen::Codex), screens::spawn_codex)
        .add_systems(OnEnter(Screen::Records), screens::spawn_records)
        .add_systems(OnEnter(Screen::Ending), screens::spawn_ending)
        .add_systems(OnEnter(Screen::Reload), apply_reload);
    for screen in SCREENS {
        app.add_systems(OnExit(screen), clear_screen);
    }
    app.add_systems(
        Update,
        (global_keys, handle_actions, sync_font, apply_settings, autosave, toast::watch_achievements)
            .chain()
            .in_set(GameSet),
    )
    .add_systems(Update, level_systems().after(handle_actions).in_set(GameSet))
    .add_systems(
        Update,
        (lab::lab_fields_input, lab::lab_tick, lab::sync_lab_network, lab::sync_lab_field, lab::sync_lab_hud)
            .chain()
            .after(handle_actions)
            .in_set(GameSet)
            .run_if(in_state(Screen::Lab)),
    );
    add_actions::<Action>(app);
}

/// Puzzle input, then view sync, then the solve check.
fn level_systems() -> ScheduleConfigs<ScheduleSystem> {
    (
        (
            level::wake_input,
            level::threshold_input,
            level::wire_input,
            level::wire_keys,
            level::kernel_input,
            level::vector_input,
            level::allocate_input,
            level::timeline_input,
            level::attention_input,
            level::gradient_input,
            level::puzzle_keys,
        )
            .chain(),
        (
            level::descent_tick,
            level::sync_wake,
            level::sync_threshold,
            level::sync_boundary,
            level::sync_wire,
            level::sync_kernel,
            level::sync_vector,
            level::sync_allocate,
            level::timeline_tick,
            level::sync_timeline,
            level::sync_rank,
            level::sync_attention,
            level::sync_descent,
            level::sync_gradient,
            level::sync_nucleus,
            level::sync_hint,
            bench_view::sync_bench,
            level::detect_solved,
        )
            .chain(),
    )
        .chain()
        .run_if(in_state(Screen::Level))
}

fn setup(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut sources: ResMut<Assets<AudioSource>>,
) {
    let pal = theme::pal();
    let tex = VisualTextures::build(&mut images);
    spawn_camera(&mut commands, &pal);
    spawn_backdrop(&mut commands, &tex, &pal, BackdropStyle::Constellation);
    commands.insert_resource(UiAssets::load(&server, tex, Lang::En));
    commands.insert_resource(visuals::FieldTextures::build(&mut images));
    // A3: the intelligence wing hums a fourth below computing.
    commands.insert_resource(build_sfx(&mut sources, 220.0));
    spawn_ambience(&mut commands, &mut sources, &Ambience::neural());
}

/// A short veil on every screen change, unless motion is reduced.
fn fade_on_enter(
    mut commands: Commands,
    screen: Res<State<Screen>>,
    reduce: Res<ReduceMotion>,
    mut last: Local<Option<Screen>>,
) {
    let now = *screen.get();
    if *last == Some(now) || matches!(now, Screen::Boot | Screen::Reload) {
        return;
    }
    *last = Some(now);
    if !reduce.0 {
        spawn_fade(&mut commands, &theme::pal());
    }
}

fn env(key: &str) -> Option<String> {
    if cfg!(debug_assertions) { std::env::var(key).ok() } else { None }
}

fn env_num<T: std::str::FromStr>(key: &str) -> Option<T> {
    env(key).and_then(|v| v.parse().ok())
}

/// Index of the last solved level, for the archive's initial selection.
fn last_solved(save: &SaveData) -> u16 {
    (0..TOTAL).rev().find(|&i| save.is_complete(i)).unwrap_or(0)
}

/// Loads the save, then honours debug overrides:
/// `OOI_FRESH`, `OOI_LANG=zh|en|ja|ko`, `OOI_TASK=<level id or index>`,
/// `OOI_SCREEN=title|hub|level|card|choice|ending|settings|lab|codex|records`,
/// `OOI_DONE=n` (mark the first n levels solved), `OOI_REF` (show the working solution), `OOI_RUN` (press Run once).
fn boot(
    mut save: ResMut<SaveRes>,
    mut lang: ResMut<LangRes>,
    mut current: ResMut<CurrentLevel>,
    mut active: ResMut<ActivePuzzle>,
    mut lab: ResMut<LabState>,
    mut hint: ResMut<HintState>,
    mut selection: ResMut<CodexSelection>,
    mut return_to: ResMut<SettingsReturn>,
    mut next: ResMut<NextState<Screen>>,
) {
    let mut data = save_io::load().unwrap_or_else(|| SaveData::new(now_secs(), new_seed()));
    data.last_seen = now_secs();
    // Forge results from saves made before the lifetime sets existed.
    data.meta.forge_mastered.extend(data.lab.mastered.iter().cloned());
    data.meta.forge_trials.extend(data.lab.trials.iter().cloned());
    if let Some(l) = env("OOI_LANG") {
        data.settings.lang = Lang::from_code(&l);
    }
    lang.0 = data.lang();
    for i in 0..env_num::<u16>("OOI_DONE").unwrap_or(0).min(TOTAL) {
        data.complete(i, oo_base::Outcome { stars: 3 - (i % 3) as u8, metric: 0.0, alt: 0.0 }, false);
    }
    selection.0 = last_solved(&data);
    current.0 = data.resume_index();

    let task = env("OOI_TASK").and_then(|id| {
        id.parse::<u16>().ok().filter(|&i| i < TOTAL).or_else(|| (0..TOTAL).find(|&i| level_at(i).unwrap().id == id))
    });
    if let Some(i) = task {
        current.0 = i;
    }
    active.set(ooi_core::puzzle::make_puzzle(level_at(current.0).unwrap()));
    *hint = HintState { level: Some(current.0), ..default() };

    let screen = match env("OOI_SCREEN").as_deref() {
        Some("hub") => Screen::Hub,
        Some("level") => Screen::Level,
        Some("card") => Screen::Card,
        Some("choice") => Screen::Choice,
        Some("lab") => Screen::Lab,
        Some("codex") => Screen::Codex,
        Some("records") => Screen::Records,
        Some("settings") => {
            return_to.0 = Screen::Hub;
            Screen::Settings
        }
        Some("ending") => {
            let code = env("OOI_ENDING").unwrap_or_else(|| ooi_core::ENDINGS[2].into());
            data.narrative.ending = Some(code.clone());
            data.narrative.principles = vec![2, 0, 1];
            data.meta.remember_ending(&code);
            Screen::Ending
        }
        _ if task.is_some() => Screen::Level,
        _ => Screen::Title,
    };
    if screen == Screen::Level {
        debug_level(&mut active.0, &mut hint);
    }
    if screen == Screen::Codex
        && let Some(i) = env_num::<u16>("OOI_CODEX_SELECT").filter(|&i| i < TOTAL)
    {
        selection.0 = i;
    }
    if env("OOI_RECORDS_DEMO").is_some() {
        records_demo(&mut data);
    }
    if screen == Screen::Lab {
        debug_lab(&mut lab, data.seed);
    }
    lab.sync_from_save(&data);
    save.0 = data;
    next.set(screen);
}

/// Debug puzzle state for screenshots of a level mid-play.
fn debug_level(puzzle: &mut Puzzle, hint: &mut HintState) {
    if let Some(stage) = env_num::<u8>("OOI_HINT_STAGE") {
        hint.stage = stage.min(3);
    }
    if env("OOI_REF").is_some() {
        puzzle.solve();
    }
    let run = env("OOI_RUN").is_some();
    match puzzle {
        Puzzle::Wake(p) => (0..env_num::<u32>("OOI_WAKE_PULSES").unwrap_or(0)).for_each(|_| { p.pulse(); }),
        Puzzle::Boundary(p) if run => {
            p.commit();
        }
        Puzzle::Descent(p) => {
            if let Some(i) = env_num::<usize>("OOI_DESCENT_START") {
                p.select(i);
            }
            if let Some(steps) = env_num::<u32>("OOI_DESCENT_STEPS")
                && p.run()
            {
                (0..steps).for_each(|_| { p.step(); });
            }
        }
        Puzzle::Gradient(p) => {
            for _ in 0..env_num::<u32>("OOI_GRADIENT_STEPS").unwrap_or(u32::from(run)) {
                let (x, y) = p.position();
                p.set_aim((-y).atan2(-x));
                p.step();
            }
        }
        Puzzle::Nucleus(p) => {
            for i in 0..env_num::<usize>("OOI_NUCLEUS_TOKENS").unwrap_or(0).min(5) {
                if !p.included(i) {
                    p.toggle(i);
                }
            }
        }
        _ => {}
    }
}

/// A plausible lifetime profile for the Records screen.
fn records_demo(data: &mut SaveData) {
    for code in &ooi_core::ENDINGS[..2] {
        data.meta.endings_seen.insert((*code).into());
    }
    data.meta.runs_completed = 2;
    data.meta.levels_cleared = TOTAL as u32;
    data.meta.hints_used = 3;
    data.meta.demonstrations = 1;
    for id in ["xor", "ring", "spiral"] {
        data.meta.forge_mastered.insert(id.into());
        data.meta.forge_trials.insert(id.into());
    }
    data.meta.no_hint_runs = 1;
}

/// `OOI_LAB_DATASET=xor|ring|spiral`, `OOI_LAB_RUN`, `OOI_LAB_EPOCHS=n`.
fn debug_lab(lab: &mut LabState, seed: u64) {
    use ooi_core::lab::DatasetKind;
    if let Some(kind) = env("OOI_LAB_DATASET").and_then(|k| DatasetKind::from_id(&k)) {
        lab.select_dataset(DatasetKind::ALL.iter().position(|k| *k == kind).unwrap_or(0), seed);
    }
    if env("OOI_LAB_RUN").is_some() {
        lab.step(env_num("OOI_LAB_EPOCHS").unwrap_or(260));
        lab.running = true;
    }
}

/// Esc opens / closes Settings; M toggles mute.
fn global_keys(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<Screen>>,
    mut next: ResMut<NextState<Screen>>,
    mut return_to: ResMut<SettingsReturn>,
    mut save: ResMut<SaveRes>,
) {
    if keys.just_pressed(KeyCode::KeyM) {
        save.0.settings.muted = !save.0.settings.muted;
    }
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    match *state.get() {
        Screen::Boot | Screen::Reload => {}
        Screen::Settings => next.set(return_to.0),
        current => {
            return_to.0 = current;
            next.set(Screen::Settings);
        }
    }
}

/// Keeps the UI font on the script of the current language.
fn sync_font(lang: Res<LangRes>, assets: Option<ResMut<UiAssets>>, mut last: Local<Option<Lang>>) {
    let Some(mut assets) = assets else { return };
    if *last != Some(lang.0) {
        *last = Some(lang.0);
        assets.set_lang(lang.0);
    }
}

/// Mirrors settings into the kit (volume, motion, toasts) and the window.
fn apply_settings(
    save: Res<SaveRes>,
    mut last: Local<Option<Settings>>,
    mut gain: ResMut<MasterGain>,
    mut reduce: ResMut<ReduceMotion>,
    mut toasts: ResMut<ToastsEnabled>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let s = &save.0.settings;
    if last.as_ref() == Some(s) {
        return;
    }
    *last = Some(s.clone());
    oo_kit::sync_settings(s, &mut gain, &mut reduce, &mut toasts);
    oo_kit::apply_window_mode(s.fullscreen, &mut windows);
}

/// Writes at most once a second while the save changes, and on exit.
fn autosave(
    time: Res<Time>,
    save: Res<SaveRes>,
    mut exits: MessageReader<AppExit>,
    mut pending: Local<bool>,
    mut since: Local<f32>,
) {
    if save.is_changed() && !save.is_added() {
        *pending = true;
    }
    *since += time.delta_secs();
    let exiting = exits.read().count() > 0;
    if *pending && (exiting || *since >= 1.0) {
        *pending = false;
        *since = 0.0;
        save_io::save(&save.0);
    }
}

fn apply_reload(target: Res<ReloadTarget>, mut next: ResMut<NextState<Screen>>) {
    next.set(target.0);
}

fn start_level(index: u16, current: &mut CurrentLevel, active: &mut ActivePuzzle, hint: &mut HintState, next: &mut NextState<Screen>) {
    let Some(def) = level_at(index) else { return };
    current.0 = index;
    active.set(ooi_core::puzzle::make_puzzle(def));
    *hint = HintState { level: Some(index), ..default() };
    next.set(Screen::Level);
}

/// Puzzle-side actions; a rejected move plays the error cue.
fn puzzle_action(action: Action, puzzle: &mut Puzzle, cues: &mut CueQueue) {
    let wrong_before = match puzzle {
        Puzzle::Bench(p) => p.state().wrong,
        _ => 0,
    };
    let ok = match (action, &mut *puzzle) {
        (Action::Option(i), Puzzle::Rank(p)) => {
            p.pick(i);
            true
        }
        (Action::Option(i), Puzzle::Descent(p)) => p.select(i),
        (Action::Option(i), Puzzle::Nucleus(p)) => p.toggle(i),
        (Action::Run, Puzzle::Boundary(p)) => {
            p.commit();
            true
        }
        (Action::Run, Puzzle::Descent(p)) => {
            p.run();
            true
        }
        (Action::Run, Puzzle::Gradient(p)) => p.is_solved() || p.step(),
        (Action::ToggleGate(slot), Puzzle::Wire(p)) => {
            p.toggle_gate(slot);
            true
        }
        (Action::BenchDial(d, delta), Puzzle::Bench(p)) => {
            p.nudge(d, delta);
            true
        }
        (Action::BenchToggle(i), Puzzle::Bench(p)) => {
            p.toggle(i);
            true
        }
        (Action::BenchTarget(i), Puzzle::Bench(p)) => {
            p.click(i);
            p.state().wrong == wrong_before
        }
        _ => true,
    };
    if !ok {
        cues.play(Cue::Error);
    }
}

/// What the action handler touches besides the queue.
#[derive(bevy::ecs::system::SystemParam)]
struct Ctx<'w> {
    state: Res<'w, State<Screen>>,
    next: ResMut<'w, NextState<Screen>>,
    reload: ResMut<'w, ReloadTarget>,
    current: ResMut<'w, CurrentLevel>,
    active: ResMut<'w, ActivePuzzle>,
    save: ResMut<'w, SaveRes>,
    lang: ResMut<'w, LangRes>,
    hint: ResMut<'w, HintState>,
    lab: ResMut<'w, LabState>,
    selection: ResMut<'w, CodexSelection>,
    return_to: ResMut<'w, SettingsReturn>,
    records_return: ResMut<'w, RecordsReturn>,
    cues: ResMut<'w, CueQueue>,
}

impl Ctx<'_> {
    fn start(&mut self, index: u16) {
        start_level(index, &mut self.current, &mut self.active, &mut self.hint, &mut self.next);
    }

    fn reload(&mut self, screen: Screen) {
        self.reload.0 = screen;
        self.next.set(Screen::Reload);
    }
}

fn handle_actions(mut actions: ResMut<Actions>, mut cx: Ctx, mut exit: MessageWriter<AppExit>) {
    for action in std::mem::take(&mut actions.0) {
        cx.cues.play(Cue::Click);
        act(&mut cx, action, &mut exit);
    }
}

fn act(cx: &mut Ctx, action: Action, exit: &mut MessageWriter<AppExit>) {
    match action {
        Action::NewGame => {
            // A new run resets the campaign, never settings or the lifetime profile.
            cx.save.0.new_run();
            *cx.lab = LabState::new(cx.save.0.seed);
            cx.lab.sync_from_save(&cx.save.0);
            cx.start(0);
        }
        Action::Continue => {
            let index = cx.save.0.resume_index();
            cx.start(index);
        }
        Action::Play(index) => {
            if cx.save.0.is_unlocked(index) {
                cx.start(index);
            }
        }
        Action::Option(_)
        | Action::Run
        | Action::ToggleGate(_)
        | Action::BenchDial(..)
        | Action::BenchToggle(_)
        | Action::BenchTarget(_) => puzzle_action(action, &mut cx.active.0, &mut cx.cues),
        Action::Next => {
            let next_index = cx.current.0 + 1;
            if next_index >= TOTAL {
                cx.next.set(Screen::Ending);
            } else if cx.save.0.is_unlocked(next_index) {
                cx.start(next_index);
            } else {
                cx.next.set(Screen::Hub);
            }
        }
        Action::Hint => hint(cx),
        Action::ResetLevel => {
            cx.active.0.reset();
            cx.hint.resets += 1;
            if let Some(def) = level_at(cx.current.0) {
                cx.save.0.campaign.note_attempt(def.id);
            }
        }
        Action::FinaleChoice(i) => {
            let (_, code) = ending_for(i, &cx.save.0.narrative.principles);
            let save = &mut cx.save.0;
            save.narrative.ending = Some(code.into());
            save.meta.record_ending(code);
            if save.narrative.run_hints == 0 {
                save.meta.record_no_hint_run();
            }
            save_io::save(save);
            cx.next.set(Screen::Ending);
        }
        Action::LabOpen => {
            cx.lab.sync_from_save(&cx.save.0);
            cx.next.set(Screen::Lab);
        }
        Action::LabTrial(index) => {
            cx.lab.select_dataset(index, cx.save.0.seed);
            cx.lab.sync_from_save(&cx.save.0);
            cx.next.set(Screen::Lab);
        }
        Action::LabDataset(index) => {
            cx.lab.select_dataset(index, cx.save.0.seed);
            cx.lab.sync_from_save(&cx.save.0);
            cx.reload(Screen::Lab);
        }
        Action::LabDepth(delta) => {
            cx.lab.change_depth(delta, cx.save.0.seed);
            cx.lab.sync_from_save(&cx.save.0);
            cx.reload(Screen::Lab);
        }
        Action::LabWidth(delta) => {
            cx.lab.change_width(delta, cx.save.0.seed);
            cx.lab.sync_from_save(&cx.save.0);
            cx.reload(Screen::Lab);
        }
        Action::LabRate(index) => cx.lab.set_rate(index),
        Action::LabToggle => {
            cx.lab.running = !cx.lab.running;
            cx.lab.epoch_accum = 0.0;
        }
        Action::LabStep => {
            cx.lab.running = false;
            cx.lab.step(24);
        }
        Action::LabReset => {
            cx.lab.reset(cx.save.0.seed);
            cx.lab.sync_from_save(&cx.save.0);
        }
        other => menu_action(cx, other, exit),
    }
}

/// Two text hints, then "show me" (solves the level; the solve counts as assisted).
fn hint(cx: &mut Ctx) {
    if cx.hint.stage >= 3 {
        return;
    }
    cx.hint.stage += 1;
    let stage = cx.hint.stage;
    let save = &mut cx.save.0;
    if let Some(def) = level_at(cx.current.0) {
        save.campaign.note_hint(def.id, stage);
    }
    save.meta.hints_used += 1;
    save.narrative.run_hints += 1;
    if stage >= 3 {
        cx.active.0.solve();
        save.meta.demonstrations += 1;
    }
}

/// Navigation, settings and archive actions.
fn menu_action(cx: &mut Ctx, action: Action, exit: &mut MessageWriter<AppExit>) {
    let screen = *cx.state.get();
    match action {
        Action::Records => {
            cx.records_return.0 = screen;
            cx.next.set(Screen::Records);
        }
        Action::RecordsClose => cx.next.set(cx.records_return.0),
        Action::CodexOpen => {
            cx.selection.0 = last_solved(&cx.save.0);
            cx.next.set(Screen::Codex);
        }
        Action::CodexSelect(index) => {
            if level_at(index).is_some() {
                cx.selection.0 = index;
                cx.reload(Screen::Codex);
            }
        }
        Action::CodexPlay(index) => {
            if cx.save.0.is_complete(index) {
                cx.start(index);
            }
        }
        Action::Settings => {
            if screen != Screen::Settings {
                cx.return_to.0 = screen;
                cx.next.set(Screen::Settings);
            }
        }
        Action::CloseSettings => {
            let back = if cx.return_to.0 == Screen::Settings { Screen::Title } else { cx.return_to.0 };
            cx.next.set(back);
        }
        Action::VolumeDown => cx.save.0.settings.volume_down(),
        Action::VolumeUp => cx.save.0.settings.volume_up(),
        Action::ToggleFullscreen => cx.save.0.settings.fullscreen ^= true,
        Action::ToggleMotion => cx.save.0.settings.reduce_motion ^= true,
        Action::ToggleToasts => cx.save.0.settings.toasts ^= true,
        Action::Quit => {
            save_io::save(&cx.save.0);
            exit.write(AppExit::Success);
        }
        Action::Hub => cx.next.set(Screen::Hub),
        Action::Title => cx.next.set(Screen::Title),
        Action::ToggleLang => {
            let l = cx.lang.0.toggled();
            cx.save.0.settings.lang = l;
            cx.lang.0 = l;
            cx.reload(screen);
        }
        _ => {}
    }
}

#[cfg(test)]
mod smoke {
    use super::*;
    use bevy::state::app::StatesPlugin;
    use ooi_core::level_by_id;
    use oo_kit::ui::ScreenEntity;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, AssetPlugin::default(), bevy::input::InputPlugin))
            .insert_resource(SaveRes(SaveData::new(0, 1)))
            .insert_resource(UiAssets::placeholder())
            .init_resource::<visuals::FieldTextures>();
        build_game(&mut app);
        app
    }

    fn go(app: &mut App, screen: Screen) {
        app.world_mut().resource_mut::<NextState<Screen>>().set(screen);
        app.update();
        app.update();
    }

    fn push(app: &mut App, action: Action) {
        app.world_mut().resource_mut::<Actions>().0.push(action);
        app.update();
        app.update();
    }

    fn screen(app: &App) -> Screen {
        *app.world().resource::<State<Screen>>().get()
    }

    fn screen_entities(app: &mut App) -> usize {
        app.world_mut().query_filtered::<Entity, With<ScreenEntity>>().iter(app.world()).count()
    }

    fn index_of(id: &str) -> u16 {
        (0..TOTAL).find(|&i| level_at(i).unwrap().id == id).expect("level id")
    }

    fn load(app: &mut App, id: &str) {
        let i = index_of(id);
        app.world_mut().resource_mut::<CurrentLevel>().0 = i;
        app.world_mut().resource_mut::<ActivePuzzle>().set(ooi_core::puzzle::make_puzzle(level_by_id(id).unwrap()));
        go(app, Screen::Level);
    }

    fn complete(app: &mut App, i: u16) {
        let outcome = oo_base::Outcome { stars: 3, metric: 0.0, alt: 0.0 };
        app.world_mut().resource_mut::<SaveRes>().0.complete(i, outcome, false);
    }

    #[test]
    fn every_screen_spawns_entities() {
        let mut app = test_app();
        for s in SCREENS {
            if s == Screen::Level {
                continue;
            }
            go(&mut app, s);
            assert!(screen_entities(&mut app) > 3, "{s:?} spawned too little");
        }
    }

    #[test]
    fn every_level_spawns_and_clears() {
        let mut app = test_app();
        go(&mut app, Screen::Hub);
        for i in 0..TOTAL {
            let id = level_at(i).unwrap().id;
            load(&mut app, id);
            assert!(screen_entities(&mut app) > 5, "level {id} spawned too little");
            go(&mut app, Screen::Hub);
        }
    }

    #[test]
    fn every_level_in_every_language_spawns() {
        let mut app = test_app();
        for l in Lang::ALL {
            app.world_mut().resource_mut::<LangRes>().0 = l;
            for i in 0..TOTAL {
                load(&mut app, level_at(i).unwrap().id);
                go(&mut app, Screen::Card);
            }
        }
    }

    #[test]
    fn bench_actions_reach_the_puzzle() {
        let mut app = test_app();
        load(&mut app, "ch6_l5");
        let before = match &app.world().resource::<ActivePuzzle>().0 {
            Puzzle::Bench(p) => p.state().version,
            _ => panic!("ch6_l5 should be a bench"),
        };
        push(&mut app, Action::BenchDial(0, 1));
        let Puzzle::Bench(p) = &app.world().resource::<ActivePuzzle>().0 else { unreachable!() };
        assert!(p.state().version > before, "dial did not move");
        assert_eq!(screen(&app), Screen::Level);
    }

    #[test]
    fn solving_a_level_shows_its_card() {
        let mut app = test_app();
        load(&mut app, "ch1_l3");
        app.world_mut().resource_mut::<ActivePuzzle>().0.solve();
        app.world_mut().resource_mut::<level::SolveDelay>().0 = 2.0;
        app.update();
        app.update();
        assert_eq!(screen(&app), Screen::Card);
        let save = &app.world().resource::<SaveRes>().0;
        assert!(save.is_complete(index_of("ch1_l3")));
        assert_eq!(save.campaign.stars("ch1_l3"), 3);
    }

    #[test]
    fn hints_step_to_show_me_and_cost_stars() {
        let mut app = test_app();
        load(&mut app, "ch1_l3");
        for _ in 0..4 {
            push(&mut app, Action::Hint);
        }
        let save = &app.world().resource::<SaveRes>().0;
        assert_eq!(save.meta.hints_used, 3);
        assert_eq!(save.meta.demonstrations, 1);
        assert_eq!(save.narrative.run_hints, 3);
        let hint = app.world().resource::<HintState>();
        assert_eq!(hint.stage, 3);
        assert_eq!(hint.stars(), 2);
        assert!(app.world().resource::<ActivePuzzle>().0.is_solved());
    }

    #[test]
    fn settings_are_wired_and_return_to_the_paused_screen() {
        let mut app = test_app();
        go(&mut app, Screen::Hub);
        push(&mut app, Action::Settings);
        assert_eq!(screen(&app), Screen::Settings);
        assert_eq!(app.world().resource::<SettingsReturn>().0, Screen::Hub);

        let before = app.world().resource::<SaveRes>().0.settings.volume;
        push(&mut app, Action::VolumeDown);
        assert!(app.world().resource::<SaveRes>().0.settings.volume < before);
        push(&mut app, Action::ToggleMotion);
        assert!(app.world().resource::<SaveRes>().0.settings.reduce_motion);
        assert!(app.world().resource::<ReduceMotion>().0, "kit did not see the setting");
        let toasts = app.world().resource::<SaveRes>().0.settings.toasts;
        push(&mut app, Action::ToggleToasts);
        assert_ne!(app.world().resource::<SaveRes>().0.settings.toasts, toasts);

        push(&mut app, Action::CloseSettings);
        assert_eq!(screen(&app), Screen::Hub);
    }

    #[test]
    fn escape_pauses_without_losing_hints() {
        use bevy::ecs::system::RunSystemOnce;
        let mut app = test_app();
        load(&mut app, "ch1_l3");
        push(&mut app, Action::Hint);
        push(&mut app, Action::Hint);
        for want in [Screen::Settings, Screen::Level] {
            // The input plugin clears key edges in PreUpdate, so run the handler directly.
            app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::Escape);
            app.world_mut().run_system_once(global_keys).unwrap();
            app.world_mut().resource_mut::<ButtonInput<KeyCode>>().reset(KeyCode::Escape);
            app.update();
            app.update();
            assert_eq!(screen(&app), want);
        }
        assert_eq!(app.world().resource::<HintState>().stage, 2, "pause erased the hints");
    }

    #[test]
    fn language_cycles_through_all_four() {
        let mut app = test_app();
        go(&mut app, Screen::Hub);
        let mut seen = Vec::new();
        for _ in 0..4 {
            push(&mut app, Action::ToggleLang);
            app.update();
            seen.push(app.world().resource::<LangRes>().0);
            assert_eq!(screen(&app), Screen::Hub);
        }
        for l in Lang::ALL {
            assert!(seen.contains(&l), "{l:?} never reached");
        }
    }

    #[test]
    fn codex_opens_selects_and_replays_completed_concepts() {
        let mut app = test_app();
        for i in 0..3 {
            complete(&mut app, i);
        }
        go(&mut app, Screen::Hub);
        push(&mut app, Action::CodexOpen);
        assert_eq!(screen(&app), Screen::Codex);
        assert_eq!(app.world().resource::<CodexSelection>().0, 2);

        push(&mut app, Action::CodexSelect(5));
        assert_eq!(app.world().resource::<CodexSelection>().0, 5);
        push(&mut app, Action::CodexPlay(5));
        assert_eq!(screen(&app), Screen::Codex, "locked concept replayed");

        complete(&mut app, 5);
        push(&mut app, Action::CodexPlay(5));
        assert_eq!(screen(&app), Screen::Level);
        assert_eq!(app.world().resource::<CurrentLevel>().0, 5);
    }

    #[test]
    fn forge_actions_are_wired() {
        let mut app = test_app();
        go(&mut app, Screen::Hub);
        push(&mut app, Action::LabOpen);
        assert_eq!(screen(&app), Screen::Lab);
        push(&mut app, Action::LabToggle);
        assert!(app.world().resource::<LabState>().running);
        push(&mut app, Action::LabStep);
        let lab = app.world().resource::<LabState>();
        assert!(!lab.running);
        assert_eq!(lab.training.epochs, 24);
    }

    #[test]
    fn chapter_trial_opens_forge_on_the_right_dataset() {
        let mut app = test_app();
        push(&mut app, Action::LabTrial(2));
        assert_eq!(screen(&app), Screen::Lab);
        assert_eq!(app.world().resource::<LabState>().kind, ooi_core::lab::DatasetKind::Spiral);
    }

    #[test]
    fn descent_selects_and_runs() {
        let mut app = test_app();
        let id = (0..TOTAL).map(|i| level_at(i).unwrap()).find(|d| matches!(ooi_core::puzzle::make_puzzle(d), Puzzle::Descent(_)));
        let Some(def) = id else { return };
        load(&mut app, def.id);
        push(&mut app, Action::Option(0));
        push(&mut app, Action::Run);
        let Puzzle::Descent(p) = &app.world().resource::<ActivePuzzle>().0 else { unreachable!() };
        assert_eq!(p.selected(), Some(0));
        assert!(p.is_running() || p.steps() > 0);
    }

    #[test]
    fn new_game_keeps_settings_and_profile() {
        let mut app = test_app();
        {
            let save = &mut app.world_mut().resource_mut::<SaveRes>().0;
            save.settings.volume_down();
            save.meta.record_ending(ooi_core::ENDINGS[0]);
            save.meta.record_forge_mastered("xor");
        }
        complete(&mut app, 0);
        let volume = app.world().resource::<SaveRes>().0.settings.volume;
        go(&mut app, Screen::Title);
        push(&mut app, Action::NewGame);
        let save = &app.world().resource::<SaveRes>().0;
        assert_eq!(save.completed_count(), 0);
        assert_eq!(save.settings.volume, volume);
        assert!(save.meta.has_ending(ooi_core::ENDINGS[0]));
        assert!(save.meta.forge_mastered.contains("xor"));
        assert_eq!(screen(&app), Screen::Level);
    }

    #[test]
    fn records_returns_to_where_it_opened() {
        let mut app = test_app();
        for from in [Screen::Title, Screen::Hub] {
            go(&mut app, from);
            push(&mut app, Action::Records);
            assert_eq!(screen(&app), Screen::Records);
            assert!(screen_entities(&mut app) > 5);
            push(&mut app, Action::RecordsClose);
            assert_eq!(screen(&app), from);
        }
    }

    #[test]
    fn full_playthrough_reaches_an_ending() {
        let mut app = test_app();
        go(&mut app, Screen::Title);
        push(&mut app, Action::NewGame);
        for _ in 0..200 {
            match screen(&app) {
                Screen::Ending => break,
                Screen::Level => {
                    app.world_mut().resource_mut::<ActivePuzzle>().0.solve();
                    app.world_mut().resource_mut::<level::SolveDelay>().0 = 2.0;
                    app.update();
                    app.update();
                }
                Screen::Choice => push(&mut app, Action::FinaleChoice(2)),
                Screen::Card => push(&mut app, Action::Next),
                other => panic!("unexpected screen {other:?} during playthrough"),
            }
        }
        assert_eq!(screen(&app), Screen::Ending);
        assert!(screen_entities(&mut app) > 5);
        let save = &app.world().resource::<SaveRes>().0;
        assert!(save.all_complete());
        assert_eq!(save.meta.levels_cleared, TOTAL as u32);
        assert_eq!(save.narrative.ending.as_deref(), Some(ooi_core::ENDINGS[2]));
        assert!(!save.narrative.principles.is_empty(), "ch7_l5 did not record principles");
        assert_eq!(save.meta.runs_completed, 1);
        assert_eq!(save.meta.no_hint_runs, 1);
    }
}

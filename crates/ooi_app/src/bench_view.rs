//! 工作台视图：旋钮、开关、目标、读数与热图。
//!
//! 结构在进关时画好；之后 `sync_bench` 按 `BenchState::version` 刷新文字、颜色与位置。
//! 目标数量变了（极少）就整屏重载。曲线、图表、分布条和决策场属于动态层，每次刷新整层重画。

use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::text::{LineBreak, TextBounds};
use oo_kit::theme::mix;
use oo_kit::ui::Ui;

use crate::state::{ActivePuzzle, LangRes, ReloadTarget, Screen};
use crate::theme;
use crate::ui::{self, Action, ButtonUi, UiAssets};

use ooi_core::puzzle::bench::{Bar, BenchPuzzle, Chart, Curve, Eval, Image, Layout, Mark};
use ooi_core::{Lang, Puzzle, t};

const TOP: f32 = 205.0;
const AREA_Y: f32 = -20.0;
const AREA_H: f32 = 430.0;

/// What is on screen. `level::spawn` resets it whenever the bench is drawn.
#[derive(Resource, Default)]
pub struct BenchDrawn {
    /// `(version, lang)` of the last refresh; `None` forces one.
    pub key: Option<(u32, Lang)>,
    /// Targets laid out at spawn; a different count needs a full redraw.
    pub targets: usize,
    pub frames: Frames,
}

/// Where the dynamic layer goes; fixed at spawn.
#[derive(Clone, Copy, Default)]
pub struct Frames {
    /// Bottom-left corner and side of the plot square.
    plot: Option<(Vec2, f32)>,
    /// Bottom-left corner and size of the chart box.
    chart: Option<(Vec2, Vec2)>,
    /// Top-left corner, width and row height of the bar list.
    bars: Option<(Vec2, f32, f32)>,
}

/// Redrawn from scratch on every refresh.
#[derive(Component)]
pub struct Dyn;

#[derive(Component)]
pub struct DialValue(usize);
#[derive(Component)]
pub struct DialStep;
#[derive(Component)]
pub struct ToggleChip(usize);
#[derive(Component)]
pub struct ToggleDot(usize);
#[derive(Component)]
pub struct TargetChip(usize);
#[derive(Component)]
pub struct TargetLabel(usize);
#[derive(Component)]
pub struct PlotDot {
    i: usize,
    origin: Vec2,
    side: f32,
}
#[derive(Component)]
pub struct PlotLine {
    origin: Vec2,
    side: f32,
}
#[derive(Component)]
pub struct ReadLabel(usize);
#[derive(Component)]
pub struct ReadValue(usize);
#[derive(Component)]
pub struct Pixel {
    image: usize,
    index: usize,
}
#[derive(Component)]
pub struct FaultText;

/// Left edge and width of the three bands: controls | targets & images | readouts.
#[derive(Clone, Copy)]
struct Cols {
    ctrl: (f32, f32),
    area: (f32, f32),
    read: (f32, f32),
}

fn cols(controls: bool, area: bool) -> Cols {
    match (controls, area) {
        (true, true) => Cols { ctrl: (-565.0, 350.0), area: (-185.0, 470.0), read: (315.0, 255.0) },
        (false, true) => Cols { ctrl: (0.0, 0.0), area: (-565.0, 640.0), read: (110.0, 460.0) },
        (true, false) => Cols { ctrl: (-565.0, 460.0), area: (0.0, 0.0), read: (-40.0, 610.0) },
        (false, false) => Cols { ctrl: (0.0, 0.0), area: (0.0, 0.0), read: (-300.0, 600.0) },
    }
}

/// Rough advance width; enough to lay out chips before the text exists.
fn text_w(s: &str, size: f32) -> f32 {
    s.chars().map(|c| if c.is_ascii() { 0.56 } else { 1.0 }).sum::<f32>() * size
}

fn mark_fill(m: Mark) -> Color {
    let base = theme::glass_raised();
    match m {
        Mark::Neutral => base,
        Mark::Pos => mix(base, theme::cyan(), 0.30),
        Mark::Neg => mix(base, theme::violet(), 0.45),
        Mark::Hot => mix(base, theme::amber(), 0.42),
        Mark::Done => mix(base, theme::mint(), 0.26),
    }
}

fn mark_dot(m: Mark) -> Color {
    match m {
        Mark::Neutral => theme::text_dim(),
        Mark::Pos => theme::cyan(),
        Mark::Neg => theme::violet(),
        Mark::Hot => theme::amber(),
        Mark::Done => theme::mint(),
    }
}

fn pixel_color(v: f32) -> Color {
    let v = v.clamp(-1.0, 1.0);
    let floor = theme::bg_deep();
    if v >= 0.0 { mix(floor, theme::cyan(), v) } else { mix(floor, theme::amber(), -v) }
}

fn ok_color(ok: Option<bool>) -> Color {
    match ok {
        Some(true) => theme::mint(),
        Some(false) => theme::amber(),
        None => theme::text(),
    }
}

/// An empty text slot filled in by `sync_bench`.
fn slot(commands: &mut Commands, assets: &UiAssets, pos: Vec2, size: f32, anchor: Anchor, width: Option<f32>, tag: impl Bundle) {
    let mut e = commands.spawn((
        Text2d::new(""),
        TextFont { font: assets.source(), font_size: FontSize::Px(size), ..default() },
        TextColor(theme::text()),
        anchor,
        Transform::from_xyz(pos.x, pos.y, 22.0),
        ui::ScreenEntity,
        tag,
    ));
    if let Some(w) = width {
        e.insert((TextLayout::new(Justify::Left, LineBreak::WordOrCharacter), TextBounds::new_horizontal(w)));
    }
}

fn caption(commands: &mut Commands, assets: &UiAssets, s: &str, left: f32) {
    ui::caption(commands, assets, s, Vec2::new(left, TOP + 18.0), theme::text_faint(), Anchor::CENTER_LEFT);
}

pub fn spawn_bench(commands: &mut Commands, assets: &UiAssets, lang: Lang, p: &BenchPuzzle) -> Frames {
    let spec = p.spec();
    let eval = p.eval(lang);
    let targets = p.targets(lang);
    let controls = !spec.dials.is_empty() || !spec.toggles.is_empty() || spec.fault.is_some();
    let side_panel = !eval.images.is_empty() || (!eval.bars.is_empty() && matches!(spec.layout, Layout::Plot | Layout::Grid(_)));
    let has_area = spec.layout != Layout::None || side_panel || !eval.bars.is_empty();
    let c = cols(controls, has_area);
    let mut frames = Frames::default();

    if controls {
        caption(commands, assets, t(lang, "ADJUST", "调整"), c.ctrl.0);
        let bottom = spawn_controls(commands, assets, lang, p, c.ctrl);
        if eval.chart.is_some() {
            frames.chart = Some(spawn_chart_box(commands, assets, (c.ctrl.0, c.ctrl.1), bottom));
        }
    }
    if has_area {
        caption(commands, assets, t(lang, "OBSERVE", "观察"), c.area.0);
        let (left, w) = c.area;
        let split = spec.layout != Layout::None && side_panel;
        let target_w = if split { w * 0.5 - 10.0 } else { w };
        let mut below = TOP - 8.0;
        match spec.layout {
            Layout::Plot => frames.plot = Some(spawn_plot(commands, assets, (left, target_w), targets.len())),
            Layout::Grid(n) => spawn_grid(commands, assets, (left, target_w), n, targets.len()),
            Layout::Row => {
                let labels: Vec<&str> = targets.iter().map(|t| t.label.as_str()).collect();
                below = spawn_row(commands, assets, (left, target_w), &labels) - 22.0;
            }
            Layout::None => {}
        }
        let side = if split { (left + w * 0.5 + 10.0, w * 0.5 - 10.0) } else { (left, w) };
        if !eval.images.is_empty() {
            spawn_images(commands, assets, side, &eval);
        } else if !eval.bars.is_empty() {
            let top = if split { TOP - 8.0 } else { below };
            let room = top - 28.0 - (AREA_Y - AREA_H / 2.0);
            let row = (room / eval.bars.len() as f32).clamp(30.0, 50.0);
            frames.bars = Some((Vec2::new(side.0, top), side.1, row));
        }
    }
    caption(commands, assets, t(lang, "READOUT", "读数"), c.read.0);
    spawn_readouts(commands, assets, c.read, &eval);
    frames
}

/// The chart's well under the controls; its contents are dynamic.
fn spawn_chart_box(commands: &mut Commands, assets: &UiAssets, (left, w): (f32, f32), top: f32) -> (Vec2, Vec2) {
    let bottom = AREA_Y - AREA_H / 2.0 + 30.0;
    let top = (top - 46.0).min(bottom + 280.0);
    let size = Vec2::new(w - 20.0, (top - bottom).max(120.0));
    let origin = Vec2::new(left + 10.0, bottom);
    let mut u = Ui::new(commands, assets, theme::pal());
    u.well(origin + size / 2.0, size + 20.0);
    for k in 1..4 {
        let f = k as f32 / 4.0;
        u.line(origin + Vec2::new(0.0, size.y * f), origin + Vec2::new(size.x, size.y * f), 1.0, theme::line().with_alpha(0.18), 5.5);
    }
    (origin, size)
}

/// Returns the y below the last control.
fn spawn_controls(commands: &mut Commands, assets: &UiAssets, lang: Lang, p: &BenchPuzzle, (left, w): (f32, f32)) -> f32 {
    let spec = p.spec();
    let mut y = TOP - 8.0;
    if spec.fault.is_some() {
        slot(commands, assets, Vec2::new(left, y), 13.0, Anchor::TOP_LEFT, Some(w), FaultText);
        y -= 78.0;
    }
    for (i, d) in spec.dials.iter().enumerate() {
        ui::text(commands, assets, d.label.get(lang), Vec2::new(left, y - 8.0), 13.0, theme::text_dim(), Anchor::CENTER_LEFT);
        let row = y - 38.0;
        let coarse = d.max - d.min > 12;
        let steps: &[(i32, f32)] = if coarse {
            &[(-5, 22.0), (-1, 68.0), (1, 212.0), (5, 258.0)]
        } else {
            &[(-1, 22.0), (1, 212.0)]
        };
        for &(delta, dx) in steps {
            let label = match delta {
                -1 => "-".to_string(),
                1 => "+".to_string(),
                d if d < 0 => format!("{d}"),
                d => format!("+{d}"),
            };
            let size = Vec2::new(if delta.abs() == 1 { 40.0 } else { 44.0 }, 30.0);
            let e = ui::action_button(commands, assets, Vec2::new(left + dx, row), size, label, 14.0, Action::BenchDial(i, delta), false);
            commands.entity(e).insert(DialStep);
        }
        slot(commands, assets, Vec2::new(left + 140.0, row), 16.0, Anchor::CENTER, None, DialValue(i));
        y -= 72.0;
    }
    for i in 0..spec.toggles.len() {
        let pos = Vec2::new(left + w / 2.0, y - 18.0);
        let size = Vec2::new(w, 36.0);
        commands.spawn((
            assets.tex.chip_sprite(theme::glass_raised(), size),
            Transform::from_xyz(pos.x, pos.y, 10.0),
            ui::hit(size / 2.0, Action::BenchToggle(i), theme::glass_raised(), theme::panel_hover()),
            ToggleChip(i),
            ui::ScreenEntity,
        ));
        commands.spawn((
            assets.tex.dot_sprite(theme::text_faint(), 10.0),
            Transform::from_xyz(left + 18.0, pos.y, 11.0),
            ToggleDot(i),
            ui::ScreenEntity,
        ));
        ui::text(commands, assets, spec.toggles[i].get(lang), Vec2::new(left + 34.0, pos.y), 13.0, theme::text(), Anchor::CENTER_LEFT);
        y -= 46.0;
    }
    y
}

fn spawn_plot(commands: &mut Commands, assets: &UiAssets, (left, w): (f32, f32), count: usize) -> (Vec2, f32) {
    let side = w.min(AREA_H - 30.0);
    let origin = Vec2::new(left + w / 2.0 - side / 2.0, AREA_Y - side / 2.0);
    let center = origin + Vec2::splat(side / 2.0);
    let mut u = Ui::new(commands, assets, theme::pal());
    u.well(center, Vec2::splat(side + 16.0));
    for k in 1..4 {
        let f = k as f32 / 4.0 * side;
        u.line(origin + Vec2::new(f, 0.0), origin + Vec2::new(f, side), 1.0, theme::line().with_alpha(0.25), 5.5);
        u.line(origin + Vec2::new(0.0, f), origin + Vec2::new(side, f), 1.0, theme::line().with_alpha(0.25), 5.5);
    }
    commands.spawn((
        Sprite::from_color(theme::cyan_hot(), Vec2::new(1.0, 2.0)),
        Transform::from_xyz(center.x, center.y, 8.0),
        PlotLine { origin, side },
        ui::ScreenEntity,
    ));
    for i in 0..count {
        commands.spawn((
            assets.tex.dot_sprite(theme::text_dim(), 18.0),
            Transform::from_xyz(center.x, center.y, 12.0),
            ui::hit(Vec2::splat(12.0), Action::BenchTarget(i), theme::text_dim(), theme::text()),
            PlotDot { i, origin, side },
            ui::ScreenEntity,
        ));
    }
    (origin, side)
}

fn spawn_grid(commands: &mut Commands, assets: &UiAssets, (left, w): (f32, f32), cols: usize, count: usize) {
    let cols = cols.max(1);
    let rows = count.div_ceil(cols);
    let cell = (w / cols as f32).min((AREA_H - 40.0) / rows.max(1) as f32).min(56.0);
    let x0 = left + w / 2.0 - cell * cols as f32 / 2.0 + cell / 2.0;
    let y0 = AREA_Y + cell * rows as f32 / 2.0 - cell / 2.0;
    for i in 0..count {
        let pos = Vec2::new(x0 + (i % cols) as f32 * cell, y0 - (i / cols) as f32 * cell);
        let size = Vec2::splat(cell - 4.0);
        commands.spawn((
            assets.tex.chip_sprite(theme::glass_raised(), size),
            Transform::from_xyz(pos.x, pos.y, 10.0),
            ui::hit(size / 2.0, Action::BenchTarget(i), theme::glass_raised(), theme::panel_hover()),
            TargetChip(i),
            ui::ScreenEntity,
        ));
        slot(commands, assets, pos, 12.0, Anchor::CENTER, None, TargetLabel(i));
    }
}

/// Returns the bottom edge of the last row.
fn spawn_row(commands: &mut Commands, assets: &UiAssets, (left, w): (f32, f32), labels: &[&str]) -> f32 {
    // Rows of chips; when the full-size chips would run past the area, shrink them.
    let rows = |font: f32| {
        let (mut x, mut n) = (left, 1);
        for label in labels {
            let cw = (text_w(label, font) + 30.0).min(w);
            if x > left && x + cw > left + w {
                x = left;
                n += 1;
            }
            x += cw + 10.0;
        }
        n as f32
    };
    let (h, gap, font) = if rows(14.0) * 50.0 > AREA_H - 20.0 { (28.0, 6.0, 12.0) } else { (40.0, 10.0, 14.0) };
    let (mut x, mut y) = (left, TOP - 8.0 - h / 2.0);
    for (i, label) in labels.iter().enumerate() {
        let cw = (text_w(label, font) + 30.0).min(w);
        if x > left && x + cw > left + w {
            x = left;
            y -= h + gap;
        }
        let pos = Vec2::new(x + cw / 2.0, y);
        let size = Vec2::new(cw, h);
        commands.spawn((
            assets.tex.chip_sprite(theme::glass_raised(), size),
            Transform::from_xyz(pos.x, pos.y, 10.0),
            ui::hit(size / 2.0, Action::BenchTarget(i), theme::glass_raised(), theme::panel_hover()),
            TargetChip(i),
            ui::ScreenEntity,
        ));
        slot(commands, assets, pos, font, Anchor::CENTER, None, TargetLabel(i));
        x += cw + 10.0;
    }
    y - h / 2.0
}

fn spawn_images(commands: &mut Commands, assets: &UiAssets, (left, w): (f32, f32), eval: &Eval) {
    // One row for up to three images; four or more wrap into two columns.
    let count = eval.images.len();
    let cols = if count >= 4 { 2 } else { count.max(1) };
    let rows = count.div_ceil(cols);
    let (n, r) = (cols as f32, rows as f32);
    let gap = 24.0;
    let side = ((w - gap * (n - 1.0)) / n).min((AREA_H - 20.0) / r - 40.0).min(260.0);
    let total = side * n + gap * (n - 1.0);
    let pitch = side + 40.0;
    let mut u = Ui::new(commands, assets, theme::pal());
    for (k, img) in eval.images.iter().enumerate() {
        let (c, rw) = ((k % cols) as f32, (k / cols) as f32);
        let cx = left + (w - total) / 2.0 + side / 2.0 + c * (side + gap);
        let center = Vec2::new(cx, AREA_Y + 10.0 + (r - 1.0) * pitch / 2.0 - rw * pitch);
        let wd = img.w.max(1);
        let ht = img.px.len().div_ceil(wd).max(1);
        let px = side / wd.max(ht) as f32;
        u.well(center, Vec2::new(px * wd as f32, px * ht as f32) + 12.0);
        for index in 0..img.px.len() {
            let (cx0, cy0) = ((index % wd) as f32, (index / wd) as f32);
            let pos = center + Vec2::new((cx0 - (wd as f32 - 1.0) / 2.0) * px, ((ht as f32 - 1.0) / 2.0 - cy0) * px);
            u.commands.spawn((
                Sprite::from_color(theme::bg_deep(), Vec2::splat(px - 1.0)),
                Transform::from_xyz(pos.x, pos.y, 7.0),
                Pixel { image: k, index },
                ui::ScreenEntity,
            ));
        }
        u.text(img.label.clone(), center - Vec2::new(0.0, px * ht as f32 / 2.0 + 22.0), 13.0, theme::text_dim(), Anchor::CENTER);
    }
}

fn spawn_readouts(commands: &mut Commands, assets: &UiAssets, (left, w): (f32, f32), eval: &Eval) {
    let n = eval.readouts.len().max(1) as f32;
    let step = (AREA_H / n).min(58.0);
    for i in 0..eval.readouts.len() {
        let y = TOP - 4.0 - i as f32 * step;
        commands.spawn((
            Sprite::from_color(theme::line().with_alpha(0.35), Vec2::new(w, 1.0)),
            Transform::from_xyz(left + w / 2.0, y + 4.0, 5.5),
            ui::ScreenEntity,
        ));
        slot(commands, assets, Vec2::new(left, y - 3.0), 12.0, Anchor::TOP_LEFT, Some(w), ReadLabel(i));
        slot(commands, assets, Vec2::new(left, y - 20.0), 15.0, Anchor::TOP_LEFT, Some(w), ReadValue(i));
    }
}

/// Two points where `w0·x + w1·y + b = 0` crosses the unit square.
fn clip_line([a, b, c]: [f32; 3]) -> Option<(Vec2, Vec2)> {
    let mut pts: Vec<Vec2> = Vec::new();
    let mut add = |p: Vec2| {
        if (-1e-4..=1.0001).contains(&p.x) && (-1e-4..=1.0001).contains(&p.y) && pts.iter().all(|q| q.distance(p) > 1e-3) {
            pts.push(p);
        }
    };
    if b.abs() > 1e-6 {
        add(Vec2::new(0.0, -c / b));
        add(Vec2::new(1.0, -(a + c) / b));
    }
    if a.abs() > 1e-6 {
        add(Vec2::new(-c / a, 0.0));
        add(Vec2::new(-(b + c) / a, 1.0));
    }
    (pts.len() >= 2).then(|| (pts[0], pts[1]))
}

type BenchTexts<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Text2d,
        &'static mut TextColor,
        AnyOf<(&'static DialValue, &'static TargetLabel, &'static ReadLabel, &'static ReadValue, &'static FaultText)>,
    ),
>;

type BenchSprites<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Sprite,
        &'static mut Transform,
        Option<&'static mut ButtonUi>,
        AnyOf<(
            &'static ToggleChip,
            &'static ToggleDot,
            &'static TargetChip,
            &'static PlotDot,
            &'static PlotLine,
            &'static Pixel,
            &'static DialStep,
        )>,
    ),
>;

#[allow(clippy::too_many_arguments)]
pub fn sync_bench(
    mut commands: Commands,
    assets: Res<UiAssets>,
    old: Query<Entity, With<Dyn>>,
    active: Res<ActivePuzzle>,
    lang: Res<LangRes>,
    mut drawn: ResMut<BenchDrawn>,
    mut reload: ResMut<ReloadTarget>,
    mut next: ResMut<NextState<Screen>>,
    mut texts: BenchTexts,
    mut sprites: BenchSprites,
) {
    let Puzzle::Bench(p) = &active.0 else { return };
    let l = lang.0;
    let key = (p.state().version, l);
    if drawn.key == Some(key) {
        return;
    }
    let targets = p.targets(l);
    if drawn.key.is_some() && targets.len() != drawn.targets {
        reload.0 = Screen::Level;
        next.set(Screen::Reload);
        return;
    }
    drawn.key = Some(key);
    let spec = p.spec();
    let state = p.state();
    let eval = p.eval(l);
    let locked = p.locked();
    for e in &old {
        commands.entity(e).despawn();
    }
    draw_dynamic(&mut commands, &assets, drawn.frames, &eval);

    for (mut text, mut color, tags) in &mut texts {
        let (s, c) = match tags {
            (Some(DialValue(i)), ..) => {
                let d = &spec.dials[*i];
                (d.text(l, state.dials.get(*i).copied().unwrap_or(d.start)), if locked { theme::text_faint() } else { theme::text() })
            }
            (_, Some(TargetLabel(i)), ..) => (targets.get(*i).map(|t| t.label.clone()).unwrap_or_default(), theme::text()),
            (_, _, Some(ReadLabel(i)), ..) => (eval.readouts.get(*i).map(|r| r.label.clone()).unwrap_or_default(), theme::text_dim()),
            (_, _, _, Some(ReadValue(i)), _) => match eval.readouts.get(*i) {
                Some(r) => {
                    let mark = match r.ok {
                        Some(true) => "\u{2713}  ",
                        Some(false) => "\u{00d7}  ",
                        None => "",
                    };
                    (format!("{mark}{}", r.value), ok_color(r.ok))
                }
                None => (String::new(), theme::text()),
            },
            _ => match spec.fault {
                Some(f) if !locked => (format!("{}\n{}", t(l, "FOUND", "找到了"), f.get(l)), theme::mint()),
                _ => (
                    t(l, "Something is wrong. Click the evidence first; the controls unlock once you find it.", "有地方不对。先点出证据，找到后才能动手修。").to_string(),
                    theme::amber(),
                ),
            },
        };
        text.0 = s;
        color.0 = c;
    }

    let line = eval.line.and_then(clip_line);
    for (mut sprite, mut tf, button, tags) in &mut sprites {
        match tags {
            (Some(ToggleChip(i)), ..) => {
                let on = state.toggles.get(*i).copied().unwrap_or(false);
                let base = if on { mix(theme::glass_raised(), theme::cyan(), 0.22) } else { theme::glass_raised() };
                if let Some(mut b) = button {
                    b.base = base;
                    b.enabled = !locked;
                }
                sprite.color = if locked { base.with_alpha(0.45) } else { base };
            }
            (_, Some(ToggleDot(i)), ..) => {
                let on = state.toggles.get(*i).copied().unwrap_or(false);
                sprite.color = if on { theme::cyan_hot() } else { theme::text_faint() };
            }
            (_, _, Some(TargetChip(i)), ..) => {
                let fill = targets.get(*i).map_or(theme::glass_raised(), |t| mark_fill(t.mark));
                if let Some(mut b) = button {
                    b.base = fill;
                    b.hover = mix(fill, theme::text(), 0.12);
                }
            }
            (_, _, _, Some(d), ..) => {
                let Some(target) = targets.get(d.i) else { continue };
                let pos = d.origin + Vec2::new(target.pos.0, target.pos.1) * d.side;
                tf.translation.x = pos.x;
                tf.translation.y = pos.y;
                if let Some(mut b) = button {
                    b.base = mark_dot(target.mark);
                    b.hover = mix(b.base, theme::text(), 0.4);
                }
            }
            (_, _, _, _, Some(pl), ..) => match line {
                Some((a, b)) => {
                    let (a, b) = (pl.origin + a * pl.side, pl.origin + b * pl.side);
                    let mid = (a + b) / 2.0;
                    let dir = b - a;
                    sprite.custom_size = Some(Vec2::new(dir.length(), 2.0));
                    sprite.color = theme::cyan_hot();
                    *tf = Transform::from_xyz(mid.x, mid.y, 8.0).with_rotation(Quat::from_rotation_z(dir.y.atan2(dir.x)));
                }
                None => sprite.color = Color::NONE,
            },
            (_, _, _, _, _, Some(px), _) => {
                let v = eval.images.get(px.image).and_then(|img| img.px.get(px.index)).copied().unwrap_or(0.0);
                sprite.color = pixel_color(v);
            }
            (.., Some(DialStep)) => {
                if let Some(mut b) = button {
                    b.enabled = !locked;
                }
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------- 动态层

const DASH: f32 = 7.0;
const GAP: f32 = 5.0;

/// `Ui` that tags everything it draws as `Dyn`.
struct Pen<'a, 'w, 's>(Ui<'a, 'w, 's>);

impl Pen<'_, '_, '_> {
    fn tag(&mut self, e: Entity) {
        self.0.commands.entity(e).insert(Dyn);
    }
    fn line(&mut self, a: Vec2, b: Vec2, width: f32, color: Color, z: f32) {
        let e = self.0.line(a, b, width, color, z);
        self.tag(e);
    }
    fn rect(&mut self, pos: Vec2, size: Vec2, color: Color, z: f32) {
        let e = self.0.rect(pos, size, color, z);
        self.tag(e);
    }
    fn chip(&mut self, pos: Vec2, size: Vec2, color: Color, z: f32) {
        let e = self.0.chip(pos, size, color, z);
        self.tag(e);
    }
    fn dot(&mut self, pos: Vec2, size: f32, color: Color, z: f32) {
        let e = self.0.dot(pos, size, color, z);
        self.tag(e);
    }
    fn text(&mut self, s: String, pos: Vec2, size: f32, color: Color, anchor: Anchor) {
        let e = self.0.text(s, pos, size, color, anchor);
        self.tag(e);
    }
}

/// A polyline in 0..1 space mapped into `origin + p·size`; points outside the box break it.
fn polyline(u: &mut Pen, curve: &Curve, origin: Vec2, size: Vec2, z: f32) {
    let color = mark_dot(curve.mark);
    let inside = |p: (f32, f32)| (-0.001..=1.001).contains(&p.0) && (-0.001..=1.001).contains(&p.1);
    let mut walked = 0.0;
    for w in curve.pts.windows(2) {
        if !inside(w[0]) || !inside(w[1]) {
            continue;
        }
        let a = origin + Vec2::new(w[0].0, w[0].1) * size;
        let b = origin + Vec2::new(w[1].0, w[1].1) * size;
        let len = a.distance(b);
        if len < 0.5 {
            continue;
        }
        if !curve.dashed {
            u.line(a, b, 2.0, color, z);
            continue;
        }
        // Dashes follow the arc length so they stay even across short segments.
        let mut t = 0.0;
        while t < len {
            let phase = (walked + t) % (DASH + GAP);
            let run = if phase < DASH { (DASH - phase).min(len - t) } else { (DASH + GAP - phase).min(len - t) };
            if phase < DASH {
                u.line(a.lerp(b, t / len), a.lerp(b, (t + run) / len), 1.5, color.with_alpha(0.85), z);
            }
            t += run.max(0.01);
        }
        walked += len;
    }
}

fn field(u: &mut Pen, img: &Image, origin: Vec2, side: f32) {
    let n = img.w.max(1);
    let rows = img.px.len().div_ceil(n).max(1);
    let cell = Vec2::new(side / n as f32, side / rows as f32);
    for (i, &v) in img.px.iter().enumerate() {
        let (c, r) = ((i % n) as f32, (i / n) as f32);
        let pos = origin + Vec2::new((c + 0.5) * cell.x, side - (r + 0.5) * cell.y);
        let tint = if v >= 0.0 { theme::cyan() } else { theme::violet() };
        u.rect(pos, cell + 0.5, tint.with_alpha(v.abs().min(1.0) * 0.22), 6.0);
    }
}

fn chart(u: &mut Pen, c: &Chart, origin: Vec2, size: Vec2) {
    let top = origin.y + size.y;
    u.text(c.title.clone(), Vec2::new(origin.x + 2.0, top + 2.0), 13.0, theme::text(), Anchor::BOTTOM_LEFT);
    u.text(c.x_label.clone(), Vec2::new(origin.x + size.x, origin.y - 8.0), 11.0, theme::text_faint(), Anchor::TOP_RIGHT);
    if let Some(y) = c.limit {
        let limit = Curve::new("", vec![(0.0, y), (1.0, y)], Mark::Done).dashed();
        polyline(u, &limit, origin, size, 7.0);
    }
    if let Some(x) = c.cursor {
        let x = origin.x + x.clamp(0.0, 1.0) * size.x;
        u.rect(Vec2::new(x, origin.y + size.y / 2.0), Vec2::new(2.0, size.y), theme::amber().with_alpha(0.75), 7.5);
        u.dot(Vec2::new(x, top), 9.0, theme::amber(), 8.5);
    }
    for curve in &c.curves {
        polyline(u, curve, origin, size, 8.0);
    }
    // Legend, right to left along the top edge.
    let mut x = origin.x + size.x - 4.0;
    for curve in c.curves.iter().rev() {
        let w = text_w(&curve.label, 11.0);
        u.text(curve.label.clone(), Vec2::new(x, top - 12.0), 11.0, theme::text_dim(), Anchor::CENTER_RIGHT);
        x -= w + 8.0;
        let swatch = Curve::new("", vec![(0.0, 0.5), (1.0, 0.5)], curve.mark);
        let swatch = if curve.dashed { swatch.dashed() } else { swatch };
        polyline(u, &swatch, Vec2::new(x - 18.0, top - 13.0), Vec2::new(18.0, 2.0), 9.0);
        x -= 32.0;
    }
}

fn bars(u: &mut Pen, title: &str, list: &[Bar], (top_left, w, row): (Vec2, f32, f32)) {
    u.text(title.to_string(), top_left, 13.0, theme::text(), Anchor::TOP_LEFT);
    let track_h = (row * 0.22).clamp(7.0, 10.0);
    for (i, b) in list.iter().enumerate() {
        let y = top_left.y - 30.0 - i as f32 * row;
        let color = mark_dot(b.mark);
        u.text(b.label.clone(), Vec2::new(top_left.x, y), 13.0, if b.mark == Mark::Neutral { theme::text_dim() } else { theme::text() }, Anchor::CENTER_LEFT);
        u.text(b.text.clone(), Vec2::new(top_left.x + w, y), 13.0, color, Anchor::CENTER_RIGHT);
        let ty = y - row * 0.42;
        u.chip(Vec2::new(top_left.x + w / 2.0, ty), Vec2::new(w, track_h), theme::bg_deep().with_alpha(0.9), 6.0);
        let fill = b.value.clamp(0.0, 1.0) * w;
        if fill > 1.0 {
            u.chip(Vec2::new(top_left.x + fill / 2.0, ty), Vec2::new(fill.max(track_h), track_h), color.with_alpha(0.9), 6.5);
        }
        if let Some(g) = b.ghost {
            let gx = top_left.x + g.clamp(0.0, 1.0) * w;
            u.rect(Vec2::new(gx, ty), Vec2::new(2.0, track_h + 8.0), theme::text().with_alpha(0.7), 7.0);
        }
    }
}

/// Clears and redraws everything that depends on the current eval but not on the layout.
pub fn draw_dynamic(commands: &mut Commands, assets: &UiAssets, frames: Frames, eval: &Eval) {
    let mut u = Pen(Ui::new(commands, assets, theme::pal()));
    if let Some((origin, side)) = frames.plot {
        if let Some(img) = &eval.field {
            field(&mut u, img, origin, side);
        }
        for c in &eval.curves {
            polyline(&mut u, c, origin, Vec2::splat(side), 9.0);
        }
    }
    if let (Some(c), Some((origin, size))) = (&eval.chart, frames.chart) {
        chart(&mut u, c, origin, size);
    }
    if let Some(frame) = frames.bars {
        bars(&mut u, &eval.bars_title, &eval.bars, frame);
    }
}

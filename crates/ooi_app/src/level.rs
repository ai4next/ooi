//! 关卡屏：按谜题类型分发到不同视图。

use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::text::{LineBreak, TextBounds};
use bevy::window::PrimaryWindow;

use crate::bench_view::{BenchDrawn, spawn_bench};
use crate::save_io;
use crate::visuals::FieldTextures;
use ooi_core::text::{attention_tokens, level_hint, level_prompt, nucleus_token, rank_item, wire_label, word_label};
use crate::state::{ActivePuzzle, CurrentLevel, HintState, LangRes, SaveRes, Screen};
use crate::theme;
use crate::ui::{self, Action, UiAssets};
use ooi_core::boundary::Point2;
use ooi_core::puzzle::descent::{DOMAIN_X, DOMAIN_Y};
use ooi_core::puzzle::gradient::DOMAIN as G_DOMAIN;
use oo_base::Outcome;
use oo_kit::audio::{Cue, CueQueue};
use ooi_core::puzzle::bench::principle_order;
use ooi_core::{Lang, Puzzle, TOTAL, level, t};

const AXIS_Y: f32 = -150.0;
const AXIS_HALF: f32 = 560.0;
const LINE_H: f32 = 200.0;
const LINE_CY: f32 = -45.0;

const CANVAS_C: Vec2 = Vec2::new(0.0, -20.0);
const CANVAS_HALF: f32 = 160.0;

// Loss-landscape level.
const D_C: Vec2 = Vec2::new(0.0, -20.0);
const D_HALF: Vec2 = Vec2::new(330.0, 175.0);
const D_TRAIL_MAX: usize = 152;

// Gradient-step level: a square bowl so the contour rings stay circular.
const G_C: Vec2 = Vec2::new(0.0, -15.0);
const G_HALF: Vec2 = Vec2::splat(180.0);
const G_TRAIL_MAX: usize = 18;

// Top-p nucleus level: five horizontal probability bars.
const N_C: Vec2 = Vec2::new(0.0, -20.0);
const N_BAR_X: f32 = -300.0;
const N_BAR_W: f32 = 560.0;
const N_BAR_H: f32 = 24.0;
const N_MASS_W: f32 = 700.0;
const N_ROW_Y: [f32; 5] = [95.0, 50.0, 5.0, -40.0, -85.0];

fn gmap(x: f32, y: f32) -> Vec2 {
    G_C + Vec2::new(x / G_DOMAIN * G_HALF.x, y / G_DOMAIN * G_HALF.y)
}

fn dmap(x: f32, y: f32) -> Vec2 {
    D_C + Vec2::new(x / DOMAIN_X * D_HALF.x, y / DOMAIN_Y * D_HALF.y)
}

fn theta_to_x(theta: f32) -> f32 {
    -AXIS_HALF + theta / 10.0 * AXIS_HALF * 2.0
}

fn x_to_theta(x: f32) -> f32 {
    ((x + AXIS_HALF) / (AXIS_HALF * 2.0) * 10.0).clamp(0.0, 10.0)
}

fn map01(p: (f32, f32)) -> Vec2 {
    CANVAS_C
        + Vec2::new(
            (p.0 - 0.5) * CANVAS_HALF * 2.0,
            (p.1 - 0.5) * CANVAS_HALF * 2.0,
        )
}

#[derive(Resource, Default)]
pub struct SolveDelay(pub f32);

#[derive(Resource, Default)]
pub struct ThresholdDrag(bool);

// ---------------------------------------------------------------- 组件

#[derive(Component)]
pub struct TDot {
    x: f32,
    label: bool,
}

#[derive(Component)]
pub struct TLine;

#[derive(Component)]
pub struct THandle;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum THud {
    FalseAlarms,
    Misses,
    Status,
}

#[derive(Component)]
pub struct BDot(Point2);

#[derive(Component)]
pub struct BDotGlow(Point2);

#[derive(Component)]
pub struct BFieldCell {
    x: usize,
    y: usize,
}

#[derive(Component)]
pub struct BLine;

#[derive(Component)]
pub struct BReadout;

#[derive(Component)]
pub struct DescentStartRing(usize);

#[derive(Component)]
pub struct DescentBall;

#[derive(Component)]
pub struct DescentBallGlow;

#[derive(Component)]
pub struct DescentTrailDot(usize);

#[derive(Component)]
pub struct DescentStatusText;

#[derive(Component)]
pub struct GradientBall;

#[derive(Component)]
pub struct GradientBallGlow;

#[derive(Component)]
pub struct GradientTrailDot(usize);

#[derive(Component)]
pub struct GradientCandidateRing;

#[derive(Component)]
pub struct GradientAimLine;

#[derive(Component)]
pub struct GradientArrowLine;

#[derive(Component)]
pub struct GradientArrowTip;

#[derive(Component)]
pub struct GradientStatusText;

#[derive(Component)]
pub struct NucleusBar(usize);

#[derive(Component)]
pub struct NucleusMassFill;

#[derive(Component)]
pub struct NucleusStatusText;

#[derive(Component)]
pub struct WakeCore;

#[derive(Component)]
pub struct WakeHalo;

#[derive(Component)]
pub struct WakeBarFill {
    origin: Vec2,
    width: f32,
}

#[derive(Component)]
pub struct WakeChargeText;

#[derive(Component)]
pub struct WakeCountText;

// ---------------------------------------------------------------- 场景

pub fn spawn(
    mut commands: Commands,
    assets: Res<UiAssets>,
    fields: Res<FieldTextures>,
    lang: Res<LangRes>,
    active: Res<ActivePuzzle>,
    current: Res<CurrentLevel>,
    mut delay: ResMut<SolveDelay>,
    mut hint: ResMut<HintState>,
    mut bench: ResMut<BenchDrawn>,
) {
    delay.0 = 0.0;
    let l = lang.0;
    let def = level(current.0).expect("level exists");
    // Hints survive a pause/settings round-trip; they reset when the level
    // changes or a new attempt starts via start_level.
    if hint.level != Some(current.0) {
        *hint = HintState { level: Some(current.0), ..default() };
    }

    // Thin global progress rail.
    ui::progress_bar(
        &mut commands,
        &assets,
        Vec2::new(0.0, 354.0),
        Vec2::new(1180.0, 2.0),
        (current.0 as f32 + 1.0) / TOTAL as f32,
        theme::amber(),
    );

    // Hush the spinning backdrop so the instruments read cleanly; the panel itself gets a deeper floor.
    commands.spawn((
        Sprite::from_color(theme::bg_deep().with_alpha(0.55), Vec2::new(1400.0, 800.0)),
        Transform::from_xyz(0.0, 0.0, -60.0),
        ui::ScreenEntity,
    ));
    commands.spawn((
        assets
            .tex
            .panel_sprite(theme::bg_deep().with_alpha(0.62), Vec2::new(1180.0, 500.0)),
        Transform::from_xyz(0.0, -10.0, 0.25),
        ui::ScreenEntity,
    ));
    // Shared observation window.
    commands.spawn((
        assets
            .tex
            .panel_sprite(theme::glass().with_alpha(0.34), Vec2::new(1180.0, 500.0)),
        Transform::from_xyz(0.0, -10.0, 0.3),
        ui::ScreenEntity,
    ));
    commands.spawn((
        assets
            .tex
            .outline_sprite(theme::line().with_alpha(0.38), Vec2::new(1180.0, 500.0)),
        Transform::from_xyz(0.0, -10.0, 0.45),
        ui::ScreenEntity,
    ));
    for (x, y, sx, sy) in [
        (-590.0, 240.0, 1.0, 1.0),
        (590.0, 240.0, -1.0, 1.0),
        (-590.0, -260.0, 1.0, -1.0),
        (590.0, -260.0, -1.0, -1.0),
    ] {
        commands.spawn((
            Sprite::from_color(theme::cyan().with_alpha(0.55), Vec2::new(18.0, 1.0)),
            Transform::from_xyz(x + sx * 9.0, y, 0.6),
            ui::ScreenEntity,
        ));
        commands.spawn((
            Sprite::from_color(theme::cyan().with_alpha(0.55), Vec2::new(1.0, 18.0)),
            Transform::from_xyz(x, y - sy * 9.0, 0.6),
            ui::ScreenEntity,
        ));
    }

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

    if let Some(chapter) = ooi_core::chapter_of(current.0) {
        ui::caption(
            &mut commands,
            &assets,
            chapter.title(l),
            Vec2::new(-480.0, 344.0),
            theme::text_faint(),
            Anchor::CENTER_LEFT,
        );
    }
    ui::text(
        &mut commands,
        &assets,
        def.title(l),
        Vec2::new(-480.0, 318.0),
        23.0,
        theme::text(),
        Anchor::CENTER_LEFT,
    );
    let prompt = level_prompt(l, def.id);
    if !prompt.is_empty() {
        ui::wrapped_text(
            &mut commands,
            &assets,
            prompt,
            Vec2::new(-480.0, 294.0),
            880.0,
            14.0,
            theme::text_dim(),
            Anchor::TOP_LEFT,
        );
    }

    ui::caption(
        &mut commands,
        &assets,
        format!("{:02} / {:02}", current.0 + 1, TOTAL),
        Vec2::new(590.0, 344.0),
        theme::amber().with_alpha(0.85),
        Anchor::CENTER_RIGHT,
    );

    ui::action_button(
        &mut commands,
        &assets,
        Vec2::new(520.0, 282.0),
        Vec2::new(140.0, 30.0),
        "",
        11.0,
        Action::Hint,
        false,
    );
    commands.spawn((
        Text2d::new(t(l, "HINT", "提示")),
        TextFont {
            font: assets.source(),
            font_size: FontSize::Px(11.0),
            ..default()
        },
        TextColor(theme::text()),
        Anchor::CENTER,
        Transform::from_xyz(520.0, 282.0, 21.0),
        HintButtonText,
        ui::ScreenEntity,
    ));

    commands.spawn((
        Text2d::new(""),
        TextFont {
            font: assets.source(),
            font_size: FontSize::Px(12.0),
            ..default()
        },
        TextColor(theme::amber()),
        Anchor::CENTER,
        Transform::from_xyz(0.0, 256.0, 21.0),
        HintLine,
        ui::ScreenEntity,
    ));

    ui::action_button(
        &mut commands,
        &assets,
        Vec2::new(-558.0, -330.0),
        Vec2::new(110.0, 28.0),
        t(l, "RESET", "重置"),
        10.0,
        Action::ResetLevel,
        false,
    );
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

    match &active.0 {
        Puzzle::Wake(p) => spawn_wake(&mut commands, &assets, l, p.charge(), p.pulses()),
        Puzzle::Threshold(p) => spawn_threshold(&mut commands, &assets, l, p.config(), p.theta()),
        Puzzle::Boundary(p) => spawn_boundary(&mut commands, &assets, l, p.config()),
        Puzzle::Wire(p) => spawn_wire(&mut commands, &assets, l, def.id, p),
        Puzzle::Kernel(p) => spawn_kernel(&mut commands, &assets, l, p),
        Puzzle::Bench(p) => {
            let frames = spawn_bench(&mut commands, &assets, l, p);
            *bench = BenchDrawn { key: None, targets: p.targets(l).len(), frames };
        }
        Puzzle::Vector(p) => spawn_vector(&mut commands, &assets, l, p),
        Puzzle::Allocate(p) => spawn_allocate(&mut commands, &assets, l, p),
        Puzzle::Timeline(p) => spawn_timeline(&mut commands, &assets, l, p),
        Puzzle::Rank(p) => spawn_rank(&mut commands, &assets, l, def.id, p),
        Puzzle::Attention(p) => spawn_attention(&mut commands, &assets, l, def.id, p),
        Puzzle::Descent(p) => spawn_descent(&mut commands, &assets, &fields.descent, l, p),
        Puzzle::Gradient(p) => spawn_gradient(&mut commands, &assets, &fields.gradient, l, p),
        Puzzle::Nucleus(p) => spawn_nucleus(&mut commands, &assets, l, p),
        Puzzle::Pending(_) => {
            ui::text(
                &mut commands,
                &assets,
                t(l, "THIS MECHANIC IS NOT WIRED YET", "这个机制还没接上"),
                Vec2::new(0.0, 0.0),
                22.0,
                theme::text_dim(),
                Anchor::CENTER,
            );
        }
    }
}

fn spawn_nucleus(
    commands: &mut Commands,
    assets: &UiAssets,
    lang: Lang,
    puzzle: &ooi_core::NucleusPuzzle,
) {
    let cfg = puzzle.config();
    ui::card(
        commands,
        assets,
        N_C,
        Vec2::new(940.0, 430.0),
        theme::cyan().with_alpha(0.45),
    );
    ui::caption(
        commands,
        assets,
        t(lang, "NUCLEUS", "核采样"),
        Vec2::new(N_C.x - 430.0, 175.0),
        theme::text_faint(),
        Anchor::CENTER_LEFT,
    );
    ui::caption(
        commands,
        assets,
        format!(
            "{} {:.0}%",
            t(lang, "TARGET", "目标"),
            cfg.threshold * 100.0
        ),
        Vec2::new(N_C.x + 430.0, 175.0),
        theme::amber().with_alpha(0.8),
        Anchor::CENTER_RIGHT,
    );

    // Cumulative mass gauge with a 90% tick.
    commands.spawn((
        Sprite::from_color(theme::cyan_dim().with_alpha(0.55), Vec2::new(N_MASS_W, 6.0)),
        Transform::from_xyz(N_C.x, 150.0, 5.0),
        ui::ScreenEntity,
    ));
    commands.spawn((
        Sprite::from_color(theme::cyan(), Vec2::new(1.0, 6.0)),
        Transform::from_xyz(N_C.x - N_MASS_W * 0.5, 150.0, 6.0),
        NucleusMassFill,
        ui::ScreenEntity,
    ));
    let tick_x = N_C.x - N_MASS_W * 0.5 + cfg.threshold * N_MASS_W;
    commands.spawn((
        Sprite::from_color(theme::amber(), Vec2::new(2.0, 18.0)),
        Transform::from_xyz(tick_x, 150.0, 7.0),
        ui::ScreenEntity,
    ));

    for (index, &y) in N_ROW_Y.iter().enumerate() {
        let p = cfg.probs[index];
        ui::text(
            commands,
            assets,
            nucleus_token(lang, index),
            Vec2::new(N_C.x - 430.0, y),
            13.0,
            theme::text(),
            Anchor::CENTER_LEFT,
        );
        commands.spawn((
            assets.tex.panel_sprite(
                Color::srgba(0.025, 0.045, 0.065, 0.95),
                Vec2::new(N_BAR_W, N_BAR_H),
            ),
            Transform::from_xyz(N_C.x + N_BAR_X + N_BAR_W * 0.5, y, 5.0),
            ui::ScreenEntity,
        ));
        commands.spawn((
            Sprite::from_color(theme::cyan(), Vec2::new(1.0, N_BAR_H)),
            Transform::from_xyz(N_C.x + N_BAR_X, y, 6.0),
            NucleusBar(index),
            ui::ScreenEntity,
        ));
        ui::text(
            commands,
            assets,
            format!("{:.0}%", p * 100.0),
            Vec2::new(N_C.x + 430.0, y),
            11.0,
            theme::text_dim(),
            Anchor::CENTER_RIGHT,
        );
        // One large invisible hit area makes the label and bar a single target.
        commands.spawn((
            assets.tex.panel_sprite(Color::NONE, Vec2::new(880.0, 34.0)),
            Transform::from_xyz(N_C.x, y, 10.0),
            ui::hit(Vec2::new(440.0, 17.0), Action::Option(index), Color::NONE, theme::cyan().with_alpha(0.06)),
            ui::ScreenEntity,
        ));
    }

    commands.spawn((
        Text2d::new(""),
        TextFont {
            font: assets.source(),
            font_size: FontSize::Px(13.0),
            ..default()
        },
        TextColor(theme::text_dim()),
        Anchor::CENTER,
        Transform::from_xyz(0.0, -180.0, 20.0),
        NucleusStatusText,
        ui::ScreenEntity,
    ));
    ui::text(
        commands,
        assets,
        t(lang, "CLICK TOKENS TO BUILD THE NUCLEUS   |   R = RESET", "点击词元构建核集合   |   R = 重置"),
        Vec2::new(0.0, -280.0),
        11.0,
        theme::text_faint(),
        Anchor::CENTER,
    );
}

fn spawn_gradient(
    commands: &mut Commands,
    assets: &UiAssets,
    field: &Handle<Image>,
    lang: Lang,
    puzzle: &ooi_core::GradientPuzzle,
) {
    let cfg = puzzle.config();
    let start = gmap(cfg.start.0, cfg.start.1);

    commands.spawn((
        Sprite {
            image: field.clone(),
            custom_size: Some(G_HALF * 2.0),
            ..default()
        },
        Transform::from_xyz(G_C.x, G_C.y, 2.0),
        ui::ScreenEntity,
    ));
    commands.spawn((
        assets.tex.outline_sprite(
            theme::line().with_alpha(0.55),
            G_HALF * 2.0 + Vec2::splat(2.0),
        ),
        Transform::from_xyz(G_C.x, G_C.y, 2.4),
        ui::ScreenEntity,
    ));
    ui::caption(
        commands,
        assets,
        t(lang, "GRADIENT", "梯度"),
        Vec2::new(G_C.x - G_HALF.x, G_C.y + G_HALF.y + 8.0),
        theme::text_faint(),
        Anchor::CENTER_LEFT,
    );

    // The centre is the target; rings reinforce the bowl's low-loss middle.
    for radius in [18.0_f32, 30.0, 42.0] {
        commands.spawn((
            Sprite {
                image: assets.tex.ring.clone(),
                color: theme::cyan().with_alpha(0.14),
                custom_size: Some(Vec2::splat(radius * 2.0)),
                ..default()
            },
            Transform::from_xyz(G_C.x, G_C.y, 3.0),
            ui::ScreenEntity,
        ));
    }
    ui::caption(
        commands,
        assets,
        t(lang, "TARGET", "目标"),
        G_C + Vec2::new(0.0, -54.0),
        theme::text_faint(),
        Anchor::CENTER,
    );

    // One visible start.
    commands.spawn((
        Sprite {
            image: assets.tex.ring.clone(),
            color: theme::ice().with_alpha(0.75),
            custom_size: Some(Vec2::splat(34.0)),
            ..default()
        },
        Transform::from_xyz(start.x, start.y, 8.0),
        ui::ScreenEntity,
    ));
    ui::text(
        commands,
        assets,
        t(lang, "START", "起点"),
        start + Vec2::new(0.0, -26.0),
        9.0,
        theme::text(),
        Anchor::CENTER,
    );

    for index in 0..G_TRAIL_MAX {
        commands.spawn((
            Sprite {
                image: assets.tex.dot.clone(),
                color: Color::NONE,
                custom_size: Some(Vec2::splat(3.2)),
                ..default()
            },
            Transform::from_xyz(G_C.x, G_C.y, 6.5),
            GradientTrailDot(index),
            ui::ScreenEntity,
        ));
    }
    commands.spawn((
        Sprite {
            image: assets.tex.glow.clone(),
            color: Color::NONE,
            custom_size: Some(Vec2::splat(34.0)),
            ..default()
        },
        Transform::from_xyz(start.x, start.y, 9.0),
        GradientBallGlow,
        ui::ScreenEntity,
    ));
    commands.spawn((
        Sprite {
            image: assets.tex.dot.clone(),
            color: theme::cyan_hot(),
            custom_size: Some(Vec2::splat(10.0)),
            ..default()
        },
        Transform::from_xyz(start.x, start.y, 9.3),
        GradientBall,
        ui::ScreenEntity,
    ));

    // Aim line and candidate ring.
    commands.spawn((
        Sprite::from_color(theme::cyan().with_alpha(0.55), Vec2::new(1.0, 1.5)),
        Transform::from_xyz(start.x, start.y, 7.0),
        GradientAimLine,
        ui::ScreenEntity,
    ));
    commands.spawn((
        Sprite {
            image: assets.tex.ring.clone(),
            color: theme::ice().with_alpha(0.7),
            custom_size: Some(Vec2::splat(24.0)),
            ..default()
        },
        Transform::from_xyz(start.x, start.y, 8.2),
        GradientCandidateRing,
        ui::ScreenEntity,
    ));

    // Uphill gradient arrow.
    commands.spawn((
        Sprite::from_color(theme::amber().with_alpha(0.8), Vec2::new(1.0, 2.2)),
        Transform::from_xyz(start.x, start.y, 7.4),
        GradientArrowLine,
        ui::ScreenEntity,
    ));
    commands.spawn((
        Sprite {
            image: assets.tex.dot.clone(),
            color: theme::amber(),
            custom_size: Some(Vec2::splat(5.0)),
            ..default()
        },
        Transform::from_xyz(start.x, start.y, 7.6),
        GradientArrowTip,
        ui::ScreenEntity,
    ));

    commands.spawn((
        Text2d::new(""),
        TextFont {
            font: assets.source(),
            font_size: FontSize::Px(12.5),
            ..default()
        },
        TextColor(theme::text_dim()),
        Anchor::CENTER,
        Transform::from_xyz(0.0, -230.0, 20.0),
        GradientStatusText,
        ui::ScreenEntity,
    ));
    ui::action_button(
        commands,
        assets,
        Vec2::new(350.0, -330.0),
        Vec2::new(120.0, 32.0),
        t(lang, "STEP", "单步"),
        12.0,
        Action::Run,
        true,
    );
    ui::text(
        commands,
        assets,
        t(lang, "MOVE TO AIM   |   STEP = APPLY   |   R = RESET", "移动鼠标瞄准   |   单步 = 执行   |   R = 重置"),
        Vec2::new(0.0, -330.0),
        11.0,
        theme::text_faint(),
        Anchor::CENTER,
    );
}

fn spawn_descent(
    commands: &mut Commands,
    assets: &UiAssets,
    field: &Handle<Image>,
    lang: Lang,
    puzzle: &ooi_core::DescentPuzzle,
) {
    let cfg = puzzle.config();

    // Field background and frame.
    commands.spawn((
        assets.tex.panel_sprite(
            Color::srgba(0.010, 0.018, 0.030, 0.96),
            D_HALF * 2.0 + Vec2::splat(10.0),
        ),
        Transform::from_xyz(D_C.x, D_C.y, 1.4),
        ui::ScreenEntity,
    ));
    commands.spawn((
        assets
            .tex
            .outline_sprite(theme::line().with_alpha(0.55), D_HALF * 2.0),
        Transform::from_xyz(D_C.x, D_C.y, 1.8),
        ui::ScreenEntity,
    ));

    // The surface is one baked texture: crisp contour lines, and one entity
    // instead of thousands of per-cell sprites.
    commands.spawn((
        Sprite {
            image: field.clone(),
            custom_size: Some(D_HALF * 2.0),
            ..default()
        },
        Transform::from_xyz(D_C.x, D_C.y, 2.0),
        ui::ScreenEntity,
    ));
    for (sx, sy) in [(-1.0, 1.0), (1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)] {
        commands.spawn((
            Sprite::from_color(theme::cyan().with_alpha(0.45), Vec2::new(14.0, 1.0)),
            Transform::from_xyz(
                D_C.x + sx * D_HALF.x * 0.995,
                D_C.y + sy * D_HALF.y * 0.995,
                3.0,
            ),
            ui::ScreenEntity,
        ));
        commands.spawn((
            Sprite::from_color(theme::cyan().with_alpha(0.45), Vec2::new(1.0, 14.0)),
            Transform::from_xyz(
                D_C.x + sx * D_HALF.x * 0.995,
                D_C.y + sy * D_HALF.y * 0.995,
                3.0,
            ),
            ui::ScreenEntity,
        ));
    }
    ui::caption(
        commands,
        assets,
        t(lang, "LOSS", "损失"),
        Vec2::new(D_C.x - D_HALF.x, D_C.y + D_HALF.y + 8.0),
        theme::text_faint(),
        Anchor::CENTER_LEFT,
    );

    // Contour rings around both minima. The global basin gets one extra ring,
    // so depth is legible without labelling the answer.
    for ((cx, cy), rings, base) in [(cfg.global, 4usize, 20.0f32), (cfg.local, 3usize, 17.0f32)] {
        let center = dmap(cx, cy);
        for ring in 0..rings {
            let radius = base + ring as f32 * 13.0;
            commands.spawn((
                Sprite {
                    image: assets.tex.ring.clone(),
                    color: theme::cyan().with_alpha(0.12 - ring as f32 * 0.018),
                    custom_size: Some(Vec2::splat(radius * 2.0)),
                    ..default()
                },
                Transform::from_xyz(center.x, center.y, 3.5),
                ui::ScreenEntity,
            ));
        }
    }

    // Start markers. Clicking one (re)starts the run from that point.
    for (index, (sx, sy)) in cfg.starts.iter().copied().enumerate() {
        let pos = dmap(sx, sy);
        commands.spawn((
            Sprite {
                image: assets.tex.ring.clone(),
                color: theme::ice().with_alpha(0.75),
                custom_size: Some(Vec2::splat(34.0)),
                ..default()
            },
            Transform::from_xyz(pos.x, pos.y, 8.0),
            DescentStartRing(index),
            ui::ScreenEntity,
        ));
        ui::text(
            commands,
            assets,
            format!("{} {}", t(lang, "START", "起点"), index + 1),
            pos + Vec2::new(0.0, -26.0),
            9.0,
            theme::text(),
            Anchor::CENTER,
        );
        commands.spawn((
            Sprite {
                image: assets.tex.ring.clone(),
                color: Color::NONE,
                custom_size: Some(Vec2::splat(46.0)),
                ..default()
            },
            Transform::from_xyz(pos.x, pos.y, 11.0),
            ui::hit(Vec2::splat(23.0), Action::Option(index), Color::NONE, theme::cyan().with_alpha(0.20)),
            ui::ScreenEntity,
        ));
    }

    // Trail dots are preallocated; inactive ones are transparent.
    for index in 0..D_TRAIL_MAX {
        commands.spawn((
            Sprite {
                image: assets.tex.dot.clone(),
                color: Color::NONE,
                custom_size: Some(Vec2::splat(3.5)),
                ..default()
            },
            Transform::from_xyz(D_C.x, D_C.y, 6.5),
            DescentTrailDot(index),
            ui::ScreenEntity,
        ));
    }

    commands.spawn((
        Sprite {
            image: assets.tex.glow.clone(),
            color: Color::NONE,
            custom_size: Some(Vec2::splat(38.0)),
            ..default()
        },
        Transform::from_xyz(D_C.x, D_C.y, 9.0),
        DescentBallGlow,
        ui::ScreenEntity,
    ));
    commands.spawn((
        Sprite {
            image: assets.tex.dot.clone(),
            color: Color::NONE,
            custom_size: Some(Vec2::splat(11.0)),
            ..default()
        },
        Transform::from_xyz(D_C.x, D_C.y, 9.3),
        DescentBall,
        ui::ScreenEntity,
    ));

    commands.spawn((
        Text2d::new(""),
        TextFont {
            font: assets.source(),
            font_size: FontSize::Px(12.5),
            ..default()
        },
        TextColor(theme::text_dim()),
        Anchor::CENTER,
        Transform::from_xyz(0.0, -230.0, 20.0),
        DescentStatusText,
        ui::ScreenEntity,
    ));

    ui::action_button(
        commands,
        assets,
        Vec2::new(330.0, -330.0),
        Vec2::new(130.0, 32.0),
        t(lang, "DESCEND", "下降"),
        12.0,
        Action::Run,
        true,
    );
    ui::text(
        commands,
        assets,
        t(lang, "CLICK A START, THEN DESCEND   |   R = RESET", "选一个起点，然后下降   |   R = 重置"),
        Vec2::new(0.0, -330.0),
        11.0,
        theme::text_faint(),
        Anchor::CENTER,
    );
}

fn spawn_wake(commands: &mut Commands, assets: &UiAssets, lang: Lang, charge: f32, pulses: u8) {
    let center = Vec2::new(0.0, 42.0);

    // Radial filaments: the first neuron as a small physical object.
    for index in 0..12 {
        let angle = index as f32 / 12.0 * std::f32::consts::TAU;
        let inner = center + Vec2::new(angle.cos(), angle.sin()) * 58.0;
        let outer = center + Vec2::new(angle.cos(), angle.sin()) * 222.0;
        let mid = (inner + outer) * 0.5;
        let delta = outer - inner;
        commands.spawn((
            Sprite::from_color(
                theme::cyan().with_alpha(if index % 3 == 0 { 0.16 } else { 0.07 }),
                Vec2::new(delta.length(), 1.0),
            ),
            Transform::from_xyz(mid.x, mid.y, 2.0)
                .with_rotation(Quat::from_rotation_z(delta.y.atan2(delta.x))),
            ui::ScreenEntity,
        ));
        commands.spawn((
            Sprite {
                image: assets.tex.dot.clone(),
                color: theme::cyan().with_alpha(0.30),
                custom_size: Some(Vec2::splat(if index % 3 == 0 { 5.0 } else { 3.0 })),
                ..default()
            },
            Transform::from_xyz(outer.x, outer.y, 3.0),
            ui::ScreenEntity,
        ));
    }

    for (size, alpha, dashed) in [
        (330.0, 0.10, true),
        (250.0, 0.18, false),
        (170.0, 0.10, true),
    ] {
        commands.spawn((
            Sprite {
                image: if dashed {
                    assets.tex.ring_dash.clone()
                } else {
                    assets.tex.ring.clone()
                },
                color: theme::cyan().with_alpha(alpha),
                custom_size: Some(Vec2::splat(size)),
                ..default()
            },
            Transform::from_xyz(center.x, center.y, 2.5),
            ui::ScreenEntity,
        ));
    }

    commands.spawn((
        Sprite {
            image: assets.tex.glow.clone(),
            color: theme::cyan_hot().with_alpha(0.30),
            custom_size: Some(Vec2::splat(250.0)),
            ..default()
        },
        Transform::from_xyz(center.x, center.y, 4.0),
        WakeHalo,
        ui::ScreenEntity,
    ));
    commands.spawn((
        Sprite {
            image: assets.tex.glow.clone(),
            color: theme::cyan_hot().with_alpha(0.55),
            custom_size: Some(Vec2::splat(90.0)),
            ..default()
        },
        Transform::from_xyz(center.x, center.y, 4.2),
        ui::ScreenEntity,
    ));
    commands.spawn((
        Sprite {
            image: assets.tex.dot.clone(),
            color: theme::text(),
            custom_size: Some(Vec2::splat(30.0)),
            ..default()
        },
        Transform::from_xyz(center.x, center.y, 5.0),
        WakeCore,
        ui::ScreenEntity,
    ));

    ui::text(
        commands,
        assets,
        t(lang, "TOUCH THE CORE", "触碰核心"),
        Vec2::new(0.0, -74.0),
        12.0,
        theme::text_faint(),
        Anchor::CENTER,
    );

    // Charge instrument.
    let bar = Vec2::new(0.0, -142.0);
    let width = 440.0;
    commands.spawn((
        assets
            .tex
            .panel_sprite(theme::panel_hover(), Vec2::new(width, 7.0)),
        Transform::from_xyz(bar.x, bar.y, 5.0),
        ui::ScreenEntity,
    ));
    let fill = commands
        .spawn((
            assets.tex.panel_sprite(theme::cyan(), Vec2::new(2.0, 7.0)),
            Transform::from_xyz(bar.x - width / 2.0 + 1.0, bar.y, 5.2),
            WakeBarFill { origin: bar, width },
            ui::ScreenEntity,
        ))
        .id();
    let _ = fill;
    commands.spawn((
        Sprite::from_color(theme::amber().with_alpha(0.75), Vec2::new(1.0, 20.0)),
        Transform::from_xyz(bar.x + width / 2.0, bar.y, 6.0),
        ui::ScreenEntity,
    ));
    ui::caption(
        commands,
        assets,
        t(lang, "THRESHOLD", "阈值"),
        Vec2::new(bar.x + width / 2.0 - 2.0, bar.y - 20.0),
        theme::amber().with_alpha(0.8),
        Anchor::CENTER_RIGHT,
    );
    ui::caption(
        commands,
        assets,
        t(lang, "CHARGE", "充能"),
        Vec2::new(bar.x - width / 2.0, bar.y - 20.0),
        theme::text_faint(),
        Anchor::CENTER_LEFT,
    );

    commands.spawn((
        Text2d::new(""),
        TextFont {
            font: assets.source(),
            font_size: FontSize::Px(19.0),
            ..default()
        },
        TextColor(theme::text()),
        Anchor::CENTER,
        Transform::from_xyz(0.0, -178.0, 8.0),
        WakeChargeText,
        ui::ScreenEntity,
    ));
    commands.spawn((
        Text2d::new(""),
        TextFont {
            font: assets.source(),
            font_size: FontSize::Px(11.0),
            ..default()
        },
        TextColor(theme::text_faint()),
        Anchor::CENTER,
        Transform::from_xyz(0.0, -204.0, 8.0),
        WakeCountText,
        ui::ScreenEntity,
    ));
    let _ = (charge, pulses);
}

fn spawn_threshold(
    commands: &mut Commands,
    assets: &UiAssets,
    lang: Lang,
    cfg: &ooi_core::ThresholdConfig,
    theta: f32,
) {
    commands.spawn((
        Sprite::from_color(theme::axis(), Vec2::new(AXIS_HALF * 2.0, 1.5)),
        Transform::from_xyz(0.0, AXIS_Y, 0.5),
        ui::ScreenEntity,
    ));

    for i in 0..=20 {
        let t = i as f32 * 0.5;
        let tall = i % 4 == 0;
        commands.spawn((
            Sprite::from_color(theme::axis(), Vec2::new(1.0, if tall { 8.0 } else { 4.0 })),
            Transform::from_xyz(theta_to_x(t), AXIS_Y - 5.0, 0.5),
            ui::ScreenEntity,
        ));
    }

    for i in 0..=5 {
        let t = i as f32 * 2.0;
        ui::text(
            commands,
            assets,
            format!("{t:.0}"),
            Vec2::new(theta_to_x(t), AXIS_Y - 26.0),
            14.0,
            theme::axis_label(),
            Anchor::CENTER,
        );
    }

    for (i, s) in cfg.samples.iter().enumerate() {
        let jitter = ((i as u32 * 37) % 23) as f32 - 11.0;
        let y = if s.label { 34.0 } else { -52.0 } + jitter;
        let glyph = if s.label { "●" } else { "▲" };
        commands.spawn((
            Text2d::new(glyph),
            TextFont {
                font: assets.source(),
                font_size: FontSize::Px(24.0),
                ..default()
            },
            TextColor(theme::text_dim()),
            Anchor::CENTER,
            Transform::from_xyz(theta_to_x(s.x), y, 4.0),
            TDot {
                x: s.x,
                label: s.label,
            },
            ui::ScreenEntity,
        ));
    }

    let x = theta_to_x(theta);
    commands.spawn((
        Sprite::from_color(theme::cyan(), Vec2::new(3.0, LINE_H)),
        Transform::from_xyz(x, LINE_CY, 2.0),
        TLine,
        ui::ScreenEntity,
    ));
    commands.spawn((
        Sprite::from_color(theme::cyan(), Vec2::new(30.0, 12.0)),
        Transform::from_xyz(x, LINE_CY + LINE_H / 2.0 + 8.0, 3.0),
        THandle,
        ui::ScreenEntity,
    ));

    for (kind, pos) in [
        (THud::FalseAlarms, Vec2::new(-520.0, 216.0)),
        (THud::Misses, Vec2::new(-260.0, 216.0)),
        (THud::Status, Vec2::new(-520.0, 178.0)),
    ] {
        commands.spawn((
            Text2d::new(""),
            TextFont {
                font: assets.source(),
                font_size: FontSize::Px(if kind == THud::Status { 16.0 } else { 22.0 }),
                ..default()
            },
            TextColor(theme::text()),
            Anchor::TOP_LEFT,
            Transform::from_xyz(pos.x, pos.y, 10.0),
            kind,
            ui::ScreenEntity,
        ));
    }

    ui::text(
        commands,
        assets,
        t(lang, "DRAG THE LINE   |   ARROW KEYS   |   R = RESET", "拖动这条线   |   方向键微调   |   R = 重置"),
        Vec2::new(0.0, -320.0),
        14.0,
        theme::text_dim(),
        Anchor::CENTER,
    );
}

fn spawn_boundary(
    commands: &mut Commands,
    assets: &UiAssets,
    lang: Lang,
    cfg: &ooi_core::BoundaryConfig,
) {
    // A live decision field: every cell is tested against the current weights.
    commands.spawn((
        assets.tex.panel_sprite(
            theme::glass().with_alpha(0.78),
            Vec2::splat(CANVAS_HALF * 2.0 + 20.0),
        ),
        Transform::from_xyz(CANVAS_C.x, CANVAS_C.y, 1.0),
        ui::ScreenEntity,
    ));
    const CELLS: usize = 24;
    let cell = (CANVAS_HALF * 2.0) / CELLS as f32;
    for iy in 0..CELLS {
        for ix in 0..CELLS {
            let x = CANVAS_C.x - CANVAS_HALF + (ix as f32 + 0.5) * cell;
            let y = CANVAS_C.y - CANVAS_HALF + (iy as f32 + 0.5) * cell;
            commands.spawn((
                Sprite::from_color(theme::cyan().with_alpha(0.04), Vec2::splat(cell + 0.6)),
                Transform::from_xyz(x, y, 1.5),
                BFieldCell { x: ix, y: iy },
                ui::ScreenEntity,
            ));
        }
    }
    for step in 1..4 {
        let t = step as f32 / 4.0;
        commands.spawn((
            Sprite::from_color(
                theme::text().with_alpha(0.055),
                Vec2::new(1.0, CANVAS_HALF * 2.0),
            ),
            Transform::from_xyz(
                CANVAS_C.x - CANVAS_HALF + t * CANVAS_HALF * 2.0,
                CANVAS_C.y,
                2.2,
            ),
            ui::ScreenEntity,
        ));
        commands.spawn((
            Sprite::from_color(
                theme::text().with_alpha(0.055),
                Vec2::new(CANVAS_HALF * 2.0, 1.0),
            ),
            Transform::from_xyz(
                CANVAS_C.x,
                CANVAS_C.y - CANVAS_HALF + t * CANVAS_HALF * 2.0,
                2.2,
            ),
            ui::ScreenEntity,
        ));
    }
    commands.spawn((
        assets.tex.outline_sprite(
            theme::line().with_alpha(0.5),
            Vec2::splat(CANVAS_HALF * 2.0 + 20.0),
        ),
        Transform::from_xyz(CANVAS_C.x, CANVAS_C.y, 2.4),
        ui::ScreenEntity,
    ));
    ui::caption(
        commands,
        assets,
        "x1",
        Vec2::new(
            CANVAS_C.x + CANVAS_HALF - 4.0,
            CANVAS_C.y - CANVAS_HALF - 16.0,
        ),
        theme::text_faint(),
        Anchor::CENTER_RIGHT,
    );
    ui::caption(
        commands,
        assets,
        "x2",
        Vec2::new(
            CANVAS_C.x - CANVAS_HALF - 14.0,
            CANVAS_C.y + CANVAS_HALF - 4.0,
        ),
        theme::text_faint(),
        Anchor::CENTER,
    );
    ui::caption(
        commands,
        assets,
        "0",
        Vec2::new(
            CANVAS_C.x - CANVAS_HALF - 8.0,
            CANVAS_C.y - CANVAS_HALF - 12.0,
        ),
        theme::text_faint(),
        Anchor::CENTER_RIGHT,
    );
    ui::caption(
        commands,
        assets,
        "1",
        Vec2::new(CANVAS_C.x + CANVAS_HALF + 8.0, CANVAS_C.y + CANVAS_HALF),
        theme::text_faint(),
        Anchor::CENTER_LEFT,
    );

    for p in &cfg.points {
        let pos = map01((p.x, p.y));
        let label_color = if p.label {
            theme::cyan()
        } else {
            theme::amber()
        };
        commands.spawn((
            Sprite {
                image: assets.tex.glow.clone(),
                color: label_color.with_alpha(0.14),
                custom_size: Some(Vec2::splat(28.0)),
                ..default()
            },
            Transform::from_xyz(pos.x, pos.y, 3.4),
            BDotGlow(*p),
            ui::ScreenEntity,
        ));
        let mut sprite = if p.label {
            Sprite {
                image: assets.tex.dot.clone(),
                color: label_color,
                custom_size: Some(Vec2::splat(11.0)),
                ..default()
            }
        } else {
            Sprite::from_color(label_color, Vec2::splat(8.5))
        };
        sprite.color = label_color;
        let mut transform = Transform::from_xyz(pos.x, pos.y, 4.0);
        if !p.label {
            transform.rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_4);
        }
        commands.spawn((sprite, transform, BDot(*p), ui::ScreenEntity));
    }

    commands.spawn((
        Sprite {
            color: theme::cyan(),
            custom_size: Some(Vec2::new(10.0, 3.0)),
            ..default()
        },
        Transform::from_xyz(CANVAS_C.x, CANVAS_C.y, 3.0),
        BLine,
        ui::ScreenEntity,
    ));

    commands.spawn((
        assets
            .tex
            .panel_sprite(theme::bg_deep().with_alpha(0.72), Vec2::new(700.0, 28.0)),
        Transform::from_xyz(0.0, -226.0, 9.0),
        ui::ScreenEntity,
    ));
    commands.spawn((
        Text2d::new(""),
        TextFont {
            font: assets.source(),
            font_size: FontSize::Px(14.0),
            ..default()
        },
        TextColor(theme::text()),
        Anchor::CENTER,
        Transform::from_xyz(0.0, -226.0, 10.0),
        BReadout,
        ui::ScreenEntity,
    ));

    ui::action_button(
        commands,
        assets,
        Vec2::new(430.0, -300.0),
        Vec2::new(120.0, 38.0),
        t(lang, "RUN", "试运行"),
        13.0,
        Action::Run,
        true,
    );

    ui::text(
        commands,
        assets,
        t(lang, "W/S = W1   A/D = W2   Q/E = BIAS   |   R = RESET", "W/S 调权重一   A/D 调权重二   Q/E 调偏置   |   R = 重置"),
        Vec2::new(0.0, -332.0),
        11.0,
        theme::text_faint(),
        Anchor::CENTER,
    );
}

// ---------------------------------------------------------------- 输入

pub fn wake_input(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut active: ResMut<ActivePuzzle>,
    mut cues: ResMut<CueQueue>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Puzzle::Wake(p) = &mut active.0 else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok((camera, cam_tf)) = cameras.single() else {
        return;
    };
    let Ok(world) = camera.viewport_to_world_2d(cam_tf, cursor) else {
        return;
    };
    if (world - Vec2::new(0.0, 42.0)).length() < 180.0 && p.pulse() {
        cues.play(Cue::Click);
    }
}

pub fn threshold_input(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut active: ResMut<ActivePuzzle>,
    mut drag: ResMut<ThresholdDrag>,
) {
    let Puzzle::Threshold(p) = &mut active.0 else {
        return;
    };

    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok((camera, cam_tf)) = cameras.single() else {
        return;
    };
    let Ok(world) = camera.viewport_to_world_2d(cam_tf, cursor) else {
        return;
    };

    if mouse.just_pressed(MouseButton::Left) {
        let line_x = theta_to_x(p.theta());
        drag.0 = (world.x - line_x).abs() <= 40.0;
    }
    if mouse.just_released(MouseButton::Left) {
        drag.0 = false;
    }
    if drag.0 && mouse.pressed(MouseButton::Left) {
        p.set_theta(x_to_theta(world.x));
    }
}

pub fn puzzle_keys(keys: Res<ButtonInput<KeyCode>>, mut active: ResMut<ActivePuzzle>) {
    if keys.just_pressed(KeyCode::KeyR) {
        active.0.reset();
        return;
    }

    match &mut active.0 {
        Puzzle::Threshold(p) => {
            let step = if keys.pressed(KeyCode::ShiftLeft) {
                10
            } else {
                1
            };
            let mut delta = 0;
            if keys.just_pressed(KeyCode::ArrowLeft) {
                delta -= step;
            }
            if keys.just_pressed(KeyCode::ArrowRight) {
                delta += step;
            }
            if delta != 0 {
                let max = p.config().max_index();
                let idx = (p.index() + delta).clamp(0, max);
                let t = p.theta_at(idx);
                p.set_theta(t);
            }
        }
        Puzzle::Boundary(p) => {
            let step = 0.1;
            let (mut w1, mut w2, mut b) = p.weights();
            if keys.just_pressed(KeyCode::KeyW) {
                w1 += step;
            }
            if keys.just_pressed(KeyCode::KeyS) {
                w1 -= step;
            }
            if keys.just_pressed(KeyCode::KeyD) {
                w2 += step;
            }
            if keys.just_pressed(KeyCode::KeyA) {
                w2 -= step;
            }
            if keys.just_pressed(KeyCode::KeyE) {
                b += step;
            }
            if keys.just_pressed(KeyCode::KeyQ) {
                b -= step;
            }
            p.set_weights(w1, w2, b);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------- 同步

pub fn sync_wake(
    active: Res<ActivePuzzle>,
    lang: Res<LangRes>,
    mut core: Query<(&mut Transform, &mut Sprite), With<WakeCore>>,
    mut halos: Query<
        (&mut Transform, &mut Sprite),
        (With<WakeHalo>, Without<WakeCore>, Without<WakeBarFill>),
    >,
    mut fill: Query<
        (&WakeBarFill, &mut Transform, &mut Sprite),
        (Without<WakeCore>, Without<WakeHalo>),
    >,
    mut charge_text: Query<&mut Text2d, With<WakeChargeText>>,
    mut count_text: Query<&mut Text2d, (With<WakeCountText>, Without<WakeChargeText>)>,
) {
    let Puzzle::Wake(p) = &active.0 else {
        return;
    };
    let charge = p.normalized_charge();
    let color = if p.is_solved() {
        theme::amber()
    } else {
        theme::cyan()
    };

    for (mut transform, mut sprite) in &mut core {
        transform.scale = Vec3::splat(0.85 + charge * 0.55);
        sprite.color = color;
    }
    for (mut transform, mut sprite) in &mut halos {
        transform.scale = Vec3::splat(0.75 + charge * 0.65);
        sprite.color = color.with_alpha(0.14 + charge * 0.30);
    }
    for (bar, mut transform, mut sprite) in &mut fill {
        let w = (bar.width * charge).max(2.0);
        transform.translation.x = bar.origin.x - bar.width / 2.0 + w / 2.0;
        sprite.custom_size = Some(Vec2::new(w, 7.0));
        sprite.color = color;
    }
    for mut text in &mut charge_text {
        text.0 = format!("{:.2} / 1.00", p.charge().min(1.0));
        // Color is set once by the spawn system; the number carries the state.
    }
    for mut text in &mut count_text {
        text.0 = format!(
            "{} {}   \u{00b7}   {}",
            t(lang.0, "INPUT PULSES", "输入脉冲"),
            p.pulses(),
            if p.is_solved() {
                t(lang.0, "SOLVED", "已解")
            } else {
                ""
            }
        );
    }
}

#[allow(clippy::type_complexity)]
pub fn sync_threshold(
    active: Res<ActivePuzzle>,
    lang: Res<LangRes>,
    mut dots: Query<
        (&TDot, &mut TextColor, &mut Transform),
        (Without<TLine>, Without<THandle>, Without<THud>),
    >,
    mut line_parts: Query<
        (&mut Transform, &mut Sprite),
        (
            Or<(With<TLine>, With<THandle>)>,
            Without<TDot>,
            Without<THud>,
        ),
    >,
    mut huds: Query<(&THud, &mut Text2d, &mut TextColor), Without<TDot>>,
) {
    let Puzzle::Threshold(p) = &active.0 else {
        return;
    };
    let theta = p.theta();
    let c = p.confusion();
    let solved = p.is_solved();

    for (dot, mut color, mut transform) in &mut dots {
        let fired = dot.x >= theta;
        color.0 = match (dot.label, fired) {
            (true, true) => theme::cyan(),
            (true, false) => theme::cyan_dim(),
            (false, true) => theme::ember_lit(),
            (false, false) => theme::ember_dim(),
        };
        let scale = if fired && !dot.label {
            1.7
        } else if fired {
            1.15
        } else {
            1.0
        };
        transform.scale = Vec3::splat(scale);
    }

    let color = if solved {
        theme::amber()
    } else {
        theme::cyan()
    };
    let x = theta_to_x(theta);
    for (mut transform, mut sprite) in &mut line_parts {
        transform.translation.x = x;
        sprite.color = color;
    }

    for (kind, mut text, mut color) in &mut huds {
        match kind {
            THud::FalseAlarms => {
                text.0 = format!("{}  {}", t(lang.0, "FALSE ALARMS", "误报"), c.false_pos);
                color.0 = if c.false_pos > 0 {
                    theme::ember_lit()
                } else {
                    theme::text()
                };
            }
            THud::Misses => {
                text.0 = format!("{}  {}", t(lang.0, "MISSES", "漏报"), c.false_neg);
                color.0 = if c.false_neg > 0 {
                    theme::ember_lit()
                } else {
                    theme::text()
                };
            }
            THud::Status => {
                let (msg, col) = if solved {
                    (t(lang.0, "SOLVED - IT FIRES ONLY FOR FIREFLIES", "已解：它只为萤火而亮"), theme::amber())
                } else if c.false_pos > 0 && c.false_neg > 0 {
                    (t(lang.0, "BOTH SIDES ARE WRONG - FIND THE GAP", "两边都在出错：找那道空隙"), theme::amber())
                } else if c.false_neg > 0 {
                    (t(lang.0, "SOMETHING DID NOT LIGHT UP - LINE TOO FAR RIGHT", "有萤火没亮：线太靠右"), theme::amber())
                } else if c.false_pos > 0 {
                    (t(lang.0, "SOMETHING LIT THAT SHOULD NOT - LINE TOO FAR LEFT", "有余烬亮了：线太靠左"), theme::amber())
                } else {
                    ("", theme::text_dim())
                };
                text.0 = msg.to_string();
                color.0 = col;
            }
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn sync_boundary(
    active: Res<ActivePuzzle>,
    mut dots: Query<
        (&BDot, &mut Sprite, &mut Transform),
        (Without<BLine>, Without<BDotGlow>, Without<BFieldCell>),
    >,
    mut glows: Query<
        (&BDotGlow, &mut Sprite),
        (Without<BDot>, Without<BLine>, Without<BFieldCell>),
    >,
    mut cells: Query<
        (&BFieldCell, &mut Sprite),
        (Without<BDot>, Without<BDotGlow>, Without<BLine>),
    >,
    mut line: Query<
        (&mut Transform, &mut Sprite),
        (With<BLine>, Without<BDot>, Without<BFieldCell>),
    >,
    mut readout: Query<&mut Text2d, (With<BReadout>, Without<BDot>)>,
) {
    let Puzzle::Boundary(p) = &active.0 else {
        return;
    };
    let (w1, w2, b) = p.weights();

    for (cell, mut sprite) in &mut cells {
        let x = (cell.x as f32 + 0.5) / 24.0;
        let y = (cell.y as f32 + 0.5) / 24.0;
        let fired = p.fires(&Point2::new(x, y, false));
        sprite.color = if fired {
            theme::cyan().with_alpha(0.065)
        } else {
            theme::amber().with_alpha(0.018)
        };
    }

    for (dot, mut sprite, mut transform) in &mut dots {
        let fired = p.fires(&dot.0);
        let correct = fired == dot.0.label;
        let color = match (dot.0.label, fired) {
            (_, false) if correct => theme::amber(),
            (_, true) if correct => theme::cyan(),
            (_, true) => theme::ember_lit(),
            (_, false) => theme::cyan_dim(),
        };
        sprite.color = color;
        let scale = if !correct {
            1.75
        } else if fired {
            1.25
        } else {
            1.0
        };
        transform.scale = Vec3::splat(scale);
    }

    for (glow, mut sprite) in &mut glows {
        let fired = p.fires(&glow.0);
        let correct = fired == glow.0.label;
        let color = match (glow.0.label, fired) {
            (_, true) if !correct => theme::ember_lit(),
            (true, _) => theme::cyan(),
            (false, _) => theme::amber(),
        };
        sprite.color = color.with_alpha(if correct { 0.12 } else { 0.30 });
    }

    // Keep the boundary line clipped to the field.
    let n2 = w1 * w1 + w2 * w2;
    for (mut transform, mut sprite) in &mut line {
        if n2 < 1e-6 {
            sprite.custom_size = Some(Vec2::ZERO);
            continue;
        }
        let p0 = (-b * w1 / n2, -b * w2 / n2);
        let d = (-w2, w1);
        let origin = map01(p0);
        let dir = Vec2::new(d.0, d.1) * (CANVAS_HALF * 2.0);

        if let Some((a, z)) = clip_to_box(origin, dir, CANVAS_C, CANVAS_HALF) {
            let mid = (a + z) * 0.5;
            let len = (z - a).length();
            transform.translation.x = mid.x;
            transform.translation.y = mid.y;
            transform.rotation = Quat::from_rotation_z(dir.y.atan2(dir.x));
            sprite.custom_size = Some(Vec2::new(len.max(1.0), 2.0));
            sprite.color = if p.is_solved() {
                theme::amber()
            } else {
                theme::cyan()
            };
        } else {
            sprite.custom_size = Some(Vec2::ZERO);
        }
    }

    for mut text in &mut readout {
        text.0 = format!(
            "w1 {w1:+.1}   w2 {w2:+.1}   b {b:+.1}   \u{00b7}   errors {}   \u{00b7}   tries {}",
            p.confusion().errors(),
            p.attempts()
        );
    }
}

fn clip_to_box(p0: Vec2, dir: Vec2, center: Vec2, half: f32) -> Option<(Vec2, Vec2)> {
    let mut t0 = f32::NEG_INFINITY;
    let mut t1 = f32::INFINITY;

    let axes = [
        (p0.x, dir.x, center.x - half, center.x + half),
        (p0.y, dir.y, center.y - half, center.y + half),
    ];

    for (p, d, lo, hi) in axes {
        if d.abs() < 1e-6 {
            if p < lo || p > hi {
                return None;
            }
        } else {
            let a = (lo - p) / d;
            let b = (hi - p) / d;
            t0 = t0.max(a.min(b));
            t1 = t1.min(a.max(b));
        }
    }

    if t0 > t1 {
        None
    } else {
        Some((p0 + dir * t0, p0 + dir * t1))
    }
}

// ---------------------------------------------------------------- 通关

pub fn detect_solved(
    active: Res<ActivePuzzle>,
    current: Res<CurrentLevel>,
    hint: Res<HintState>,
    time: Res<Time>,
    mut delay: ResMut<SolveDelay>,
    mut save: ResMut<SaveRes>,
    mut next: ResMut<NextState<Screen>>,
    mut cues: ResMut<CueQueue>,
) {
    if !active.0.is_solved() {
        delay.0 = 0.0;
        return;
    }

    #[cfg(debug_assertions)]
    if std::env::var("OOI_HOLD_SOLVED").is_ok() {
        return;
    }

    delay.0 += time.delta_secs();
    let hold = if matches!(&active.0, Puzzle::Descent(_)) { 1.2 } else { 0.45 };
    if delay.0 < hold {
        return;
    }

    let index = current.0;
    let first = !save.0.is_complete(index);
    let outcome = Outcome { stars: hint.stars(), metric: 0.0, alt: 0.0 };
    save.0.complete(index, outcome, hint.stage >= 3);
    if first {
        save.0.meta.levels_cleared += 1;
    }
    if let Puzzle::Bench(p) = &active.0
        && p.id() == "ch7_l5"
    {
        save.0.narrative.principles = principle_order(p.state());
    }
    save_io::save(&save.0);
    cues.play(Cue::Success);
    let finale = level(index).is_some_and(|d| d.id == "finale");
    next.set(if finale { Screen::Choice } else { Screen::Card });
}

// ---------------------------------------------------------------- 连线

#[derive(Component)]
pub struct WireNodeUi {
    pub index: usize,
}

#[derive(Component)]
pub struct WireEdgeUi {
    pub a: usize,
    pub b: usize,
}

#[derive(Component)]
pub struct WireGateText {
    pub slot: usize,
}

#[derive(Component)]
pub struct WireStatus;

#[derive(Resource, Default)]
pub struct WireSelection(pub Option<usize>);

fn spawn_wire(
    commands: &mut Commands,
    assets: &UiAssets,
    lang: Lang,
    id: &str,
    puzzle: &ooi_core::WirePuzzle,
) {
    let cfg = puzzle.config();
    let is_xor = matches!(cfg.task, ooi_core::WireTask::Xor);

    if is_xor {
        for (x, label) in [
            (-430.0, t(lang, "INPUTS", "输入")),
            (-60.0, t(lang, "HIDDEN", "隐藏")),
            (360.0, t(lang, "OUTPUT", "输出")),
        ] {
            ui::caption(
                commands,
                assets,
                label,
                Vec2::new(x, 208.0),
                theme::text_faint(),
                Anchor::CENTER,
            );
        }
    } else {
        ui::caption(
            commands,
            assets,
            t(lang, "WIRE THE PIPELINE IN ORDER", "按顺序接通流水线"),
            Vec2::new(0.0, 208.0),
            theme::text_faint(),
            Anchor::CENTER,
        );
    }

    // Rails behind each layer.
    let groups: &[(f32, f32)] = if is_xor {
        &[(-430.0, 250.0), (-60.0, 250.0), (360.0, 190.0)]
    } else {
        &[]
    };
    for (x, h) in groups {
        commands.spawn((
            Sprite::from_color(theme::line().with_alpha(0.16), Vec2::new(1.0, *h)),
            Transform::from_xyz(*x, 0.0, 1.5),
            ui::ScreenEntity,
        ));
    }

    for (a, b) in &cfg.allowed {
        let (ax, ay) = cfg.positions[*a];
        let (bx, by) = cfg.positions[*b];
        let a_pos = Vec2::new(ax, ay);
        let b_pos = Vec2::new(bx, by);
        let mid = (a_pos + b_pos) * 0.5;
        let delta = b_pos - a_pos;
        commands.spawn((
            Sprite {
                color: theme::line().with_alpha(0.22),
                custom_size: Some(Vec2::new(delta.length(), 2.0)),
                ..default()
            },
            Transform::from_xyz(mid.x, mid.y, 2.0)
                .with_rotation(Quat::from_rotation_z(delta.y.atan2(delta.x))),
            WireEdgeUi { a: *a, b: *b },
            ui::ScreenEntity,
        ));
    }

    for (index, (x, y)) in cfg.positions.iter().enumerate() {
        let pos = Vec2::new(*x, *y);
        ui::halo(
            commands,
            assets,
            pos,
            110.0,
            theme::cyan().with_alpha(0.055),
            4.0,
        );
        commands.spawn((
            assets
                .tex
                .panel_sprite(theme::glass_raised(), Vec2::new(104.0, 58.0)),
            Transform::from_xyz(pos.x, pos.y, 5.0),
            WireNodeUi { index },
            ui::ScreenEntity,
        ));
        commands.spawn((
            assets
                .tex
                .outline_sprite(theme::line().with_alpha(0.5), Vec2::new(104.0, 58.0)),
            Transform::from_xyz(pos.x, pos.y, 5.4),
            ui::ScreenEntity,
        ));
        ui::text(
            commands,
            assets,
            wire_label(lang, id, index),
            pos,
            16.0,
            theme::text(),
            Anchor::CENTER,
        );

        if is_xor && (index == 2 || index == 3) {
            let gate_y = *y - 52.0;
            ui::action_button(
                commands,
                assets,
                Vec2::new(*x, gate_y),
                Vec2::new(64.0, 26.0),
                "",
                11.0,
                Action::ToggleGate(index - 2),
                false,
            );
            commands.spawn((
                Text2d::new("AND"),
                TextFont {
                    font: assets.source(),
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(theme::amber()),
                Anchor::CENTER,
                Transform::from_xyz(*x, gate_y, 21.0),
                WireGateText { slot: index - 2 },
                ui::ScreenEntity,
            ));
        }
    }

    commands.spawn((
        Text2d::new(""),
        TextFont {
            font: assets.source(),
            font_size: FontSize::Px(15.0),
            ..default()
        },
        TextColor(theme::text()),
        Anchor::CENTER,
        Transform::from_xyz(0.0, -234.0, 9.0),
        WireStatus,
        ui::ScreenEntity,
    ));

    let hint = if is_xor {
        t(lang, "CLICK A NODE, THEN ANOTHER TO CONNECT   |   1 / 2 = OR / AND   |   R = RESET", "点一个节点，再点另一个来连线   |   1 / 2 切换 OR / AND   |   R = 重置")
    } else {
        t(lang, "CLICK A NODE, THEN ANOTHER TO CONNECT   |   R = RESET", "点一个节点，再点另一个来连线   |   R = 重置")
    };
    ui::text(
        commands,
        assets,
        hint,
        Vec2::new(0.0, -326.0),
        11.0,
        theme::text_faint(),
        Anchor::CENTER,
    );
}

pub fn wire_input(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut active: ResMut<ActivePuzzle>,
    mut selection: ResMut<WireSelection>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Puzzle::Wire(p) = &mut active.0 else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok((camera, cam_tf)) = cameras.single() else {
        return;
    };
    let Ok(world) = camera.viewport_to_world_2d(cam_tf, cursor) else {
        return;
    };

    let Some(node) = p
        .config()
        .positions
        .iter()
        .position(|(x, y)| (world - Vec2::new(*x, *y)).length() < 52.0)
    else {
        return;
    };

    match selection.0 {
        None => selection.0 = Some(node),
        Some(from) if from == node => selection.0 = None,
        Some(from) => {
            p.connect(from, node);
            selection.0 = None;
        }
    }
}

pub fn wire_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut active: ResMut<ActivePuzzle>,
    mut selection: ResMut<WireSelection>,
) {
    if keys.just_pressed(KeyCode::KeyR) {
        active.0.reset();
        selection.0 = None;
        return;
    }
    if let Puzzle::Wire(p) = &mut active.0 {
        if keys.just_pressed(KeyCode::Digit1) {
            p.toggle_gate(0);
        }
        if keys.just_pressed(KeyCode::Digit2) {
            p.toggle_gate(1);
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn sync_wire(
    active: Res<ActivePuzzle>,
    selection: Res<WireSelection>,
    mut nodes: Query<(&WireNodeUi, &mut Sprite), Without<WireEdgeUi>>,
    mut edges: Query<(&WireEdgeUi, &mut Sprite), Without<WireNodeUi>>,
    mut gates: Query<(&WireGateText, &mut Text2d), Without<WireStatus>>,
    mut status: Query<&mut Text2d, (With<WireStatus>, Without<WireGateText>)>,
) {
    let Puzzle::Wire(p) = &active.0 else {
        return;
    };
    let solved = p.is_solved();

    for (node, mut sprite) in &mut nodes {
        sprite.color = if selection.0 == Some(node.index) {
            theme::action()
        } else if solved {
            Color::srgba(0.22, 0.16, 0.06, 1.0)
        } else {
            theme::glass_raised()
        };
    }

    for (edge, mut sprite) in &mut edges {
        sprite.color = if p.is_selected(edge.a, edge.b) {
            if solved {
                theme::amber()
            } else {
                theme::cyan()
            }
        } else {
            Color::srgba(0.28, 0.38, 0.50, 0.28)
        };
    }

    for (gate, mut text) in &mut gates {
        text.0 = p.gates()[gate.slot].label().to_string();
    }

    let wire_count = p.selected_count();
    let message = if matches!(p.config().task, ooi_core::WireTask::Xor) {
        let t = p.truth_table();
        let bit = |b: bool| if b { '1' } else { '0' };
        format!(
            "00 → {}   01 → {}   10 → {}   11 → {}   |   wires {}",
            bit(t[0]),
            bit(t[1]),
            bit(t[2]),
            bit(t[3]),
            wire_count
        )
    } else {
        format!("wires {}", wire_count)
    };

    for mut text in &mut status {
        text.0 = message.clone();
    }
}

// ---------------------------------------------------------------- 卷积核

#[derive(Component)]
pub struct KernelCell {
    pub x: usize,
    pub y: usize,
}

#[derive(Component)]
pub struct KernelValue {
    pub x: usize,
    pub y: usize,
}

#[derive(Component)]
pub struct KernelDot {
    pub index: usize,
}

#[derive(Component)]
pub struct KernelStatus;

const IMAGE_CELL: f32 = 38.0;
const KERNEL_CELL: f32 = 54.0;
const IMAGE_C: Vec2 = Vec2::new(-200.0, -10.0);
const KERNEL_C: Vec2 = Vec2::new(250.0, 20.0);

fn kernel_cell_center(x: usize, y: usize, w: usize, h: usize, c: Vec2, size: f32) -> Vec2 {
    let left = c.x - (w as f32) * size / 2.0;
    let top = c.y + (h as f32) * size / 2.0;
    Vec2::new(
        left + (x as f32 + 0.5) * size,
        top - (y as f32 + 0.5) * size,
    )
}

fn spawn_kernel(
    commands: &mut Commands,
    assets: &UiAssets,
    lang: Lang,
    puzzle: &ooi_core::KernelPuzzle,
) {
    let cfg = puzzle.config();
    let (w, h) = (cfg.width, cfg.height);

    commands.spawn((
        Sprite::from_color(
            theme::panel(),
            Vec2::new(w as f32 * IMAGE_CELL + 16.0, h as f32 * IMAGE_CELL + 16.0),
        ),
        Transform::from_xyz(IMAGE_C.x, IMAGE_C.y, 1.0),
        ui::ScreenEntity,
    ));

    for y in 0..h {
        for x in 0..w {
            let pos = kernel_cell_center(x, y, w, h, IMAGE_C, IMAGE_CELL);
            let value = cfg.image[y * w + x];
            let color = if value > 0.5 {
                theme::cyan()
            } else {
                theme::panel_hover()
            };
            commands.spawn((
                Sprite::from_color(color, Vec2::splat(IMAGE_CELL - 3.0)),
                Transform::from_xyz(pos.x, pos.y, 3.0),
                ui::ScreenEntity,
            ));

            let target_color = if cfg.target[y * w + x] {
                Color::srgba(1.0, 0.76, 0.30, 0.45)
            } else {
                Color::NONE
            };
            commands.spawn((
                Sprite::from_color(target_color, Vec2::splat(9.0)),
                Transform::from_xyz(pos.x - 12.0, pos.y + 12.0, 4.0),
                ui::ScreenEntity,
            ));

            commands.spawn((
                Sprite::from_color(Color::NONE, Vec2::splat(9.0)),
                Transform::from_xyz(pos.x + 12.0, pos.y - 12.0, 4.0),
                KernelDot { index: y * w + x },
                ui::ScreenEntity,
            ));
        }
    }

    commands.spawn((
        Sprite::from_color(theme::panel(), Vec2::splat(3.0 * KERNEL_CELL + 16.0)),
        Transform::from_xyz(KERNEL_C.x, KERNEL_C.y, 1.0),
        ui::ScreenEntity,
    ));

    for y in 0..3 {
        for x in 0..3 {
            let pos = kernel_cell_center(x, y, 3, 3, KERNEL_C, KERNEL_CELL);
            commands.spawn((
                Sprite::from_color(theme::panel_hover(), Vec2::splat(KERNEL_CELL - 4.0)),
                Transform::from_xyz(pos.x, pos.y, 3.0),
                KernelCell { x, y },
                ui::ScreenEntity,
            ));
            commands.spawn((
                Text2d::new("0"),
                TextFont {
                    font: assets.source(),
                    font_size: FontSize::Px(24.0),
                    ..default()
                },
                TextColor(theme::text()),
                Anchor::CENTER,
                Transform::from_xyz(pos.x, pos.y, 8.0),
                KernelValue { x, y },
                ui::ScreenEntity,
            ));
        }
    }

    ui::text(
        commands,
        assets,
        t(lang, "target", "目标"),
        Vec2::new(IMAGE_C.x - 120.0, IMAGE_C.y - 182.0),
        13.0,
        Color::srgba(1.0, 0.76, 0.30, 0.55),
        Anchor::CENTER,
    );
    ui::text(
        commands,
        assets,
        t(lang, "fires", "触发"),
        Vec2::new(IMAGE_C.x + 40.0, IMAGE_C.y - 182.0),
        13.0,
        theme::amber(),
        Anchor::CENTER,
    );

    commands.spawn((
        Text2d::new(""),
        TextFont {
            font: assets.source(),
            font_size: FontSize::Px(18.0),
            ..default()
        },
        TextColor(theme::text()),
        Anchor::CENTER,
        Transform::from_xyz(0.0, -250.0, 9.0),
        KernelStatus,
        ui::ScreenEntity,
    ));

    ui::text(
        commands,
        assets,
        t(lang, "CLICK THE 3x3 CELLS TO SET WEIGHTS   |   R = RESET", "点击 3x3 网格设置权重   |   R = 重置"),
        Vec2::new(0.0, -305.0),
        14.0,
        theme::text_dim(),
        Anchor::CENTER,
    );
}

pub fn kernel_input(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut active: ResMut<ActivePuzzle>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Puzzle::Kernel(p) = &mut active.0 else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok((camera, cam_tf)) = cameras.single() else {
        return;
    };
    let Ok(world) = camera.viewport_to_world_2d(cam_tf, cursor) else {
        return;
    };

    let left = KERNEL_C.x - 1.5 * KERNEL_CELL;
    let top = KERNEL_C.y + 1.5 * KERNEL_CELL;
    let x = ((world.x - left) / KERNEL_CELL).floor();
    let y = ((top - world.y) / KERNEL_CELL).floor();
    if (0.0..3.0).contains(&x) && (0.0..3.0).contains(&y) {
        p.cycle(x as usize, y as usize);
    }
}

#[allow(clippy::type_complexity)]
pub fn sync_kernel(
    active: Res<ActivePuzzle>,
    mut cells: Query<(&KernelCell, &mut Sprite), Without<KernelDot>>,
    mut dots: Query<(&KernelDot, &mut Sprite), Without<KernelCell>>,
    mut values: Query<(&KernelValue, &mut Text2d, &mut TextColor), Without<KernelStatus>>,
    mut status: Query<&mut Text2d, (With<KernelStatus>, Without<KernelValue>)>,
) {
    let Puzzle::Kernel(p) = &active.0 else {
        return;
    };
    let weights = p.weights();

    for (cell, mut sprite) in &mut cells {
        sprite.color = match weights[cell.y * 3 + cell.x] {
            1 => theme::cyan(),
            -1 => theme::ember_lit(),
            _ => theme::panel_hover(),
        };
    }

    for (value, mut text, mut color) in &mut values {
        let w = weights[value.y * 3 + value.x];
        text.0 = match w {
            1 => "+1".to_string(),
            -1 => "-1".to_string(),
            _ => "0".to_string(),
        };
        color.0 = if w == 0 {
            theme::text_dim()
        } else {
            theme::bg()
        };
    }

    let predict = p.predict();
    for (dot, mut sprite) in &mut dots {
        sprite.color = if predict[dot.index] {
            theme::amber()
        } else {
            Color::NONE
        };
    }

    let matched = p.matched();
    for mut text in &mut status {
        text.0 = format!("MATCH {matched}/{}", predict.len());
    }
}

// ---------------------------------------------------------------- 词向量

const V_C: Vec2 = Vec2::new(0.0, 14.0);
const V_HALF: f32 = 210.0;
const ANCHORS: [(f32, f32); 3] = [(-0.5, -0.3), (0.3, -0.3), (-0.5, 0.5)];

#[derive(Component)]
pub struct VectorDot;

#[derive(Component)]
pub struct VectorLabel;

#[derive(Component)]
pub struct VectorGlow;

#[derive(Component)]
pub struct VectorStatus;

#[derive(Resource, Default)]
pub struct VectorDrag(pub bool);

fn vmap(p: (f32, f32)) -> Vec2 {
    V_C + Vec2::new(p.0, p.1) * V_HALF
}

fn spawn_vector(
    commands: &mut Commands,
    assets: &UiAssets,
    lang: Lang,
    puzzle: &ooi_core::VectorPuzzle,
) {
    let cfg = puzzle.config();

    commands.spawn((
        assets.tex.panel_sprite(
            theme::glass().with_alpha(0.72),
            Vec2::splat(V_HALF * 2.0 + 24.0),
        ),
        Transform::from_xyz(V_C.x, V_C.y, 1.0),
        ui::ScreenEntity,
    ));
    commands.spawn((
        assets.tex.outline_sprite(
            theme::line().with_alpha(0.45),
            Vec2::splat(V_HALF * 2.0 + 24.0),
        ),
        Transform::from_xyz(V_C.x, V_C.y, 2.6),
        ui::ScreenEntity,
    ));

    for step in 1..4 {
        let t = step as f32 / 4.0;
        commands.spawn((
            Sprite::from_color(theme::text().with_alpha(0.06), Vec2::new(1.0, V_HALF * 2.0)),
            Transform::from_xyz(V_C.x - V_HALF + t * V_HALF * 2.0, V_C.y, 2.0),
            ui::ScreenEntity,
        ));
        commands.spawn((
            Sprite::from_color(theme::text().with_alpha(0.06), Vec2::new(V_HALF * 2.0, 1.0)),
            Transform::from_xyz(V_C.x, V_C.y - V_HALF + t * V_HALF * 2.0, 2.0),
            ui::ScreenEntity,
        ));
    }
    for axis in 0..2 {
        let size = if axis == 0 {
            Vec2::new(V_HALF * 2.0, 1.0)
        } else {
            Vec2::new(1.0, V_HALF * 2.0)
        };
        commands.spawn((
            Sprite::from_color(theme::axis().with_alpha(0.7), size),
            Transform::from_xyz(V_C.x, V_C.y, 2.2),
            ui::ScreenEntity,
        ));
    }
    ui::caption(
        commands,
        assets,
        "x",
        Vec2::new(V_C.x + V_HALF - 4.0, V_C.y - 14.0),
        theme::text_faint(),
        Anchor::CENTER_RIGHT,
    );
    ui::caption(
        commands,
        assets,
        "y",
        Vec2::new(V_C.x - 12.0, V_C.y + V_HALF - 4.0),
        theme::text_faint(),
        Anchor::CENTER_RIGHT,
    );

    let man = vmap(ANCHORS[0]);
    let woman = vmap(ANCHORS[1]);
    let king = vmap(ANCHORS[2]);
    let target = vmap(cfg.target);

    // Static relationship guides: man -> woman and king -> target are the same
    // displacement. Seeing the parallelogram is the whole idea.
    for (a, b, color) in [
        (man, woman, theme::cyan().with_alpha(0.28)),
        (king, target, theme::amber().with_alpha(0.20)),
        (man, king, theme::text().with_alpha(0.07)),
        (woman, target, theme::text().with_alpha(0.07)),
    ] {
        let delta = b - a;
        let mid = (a + b) * 0.5;
        commands.spawn((
            Sprite::from_color(color, Vec2::new(delta.length(), 1.0)),
            Transform::from_xyz(mid.x, mid.y, 3.0)
                .with_rotation(Quat::from_rotation_z(delta.y.atan2(delta.x))),
            ui::ScreenEntity,
        ));
    }

    // Target tolerance ring.
    commands.spawn((
        Sprite {
            image: assets.tex.ring_dash.clone(),
            color: theme::amber().with_alpha(0.55),
            custom_size: Some(Vec2::splat(cfg.tolerance * V_HALF * 3.2)),
            ..default()
        },
        Transform::from_xyz(target.x, target.y, 3.4),
        ui::ScreenEntity,
    ));
    ui::caption(
        commands,
        assets,
        t(lang, "TARGET", "目标"),
        target + Vec2::new(0.0, 28.0),
        theme::amber().with_alpha(0.7),
        Anchor::CENTER,
    );

    let labels = [
        (ANCHORS[0], Vec2::new(0.0, -24.0), Anchor::CENTER),
        (ANCHORS[1], Vec2::new(0.0, -24.0), Anchor::CENTER),
        (ANCHORS[2], Vec2::new(0.0, 24.0), Anchor::CENTER),
    ];
    for (i, (anchor, offset, anchor_kind)) in labels.iter().enumerate() {
        let pos = vmap(*anchor);
        ui::halo(
            commands,
            assets,
            pos,
            56.0,
            theme::cyan().with_alpha(0.10),
            3.5,
        );
        commands.spawn((
            Sprite {
                image: assets.tex.ring.clone(),
                color: theme::cyan().with_alpha(0.55),
                custom_size: Some(Vec2::splat(20.0)),
                ..default()
            },
            Transform::from_xyz(pos.x, pos.y, 4.0),
            ui::ScreenEntity,
        ));
        commands.spawn((
            Sprite {
                image: assets.tex.dot.clone(),
                color: theme::cyan(),
                custom_size: Some(Vec2::splat(9.0)),
                ..default()
            },
            Transform::from_xyz(pos.x, pos.y, 4.2),
            ui::ScreenEntity,
        ));
        ui::text(
            commands,
            assets,
            word_label(lang, i),
            pos + *offset,
            14.0,
            theme::text_dim(),
            *anchor_kind,
        );
    }

    let start = vmap(puzzle.pos());
    commands.spawn((
        Sprite {
            image: assets.tex.glow.clone(),
            color: theme::amber().with_alpha(0.25),
            custom_size: Some(Vec2::splat(52.0)),
            ..default()
        },
        Transform::from_xyz(start.x, start.y, 4.4),
        VectorGlow,
        ui::ScreenEntity,
    ));
    commands.spawn((
        Sprite {
            image: assets.tex.ring.clone(),
            color: theme::amber().with_alpha(0.9),
            custom_size: Some(Vec2::splat(28.0)),
            ..default()
        },
        Transform::from_xyz(start.x, start.y, 4.6),
        ui::ScreenEntity,
    ));
    commands.spawn((
        Sprite {
            image: assets.tex.dot.clone(),
            color: theme::amber(),
            custom_size: Some(Vec2::splat(13.0)),
            ..default()
        },
        Transform::from_xyz(start.x, start.y, 4.8),
        VectorDot,
        ui::ScreenEntity,
    ));
    let label = ui::text(
        commands,
        assets,
        word_label(lang, 3),
        start + Vec2::new(0.0, 36.0),
        14.0,
        theme::amber(),
        Anchor::CENTER,
    );
    commands.entity(label).insert(VectorLabel);

    commands.spawn((
        Text2d::new(""),
        TextFont {
            font: assets.source(),
            font_size: FontSize::Px(13.0),
            ..default()
        },
        TextColor(theme::text_dim()),
        Anchor::CENTER,
        Transform::from_xyz(0.0, -236.0, 9.0),
        VectorStatus,
        ui::ScreenEntity,
    ));

    ui::text(
        commands,
        assets,
        t(lang, "DRAG THE QUESTION MARK   |   R = RESET", "拖动问号   |   R = 重置"),
        Vec2::new(0.0, -326.0),
        11.0,
        theme::text_faint(),
        Anchor::CENTER,
    );
}

pub fn vector_input(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut active: ResMut<ActivePuzzle>,
    mut drag: ResMut<VectorDrag>,
) {
    let Puzzle::Vector(p) = &mut active.0 else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok((camera, cam_tf)) = cameras.single() else {
        return;
    };
    let Ok(world) = camera.viewport_to_world_2d(cam_tf, cursor) else {
        return;
    };

    let dot = vmap(p.pos());
    if mouse.just_pressed(MouseButton::Left) {
        drag.0 = (world - dot).length() < 60.0;
    }
    if mouse.just_released(MouseButton::Left) {
        drag.0 = false;
    }
    if drag.0 && mouse.pressed(MouseButton::Left) {
        let rel = (world - V_C) / V_HALF;
        p.set_pos(rel.x, rel.y);
    }
}

pub fn sync_vector(
    active: Res<ActivePuzzle>,
    lang: Res<LangRes>,
    mut dot: Query<(&mut Transform, &mut Sprite), With<VectorDot>>,
    mut glow: Query<(&mut Transform, &mut Sprite), (With<VectorGlow>, Without<VectorDot>)>,
    mut label: Query<&mut Transform, (With<VectorLabel>, Without<VectorDot>, Without<VectorGlow>)>,
    mut status: Query<&mut Text2d, With<VectorStatus>>,
) {
    let Puzzle::Vector(p) = &active.0 else {
        return;
    };
    let pos = vmap(p.pos());
    let solved = p.is_solved();
    let color = if solved {
        theme::amber()
    } else {
        theme::cyan()
    };
    for (mut transform, mut sprite) in &mut dot {
        transform.translation.x = pos.x;
        transform.translation.y = pos.y;
        sprite.color = color;
    }
    for (mut transform, mut sprite) in &mut glow {
        transform.translation.x = pos.x;
        transform.translation.y = pos.y;
        sprite.color = color.with_alpha(0.25);
    }
    for mut transform in &mut label {
        transform.translation.x = pos.x;
        transform.translation.y = pos.y + 36.0;
    }
    for mut text in &mut status {
        text.0 = if solved {
            t(lang.0, "SOLVED", "已解").to_string()
        } else {
            format!(
                "{} {:.2}   \u{00b7}   ({:+.2}, {:+.2})",
                t(lang.0, "DISTANCE", "距离"),
                p.distance(),
                p.pos().0,
                p.pos().1
            )
        };
    }
}

// ---------------------------------------------------------------- 数据划分

const BAR_W: f32 = 700.0;
const BAR_C: Vec2 = Vec2::new(0.0, -20.0);

#[derive(Component)]
pub struct AllocSegment(pub usize);

#[derive(Component)]
pub struct AllocHandle(pub usize);

#[derive(Component)]
pub struct AllocLabel(pub usize);

#[derive(Component)]
pub struct AllocStatus;

#[derive(Resource, Default)]
pub struct AllocDrag(pub Option<usize>);

fn bar_x(cut: u32, total: u32) -> f32 {
    BAR_C.x - BAR_W / 2.0 + cut as f32 / total as f32 * BAR_W
}

fn spawn_allocate(
    commands: &mut Commands,
    assets: &UiAssets,
    lang: Lang,
    puzzle: &ooi_core::AllocatePuzzle,
) {
    for i in 0..3 {
        commands.spawn((
            Sprite::from_color(theme::cyan_dim(), Vec2::new(10.0, 44.0)),
            Transform::from_xyz(BAR_C.x, BAR_C.y, 2.0),
            AllocSegment(i),
            ui::ScreenEntity,
        ));
        commands.spawn((
            Text2d::new(""),
            TextFont {
                font: assets.source(),
                font_size: FontSize::Px(15.0),
                ..default()
            },
            TextColor(theme::text()),
            Anchor::CENTER,
            Transform::from_xyz(BAR_C.x, BAR_C.y + 46.0, 8.0),
            AllocLabel(i),
            ui::ScreenEntity,
        ));
    }

    for i in 0..2 {
        commands.spawn((
            Sprite::from_color(theme::amber(), Vec2::new(10.0, 64.0)),
            Transform::from_xyz(BAR_C.x, BAR_C.y, 5.0),
            AllocHandle(i),
            ui::ScreenEntity,
        ));
    }

    commands.spawn((
        Text2d::new(""),
        TextFont {
            font: assets.source(),
            font_size: FontSize::Px(15.0),
            ..default()
        },
        TextColor(theme::text_dim()),
        Anchor::CENTER,
        Transform::from_xyz(0.0, -110.0, 8.0),
        AllocStatus,
        ui::ScreenEntity,
    ));

    let names = [
        t(lang, "TRAIN", "训练"),
        t(lang, "VALIDATE", "验证"),
        t(lang, "TEST", "测试"),
    ];
    let _ = names;

    ui::text(
        commands,
        assets,
        t(lang, "DRAG THE TWO DIVIDERS   |   R = RESET", "拖动两条分隔线   |   R = 重置"),
        Vec2::new(0.0, -305.0),
        14.0,
        theme::text_dim(),
        Anchor::CENTER,
    );

    let _ = puzzle;
}

pub fn allocate_input(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut active: ResMut<ActivePuzzle>,
    mut drag: ResMut<AllocDrag>,
) {
    let Puzzle::Allocate(p) = &mut active.0 else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok((camera, cam_tf)) = cameras.single() else {
        return;
    };
    let Ok(world) = camera.viewport_to_world_2d(cam_tf, cursor) else {
        return;
    };

    let total = p.config().total;
    let (a, b) = p.cuts();

    if mouse.just_pressed(MouseButton::Left) {
        let near_a = (world.x - bar_x(a, total)).abs() < 26.0;
        let near_b = (world.x - bar_x(b, total)).abs() < 26.0;
        let in_bar = (world.y - BAR_C.y).abs() < 60.0;
        drag.0 = if in_bar && near_a {
            Some(0)
        } else if in_bar && near_b {
            Some(1)
        } else {
            None
        };
    }
    if mouse.just_released(MouseButton::Left) {
        drag.0 = None;
    }
    if let Some(which) = drag.0 {
        let raw = ((world.x - (BAR_C.x - BAR_W / 2.0)) / BAR_W * total as f32).round() as i32;
        let value = raw.clamp(0, total as i32) as u32;
        if which == 0 {
            p.set_a(value);
        } else {
            p.set_b(value);
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn sync_allocate(
    active: Res<ActivePuzzle>,
    lang: Res<LangRes>,
    mut segments: Query<
        (&AllocSegment, &mut Sprite, &mut Transform),
        (Without<AllocHandle>, Without<AllocLabel>),
    >,
    mut handles: Query<
        (&AllocHandle, &mut Transform),
        (
            With<AllocHandle>,
            Without<AllocSegment>,
            Without<AllocLabel>,
        ),
    >,
    mut labels: Query<
        (&AllocLabel, &mut Text2d, &mut Transform),
        (
            With<AllocLabel>,
            Without<AllocSegment>,
            Without<AllocHandle>,
        ),
    >,
    mut status: Query<&mut Text2d, (With<AllocStatus>, Without<AllocLabel>)>,
) {
    let Puzzle::Allocate(p) = &active.0 else {
        return;
    };
    let total = p.config().total;
    let parts = p.parts();
    let (a, b) = p.cuts();
    let colors = [theme::cyan_dim(), theme::amber(), theme::ember_dim()];
    let names = [
        t(lang.0, "TRAIN", "训练"),
        t(lang.0, "VALIDATE", "验证"),
        t(lang.0, "TEST", "测试"),
    ];

    let mut start = 0u32;
    for (segment, mut sprite, mut transform) in &mut segments {
        let i = segment.0;
        let len = parts[i];
        let width = (len as f32 / total as f32 * BAR_W).max(1.0);
        let center =
            BAR_C.x - BAR_W / 2.0 + (start as f32 + len as f32 / 2.0) / total as f32 * BAR_W;
        sprite.custom_size = Some(Vec2::new(width - 2.0, 44.0));
        sprite.color = if p.is_solved() {
            theme::amber()
        } else {
            colors[i]
        };
        transform.translation.x = center;
        start += len;
    }

    for (handle, mut transform) in &mut handles {
        let cut = if handle.0 == 0 { a } else { b };
        transform.translation.x = bar_x(cut, total);
    }

    let ranges = p.config().ranges;
    let mut start = 0u32;
    for (label, mut text, mut transform) in &mut labels {
        let i = label.0;
        let len = parts[i];
        let center =
            BAR_C.x - BAR_W / 2.0 + (start as f32 + len as f32 / 2.0) / total as f32 * BAR_W;
        start += len;

        text.0 = format!(
            "{} {}  ({}-{})",
            names[i], parts[i], ranges[i].0, ranges[i].1
        );
        transform.translation.x = center;
        transform.translation.y = BAR_C.y + 48.0;
    }

    for mut text in &mut status {
        text.0 = if p.is_solved() {
            t(lang.0, "SOLVED", "已解").to_string()
        } else {
            String::new()
        };
    }
}

// ---------------------------------------------------------------- 早停

const PLOT_C: Vec2 = Vec2::new(0.0, -30.0);
const PLOT_HX: f32 = 380.0;
const PLOT_HY: f32 = 150.0;

#[derive(Component)]
pub struct TimelineHead;

#[derive(Component)]
pub struct TimelineStatus;

fn plot_x(epoch: u32, epochs: u32) -> f32 {
    PLOT_C.x - PLOT_HX + epoch as f32 / epochs as f32 * PLOT_HX * 2.0
}

fn plot_y(loss: f32) -> f32 {
    PLOT_C.y - PLOT_HY + loss.clamp(0.0, 1.0) * PLOT_HY * 2.0
}

fn spawn_timeline(
    commands: &mut Commands,
    assets: &UiAssets,
    lang: Lang,
    puzzle: &ooi_core::TimelinePuzzle,
) {
    let cfg = puzzle.config();

    commands.spawn((
        Sprite::from_color(
            theme::panel(),
            Vec2::new(PLOT_HX * 2.0 + 16.0, PLOT_HY * 2.0 + 16.0),
        ),
        Transform::from_xyz(PLOT_C.x, PLOT_C.y, 1.0),
        ui::ScreenEntity,
    ));

    for e in 0..cfg.epochs {
        for (kind, color) in [(0, theme::cyan()), (1, theme::amber())] {
            let (l0, l1) = if kind == 0 {
                (
                    ooi_core::TimelinePuzzle::train_loss(e),
                    ooi_core::TimelinePuzzle::train_loss(e + 1),
                )
            } else {
                (
                    ooi_core::TimelinePuzzle::val_loss(e),
                    ooi_core::TimelinePuzzle::val_loss(e + 1),
                )
            };
            let a = Vec2::new(plot_x(e, cfg.epochs), plot_y(l0));
            let b = Vec2::new(plot_x(e + 1, cfg.epochs), plot_y(l1));
            let mid = (a + b) * 0.5;
            let delta = b - a;
            commands.spawn((
                Sprite {
                    color,
                    custom_size: Some(Vec2::new(delta.length().max(1.0), 2.0)),
                    ..default()
                },
                Transform::from_xyz(mid.x, mid.y, 2.0)
                    .with_rotation(Quat::from_rotation_z(delta.y.atan2(delta.x))),
                ui::ScreenEntity,
            ));
        }
    }

    ui::text(
        commands,
        assets,
        "train",
        Vec2::new(PLOT_C.x - PLOT_HX, PLOT_C.y + PLOT_HY + 26.0),
        13.0,
        theme::cyan(),
        Anchor::TOP_LEFT,
    );
    ui::text(
        commands,
        assets,
        "validation",
        Vec2::new(PLOT_C.x - PLOT_HX + 74.0, PLOT_C.y + PLOT_HY + 26.0),
        13.0,
        theme::amber(),
        Anchor::TOP_LEFT,
    );
    for (label, epoch) in [("0", 0u32), ("15", 15), ("30", 30)] {
        ui::text(
            commands,
            assets,
            label,
            Vec2::new(plot_x(epoch, cfg.epochs), PLOT_C.y - PLOT_HY - 8.0),
            12.0,
            theme::text_dim(),
            Anchor::TOP_CENTER,
        );
    }

    commands.spawn((
        Sprite::from_color(theme::amber(), Vec2::new(3.0, PLOT_HY * 2.0)),
        Transform::from_xyz(plot_x(0, cfg.epochs), PLOT_C.y, 4.0),
        TimelineHead,
        ui::ScreenEntity,
    ));

    commands.spawn((
        Text2d::new(""),
        TextFont {
            font: assets.source(),
            font_size: FontSize::Px(16.0),
            ..default()
        },
        TextColor(theme::text()),
        Anchor::CENTER,
        Transform::from_xyz(0.0, -250.0, 9.0),
        TimelineStatus,
        ui::ScreenEntity,
    ));

    ui::text(
        commands,
        assets,
        t(lang, "SPACE = STOP WHEN VALIDATION TURNS UP   |   R = RESTART", "验证损失抬头时按空格停下   |   R = 重新开始"),
        Vec2::new(0.0, -305.0),
        14.0,
        theme::text_dim(),
        Anchor::CENTER,
    );
}

pub fn gradient_input(
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut active: ResMut<ActivePuzzle>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok((camera, cam_tf)) = cameras.single() else {
        return;
    };
    let Ok(world) = camera.viewport_to_world_2d(cam_tf, cursor) else {
        return;
    };
    let rel = world - G_C;
    if !rel.abs().cmple(G_HALF).all() {
        return;
    }
    let Puzzle::Gradient(puzzle) = &mut active.0 else {
        return;
    };
    let (px, py) = puzzle.position();
    let pos = gmap(px, py);
    let dir = world - pos;
    if dir.length_squared() > 1e-6 {
        puzzle.set_aim(dir.y.atan2(dir.x));
    }
}

#[allow(clippy::type_complexity)]
pub fn sync_nucleus(
    active: Res<ActivePuzzle>,
    lang: Res<LangRes>,
    mut bars: Query<(&NucleusBar, &mut Sprite, &mut Transform), Without<NucleusMassFill>>,
    mut mass_fill: Query<
        (&mut Sprite, &mut Transform),
        (With<NucleusMassFill>, Without<NucleusBar>),
    >,
    mut status: Query<(&mut Text2d, &mut TextColor), With<NucleusStatusText>>,
) {
    let Puzzle::Nucleus(puzzle) = &active.0 else {
        return;
    };
    let cfg = puzzle.config();
    let prefix_ok = puzzle.prefix_len().is_some();
    let solved = puzzle.is_solved();
    let too_much = puzzle.is_too_much();

    for (bar, mut sprite, mut transform) in &mut bars {
        let width = cfg.probs[bar.0] * N_BAR_W;
        transform.translation.x = N_C.x + N_BAR_X + width * 0.5;
        sprite.custom_size = Some(Vec2::new(width.max(1.0), N_BAR_H));
        let included = puzzle.included(bar.0);
        sprite.color = if !included {
            theme::cyan_dim().with_alpha(0.55)
        } else if !prefix_ok || too_much {
            theme::amber().with_alpha(0.85)
        } else {
            theme::cyan()
        };
    }

    let mass = puzzle.mass();
    let width = mass * N_MASS_W;
    for (mut sprite, mut transform) in &mut mass_fill {
        transform.translation.x = N_C.x - N_MASS_W * 0.5 + width * 0.5;
        sprite.custom_size = Some(Vec2::new(width.max(1.0), 6.0));
        sprite.color = if solved {
            theme::mint()
        } else if too_much {
            theme::amber()
        } else {
            theme::cyan()
        };
    }

    let count = puzzle.count();
    let (text, color) = if solved {
        (
            format!(
                "{} {:.2}  \u{00b7}  {} {}  \u{00b7}  {}",
                t(lang.0, "NUCLEUS", "核采样"),
                mass,
                count,
                t(lang.0, "TOKENS", "词元"),
                t(lang.0, "SOLVED", "已解")
            ),
            theme::mint(),
        )
    } else if !prefix_ok {
        (
            t(lang.0, "KEEP THE HIGHEST TOKENS FIRST", "先保留概率最高的词元").to_string(),
            theme::amber(),
        )
    } else if count == 0 {
        (
            format!(
                "{} {:.2}  \u{00b7}  {} {}  \u{00b7}  {} {:.0}%",
                t(lang.0, "MASS", "概率质量"),
                mass,
                count,
                t(lang.0, "TOKENS", "词元"),
                t(lang.0, "TARGET", "目标"),
                cfg.threshold * 100.0
            ),
            theme::text_faint(),
        )
    } else if puzzle.is_too_small() {
        (
            format!(
                "{} {:.2}  \u{00b7}  {} {}  \u{00b7}  {}",
                t(lang.0, "MASS", "概率质量"),
                mass,
                count,
                t(lang.0, "TOKENS", "词元"),
                t(lang.0, "TOO LITTLE MASS", "质量不够")
            ),
            theme::text_dim(),
        )
    } else {
        (
            format!(
                "{} {:.2}  \u{00b7}  {} {}  \u{00b7}  {}",
                t(lang.0, "MASS", "概率质量"),
                mass,
                count,
                t(lang.0, "TOKENS", "词元"),
                t(lang.0, "TOO MUCH TAIL", "尾部太多")
            ),
            theme::amber(),
        )
    };
    for (mut label, mut label_color) in &mut status {
        label.0 = text.clone();
        label_color.0 = color;
    }
}

#[allow(clippy::type_complexity)]
pub fn sync_gradient(
    active: Res<ActivePuzzle>,
    lang: Res<LangRes>,
    mut sprites: ParamSet<(
        Query<(&GradientTrailDot, &mut Sprite, &mut Transform)>,
        Query<(&mut Sprite, &mut Transform), With<GradientBall>>,
        Query<(&mut Sprite, &mut Transform), With<GradientBallGlow>>,
        Query<(&mut Sprite, &mut Transform), With<GradientCandidateRing>>,
        Query<(&mut Sprite, &mut Transform), With<GradientAimLine>>,
        Query<(&mut Sprite, &mut Transform), With<GradientArrowLine>>,
        Query<(&mut Sprite, &mut Transform), With<GradientArrowTip>>,
    )>,
    mut status: Query<(&mut Text2d, &mut TextColor), With<GradientStatusText>>,
) {
    let Puzzle::Gradient(puzzle) = &active.0 else {
        return;
    };
    let (px, py) = puzzle.position();
    let pos = gmap(px, py);
    let (cx, cy) = puzzle.candidate();
    let candidate = gmap(cx, cy);
    let trail = puzzle.trail();
    let current_loss = puzzle.current_loss();
    let candidate_loss = puzzle.candidate_loss();
    let solved = puzzle.is_solved();
    let rejected = puzzle.rejected();
    let steps = puzzle.steps();

    for (marker, mut sprite, mut transform) in sprites.p0().iter_mut() {
        if marker.0 < trail.len() {
            let (x, y) = trail[marker.0];
            let p = gmap(x, y);
            transform.translation.x = p.x;
            transform.translation.y = p.y;
            let t = (marker.0 as f32 + 1.0) / trail.len().max(1) as f32;
            sprite.color = theme::cyan().with_alpha(0.18 + 0.55 * t);
        } else {
            sprite.color = Color::NONE;
        }
    }

    for (mut sprite, mut transform) in sprites.p1().iter_mut() {
        transform.translation.x = pos.x;
        transform.translation.y = pos.y;
        sprite.color = theme::cyan_hot().with_alpha(0.95);
    }
    for (mut sprite, mut transform) in sprites.p2().iter_mut() {
        transform.translation.x = pos.x;
        transform.translation.y = pos.y;
        sprite.color = theme::cyan().with_alpha(0.20);
    }
    for (mut sprite, mut transform) in sprites.p3().iter_mut() {
        transform.translation.x = candidate.x;
        transform.translation.y = candidate.y;
        sprite.color = if solved {
            Color::NONE
        } else if candidate_loss + 1e-6 < current_loss {
            theme::cyan()
        } else {
            theme::ember_lit()
        };
    }

    let aim_delta = candidate - pos;
    if !solved && aim_delta.length_squared() > 1e-4 {
        let mid = (pos + candidate) * 0.5;
        for (mut sprite, mut transform) in sprites.p4().iter_mut() {
            transform.translation.x = mid.x;
            transform.translation.y = mid.y;
            transform.rotation = Quat::from_rotation_z(aim_delta.y.atan2(aim_delta.x));
            sprite.custom_size = Some(Vec2::new(aim_delta.length(), 1.5));
            sprite.color = theme::cyan().with_alpha(0.55);
        }
    } else {
        for (mut sprite, _) in sprites.p4().iter_mut() {
            sprite.color = Color::NONE;
        }
    }

    let (gx, gy) = puzzle.gradient();
    let gradient = Vec2::new(gx, gy);
    if !solved && gradient.length_squared() > 1e-8 {
        let dir = gradient.normalize();
        let length = 58.0;
        let end = pos + dir * length;
        let mid = (pos + end) * 0.5;
        for (mut sprite, mut transform) in sprites.p5().iter_mut() {
            transform.translation.x = mid.x;
            transform.translation.y = mid.y;
            transform.rotation = Quat::from_rotation_z(dir.y.atan2(dir.x));
            sprite.custom_size = Some(Vec2::new(length, 2.2));
            sprite.color = theme::amber().with_alpha(0.85);
        }
        for (mut sprite, mut transform) in sprites.p6().iter_mut() {
            transform.translation.x = end.x;
            transform.translation.y = end.y;
            sprite.color = theme::amber();
        }
    } else {
        for (mut sprite, _) in sprites.p5().iter_mut() {
            sprite.color = Color::NONE;
        }
        for (mut sprite, _) in sprites.p6().iter_mut() {
            sprite.color = Color::NONE;
        }
    }

    let (text, color) = if solved {
        (
            t(lang.0, "GLOBAL VALLEY  \u{00b7}  SOLVED", "全局谷底  \u{00b7}  已解").to_string(),
            theme::mint(),
        )
    } else if rejected {
        (
            t(lang.0, "NOT DOWNHILL  \u{00b7}  AIM LOWER", "不是下坡  \u{00b7}  往更低处瞄").to_string(),
            theme::amber(),
        )
    } else if steps > 0 {
        (
            format!(
                "{}  \u{00b7}  {} {:.3}  \u{00b7}  {} {}",
                t(lang.0, "DOWNHILL", "下坡"),
                t(lang.0, "LOSS", "损失"),
                current_loss,
                t(lang.0, "STEP", "单步"),
                steps
            ),
            theme::cyan(),
        )
    } else {
        (
            format!(
                "{}  \u{00b7}  {} {:.3}  \u{2192}  {:.3}",
                t(lang.0, "AIM", "瞄准"),
                t(lang.0, "LOSS", "损失"),
                current_loss,
                candidate_loss
            ),
            theme::text_dim(),
        )
    };
    for (mut label, mut label_color) in &mut status {
        label.0 = text.clone();
        label_color.0 = color;
    }
}

pub fn descent_tick(time: Res<Time>, mut active: ResMut<ActivePuzzle>, mut clock: Local<f32>) {
    let Puzzle::Descent(puzzle) = &mut active.0 else {
        *clock = 0.0;
        return;
    };
    if !puzzle.is_running() {
        *clock = 0.0;
        return;
    }
    *clock += time.delta_secs();
    let interval = 0.025;
    let mut steps = (*clock / interval).floor() as u32;
    *clock -= steps as f32 * interval;
    steps = steps.min(8);
    for _ in 0..steps {
        if !puzzle.step() {
            break;
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn sync_descent(
    active: Res<ActivePuzzle>,
    lang: Res<LangRes>,
    mut sprites: ParamSet<(
        Query<(&DescentStartRing, &mut Sprite)>,
        Query<(&DescentTrailDot, &mut Sprite, &mut Transform)>,
        Query<(&mut Sprite, &mut Transform), With<DescentBall>>,
        Query<(&mut Sprite, &mut Transform), With<DescentBallGlow>>,
    )>,
    mut status: Query<(&mut Text2d, &mut TextColor), With<DescentStatusText>>,
) {
    let Puzzle::Descent(puzzle) = &active.0 else {
        return;
    };
    let selected = puzzle.selected();
    let position = puzzle.position();
    let trail = puzzle.trail();
    let running = puzzle.is_running();
    let finished = puzzle.is_finished();
    let solved = puzzle.is_solved();
    let steps = puzzle.steps();
    let loss = puzzle.current_loss();

    for (marker, mut sprite) in sprites.p0().iter_mut() {
        let chosen = selected == Some(marker.0);
        sprite.color = if chosen {
            theme::cyan()
        } else {
            theme::ice().with_alpha(0.75)
        };
        sprite.custom_size = Some(Vec2::splat(if chosen { 38.0 } else { 34.0 }));
    }

    let trail_len = trail.len();
    for (marker, mut sprite, mut transform) in sprites.p1().iter_mut() {
        if marker.0 < trail_len {
            let (x, y) = trail[marker.0];
            let pos = dmap(x, y);
            transform.translation.x = pos.x;
            transform.translation.y = pos.y;
            let progress = (marker.0 as f32 + 1.0) / trail_len.max(1) as f32;
            sprite.color = theme::cyan().with_alpha(0.20 + 0.55 * progress);
        } else {
            sprite.color = Color::NONE;
        }
    }

    let ball_pos = position.map(|(x, y)| dmap(x, y));
    for (mut sprite, mut transform) in sprites.p2().iter_mut() {
        if let Some(pos) = ball_pos {
            transform.translation.x = pos.x;
            transform.translation.y = pos.y;
            sprite.color = theme::cyan_hot().with_alpha(0.95);
        } else {
            sprite.color = Color::NONE;
        }
    }
    for (mut sprite, mut transform) in sprites.p3().iter_mut() {
        if let Some(pos) = ball_pos {
            transform.translation.x = pos.x;
            transform.translation.y = pos.y;
            sprite.color = theme::cyan().with_alpha(0.22);
        } else {
            sprite.color = Color::NONE;
        }
    }

    let (text, color) = if selected.is_none() {
        (
            t(lang.0, "CHOOSE A STARTING POINT", "选择一个起点").to_string(),
            theme::text_faint(),
        )
    } else if running {
        (
            format!(
                "{}  \u{00b7}  {} {:.3}  \u{00b7}  {} {}",
                t(lang.0, "DESCEND", "下降"),
                t(lang.0, "LOSS", "损失"),
                loss,
                t(lang.0, "STEP", "单步"),
                steps
            ),
            theme::amber(),
        )
    } else if finished && solved {
        (
            t(lang.0, "GLOBAL VALLEY  \u{00b7}  SOLVED", "全局谷底  \u{00b7}  已解").to_string(),
            theme::mint(),
        )
    } else if finished {
        (
            t(lang.0, "LOCAL VALLEY  \u{00b7}  TRY THE DEEPER SIDE", "局部谷底  \u{00b7}  试试更深的一侧").to_string(),
            theme::amber(),
        )
    } else {
        (
            format!(
                "{} {}  \u{00b7}  {} {:.3}",
                t(lang.0, "START", "起点"),
                selected.unwrap_or(0) + 1,
                t(lang.0, "LOSS", "损失"),
                loss
            ),
            theme::text_dim(),
        )
    };
    for (mut label, mut label_color) in &mut status {
        label.0 = text.clone();
        label_color.0 = color;
    }
}

pub fn timeline_tick(time: Res<Time>, mut active: ResMut<ActivePuzzle>) {
    if let Puzzle::Timeline(p) = &mut active.0 {
        p.tick(time.delta_secs());
    }
}

pub fn timeline_input(keys: Res<ButtonInput<KeyCode>>, mut active: ResMut<ActivePuzzle>) {
    if keys.just_pressed(KeyCode::Space)
        && let Puzzle::Timeline(p) = &mut active.0
    {
        p.stop();
    }
}

pub fn sync_timeline(
    active: Res<ActivePuzzle>,
    mut head: Query<(&mut Transform, &mut Sprite), With<TimelineHead>>,
    mut status: Query<&mut Text2d, With<TimelineStatus>>,
) {
    let Puzzle::Timeline(p) = &active.0 else {
        return;
    };
    let cfg = p.config();
    let epoch = p.stopped_at().unwrap_or_else(|| p.epoch());
    let x = plot_x(epoch, cfg.epochs);

    for (mut transform, mut sprite) in &mut head {
        transform.translation.x = x;
        sprite.color = if p.stopped_at().is_some() {
            if p.is_solved() {
                theme::amber()
            } else {
                theme::ember_lit()
            }
        } else {
            theme::cyan()
        };
    }

    for mut text in &mut status {
        text.0 = format!(
            "EPOCH {epoch}   TRAIN {:.3}   VAL {:.3}",
            ooi_core::TimelinePuzzle::train_loss(epoch.min(cfg.epochs)),
            ooi_core::TimelinePuzzle::val_loss(epoch.min(cfg.epochs))
        );
    }
}

// ---------------------------------------------------------------- 排序

#[derive(Component)]
pub struct RankCard(pub usize);

#[derive(Component)]
pub struct RankBadge(pub usize);

#[derive(Component)]
pub struct RankStatus;

fn spawn_rank(
    commands: &mut Commands,
    assets: &UiAssets,
    lang: Lang,
    id: &str,
    puzzle: &ooi_core::RankPuzzle,
) {
    let n = puzzle.config().items;
    ui::caption(
        commands,
        assets,
        t(lang, "RANK", "排序"),
        Vec2::new(-420.0, 198.0),
        theme::text_faint(),
        Anchor::CENTER_LEFT,
    );
    for i in 0..n {
        let y = 135.0 - i as f32 * 116.0;
        let card = ui::action_button(
            commands,
            assets,
            Vec2::new(0.0, y),
            Vec2::new(840.0, 100.0),
            "",
            16.0,
            Action::Option(i),
            false,
        );
        commands.entity(card).insert(RankCard(i));
        commands.spawn((
            Text2d::new(""),
            TextFont {
                font: assets.source(),
                font_size: FontSize::Px(20.0),
                ..default()
            },
            TextColor(theme::text_dim()),
            Anchor::CENTER,
            Transform::from_xyz(-390.0, y, 21.0),
            RankBadge(i),
            ui::ScreenEntity,
        ));
        commands.spawn((
            Text2d::new(rank_item(lang, id, i)),
            TextFont {
                font: assets.source(),
                font_size: FontSize::Px(15.0),
                ..default()
            },
            TextColor(theme::text()),
            Anchor::CENTER_LEFT,
            TextLayout::new(Justify::Left, LineBreak::WordBoundary),
            TextBounds::from(Vec2::new(700.0, 86.0)),
            Transform::from_xyz(-345.0, y, 21.0),
            ui::ScreenEntity,
        ));
    }

    commands.spawn((
        Text2d::new(""),
        TextFont {
            font: assets.source(),
            font_size: FontSize::Px(15.0),
            ..default()
        },
        TextColor(theme::amber()),
        Anchor::CENTER,
        Transform::from_xyz(0.0, -204.0, 9.0),
        RankStatus,
        ui::ScreenEntity,
    ));

    ui::text(
        commands,
        assets,
        t(lang, "CLICK IN ORDER: BEST FIRST   |   R = RESET", "按顺序点击：最好的先点   |   R = 重置"),
        Vec2::new(0.0, -326.0),
        11.0,
        theme::text_faint(),
        Anchor::CENTER,
    );
}

#[allow(clippy::type_complexity)]
pub fn sync_rank(
    active: Res<ActivePuzzle>,
    lang: Res<LangRes>,
    mut cards: Query<(&RankCard, &mut Sprite, &mut ui::ButtonUi), Without<RankBadge>>,
    mut badges: Query<(&RankBadge, &mut Text2d, &mut TextColor), Without<RankCard>>,
    mut status: Query<&mut Text2d, (With<RankStatus>, Without<RankBadge>)>,
) {
    let Puzzle::Rank(p) = &active.0 else {
        return;
    };
    let order = p.order();

    for (card, mut sprite, mut button) in &mut cards {
        let pos = order.iter().position(|x| *x == card.0);
        let color = if p.is_complete() && p.is_solved() {
            Color::srgba(0.22, 0.16, 0.055, 1.0)
        } else if pos.is_some() {
            theme::action()
        } else {
            theme::glass_raised()
        };
        sprite.color = color;
        button.base = color;
    }

    for (badge, mut text, mut color) in &mut badges {
        let pos = order.iter().position(|x| *x == badge.0);
        text.0 = pos.map(|i| format!("{}", i + 1)).unwrap_or_default();
        color.0 = if pos.is_some() {
            theme::amber()
        } else {
            theme::text_dim()
        };
    }

    for mut text in &mut status {
        text.0 = if p.is_solved() {
            t(lang.0, "SOLVED", "已解").to_string()
        } else if p.is_complete() {
            t(lang.0, "TRY AGAIN", "再试一次").to_string()
        } else {
            String::new()
        };
    }
}

// ---------------------------------------------------------------- 注意力

#[derive(Component)]
pub struct AttnToken(pub usize);

#[derive(Component)]
pub struct AttnChip(pub usize);

#[derive(Component)]
pub struct AttnLink;

#[derive(Component)]
pub struct AttnStatus;

fn token_pos(index: usize) -> Vec2 {
    let col = index % 7;
    let row = index / 7;
    Vec2::new(-450.0 + col as f32 * 150.0, 90.0 - row as f32 * 64.0)
}

fn spawn_attention(
    commands: &mut Commands,
    assets: &UiAssets,
    lang: Lang,
    id: &str,
    puzzle: &ooi_core::AttentionPuzzle,
) {
    let tokens = attention_tokens(lang, id, puzzle.round());
    let query = puzzle.current().map(|r| r.query);

    ui::card(
        commands,
        assets,
        Vec2::new(0.0, 56.0),
        Vec2::new(1090.0, 212.0),
        theme::violet().with_alpha(0.45),
    );
    ui::caption(
        commands,
        assets,
        t(lang, "ATTENTION FIELD", "注意力场"),
        Vec2::new(-520.0, 145.0),
        theme::text_faint(),
        Anchor::CENTER_LEFT,
    );

    if let Some(q) = query
        && let Some(_word) = tokens.get(q)
    {
        let qpos = token_pos(q);
        for i in 0..tokens.len() {
            if i == q {
                continue;
            }
            let p = token_pos(i);
            let delta = p - qpos;
            let mid = (qpos + p) * 0.5;
            commands.spawn((
                Sprite::from_color(
                    theme::cyan().with_alpha(0.055),
                    Vec2::new(delta.length(), 1.0),
                ),
                Transform::from_xyz(mid.x, mid.y, 4.0)
                    .with_rotation(Quat::from_rotation_z(delta.y.atan2(delta.x))),
                ui::ScreenEntity,
            ));
        }
        ui::halo(
            commands,
            assets,
            qpos,
            96.0,
            theme::amber().with_alpha(0.16),
            4.5,
        );
        commands.spawn((
            Sprite {
                image: assets.tex.dot.clone(),
                color: theme::amber(),
                custom_size: Some(Vec2::splat(4.0)),
                ..default()
            },
            Transform::from_xyz(qpos.x, qpos.y + 30.0, 9.0),
            ui::ScreenEntity,
        ));
    }

    for (i, token) in tokens.iter().enumerate() {
        let pos = token_pos(i);
        commands.spawn((
            assets
                .tex
                .panel_sprite(theme::glass_raised(), Vec2::new(120.0, 38.0)),
            Transform::from_xyz(pos.x, pos.y, 6.0),
            AttnChip(i),
            ui::ScreenEntity,
        ));
        commands.spawn((
            assets
                .tex
                .outline_sprite(theme::line().with_alpha(0.4), Vec2::new(120.0, 38.0)),
            Transform::from_xyz(pos.x, pos.y, 6.4),
            ui::ScreenEntity,
        ));
        commands.spawn((
            Text2d::new(*token),
            TextFont {
                font: assets.source(),
                font_size: FontSize::Px(17.0),
                ..default()
            },
            TextColor(theme::text()),
            Anchor::CENTER,
            Transform::from_xyz(pos.x, pos.y, 8.0),
            AttnToken(i),
            ui::ScreenEntity,
        ));
    }

    let start = query.map(token_pos).unwrap_or(Vec2::ZERO);
    commands.spawn((
        Sprite::from_color(theme::amber(), Vec2::new(0.0, 2.0)),
        Transform::from_xyz(start.x, start.y, 7.0),
        AttnLink,
        ui::ScreenEntity,
    ));

    commands.spawn((
        Text2d::new(""),
        TextFont {
            font: assets.source(),
            font_size: FontSize::Px(13.0),
            ..default()
        },
        TextColor(theme::text_dim()),
        Anchor::CENTER,
        Transform::from_xyz(0.0, -178.0, 9.0),
        AttnStatus,
        ui::ScreenEntity,
    ));

    ui::text(
        commands,
        assets,
        t(lang, "CLICK THE WORD THAT THE HIGHLIGHTED ONE REFERS TO   |   R = RESET", "点出高亮词指的是哪个词   |   R = 重置"),
        Vec2::new(0.0, -326.0),
        11.0,
        theme::text_faint(),
        Anchor::CENTER,
    );
}

pub fn attention_input(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut active: ResMut<ActivePuzzle>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Puzzle::Attention(p) = &mut active.0 else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok((camera, cam_tf)) = cameras.single() else {
        return;
    };
    let Ok(world) = camera.viewport_to_world_2d(cam_tf, cursor) else {
        return;
    };

    for i in 0..14 {
        if (world - token_pos(i)).length() < 66.0 {
            p.choose(i);
            return;
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn sync_attention(
    active: Res<ActivePuzzle>,
    current: Res<CurrentLevel>,
    lang: Res<LangRes>,
    mut chips: Query<(&AttnChip, &mut Sprite)>,
    mut tokens: Query<(&AttnToken, &mut Text2d, &mut TextColor), Without<AttnStatus>>,
    mut link: Query<(&mut Transform, &mut Sprite), (With<AttnLink>, Without<AttnChip>)>,
    mut status: Query<&mut Text2d, (With<AttnStatus>, Without<AttnToken>)>,
) {
    let Puzzle::Attention(p) = &active.0 else {
        return;
    };
    let Some(def) = ooi_core::level(current.0) else {
        return;
    };
    let round = p.round();
    let list = attention_tokens(lang.0, def.id, round);
    let query = p.current().map(|r| r.query);
    let chosen = p.chosen();

    for (chip, mut sprite) in &mut chips {
        let color = if Some(chip.0) == query {
            Color::srgba(0.22, 0.15, 0.055, 1.0)
        } else if Some(chip.0) == chosen {
            Color::srgba(0.28, 0.075, 0.055, 1.0)
        } else {
            theme::glass_raised()
        };
        sprite.color = color;
    }

    for (token, mut text, mut color) in &mut tokens {
        if let Some(word) = list.get(token.0) {
            text.0 = (*word).to_string();
        }
        color.0 = if Some(token.0) == query {
            theme::amber()
        } else if Some(token.0) == chosen {
            theme::ember_lit()
        } else {
            theme::text()
        };
    }

    if let (Some(q), Some(c)) = (query, chosen) {
        let a = token_pos(q);
        let b = token_pos(c);
        let delta = b - a;
        let mid = (a + b) * 0.5;
        for (mut transform, mut sprite) in &mut link {
            transform.translation.x = mid.x;
            transform.translation.y = mid.y;
            transform.rotation = Quat::from_rotation_z(delta.y.atan2(delta.x));
            sprite.custom_size = Some(Vec2::new(delta.length().max(2.0), 2.0));
            sprite.color = theme::ember_lit().with_alpha(0.6);
        }
    } else {
        for (_, mut sprite) in &mut link {
            sprite.custom_size = Some(Vec2::ZERO);
        }
    }

    for mut text in &mut status {
        let total = p.config().rounds.len();
        text.0 = if p.is_solved() {
            t(lang.0, "SOLVED", "已解").to_string()
        } else {
            format!(
                "{} {} / {}",
                t(lang.0, "ROUND", "轮"),
                round + 1,
                total
            )
        };
    }
}

#[derive(Component)]
pub struct HintLine;

#[derive(Component)]
pub struct HintButtonText;

#[allow(clippy::type_complexity)]
pub fn sync_hint(
    lang: Res<LangRes>,
    hint: Res<HintState>,
    current: Res<CurrentLevel>,
    mut line: Query<&mut Text2d, (With<HintLine>, Without<HintButtonText>)>,
    mut button: Query<&mut Text2d, (With<HintButtonText>, Without<HintLine>)>,
) {
    let l = lang.0;
    let id = level(current.0).map(|d| d.id).unwrap_or("");
    let text = match hint.stage {
        0 => String::new(),
        1 | 2 => format!("{} {}/2  \u{00b7}  {}", t(l, "HINT", "提示"), hint.stage, level_hint(l, id, hint.stage)),
        _ => t(l, "SHOWN: THE WORKING SOLUTION", "已演示：一个可行解").to_string(),
    };
    for mut s in &mut line {
        s.0 = text.clone();
    }
    let label = match hint.stage {
        0 => t(l, "HINT", "提示"),
        1 => t(l, "ANOTHER HINT", "再给提示"),
        2 => t(l, "SHOW ME", "演示给我"),
        _ => t(l, "SHOWN", "已演示"),
    };
    for mut s in &mut button {
        s.0 = label.to_string();
    }
}

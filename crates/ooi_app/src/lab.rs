//! The Forge — an interactive, trainable neural network.
//!
//! The core simulation lives in `ooi_core::lab`; this module owns the layout,
//! input and the live instrumentation. Every edge, neuron, field cell and loss
//! sample below is derived from the real model, never from canned animation.

use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::window::PrimaryWindow;

use crate::save_io;
use crate::state::{LabState, LangRes, SaveRes};
use crate::theme;
use crate::ui::{self, Action, UiAssets};
use oo_kit::audio::{Cue, CueQueue};
use ooi_core::lab::DatasetKind;
use ooi_core::t;
use ooi_core::text::{dataset_name, lab_goal, lab_lesson, lab_mastered_line};

const LEFT_C: Vec2 = Vec2::new(-320.0, 32.0);
const RIGHT_C: Vec2 = Vec2::new(315.0, 32.0);
const PANEL: Vec2 = Vec2::new(540.0, 400.0);
const FIELD_C: Vec2 = Vec2::new(315.0, 78.0);
const FIELD_HALF: f32 = 140.0;
const FIELD_CELLS: usize = 28;
const FIELD_CELL: f32 = FIELD_HALF * 2.0 / FIELD_CELLS as f32;
const CURVE_SEGMENTS: usize = 140;

// ---------------------------------------------------------------- components

#[derive(Component)]
pub(crate) struct LabEdge {
    layer: usize,
    out: usize,
    input: usize,
    length: f32,
    sign: f32,
}

#[derive(Component)]
pub(crate) struct LabNeuronCore {
    layer: usize,
    index: usize,
    radius: f32,
}

#[derive(Component)]
pub(crate) struct LabNeuronGlow {
    layer: usize,
    index: usize,
    size: f32,
}

#[derive(Component)]
pub(crate) struct LabFieldCell {
    ix: usize,
    iy: usize,
}

#[derive(Component)]
pub(crate) struct LabProbeMarker;

#[derive(Component)]
pub(crate) struct LabCurveSegment(usize);

#[derive(Component)]
pub(crate) struct LabTabDot;

#[derive(Component)]
pub(crate) enum LabChipKind {
    Dataset(usize),
    Rate(usize),
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LabTextKind {
    Train,
    State,
    Epoch,
    Best,
    Shape,
    Goal,
    Trial,
    Mastery,
    Notice,
    Accuracy,
    Probe,
}

// ---------------------------------------------------------------- layout

fn layer_x(depth: usize, layer: usize) -> f32 {
    let columns = depth + 2;
    let left = -545.0;
    let right = -95.0;
    let t = layer as f32 / (columns - 1) as f32;
    left + (right - left) * t
}

fn hidden_y(index: usize, width: usize) -> f32 {
    if width <= 1 {
        return 88.0;
    }
    let top = 178.0;
    let bottom = -8.0;
    top + (bottom - top) * index as f32 / (width - 1) as f32
}

fn node_pos(layer: usize, index: usize, depth: usize, width: usize) -> Vec2 {
    let x = layer_x(depth, layer);
    let y = match layer {
        0 => {
            if index == 0 {
                132.0
            } else {
                62.0
            }
        }
        layer if layer == depth + 1 => 88.0,
        _ => hidden_y(index, width),
    };
    Vec2::new(x, y)
}

fn node_radius(width: usize) -> f32 {
    if width > 10 {
        4.6
    } else if width > 6 {
        6.2
    } else {
        8.2
    }
}

fn field_to_screen(x: f32, y: f32) -> Vec2 {
    FIELD_C
        + Vec2::new(
            (x.clamp(0.0, 1.0) - 0.5) * FIELD_HALF * 2.0,
            (y.clamp(0.0, 1.0) - 0.5) * FIELD_HALF * 2.0,
        )
}

fn screen_to_field(pos: Vec2) -> Option<(f32, f32)> {
    let rel = (pos - FIELD_C) / (FIELD_HALF * 2.0) + Vec2::splat(0.5);
    if (0.0..=1.0).contains(&rel.x) && (0.0..=1.0).contains(&rel.y) {
        Some((rel.x, rel.y))
    } else {
        None
    }
}

fn layer_name(lang: ooi_core::Lang, layer: usize, depth: usize) -> String {
    if layer == 0 {
        t(lang, "INPUT", "输入").to_string()
    } else if layer == depth + 1 {
        t(lang, "OUTPUT", "输出").to_string()
    } else if depth == 1 {
        t(lang, "HIDDEN", "隐藏").to_string()
    } else {
        format!("{} {layer}", t(lang, "HIDDEN", "隐藏"))
    }
}

// ---------------------------------------------------------------- spawn

#[allow(clippy::too_many_arguments)]
pub fn spawn_lab(
    mut commands: Commands,
    assets: Res<UiAssets>,
    lang: Res<LangRes>,
    save: Res<SaveRes>,
    lab: Res<LabState>,
) {
    let l = lang.0;
    let depth = lab.depth;
    let width = lab.width;
    let sizes = lab.training.net.sizes().to_vec();

    // Header.
    ui::action_button(
        &mut commands,
        &assets,
        Vec2::new(-558.0, 334.0),
        Vec2::new(116.0, 36.0),
        format!("\u{2190}  {}", t(l, "MATRIX", "基质")),
        13.0,
        Action::Hub,
        false,
    );
    ui::text(
        &mut commands,
        &assets,
        t(l, "THE FORGE", "塑造台"),
        Vec2::new(-480.0, 340.0),
        25.0,
        theme::text(),
        Anchor::CENTER_LEFT,
    );
    ui::text(
        &mut commands,
        &assets,
        t(l, "Build a mind. Watch it learn.", "造一个心智，看它学会。"),
        Vec2::new(-480.0, 314.0),
        13.0,
        theme::text_dim(),
        Anchor::CENTER_LEFT,
    );
    commands.spawn((
        Text2d::new(lab_goal(l, lab.kind)),
        TextFont {
            font: assets.source(),
            font_size: FontSize::Px(14.0),
            ..default()
        },
        TextColor(theme::amber().with_alpha(0.92)),
        Anchor::CENTER,
        Transform::from_xyz(64.0, 296.0, 20.0),
        LabTextKind::Goal,
        ui::ScreenEntity,
    ));

    // Dataset chooser.
    ui::caption(
        &mut commands,
        &assets,
        t(l, "DATASET", "数据集"),
        Vec2::new(-572.0, 263.0),
        theme::text_faint(),
        Anchor::CENTER_LEFT,
    );
    for (index, kind) in DatasetKind::ALL.iter().enumerate() {
        let x = -478.0 + index as f32 * 116.0;
        let mastered = save.0.lab.is_mastered(kind.id());
        let label = dataset_name(l, *kind).to_string();
        let entity = ui::action_button(
            &mut commands,
            &assets,
            Vec2::new(x, 263.0),
            Vec2::new(104.0, 30.0),
            label,
            12.5,
            Action::LabDataset(index),
            *kind == lab.kind,
        );
        commands.entity(entity).insert(LabChipKind::Dataset(index));
        if mastered {
            commands.spawn((
                Sprite {
                    image: assets.tex.dot.clone(),
                    color: theme::amber(),
                    custom_size: Some(Vec2::splat(4.0)),
                    ..default()
                },
                Transform::from_xyz(x + 42.0, 273.0, 24.0),
                LabTabDot,
                ui::ScreenEntity,
            ));
        }
    }
    commands.spawn((
        Text2d::new(""),
        TextFont {
            font: assets.source(),
            font_size: FontSize::Px(11.0),
            ..default()
        },
        TextColor(theme::text_faint()),
        Anchor::CENTER_RIGHT,
        Transform::from_xyz(570.0, 263.0, 20.0),
        LabTextKind::State,
        ui::ScreenEntity,
    ));

    // Constrained trial, displayed in the empty middle of the dataset row.
    commands.spawn((
        Text2d::new(""),
        TextFont {
            font: assets.source(),
            font_size: FontSize::Px(10.5),
            ..default()
        },
        TextColor(theme::text_faint()),
        Anchor::CENTER,
        Transform::from_xyz(140.0, 263.0, 20.0),
        LabTextKind::Trial,
        ui::ScreenEntity,
    ));

    // ------------------------------------------------------------ left panel
    ui::card(
        &mut commands,
        &assets,
        LEFT_C,
        PANEL,
        theme::cyan().with_alpha(0.65),
    );
    ui::caption(
        &mut commands,
        &assets,
        t(l, "MODEL", "模型"),
        Vec2::new(-570.0, 215.0),
        theme::text_faint(),
        Anchor::CENTER_LEFT,
    );
    ui::text(
        &mut commands,
        &assets,
        format!(
            "{} \u{00b7} {} {}",
            sizes
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join(" \u{2192} "),
            lab.training.net.params(),
            t(l, "PARAMS", "参数").to_lowercase()
        ),
        Vec2::new(-50.0, 215.0),
        12.0,
        theme::ice(),
        Anchor::CENTER_RIGHT,
    );

    // Layer captions.
    for layer in 0..sizes.len() {
        let x = layer_x(depth, layer);
        ui::caption(
            &mut commands,
            &assets,
            layer_name(l, layer, depth),
            Vec2::new(x, 196.0),
            if layer == sizes.len() - 1 {
                theme::amber().with_alpha(0.8)
            } else {
                theme::text_faint()
            },
            Anchor::CENTER,
        );
    }

    // Input markers.
    ui::text(
        &mut commands,
        &assets,
        "x",
        Vec2::new(-575.0, 132.0),
        12.0,
        theme::text_faint(),
        Anchor::CENTER_RIGHT,
    );
    ui::text(
        &mut commands,
        &assets,
        "y",
        Vec2::new(-575.0, 62.0),
        12.0,
        theme::text_faint(),
        Anchor::CENTER_RIGHT,
    );
    ui::text(
        &mut commands,
        &assets,
        "P",
        Vec2::new(-64.0, 88.0),
        12.0,
        theme::amber().with_alpha(0.8),
        Anchor::CENTER_LEFT,
    );

    // Edges, then neurons on top.
    for layer in 0..sizes.len() - 1 {
        for out in 0..sizes[layer + 1] {
            for input in 0..sizes[layer] {
                let a = node_pos(layer, input, depth, width);
                let b = node_pos(layer + 1, out, depth, width);
                let delta = b - a;
                let mid = (a + b) * 0.5;
                let sign = if out as u32 + input as u32 + layer as u32 % 2 == 0 {
                    1.0
                } else {
                    -1.0
                };
                commands.spawn((
                    Sprite::from_color(
                        theme::cyan().with_alpha(0.08),
                        Vec2::new(delta.length(), 1.0),
                    ),
                    Transform::from_xyz(mid.x, mid.y, 6.0)
                        .with_rotation(Quat::from_rotation_z(delta.y.atan2(delta.x))),
                    LabEdge {
                        layer,
                        out,
                        input,
                        length: delta.length(),
                        sign,
                    },
                    ui::ScreenEntity,
                ));
            }
        }
    }

    for (layer, count) in sizes.iter().enumerate() {
        let radius = if layer == 0 || layer == sizes.len() - 1 {
            7.0
        } else {
            node_radius(width)
        };
        for index in 0..*count {
            let pos = node_pos(layer, index, depth, width);
            commands.spawn((
                Sprite {
                    image: assets.tex.glow.clone(),
                    color: theme::cyan().with_alpha(0.10),
                    custom_size: Some(Vec2::splat(radius * 6.0)),
                    ..default()
                },
                Transform::from_xyz(pos.x, pos.y, 7.0),
                LabNeuronGlow {
                    layer,
                    index,
                    size: radius * 6.0,
                },
                ui::ScreenEntity,
            ));
            commands.spawn((
                Sprite {
                    image: assets.tex.ring.clone(),
                    color: theme::cyan().with_alpha(0.32),
                    custom_size: Some(Vec2::splat(radius * 2.3)),
                    ..default()
                },
                Transform::from_xyz(pos.x, pos.y, 8.0),
                ui::ScreenEntity,
            ));
            commands.spawn((
                Sprite {
                    image: assets.tex.dot.clone(),
                    color: theme::cyan(),
                    custom_size: Some(Vec2::splat(radius * 1.65)),
                    ..default()
                },
                Transform::from_xyz(pos.x, pos.y, 8.2),
                LabNeuronCore {
                    layer,
                    index,
                    radius,
                },
                ui::ScreenEntity,
            ));
        }
    }

    // Architecture controls.
    ui::caption(
        &mut commands,
        &assets,
        t(l, "HIDDEN LAYERS", "隐藏层"),
        Vec2::new(-568.0, -88.0),
        theme::text_faint(),
        Anchor::CENTER_LEFT,
    );
    ui::text(
        &mut commands,
        &assets,
        format!("{depth}"),
        Vec2::new(-300.0, -88.0),
        18.0,
        theme::text(),
        Anchor::CENTER,
    );
    ui::action_button(
        &mut commands,
        &assets,
        Vec2::new(-252.0, -88.0),
        Vec2::new(30.0, 26.0),
        "-",
        16.0,
        Action::LabDepth(-1),
        false,
    );
    ui::action_button(
        &mut commands,
        &assets,
        Vec2::new(-216.0, -88.0),
        Vec2::new(30.0, 26.0),
        "+",
        16.0,
        Action::LabDepth(1),
        false,
    );

    ui::caption(
        &mut commands,
        &assets,
        t(l, "NEURONS", "神经元"),
        Vec2::new(-568.0, -128.0),
        theme::text_faint(),
        Anchor::CENTER_LEFT,
    );
    ui::text(
        &mut commands,
        &assets,
        format!("{width}"),
        Vec2::new(-300.0, -128.0),
        18.0,
        theme::text(),
        Anchor::CENTER,
    );
    ui::action_button(
        &mut commands,
        &assets,
        Vec2::new(-252.0, -128.0),
        Vec2::new(30.0, 26.0),
        "-",
        16.0,
        Action::LabWidth(-1),
        false,
    );
    ui::action_button(
        &mut commands,
        &assets,
        Vec2::new(-216.0, -128.0),
        Vec2::new(30.0, 26.0),
        "+",
        16.0,
        Action::LabWidth(1),
        false,
    );
    commands.spawn((
        Text2d::new(format!(
            "{}  {} \u{00b7} {}",
            lab.training.net.params(),
            t(l, "PARAMS", "参数").to_lowercase(),
            t(l, "BUDGET", "预算")
        )),
        TextFont {
            font: assets.source(),
            font_size: FontSize::Px(10.5),
            ..default()
        },
        TextColor(theme::text_faint()),
        Anchor::CENTER_LEFT,
        Transform::from_xyz(-568.0, -157.0, 20.0),
        LabTextKind::Shape,
        ui::ScreenEntity,
    ));

    // ------------------------------------------------------------ right panel
    ui::card(
        &mut commands,
        &assets,
        RIGHT_C,
        PANEL,
        theme::amber().with_alpha(0.6),
    );
    ui::caption(
        &mut commands,
        &assets,
        t(l, "DECISION FIELD", "决策场"),
        Vec2::new(58.0, 215.0),
        theme::text_faint(),
        Anchor::CENTER_LEFT,
    );
    ui::text(
        &mut commands,
        &assets,
        format!(
            "{} {:.1}%",
            t(l, "TARGET", "目标"),
            lab.kind.target_accuracy() * 100.0
        ),
        Vec2::new(570.0, 215.0),
        11.0,
        theme::amber().with_alpha(0.85),
        Anchor::CENTER_RIGHT,
    );

    // Field background + cells.
    commands.spawn((
        Sprite::from_color(theme::bg_deep(), Vec2::splat(FIELD_HALF * 2.0 + 8.0)),
        Transform::from_xyz(FIELD_C.x, FIELD_C.y, 2.0),
        ui::ScreenEntity,
    ));
    for iy in 0..FIELD_CELLS {
        for ix in 0..FIELD_CELLS {
            let x = FIELD_C.x - FIELD_HALF + (ix as f32 + 0.5) * FIELD_CELL;
            let y = FIELD_C.y - FIELD_HALF + (iy as f32 + 0.5) * FIELD_CELL;
            commands.spawn((
                Sprite::from_color(theme::cyan().with_alpha(0.2), Vec2::splat(FIELD_CELL + 0.4)),
                Transform::from_xyz(x, y, 3.0),
                LabFieldCell { ix, iy },
                ui::ScreenEntity,
            ));
        }
    }
    // Sparse grid and border.
    for step in 1..4 {
        let t = step as f32 / 4.0;
        let x = FIELD_C.x - FIELD_HALF + t * FIELD_HALF * 2.0;
        commands.spawn((
            Sprite::from_color(
                theme::text().with_alpha(0.06),
                Vec2::new(1.0, FIELD_HALF * 2.0),
            ),
            Transform::from_xyz(x, FIELD_C.y, 4.0),
            ui::ScreenEntity,
        ));
        let y = FIELD_C.y - FIELD_HALF + t * FIELD_HALF * 2.0;
        commands.spawn((
            Sprite::from_color(
                theme::text().with_alpha(0.06),
                Vec2::new(FIELD_HALF * 2.0, 1.0),
            ),
            Transform::from_xyz(FIELD_C.x, y, 4.0),
            ui::ScreenEntity,
        ));
    }
    commands.spawn((
        assets.tex.outline_sprite(
            theme::line().with_alpha(0.55),
            Vec2::splat(FIELD_HALF * 2.0 + 8.0),
        ),
        Transform::from_xyz(FIELD_C.x, FIELD_C.y, 4.5),
        ui::ScreenEntity,
    ));
    // Four corner ticks.
    for (sx, sy) in [(-1.0, 1.0), (1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)] {
        commands.spawn((
            Sprite::from_color(theme::cyan().with_alpha(0.35), Vec2::new(1.0, 10.0)),
            Transform::from_xyz(
                FIELD_C.x + sx * (FIELD_HALF + 4.0),
                FIELD_C.y + sy * (FIELD_HALF - 2.0),
                5.0,
            ),
            ui::ScreenEntity,
        ));
    }

    let data = lab.kind.dataset();
    for sample in &data.samples {
        let pos = field_to_screen(sample.x, sample.y);
        let color = if sample.label {
            theme::cyan()
        } else {
            theme::amber()
        };
        commands.spawn((
            Sprite {
                image: assets.tex.glow.clone(),
                color: color.with_alpha(0.16),
                custom_size: Some(Vec2::splat(15.0)),
                ..default()
            },
            Transform::from_xyz(pos.x, pos.y, 5.2),
            ui::ScreenEntity,
        ));
        commands.spawn((
            Sprite {
                image: assets.tex.dot.clone(),
                color,
                custom_size: Some(Vec2::splat(5.2)),
                ..default()
            },
            Transform::from_xyz(pos.x, pos.y, 5.5),
            ui::ScreenEntity,
        ));
    }

    // Probe marker.
    let probe = field_to_screen(lab.probe.0, lab.probe.1);
    commands.spawn((
        Sprite {
            image: assets.tex.ring.clone(),
            color: theme::text().with_alpha(0.6),
            custom_size: Some(Vec2::splat(25.0)),
            ..default()
        },
        Transform::from_xyz(probe.x, probe.y, 8.0),
        LabProbeMarker,
        ui::ScreenEntity,
    ));

    commands.spawn((
        Text2d::new(""),
        TextFont {
            font: assets.source(),
            font_size: FontSize::Px(12.0),
            ..default()
        },
        TextColor(theme::text_dim()),
        Anchor::CENTER_LEFT,
        Transform::from_xyz(58.0, -84.0, 20.0),
        LabTextKind::Accuracy,
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
        Anchor::CENTER_RIGHT,
        Transform::from_xyz(570.0, -84.0, 20.0),
        LabTextKind::Epoch,
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
        Anchor::CENTER_LEFT,
        Transform::from_xyz(58.0, -106.0, 20.0),
        LabTextKind::Best,
        ui::ScreenEntity,
    ));
    commands.spawn((
        Text2d::new(""),
        TextFont {
            font: assets.source(),
            font_size: FontSize::Px(10.0),
            ..default()
        },
        TextColor(theme::text_faint()),
        Anchor::CENTER_LEFT,
        Transform::from_xyz(58.0, -156.0, 20.0),
        LabTextKind::Probe,
        ui::ScreenEntity,
    ));
    commands.spawn((
        Text2d::new(""),
        TextFont {
            font: assets.source(),
            font_size: FontSize::Px(10.0),
            ..default()
        },
        TextColor(theme::text_faint()),
        Anchor::CENTER_LEFT,
        Transform::from_xyz(58.0, -127.0, 20.0),
        LabTextKind::Mastery,
        ui::ScreenEntity,
    ));

    // Loss curve segments, hidden until there is data.
    let curve_left = 48.0;
    let curve_width = 536.0;
    for index in 0..CURVE_SEGMENTS {
        commands.spawn((
            Sprite::from_color(theme::cyan(), Vec2::ZERO),
            Transform::from_xyz(
                curve_left + index as f32 * curve_width / CURVE_SEGMENTS as f32,
                -244.0,
                6.0,
            ),
            LabCurveSegment(index),
            ui::ScreenEntity,
        ));
    }
    commands.spawn((
        Sprite::from_color(theme::line().with_alpha(0.4), Vec2::new(curve_width, 1.0)),
        Transform::from_xyz(curve_left + curve_width / 2.0, -258.0, 5.0),
        ui::ScreenEntity,
    ));
    ui::text(
        &mut commands,
        &assets,
        format!(
            "{}  \u{00b7}  {}",
            t(l, "LOSS", "损失"),
            t(l, "LIVE", "实时")
        ),
        Vec2::new(curve_left, -207.0),
        9.0,
        theme::text_faint(),
        Anchor::CENTER_LEFT,
    );

    // ------------------------------------------------------------ bottom bar
    let train = ui::action_button(
        &mut commands,
        &assets,
        Vec2::new(-512.0, -240.0),
        Vec2::new(150.0, 48.0),
        "",
        20.0,
        Action::LabToggle,
        true,
    );
    let _ = train;
    commands.spawn((
        Text2d::new(if lab.running {
            t(l, "PAUSE", "暂停")
        } else {
            t(l, "TRAIN", "训练")
        }),
        TextFont {
            font: assets.source(),
            font_size: FontSize::Px(18.0),
            ..default()
        },
        TextColor(theme::text()),
        Anchor::CENTER,
        Transform::from_xyz(-512.0, -240.0, 21.0),
        LabTextKind::Train,
        ui::ScreenEntity,
    ));
    ui::action_button(
        &mut commands,
        &assets,
        Vec2::new(-404.0, -240.0),
        Vec2::new(78.0, 44.0),
        t(l, "STEP", "单步"),
        13.0,
        Action::LabStep,
        false,
    );
    ui::action_button(
        &mut commands,
        &assets,
        Vec2::new(-317.0, -240.0),
        Vec2::new(78.0, 44.0),
        t(l, "RESET", "重置"),
        13.0,
        Action::LabReset,
        false,
    );
    ui::caption(
        &mut commands,
        &assets,
        t(l, "LEARNING RATE", "学习率").to_string(),
        Vec2::new(-212.0, -220.0),
        theme::text_faint(),
        Anchor::CENTER_LEFT,
    );
    let rates = ["0.01", "0.03", "0.08"];
    for (index, label) in rates.iter().enumerate() {
        let entity = ui::action_button(
            &mut commands,
            &assets,
            Vec2::new(-160.0 + index as f32 * 74.0, -249.0),
            Vec2::new(66.0, 30.0),
            *label,
            12.0,
            Action::LabRate(index),
            (lab.learning_rate() - [0.01, 0.03, 0.08][index]).abs() < 1e-4,
        );
        commands.entity(entity).insert(LabChipKind::Rate(index));
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
        Transform::from_xyz(330.0, -35.0, 22.0),
        LabTextKind::Notice,
        ui::ScreenEntity,
    ));

    ui::text(
        &mut commands,
        &assets,
        t(l, "MOVE OVER THE FIELD TO PROBE THE NETWORK   |   TRAIN, THEN BREAK IT", "把光标移到决策场上探测网络   |   训练它，然后拆开看"),
        Vec2::new(0.0, -306.0),
        11.5,
        theme::text_faint(),
        Anchor::CENTER,
    );
    ui::text(
        &mut commands,
        &assets,
        lab_lesson(l, lab.kind),
        Vec2::new(0.0, -283.0),
        12.0,
        theme::text_dim(),
        Anchor::CENTER,
    );
    ui::action_button(
        &mut commands,
        &assets,
        Vec2::new(540.0, -320.0),
        Vec2::new(108.0, 28.0),
        t(l, "SETTINGS", "设置"),
        10.0,
        Action::Settings,
        false,
    );
}

// ---------------------------------------------------------------- input

pub fn lab_fields_input(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut lab: ResMut<LabState>,
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
    if let Some((x, y)) = screen_to_field(world) {
        lab.probe = (x, y);
    } else if mouse.just_pressed(MouseButton::Left) {
        // Clicking outside the field clears the focus back to the centre.
        lab.probe = (0.5, 0.5);
    }
}

// ---------------------------------------------------------------- simulation

#[allow(clippy::too_many_arguments)]
pub fn lab_tick(
    time: Res<Time>,
    mut lab: ResMut<LabState>,
    mut save: ResMut<SaveRes>,
    mut cues: ResMut<CueQueue>,
) {
    let dt = time.delta_secs();
    if lab.notice_timer > 0.0 {
        lab.notice_timer = (lab.notice_timer - dt).max(0.0);
    }
    if lab.trial_notice > 0.0 {
        lab.trial_notice = (lab.trial_notice - dt).max(0.0);
    }

    if lab.running {
        // 110 epochs per second feels immediate without becoming a blur.
        lab.epoch_accum += dt * 110.0;
        let steps = (lab.epoch_accum.floor() as u32).min(4);
        if steps > 0 {
            lab.epoch_accum -= steps as f32;
            for _ in 0..steps {
                lab.training.step();
            }
            lab.frame = lab.frame.wrapping_add(1);
        }
    }

    if !lab.running && lab.frame == 0 {
        lab.frame = 1;
    }

    // Metrics update only when something actually changed.
    if lab.frame != lab.metrics_frame {
        lab.metrics_frame = lab.frame;
        lab.last_accuracy = lab.training.accuracy();
        lab.last_loss = lab.training.loss();
    }

    lab.best_accuracy = lab.best_accuracy.max(lab.last_accuracy);

    let target = lab.kind.target_accuracy();
    let mut play_success = false;
    if lab.last_accuracy >= target && !lab.is_mastered {
        lab.is_mastered = true;
        lab.running = false;
        lab.notice_timer = 4.0;
        save.0.lab.record(lab.kind.id(), lab.last_accuracy);
        save.0.meta.record_forge_mastered(lab.kind.id());
        save_io::save(&save.0);
        play_success = true;
    }

    // The trial shares the target accuracy but adds a shape budget. Completing
    // it is a separate, one-time reward; changing the shape later never
    // removes the badge.
    let spec = lab.kind.trial();
    let params = lab.training.net.params();
    if !lab.trial_done && spec.is_met(lab.depth, lab.width, params, lab.last_accuracy) {
        lab.trial_done = true;
        if save.0.lab.record_trial(lab.kind.id()) {
            save.0.meta.record_forge_trial(lab.kind.id());
            lab.trial_notice = 4.0;
            save_io::save(&save.0);
            play_success = true;
        }
    }

    // Both events can land on the same frame; play the confirmation once.
    if play_success {
        cues.play(Cue::Success);
    }

    lab.persist_timer += dt;
    if lab.persist_timer > 1.25 {
        lab.persist_timer = 0.0;
        let previous_best = save.0.lab.best(lab.kind.id());
        save.0.lab.record(lab.kind.id(), lab.last_accuracy);
        if lab.last_accuracy > previous_best {
            save_io::save(&save.0);
        }
    }
}

// ---------------------------------------------------------------- sync

#[allow(clippy::type_complexity)]
pub fn sync_lab_network(
    mut lab: ResMut<LabState>,
    mut edges: Query<(&LabEdge, &mut Sprite), (Without<LabNeuronCore>, Without<LabNeuronGlow>)>,
    mut cores: Query<
        (&LabNeuronCore, &mut Sprite, &mut Transform),
        (Without<LabEdge>, Without<LabNeuronGlow>),
    >,
    mut glows: Query<
        (&LabNeuronGlow, &mut Sprite, &mut Transform),
        (Without<LabEdge>, Without<LabNeuronCore>),
    >,
) {
    let (px, py) = lab.probe;
    let activations = lab.training.net.forward(px, py);
    let max_weight = lab.training.net.max_abs_weight();

    for (edge, mut sprite) in &mut edges {
        let weight = lab
            .training
            .net
            .edge_weight(edge.layer, edge.out, edge.input);
        let norm = (weight.abs() / max_weight).clamp(0.0, 1.0);
        let color = if weight >= 0.0 {
            theme::cyan()
        } else {
            theme::ember_lit()
        };
        sprite.color = color.with_alpha(0.07 + 0.55 * norm);
        sprite.custom_size = Some(Vec2::new(edge.length, 0.55 + 2.2 * norm));
        let _ = edge.sign;
    }

    for (core, mut sprite, mut transform) in &mut cores {
        let a = activation_at(&activations, core.layer, core.index);
        let intensity = if core.layer == 0 {
            0.35 + a * 0.65
        } else {
            a.abs().clamp(0.0, 1.0)
        };
        let color = if core.layer == activations.len() - 1 {
            if a >= 0.5 {
                theme::cyan()
            } else {
                theme::amber()
            }
        } else if a < 0.0 {
            theme::ember_lit()
        } else {
            theme::cyan()
        };
        sprite.color = color.with_alpha(0.45 + 0.55 * intensity);
        transform.scale = Vec3::splat(0.82 + intensity * 0.46);
        let _ = core.radius;
    }

    for (glow, mut sprite, mut transform) in &mut glows {
        let a = activation_at(&activations, glow.layer, glow.index);
        let intensity = if glow.layer == 0 { a } else { a.abs() };
        let color = if glow.layer == activations.len() - 1 {
            if a >= 0.5 {
                theme::cyan()
            } else {
                theme::amber()
            }
        } else if a < 0.0 {
            theme::ember_lit()
        } else {
            theme::cyan()
        };
        sprite.color = color.with_alpha(0.03 + intensity * 0.20);
        sprite.custom_size = Some(Vec2::splat(glow.size * (0.72 + intensity * 0.5)));
        transform.scale = Vec3::ONE;
    }
}

fn activation_at(activations: &[Vec<f32>], layer: usize, index: usize) -> f32 {
    activations
        .get(layer)
        .and_then(|values| values.get(index))
        .copied()
        .unwrap_or(0.0)
}

pub fn sync_lab_field(
    mut lab: ResMut<LabState>,
    mut cells: Query<(&LabFieldCell, &mut Sprite)>,
    mut probe_marker: Query<&mut Transform, With<LabProbeMarker>>,
) {
    if lab.frame.is_multiple_of(3) || lab.dirty {
        lab.dirty = false;
        for (cell, mut sprite) in &mut cells {
            let x = (cell.ix as f32 + 0.5) / FIELD_CELLS as f32;
            let y = (cell.iy as f32 + 0.5) / FIELD_CELLS as f32;
            let p = lab.training.net.predict(x, y);
            let base = if p >= 0.5 {
                theme::cyan()
            } else {
                theme::amber()
            };
            let confidence = ((p - 0.5).abs() * 2.0).powf(0.6);
            sprite.color = base.with_alpha(0.025 + 0.30 * confidence);
        }
    }

    let probe_pos = field_to_screen(lab.probe.0, lab.probe.1);
    for mut transform in &mut probe_marker {
        transform.translation.x = probe_pos.x;
        transform.translation.y = probe_pos.y;
    }
}

#[allow(clippy::type_complexity)]
pub fn sync_lab_hud(
    mut lab: ResMut<LabState>,
    lang: Res<LangRes>,
    mut texts: Query<(&LabTextKind, &mut Text2d, &mut TextColor)>,
    mut chips: Query<(&LabChipKind, &mut Sprite, &mut ui::ButtonUi), Without<LabCurveSegment>>,
    mut curve: Query<(&LabCurveSegment, &mut Sprite, &mut Transform), Without<LabChipKind>>,
) {
    let l = lang.0;
    let (px, py) = lab.probe;
    let p = lab.training.net.predict(px, py);
    let target = lab.kind.target_accuracy();

    for (kind, mut text, mut color) in &mut texts {
        match kind {
            LabTextKind::State => {
                text.0 = if lab.running {
                    t(l, "LIVE", "实时").to_string()
                } else {
                    t(l, "SANDBOX", "沙盒").to_string()
                };
                color.0 = if lab.running {
                    theme::mint()
                } else {
                    theme::text_faint()
                };
            }
            LabTextKind::Train => {
                text.0 = if lab.running {
                    t(l, "PAUSE", "暂停").to_string()
                } else {
                    t(l, "TRAIN", "训练").to_string()
                };
            }
            LabTextKind::Epoch => {
                text.0 = format!("{} {}", t(l, "EPOCH", "训练轮次"), lab.training.epochs);
            }
            LabTextKind::Best => {
                text.0 = format!(
                    "{} {:.1}%   {} {:.3}",
                    t(l, "BEST", "最佳"),
                    lab.best_accuracy * 100.0,
                    t(l, "LOSS", "损失"),
                    lab.last_loss
                );
            }
            LabTextKind::Shape => {
                text.0 = format!(
                    "{}   \u{00b7}   {:.0}% {}",
                    lab.training
                        .net
                        .sizes()
                        .iter()
                        .map(|n| n.to_string())
                        .collect::<Vec<_>>()
                        .join(" \u{2192} "),
                    target * 100.0,
                    t(l, "TARGET", "目标")
                );
            }
            LabTextKind::Goal => {
                text.0 = lab_goal(l, lab.kind).to_string();
                color.0 = theme::amber().with_alpha(0.92);
            }
            LabTextKind::Trial => {
                let spec = lab.kind.trial();
                let shape_ok = lab.trial_shape_ok();
                if lab.trial_done {
                    text.0 = t(l, "TRIAL COMPLETE", "试炼完成").to_string();
                    color.0 = theme::mint();
                } else if shape_ok {
                    text.0 = format!(
                        "{}  \u{00b7}  {}  \u{00b7}  {:.0}%",
                        t(l, "TRIAL", "试炼"),
                        t(l, "SHAPE OK", "结构合格"),
                        spec.target_accuracy * 100.0
                    );
                    color.0 = theme::amber().with_alpha(0.9);
                } else {
                    text.0 = format!(
                        "{}  \u{00b7}  {} {}  \u{00b7}  {} {} {}  \u{00b7}  {} {} {}  \u{00b7}  {:.0}%",
                        t(l, "TRIAL", "试炼"),
                        spec.max_depth,
                        t(l, "LAYER", "层"),
                        t(l, "MAX", "上限"),
                        spec.max_width,
                        t(l, "NEURONS", "神经元"),
                        t(l, "MAX", "上限"),
                        spec.max_params,
                        t(l, "PARAMS", "参数"),
                        spec.target_accuracy * 100.0
                    );
                    color.0 = theme::text_dim();
                }
            }
            LabTextKind::Mastery => {
                if lab.is_mastered && lab.last_accuracy >= target {
                    text.0 = format!(
                        "{}  \u{00b7}  {}",
                        t(l, "MASTERED", "已掌握"),
                        lab_lesson(l, lab.kind)
                    );
                    color.0 = theme::amber();
                } else if lab.is_mastered && lab.best_accuracy >= target {
                    // The dataset badge is historical. Do not claim the current
                    // network is mastered while it is still at 45%.
                    text.0 = format!(
                        "{}  \u{00b7}  {} {:.1}%  \u{00b7}  {}",
                        t(l, "MASTERED", "已掌握"),
                        t(l, "BEST", "最佳"),
                        lab.best_accuracy * 100.0,
                        lab_lesson(l, lab.kind)
                    );
                    color.0 = theme::amber().with_alpha(0.75);
                } else {
                    // The trial chip carries the active parameter budget. The
                    // old sandbox budget here contradicted it (25 vs 13).
                    text.0 = format!("{} {:.0}%", t(l, "TARGET", "目标"), target * 100.0);
                    color.0 = theme::text_faint();
                }
            }
            LabTextKind::Notice => {
                if lab.trial_notice > 0.0 {
                    text.0 = t(l, "TRIAL COMPLETE", "试炼完成").to_string();
                    color.0 = theme::mint().with_alpha((lab.trial_notice / 0.8).min(1.0));
                } else if lab.notice_timer > 0.0 {
                    text.0 = lab_mastered_line(l, lab.kind).to_string();
                    color.0 = theme::amber().with_alpha((lab.notice_timer / 0.8).min(1.0));
                } else {
                    text.0.clear();
                }
            }
            LabTextKind::Accuracy => {
                text.0 = format!(
                    "{}  {:.1}%",
                    t(l, "ACCURACY", "准确率"),
                    lab.last_accuracy * 100.0
                );
                color.0 = if lab.last_accuracy >= target {
                    theme::mint()
                } else {
                    theme::text()
                };
            }
            LabTextKind::Probe => {
                let class = if p >= 0.5 {
                    format!("P(y=1) {:.0}%", p * 100.0)
                } else {
                    format!("P(y=0) {:.0}%", (1.0 - p) * 100.0)
                };
                text.0 = format!(
                    "{}  {:.2}, {:.2}   {}",
                    t(l, "PROBE", "探测"),
                    lab.probe.0,
                    lab.probe.1,
                    class
                );
                color.0 = if p >= 0.5 {
                    theme::cyan_dim()
                } else {
                    theme::text_faint()
                };
            }
        }
    }

    for (chip, mut sprite, mut button) in &mut chips {
        let active = match chip {
            LabChipKind::Dataset(index) => DatasetKind::ALL.get(*index).copied() == Some(lab.kind),
            LabChipKind::Rate(index) => *index == lab.rate_index,
        };
        let base = if active {
            theme::action()
        } else {
            theme::glass_raised()
        };
        sprite.color = base;
        button.base = base;
    }

    // Loss sparkline. It uses the real training history, sampled uniformly
    // into the available segment count.
    let history = &lab.training.history;
    let take = history.len().min(CURVE_SEGMENTS + 1);
    if take < 2 {
        for (_, mut sprite, _) in &mut curve {
            sprite.custom_size = Some(Vec2::ZERO);
        }
        return;
    }
    let start = history.len() - take;
    let max_loss = history[start..]
        .iter()
        .copied()
        .fold(0.7_f32, f32::max)
        .min(1.4);
    let left = 48.0;
    let width = 536.0;
    for (segment, mut sprite, mut transform) in &mut curve {
        if segment.0 + 1 >= take {
            sprite.custom_size = Some(Vec2::ZERO);
            continue;
        }
        let a = history[start + segment.0];
        let b = history[start + segment.0 + 1];
        let x1 = left + segment.0 as f32 * width / (take - 1) as f32;
        let x2 = left + (segment.0 + 1) as f32 * width / (take - 1) as f32;
        let y1 = -258.0 + (a / max_loss).clamp(0.0, 1.0) * 34.0;
        let y2 = -258.0 + (b / max_loss).clamp(0.0, 1.0) * 34.0;
        let mid = Vec2::new((x1 + x2) * 0.5, (y1 + y2) * 0.5);
        transform.translation.x = mid.x;
        transform.translation.y = mid.y;
        transform.rotation = Quat::from_rotation_z((y2 - y1).atan2(x2 - x1));
        sprite.custom_size = Some(Vec2::new(
            ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt(),
            1.4,
        ));
        sprite.color = if lab.last_accuracy >= target {
            theme::mint().with_alpha(0.9)
        } else {
            theme::cyan().with_alpha(0.75)
        };
    }
}

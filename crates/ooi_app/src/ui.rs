//! 自绘 2D UI：薄薄一层，转发到 `oo_kit::ui::Ui`。
//!
//! 保留旧的自由函数签名，关卡与界面代码几乎不用改；
//! 按钮、悬停、点击都由 kit 的 `UiButton<Action>` 负责。

use bevy::prelude::*;
use bevy::sprite::Anchor;
use oo_kit::ui::{ButtonSpec, Tone, Ui};

pub use oo_kit::ui::{ScreenEntity, UiAssets};

use crate::theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    NewGame,
    Continue,
    Play(u16),
    Hub,
    Next,
    ToggleLang,
    Option(usize),
    Run,
    Hint,
    Title,
    Settings,
    CodexOpen,
    Records,
    RecordsClose,
    ToggleToasts,
    CodexSelect(u16),
    CodexPlay(u16),
    CloseSettings,
    VolumeDown,
    VolumeUp,
    ToggleFullscreen,
    ToggleMotion,
    Quit,
    LabOpen,
    LabTrial(usize),
    LabDataset(usize),
    LabDepth(i32),
    LabWidth(i32),
    LabRate(usize),
    LabToggle,
    LabStep,
    LabReset,
    ResetLevel,
    ToggleGate(usize),
    /// Bench: nudge dial `.0` by `.1` steps.
    BenchDial(usize, i32),
    BenchToggle(usize),
    BenchTarget(usize),
    /// Finale: what to do with it (0 voyage, 1 greenhouse, 2 let it choose).
    FinaleChoice(usize),
}

pub type Actions = oo_kit::ui::Actions<Action>;
pub type ButtonUi = oo_kit::ui::UiButton<Action>;

/// Invisible / custom-drawn hit area for sprites that act as buttons.
pub fn hit(half: Vec2, action: Action, base: Color, hover: Color) -> ButtonUi {
    ButtonUi { half, action, base, hover, enabled: true }
}

fn ui<'a, 'w, 's>(commands: &'a mut Commands<'w, 's>, assets: &'a UiAssets) -> Ui<'a, 'w, 's> {
    Ui::new(commands, assets, theme::pal())
}

pub fn text(
    commands: &mut Commands,
    assets: &UiAssets,
    s: impl Into<String>,
    pos: Vec2,
    size: f32,
    color: Color,
    anchor: Anchor,
) -> Entity {
    ui(commands, assets).text(s, pos, size, color, anchor)
}

/// `spacing` 只建议用于全大写拉丁小标题；中文正文保持 0。
#[allow(clippy::too_many_arguments)]
pub fn text_styled(
    commands: &mut Commands,
    assets: &UiAssets,
    s: impl Into<String>,
    pos: Vec2,
    size: f32,
    color: Color,
    anchor: Anchor,
    spacing: f32,
) -> Entity {
    ui(commands, assets).text_spaced(s, pos, size, color, anchor, spacing)
}

/// 自动换行的文本块。
#[allow(clippy::too_many_arguments)]
pub fn wrapped_text(
    commands: &mut Commands,
    assets: &UiAssets,
    s: impl Into<String>,
    pos: Vec2,
    width: f32,
    size: f32,
    color: Color,
    anchor: Anchor,
) -> Entity {
    ui(commands, assets).wrapped(s, pos, width, size, color, anchor)
}

/// 玻璃面板 + 左上角强调线。返回填充实体。
pub fn card(commands: &mut Commands, assets: &UiAssets, pos: Vec2, size: Vec2, accent: Color) -> Entity {
    ui(commands, assets).card(pos, size, accent)
}

/// 柔光块，不参与命中。
pub fn halo(commands: &mut Commands, assets: &UiAssets, pos: Vec2, size: f32, color: Color, z: f32) -> Entity {
    ui(commands, assets).glow(pos, size, color, z)
}

/// 主行动按钮。
pub fn primary_button(
    commands: &mut Commands,
    assets: &UiAssets,
    pos: Vec2,
    size: Vec2,
    label: impl Into<String>,
    action: Action,
) -> Entity {
    ui(commands, assets).button(ButtonSpec::new(pos, size, label, action).primary().font(21.0))
}

/// Button with an explicit type scale.
#[allow(clippy::too_many_arguments)]
pub fn action_button(
    commands: &mut Commands,
    assets: &UiAssets,
    pos: Vec2,
    size: Vec2,
    label: impl Into<String>,
    font_size: f32,
    action: Action,
    primary: bool,
) -> Entity {
    let tone = if primary { Tone::Primary } else { Tone::Secondary };
    ui(commands, assets).button(ButtonSpec::new(pos, size, label, action).tone(tone).font(font_size))
}

/// Small uppercase label with generous tracking.
pub fn caption(
    commands: &mut Commands,
    assets: &UiAssets,
    s: impl Into<String>,
    pos: Vec2,
    color: Color,
    anchor: Anchor,
) -> Entity {
    ui(commands, assets).caption(s, pos, color, anchor)
}

/// Label/value instrument chip.
pub fn resource_pill(
    commands: &mut Commands,
    assets: &UiAssets,
    pos: Vec2,
    width: f32,
    label: &str,
    value: impl Into<String>,
    accent: Color,
) {
    ui(commands, assets).pill(pos, width, label, value, accent);
}

/// Thin progress bar with a soft leading edge.
pub fn progress_bar(commands: &mut Commands, assets: &UiAssets, pos: Vec2, size: Vec2, value: f32, accent: Color) {
    ui(commands, assets).progress(pos, size, value, accent);
}

/// Three diamond stars.
pub fn stars(commands: &mut Commands, assets: &UiAssets, pos: Vec2, filled: u8, size: f32) {
    ui(commands, assets).stars(pos, filled, size);
}

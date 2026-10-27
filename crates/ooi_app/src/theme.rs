//! 视觉常量：共享的 `Palette::intelligence()`，外加本作独有的几种色。
//!
//! 配色来自 GDD「生物发光神经美学」：深空靛蓝为底，神经青是生命，
//! 琥珀是理解，紫罗兰只用来做极少量的深度暗示。

use bevy::prelude::*;
use oo_kit::theme::Palette;

pub fn pal() -> Palette {
    Palette::intelligence()
}

pub fn bg() -> Color {
    pal().bg
}

pub fn bg_deep() -> Color {
    pal().bg_deep
}

pub fn axis() -> Color {
    Color::srgb(0.22, 0.32, 0.46)
}

pub fn axis_label() -> Color {
    Color::srgb(0.42, 0.55, 0.70)
}

/// 神经青：阈值线、萤火。
pub fn cyan() -> Color {
    pal().primary
}

/// HDR 神经青。只给需要 bloom 的核心与状态点。
pub fn cyan_hot() -> Color {
    pal().primary_hot
}

pub fn cyan_dim() -> Color {
    Color::srgb(0.15, 0.23, 0.31)
}

/// 琥珀：错误与奖励。
pub fn amber() -> Color {
    pal().secondary
}

pub fn ember_dim() -> Color {
    Color::srgb(0.42, 0.24, 0.12)
}

/// 被误点亮的余烬：偏红，配合形状区分。
pub fn ember_lit() -> Color {
    pal().bad
}

pub fn violet() -> Color {
    pal().tertiary
}

pub fn mint() -> Color {
    pal().good
}

/// Ice-blue for secondary data and axis labels.
pub fn ice() -> Color {
    Color::srgb(0.63, 0.82, 0.96)
}

pub fn text() -> Color {
    pal().text
}

pub fn text_dim() -> Color {
    pal().text_dim
}

pub fn text_faint() -> Color {
    pal().text_faint
}

pub fn panel() -> Color {
    Color::srgb(0.07, 0.10, 0.16)
}

pub fn panel_hover() -> Color {
    Color::srgb(0.13, 0.19, 0.28)
}

pub fn glass() -> Color {
    pal().glass
}

pub fn glass_raised() -> Color {
    pal().glass_raised
}

pub fn line() -> Color {
    pal().line
}

pub fn action() -> Color {
    pal().action
}

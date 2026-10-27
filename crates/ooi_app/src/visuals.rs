//! 本作独有的视觉：烘焙好的损失地形贴图，与反复出现的"神经生命体"。
//!
//! 通用贴图、镜头、背景与转场都来自 `oo_kit`。

use bevy::image::ImageSampler;
use bevy::prelude::*;
use oo_base::Rng;
use oo_kit::scene::ReduceMotion;
use oo_kit::tex::{VisualTextures, rgba_image};

use crate::theme;
use crate::ui::ScreenEntity;
use ooi_core::puzzle::descent::{DOMAIN_X, DOMAIN_Y, normalized_loss};
use ooi_core::puzzle::gradient::{DOMAIN as G_DOMAIN, normalized_loss as gradient_loss};

/// Baked loss surfaces for the descent and gradient levels.
#[derive(Resource, Clone, Default)]
pub struct FieldTextures {
    pub descent: Handle<Image>,
    pub gradient: Handle<Image>,
}

impl FieldTextures {
    pub fn build(images: &mut Assets<Image>) -> Self {
        Self {
            descent: images.add(field_image(1320, 700, (DOMAIN_X, DOMAIN_Y), normalized_loss)),
            gradient: images.add(field_image(1320, 700, (G_DOMAIN, G_DOMAIN), gradient_loss)),
        }
    }
}

/// Thin contour lines at 10 loss bands over a dark-to-amber ramp.
fn field_color(t: f32) -> [u8; 4] {
    let t = t.clamp(0.0, 1.0);
    let edge = if (t * 10.0).fract() < 0.02 { 1.0 } else { 0.0 };
    let r = 0.012 + 0.50 * t + 0.12 * edge;
    let g = 0.045 + 0.33 * t + 0.09 * edge;
    let b = 0.20 - 0.12 * t + 0.02 * edge;
    let c = |v: f32| (v.clamp(0.0, 1.0) * 255.0) as u8;
    [c(r), c(g), c(b), 255]
}

/// Samples `loss` over `±domain` into one texture (2× world resolution).
fn field_image(width: u32, height: u32, domain: (f32, f32), loss: fn(f32, f32) -> f32) -> Image {
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        let wy = domain.1 - (y as f32 + 0.5) / height as f32 * domain.1 * 2.0;
        for x in 0..width {
            let wx = -domain.0 + (x as f32 + 0.5) / width as f32 * domain.0 * 2.0;
            pixels.extend_from_slice(&field_color(loss(wx, wy)));
        }
    }
    rgba_image(width, height, pixels, ImageSampler::linear())
}

// ---------------------------------------------------------------- 神经网络生命体

#[derive(Component)]
pub struct OrganismPulse {
    pub base_scale: f32,
    pub pulse_scale: f32,
    pub base_alpha: f32,
    pub pulse_alpha: f32,
    pub speed: f32,
    pub phase: f32,
    pub color: Color,
}

#[derive(Component)]
pub struct OrganismSignal {
    from: Vec2,
    to: Vec2,
    speed: f32,
    phase: f32,
    color: Color,
}

#[derive(Component)]
pub struct OrganismSpin(f32);

fn pulse(base_alpha: f32, pulse_alpha: f32, pulse_scale: f32, speed: f32, phase: f32, color: Color) -> OrganismPulse {
    OrganismPulse { base_scale: 1.0, pulse_scale, base_alpha, pulse_alpha, speed, phase, color }
}

fn sprite(image: &Handle<Image>, color: Color, size: f32) -> Sprite {
    Sprite { image: image.clone(), color, custom_size: Some(Vec2::splat(size)), ..default() }
}

fn link(commands: &mut Commands, a: Vec2, b: Vec2, color: Color, z: f32) {
    let d = b - a;
    let mid = (a + b) * 0.5;
    commands.spawn((
        Sprite::from_color(color, Vec2::new(d.length(), 1.0)),
        Transform::from_xyz(mid.x, mid.y, z).with_rotation(Quat::from_rotation_z(d.y.atan2(d.x))),
        ScreenEntity,
    ));
}

/// The recurring "organism" motif: a small neural constellation around a core.
/// The same silhouette appears on the title, the hub, the cards and the ending.
pub fn spawn_organism(commands: &mut Commands, tex: &VisualTextures, center: Vec2, scale: f32, seed: u64, z: f32) {
    let mut rng = Rng::new(seed.max(1));
    let at = |p: Vec2, dz: f32| Transform::from_xyz(p.x, p.y, z + dz);

    for (i, (d, alpha, speed, dashed)) in
        [(350.0, 0.10, 0.012, true), (265.0, 0.16, -0.022, false), (190.0, 0.08, 0.035, true)].into_iter().enumerate()
    {
        let image = if dashed { &tex.ring_dash } else { &tex.ring };
        commands.spawn((
            sprite(image, theme::cyan().with_alpha(alpha), d * scale),
            at(center, 0.1 + i as f32 * 0.02),
            OrganismSpin(speed),
            ScreenEntity,
        ));
    }
    commands.spawn((
        sprite(&tex.glow, theme::cyan().with_alpha(0.16), 330.0 * scale),
        at(center, 0.0),
        pulse(0.10, 0.09, 0.025, 0.7, 0.4, theme::cyan()),
        ScreenEntity,
    ));
    commands.spawn((
        sprite(&tex.glow, theme::cyan_hot().with_alpha(0.45), 95.0 * scale),
        at(center, 1.0),
        pulse(0.32, 0.30, 0.09, 1.5, 1.7, theme::cyan_hot()),
        ScreenEntity,
    ));

    // Nodes orbit at irregular angles so the silhouette never reads as a clock.
    let count = 9;
    let nodes: Vec<(Vec2, f32, bool)> = (0..count)
        .map(|i| {
            let angle = i as f32 / count as f32 * std::f32::consts::TAU + rng.range(-0.14, 0.14) as f32;
            let radius = (88.0 + rng.range(-16.0, 34.0) as f32) * scale;
            (center + Vec2::from_angle(angle) * radius, 4.0 + rng.f32() * 4.2, i % 3 == 0)
        })
        .collect();
    for (i, &(pos, _, _)) in nodes.iter().enumerate() {
        link(commands, center, pos, theme::cyan().with_alpha(0.075), z + 0.3);
        link(commands, pos, nodes[(i + 1) % count].0, theme::violet().with_alpha(0.07), z + 0.25);
    }

    for (i, &(pos, radius, hot)) in nodes.iter().enumerate() {
        let color = if hot { theme::amber() } else { theme::cyan() };
        let k = i as f32;
        commands.spawn((
            sprite(&tex.glow, color.with_alpha(if hot { 0.20 } else { 0.13 }), radius * if hot { 8.5 } else { 6.5 }),
            at(pos, 3.2),
            pulse(if hot { 0.12 } else { 0.08 }, 0.14, 0.10, 0.8 + k * 0.11, k * 1.4, color),
            ScreenEntity,
        ));
        commands.spawn((
            sprite(&tex.ring, color.with_alpha(0.45), radius * 2.8),
            at(pos, 3.5),
            OrganismSpin(if i % 2 == 0 { 0.02 } else { -0.02 }),
            ScreenEntity,
        ));
        commands.spawn((
            sprite(&tex.dot, color, radius * 1.9),
            at(pos, 4.0),
            pulse(0.7, 0.3, 0.15, 1.0 + k * 0.2, k * 0.8, color),
            ScreenEntity,
        ));
        // Signals travelling from the core to a node, then fading.
        if i % 2 == 0 {
            commands.spawn((
                sprite(&tex.dot, color, 5.5),
                at(center, 3.0),
                OrganismSignal { from: center, to: pos, speed: 0.11 + k * 0.017, phase: k * 0.19, color },
                ScreenEntity,
            ));
        }
    }

    // The central neuron is deliberately brighter than everything else.
    commands.spawn((
        sprite(&tex.dot, theme::text(), 13.0 * scale),
        at(center, 3.0),
        pulse(0.9, 0.1, 0.12, 1.9, 0.0, theme::cyan()),
        ScreenEntity,
    ));
}

type SignalQuery<'w, 's> =
    Query<'w, 's, (&'static OrganismSignal, &'static mut Transform, &'static mut Sprite), Without<OrganismPulse>>;
type SpinQuery<'w, 's> =
    Query<'w, 's, (&'static OrganismSpin, &'static mut Transform), (Without<OrganismPulse>, Without<OrganismSignal>)>;

pub fn animate_organisms(
    time: Res<Time>,
    reduce: Res<ReduceMotion>,
    mut pulses: Query<(&OrganismPulse, &mut Transform, &mut Sprite)>,
    mut signals: SignalQuery,
    mut spins: SpinQuery,
) {
    if reduce.0 {
        return;
    }
    let t = time.elapsed_secs();
    for (p, mut tf, mut s) in &mut pulses {
        let wave = 0.5 + 0.5 * (t * p.speed + p.phase).sin();
        tf.scale = Vec3::splat(p.base_scale * (1.0 + p.pulse_scale * wave));
        s.color = p.color.with_alpha(p.base_alpha + p.pulse_alpha * wave);
    }
    for (sig, mut tf, mut s) in &mut signals {
        let u = (t * sig.speed + sig.phase).fract();
        let pos = sig.from.lerp(sig.to, u);
        tf.translation.x = pos.x;
        tf.translation.y = pos.y;
        s.color = sig.color.with_alpha((u * std::f32::consts::PI).sin() * 0.75);
    }
    for (spin, mut tf) in &mut spins {
        tf.rotation = Quat::from_rotation_z(t * spin.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_bake_with_contrast() {
        let image = field_image(64, 32, (DOMAIN_X, DOMAIN_Y), normalized_loss);
        assert_eq!((image.width(), image.height()), (64, 32));
        let data = image.data.expect("pixels");
        assert_eq!(data.len(), 64 * 32 * 4);
        assert!(data.chunks(4).all(|px| px[3] == 255));
        let (lo, hi) = data.chunks(4).fold((255u8, 0u8), |(lo, hi), px| (lo.min(px[0]), hi.max(px[0])));
        assert!(hi - lo > 40, "field texture lacks contrast");
        let bowl = field_image(64, 32, (G_DOMAIN, G_DOMAIN), gradient_loss);
        assert_eq!(bowl.data.expect("pixels").len(), 64 * 32 * 4);
    }
}

use eframe::egui::{self, Color32, Id, LayerId, Order, Painter, Pos2, Rect, Shape, Stroke, vec2};

use crate::{
    config::UiPreferences,
    model::{ComponentId, SystemSnapshot},
    theme,
};

/// Draw the selected familiars over the central Observatory workspace.
///
/// Each familiar intentionally has its own motion personality:
///
/// Cat:
///   drifting patrol -> pause -> tail flick -> continue
///
/// Bat:
///   wide wandering flight with layered turns and quick wing beats
///
/// Bunny:
///   hopping bursts through a slowly wandering territory
///
/// Puppy:
///   happy wandering trot with a wagging tail and occasional head bob
///
/// The drawings are painter primitives, not font glyphs, so they work even when
/// emoji/kaomoji fonts are unavailable.
pub fn draw_familiars(
    ctx: &egui::Context,
    rect: Rect,
    preferences: &UiPreferences,
    elapsed: f32,
    snapshot: &SystemSnapshot,
    selected: ComponentId,
    incident_active: bool,
    fleet_has_offline: bool,
) {
    if !preferences.catgirl
        && !preferences.batgirl
        && !preferences.bunnygirl
        && !preferences.puppygirl
    {
        return;
    }

    let layer = LayerId::new(Order::Foreground, Id::new("observatory_familiar_layer"));

    let painter = ctx
        .layer_painter(layer)
        .with_clip_rect(rect.shrink2(vec2(14.0, 14.0)));

    let network_burst = (snapshot.network.rx_mib_s + snapshot.network.tx_mib_s).min(180.0) / 180.0;

    let thermal_alert =
        snapshot.cpu.package_temperature_c >= 78.0 || snapshot.gpu.temperature_c >= 80.0;

    let memory_pressure =
        snapshot.memory.used_bytes as f32 / snapshot.memory.total_bytes.max(1) as f32;

    let cat_time = elapsed * (1.0 + network_burst * 1.8);

    let bat_time = elapsed * if thermal_alert { 1.8 } else { 1.0 };

    let bunny_time = if incident_active {
        0.0
    } else {
        elapsed * (0.9 + memory_pressure * 0.8)
    };

    let puppy_time = if fleet_has_offline { 0.0 } else { elapsed };

    // Much larger than the first version.
    const CAT_SIZE: f32 = 31.0;
    const BAT_SIZE: f32 = 27.0;
    const BUNNY_SIZE: f32 = 29.0;
    const PUPPY_SIZE: f32 = 31.0;

    if preferences.catgirl {
        let mut pos = cat_position(rect, cat_time);

        if selected == ComponentId::Network {
            pos.y -= 18.0;
        }

        draw_cat(&painter, pos, CAT_SIZE, cat_time);
    }

    if preferences.batgirl {
        let pos = bat_position(rect, bat_time);
        draw_bat(&painter, pos, BAT_SIZE, bat_time);
    }

    if preferences.bunnygirl {
        let pos = if incident_active {
            Pos2::new(rect.right() - 105.0, rect.bottom() - 62.0)
        } else {
            bunny_position(rect, bunny_time)
        };

        draw_bunny(&painter, pos, BUNNY_SIZE, bunny_time);
    }

    if preferences.puppygirl {
        let pos = if fleet_has_offline {
            Pos2::new(rect.left() + 92.0, rect.bottom() - 52.0)
        } else {
            puppy_position(rect, puppy_time)
        };

        draw_puppy(&painter, pos, PUPPY_SIZE, puppy_time);
    }
}

// -----------------------------------------------------------------------------
// Motion
// -----------------------------------------------------------------------------

fn cat_position(rect: Rect, t: f32) -> Pos2 {
    // The cat still patrols and lingers at the ends, but the patrol lane
    // itself slowly drifts so she does not pace the exact same pixels forever.
    let cycle = (t * 0.095).fract();
    let triangle = if cycle < 0.5 {
        cycle * 2.0
    } else {
        (1.0 - cycle) * 2.0
    };

    let eased = smoothstep(triangle);

    let lane_drift = (t * 0.071).sin() * 0.05 + (t * 0.137 + 1.4).sin() * 0.025;

    let left = rect.left() + rect.width() * (0.08 + lane_drift);

    let right = rect.left() + rect.width() * (0.34 + lane_drift);

    let x = lerp_f32(left, right, eased);

    let y = rect.bottom() - 54.0 + (t * 1.9).sin() * 1.2 + (t * 0.21 + 0.8).sin() * 5.0;

    clamp_position(rect, Pos2::new(x, y), 42.0)
}

fn bat_position(rect: Rect, t: f32) -> Pos2 {
    // Layer several unrelated frequencies. The result stays smooth while
    // taking a very long time to visibly repeat, so flight feels organic
    // rather than tracing one obvious figure-eight.
    let cx = rect.center().x;
    let cy = rect.top() + rect.height() * 0.34;

    let x = cx
        + rect.width()
            * (0.27 * (t * 0.19).sin()
                + 0.10 * (t * 0.43 + 1.7).sin()
                + 0.05 * (t * 0.83 + 0.4).sin());

    let y = cy
        + rect.height()
            * (0.18 * (t * 0.23 + 2.2).sin()
                + 0.08 * (t * 0.57 + 0.3).sin()
                + 0.04 * (t * 1.11).sin());

    clamp_position(rect, Pos2::new(x, y), 46.0)
}

fn bunny_position(rect: Rect, t: f32) -> Pos2 {
    // A bunny does not glide. It hops in little bursts.
    let phase = (t * 0.42).fract();

    let direction = if ((t * 0.42) as i32) % 2 == 0 {
        1.0
    } else {
        -1.0
    };

    let travel = if direction > 0.0 { phase } else { 1.0 - phase };

    let lane = (t * 0.061 + 0.7).sin() * 0.10;

    let left = rect.left() + rect.width() * (0.54 + lane);

    let right = rect.right() - rect.width() * (0.07 - lane * 0.35);

    let x = lerp_f32(left, right, smoothstep(travel));

    // Three distinct hops per travel pass.
    let hop_wave = ((phase * 3.0).fract() * std::f32::consts::PI).sin();
    let hop = hop_wave.max(0.0);

    let y =
        rect.bottom() - 60.0 - hop * 18.0 + (t * 2.6).sin() * 0.8 + (t * 0.14 + 1.2).sin() * 4.0;

    clamp_position(rect, Pos2::new(x, y), 42.0)
}

fn puppy_position(rect: Rect, t: f32) -> Pos2 {
    // Slightly faster cheerful trot than the cat, with a slowly wandering lane.
    let cycle = (t * 0.14).fract();

    let triangle = if cycle < 0.5 {
        cycle * 2.0
    } else {
        (1.0 - cycle) * 2.0
    };

    let drift = (t * 0.052 + 2.0).sin() * 0.10;

    let left = rect.left() + rect.width() * (0.28 + drift);

    let right = rect.left() + rect.width() * (0.72 + drift * 0.45);

    let x = lerp_f32(left, right, smoothstep(triangle));

    let trot = (t * 7.2).sin().abs();

    let y = rect.bottom() - 50.0 - trot * 2.3 + (t * 0.18).sin() * 4.0;

    clamp_position(rect, Pos2::new(x, y), 42.0)
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn clamp_position(rect: Rect, position: Pos2, margin: f32) -> Pos2 {
    let safe = rect.shrink(margin);

    Pos2::new(
        position.x.clamp(safe.left(), safe.right()),
        position.y.clamp(safe.top(), safe.bottom()),
    )
}

// -----------------------------------------------------------------------------
// Cat
// -----------------------------------------------------------------------------

fn draw_cat(painter: &Painter, origin: Pos2, size: f32, elapsed: f32) {
    let outline = Stroke::new(1.8, theme::pink());
    let fill = tint(theme::pink(), 0.21);
    let light_fill = tint(theme::pink(), 0.34);
    let shadow = tint(theme::panel_alt(), 0.06);

    let facing = if (elapsed * 0.095).fract() < 0.5 {
        1.0
    } else {
        -1.0
    };

    let body = origin;
    let head = origin + vec2(size * 0.50 * facing, -size * 0.18);

    // Tiny ground shadow.
    painter.ellipse_filled(
        Rect::from_center_size(
            body + vec2(0.0, size * 0.35),
            vec2(size * 1.20, size * 0.20),
        ),
        shadow,
    );

    // Body and chest.
    painter.circle_filled(body, size * 0.42, fill);
    painter.circle_stroke(body, size * 0.42, outline);

    painter.circle_filled(
        body + vec2(size * 0.12 * facing, -size * 0.03),
        size * 0.26,
        light_fill,
    );

    // Head.
    painter.circle_filled(head, size * 0.29, fill);
    painter.circle_stroke(head, size * 0.29, outline);

    // Ears.
    let ear_y = -size * 0.20;

    painter.add(Shape::convex_polygon(
        vec![
            head + vec2(-size * 0.22, ear_y),
            head + vec2(-size * 0.11, -size * 0.52),
            head + vec2(-size * 0.01, -size * 0.18),
        ],
        light_fill,
        outline,
    ));

    painter.add(Shape::convex_polygon(
        vec![
            head + vec2(size * 0.02, -size * 0.18),
            head + vec2(size * 0.13, -size * 0.52),
            head + vec2(size * 0.23, ear_y),
        ],
        light_fill,
        outline,
    ));

    // Eyes.
    painter.circle_filled(
        head + vec2(-size * 0.085, -size * 0.025),
        1.9,
        theme::white(),
    );
    painter.circle_filled(
        head + vec2(size * 0.085, -size * 0.025),
        1.9,
        theme::white(),
    );

    // Nose.
    painter.circle_filled(head + vec2(0.0, size * 0.065), 1.35, theme::gold());

    // Tiny cat mouth.
    painter.line_segment(
        [
            head + vec2(0.0, size * 0.09),
            head + vec2(-3.0, size * 0.15),
        ],
        Stroke::new(1.0, theme::white()),
    );
    painter.line_segment(
        [head + vec2(0.0, size * 0.09), head + vec2(3.0, size * 0.15)],
        Stroke::new(1.0, theme::white()),
    );

    // Whiskers.
    for offset in [-3.0_f32, 2.0] {
        painter.line_segment(
            [head + vec2(-4.0, offset), head + vec2(-11.0, offset - 1.0)],
            Stroke::new(0.9, theme::muted()),
        );

        painter.line_segment(
            [head + vec2(4.0, offset), head + vec2(11.0, offset - 1.0)],
            Stroke::new(0.9, theme::muted()),
        );
    }

    // Legs.
    let step = (elapsed * 6.0).sin() * 2.0;

    painter.line_segment(
        [
            body + vec2(size * 0.17, size * 0.25),
            body + vec2(size * 0.22 + step, size * 0.51),
        ],
        outline,
    );
    painter.line_segment(
        [
            body + vec2(-size * 0.15, size * 0.26),
            body + vec2(-size * 0.18 - step, size * 0.50),
        ],
        outline,
    );

    // Swishy tail.
    let tail_sway = (elapsed * 3.4).sin() * size * 0.18;

    painter.add(Shape::line(
        vec![
            body + vec2(-size * 0.38 * facing, -size * 0.04),
            body + vec2(-size * 0.67 * facing, -size * 0.18 + tail_sway * 0.2),
            body + vec2(-size * 0.58 * facing, -size * 0.72 + tail_sway),
        ],
        Stroke::new(2.1, theme::pink()),
    ));
}

// -----------------------------------------------------------------------------
// Bat
// -----------------------------------------------------------------------------

fn draw_bat(painter: &Painter, origin: Pos2, size: f32, elapsed: f32) {
    let wing_lift = (elapsed * 8.8).sin() * size * 0.18;
    let outline = Stroke::new(1.8, theme::violet());
    let fill = tint(theme::violet(), 0.20);
    let wing_fill = tint(theme::violet(), 0.29);

    let body = origin;

    // Wings with scalloped lower edges.
    let left_wing = vec![
        body + vec2(-size * 0.12, 0.0),
        body + vec2(-size * 0.48, -size * 0.30 - wing_lift),
        body + vec2(-size * 0.94, -size * 0.05),
        body + vec2(-size * 0.69, size * 0.20),
        body + vec2(-size * 0.49, size * 0.06),
        body + vec2(-size * 0.31, size * 0.24),
    ];

    let right_wing = vec![
        body + vec2(size * 0.12, 0.0),
        body + vec2(size * 0.48, -size * 0.30 - wing_lift),
        body + vec2(size * 0.94, -size * 0.05),
        body + vec2(size * 0.69, size * 0.20),
        body + vec2(size * 0.49, size * 0.06),
        body + vec2(size * 0.31, size * 0.24),
    ];

    painter.add(Shape::convex_polygon(left_wing, wing_fill, outline));
    painter.add(Shape::convex_polygon(right_wing, wing_fill, outline));

    // Body + head.
    painter.circle_filled(body + vec2(0.0, size * 0.10), size * 0.23, fill);
    painter.circle_stroke(body + vec2(0.0, size * 0.10), size * 0.23, outline);

    painter.circle_filled(body + vec2(0.0, -size * 0.12), size * 0.19, fill);
    painter.circle_stroke(body + vec2(0.0, -size * 0.12), size * 0.19, outline);

    // Pointy ears.
    painter.add(Shape::convex_polygon(
        vec![
            body + vec2(-size * 0.13, -size * 0.20),
            body + vec2(-size * 0.08, -size * 0.45),
            body + vec2(-size * 0.01, -size * 0.24),
        ],
        fill,
        outline,
    ));
    painter.add(Shape::convex_polygon(
        vec![
            body + vec2(size * 0.01, -size * 0.24),
            body + vec2(size * 0.08, -size * 0.45),
            body + vec2(size * 0.13, -size * 0.20),
        ],
        fill,
        outline,
    ));

    // Eyes.
    painter.circle_filled(body + vec2(-size * 0.055, -size * 0.12), 1.6, theme::pink());
    painter.circle_filled(body + vec2(size * 0.055, -size * 0.12), 1.6, theme::pink());

    // Tiny fang.
    painter.add(Shape::convex_polygon(
        vec![
            body + vec2(-1.2, 0.0),
            body + vec2(0.0, 3.5),
            body + vec2(1.2, 0.0),
        ],
        theme::white(),
        Stroke::NONE,
    ));

    // Motion trail / sparkles.
    let trail = [
        vec2(-size * 1.00, size * 0.10),
        vec2(-size * 1.20, size * 0.30),
        vec2(-size * 1.34, -size * 0.02),
    ];

    for (index, offset) in trail.iter().enumerate() {
        painter.circle_filled(body + *offset, 1.7 - index as f32 * 0.3, theme::muted());
    }
}

// -----------------------------------------------------------------------------
// Bunny
// -----------------------------------------------------------------------------

fn draw_bunny(painter: &Painter, origin: Pos2, size: f32, elapsed: f32) {
    let outline = Stroke::new(1.8, theme::blue());
    let fill = tint(theme::blue(), 0.20);
    let inner = tint(theme::pink(), 0.28);

    let body = origin;
    let head = origin + vec2(size * 0.43, -size * 0.17);

    painter.circle_filled(body, size * 0.40, fill);
    painter.circle_stroke(body, size * 0.40, outline);

    painter.circle_filled(head, size * 0.27, fill);
    painter.circle_stroke(head, size * 0.27, outline);

    // Floppy/animated ears.
    let ear_wobble = (elapsed * 3.7).sin() * size * 0.06;

    let left_ear = Rect::from_center_size(
        head + vec2(-size * 0.09 - ear_wobble, -size * 0.49),
        vec2(size * 0.14, size * 0.55),
    );

    let right_ear = Rect::from_center_size(
        head + vec2(size * 0.08 + ear_wobble, -size * 0.49),
        vec2(size * 0.14, size * 0.55),
    );

    painter.rect_filled(left_ear, size * 0.07, fill);
    painter.rect_stroke(left_ear, size * 0.07, outline, egui::StrokeKind::Inside);

    painter.rect_filled(right_ear, size * 0.07, fill);
    painter.rect_stroke(right_ear, size * 0.07, outline, egui::StrokeKind::Inside);

    // Ear inners.
    painter.line_segment(
        [
            left_ear.center_top() + vec2(0.0, 4.0),
            left_ear.center_bottom() - vec2(0.0, 4.0),
        ],
        Stroke::new(2.0, inner),
    );

    painter.line_segment(
        [
            right_ear.center_top() + vec2(0.0, 4.0),
            right_ear.center_bottom() - vec2(0.0, 4.0),
        ],
        Stroke::new(2.0, inner),
    );

    // Face.
    painter.circle_filled(
        head + vec2(-size * 0.075, -size * 0.02),
        1.8,
        theme::white(),
    );

    painter.circle_filled(head + vec2(size * 0.075, -size * 0.02), 1.8, theme::white());

    painter.circle_filled(head + vec2(0.0, size * 0.065), 1.5, theme::pink());

    // Feet.
    painter.circle_filled(body + vec2(size * 0.19, size * 0.31), size * 0.10, fill);

    painter.circle_filled(body + vec2(-size * 0.18, size * 0.32), size * 0.10, fill);

    // Fluffy tail.
    painter.circle_filled(
        body + vec2(-size * 0.41, -size * 0.02),
        size * 0.12,
        theme::white(),
    );

    // Small hop dust motes.
    let hop_phase = ((elapsed * 0.42).fract() * 3.0).fract();
    if hop_phase > 0.72 {
        for i in 0..3 {
            painter.circle_filled(
                body + vec2(-size * 0.45 - i as f32 * 5.0, size * 0.34 + i as f32 * 1.5),
                1.3,
                theme::muted(),
            );
        }
    }
}

// -----------------------------------------------------------------------------
// Puppy
// -----------------------------------------------------------------------------

fn draw_puppy(painter: &Painter, origin: Pos2, size: f32, elapsed: f32) {
    let outline = Stroke::new(1.8, theme::gold());
    let fill = tint(theme::gold(), 0.19);
    let muzzle_fill = tint(theme::white(), 0.20);

    let body = origin;
    let head = origin + vec2(size * 0.52, -size * 0.13);

    painter.circle_filled(body, size * 0.42, fill);
    painter.circle_stroke(body, size * 0.42, outline);

    painter.circle_filled(head, size * 0.29, fill);
    painter.circle_stroke(head, size * 0.29, outline);

    // Floppy ears.
    let ear_bounce = (elapsed * 7.0).sin() * size * 0.035;

    painter.add(Shape::convex_polygon(
        vec![
            head + vec2(-size * 0.20, -size * 0.15),
            head + vec2(-size * 0.34, size * 0.05 + ear_bounce),
            head + vec2(-size * 0.19, size * 0.20),
            head + vec2(-size * 0.09, 0.0),
        ],
        tint(theme::gold(), 0.28),
        outline,
    ));

    painter.add(Shape::convex_polygon(
        vec![
            head + vec2(size * 0.18, -size * 0.13),
            head + vec2(size * 0.31, size * 0.07 + ear_bounce),
            head + vec2(size * 0.17, size * 0.19),
            head + vec2(size * 0.07, 0.0),
        ],
        tint(theme::gold(), 0.28),
        outline,
    ));

    // Muzzle.
    painter.circle_filled(
        head + vec2(size * 0.07, size * 0.10),
        size * 0.14,
        muzzle_fill,
    );

    painter.circle_filled(head + vec2(size * 0.15, size * 0.05), 1.9, theme::white());

    // Eyes.
    painter.circle_filled(
        head + vec2(-size * 0.07, -size * 0.035),
        1.8,
        theme::white(),
    );

    painter.circle_filled(
        head + vec2(size * 0.075, -size * 0.035),
        1.8,
        theme::white(),
    );

    // Happy little tongue.
    if (elapsed * 2.0).sin() > -0.15 {
        painter.circle_filled(
            head + vec2(size * 0.08, size * 0.20),
            size * 0.055,
            theme::pink(),
        );
    }

    // Trotting legs.
    let step = (elapsed * 7.2).sin() * 3.0;

    painter.line_segment(
        [
            body + vec2(size * 0.18, size * 0.25),
            body + vec2(size * 0.22 + step, size * 0.52),
        ],
        outline,
    );

    painter.line_segment(
        [
            body + vec2(-size * 0.16, size * 0.26),
            body + vec2(-size * 0.20 - step, size * 0.51),
        ],
        outline,
    );

    // Extremely important tail wag subsystem.
    let wag = (elapsed * 8.5).sin() * size * 0.27;

    painter.add(Shape::line(
        vec![
            body + vec2(-size * 0.40, -size * 0.04),
            body + vec2(-size * 0.68, -size * 0.28 + wag * 0.20),
            body + vec2(-size * 0.55, -size * 0.57 + wag),
        ],
        Stroke::new(2.2, theme::gold()),
    ));

    // Collar.
    painter.line_segment(
        [
            head + vec2(-size * 0.20, size * 0.18),
            head + vec2(size * 0.17, size * 0.21),
        ],
        Stroke::new(2.0, theme::blue()),
    );

    painter.circle_filled(head + vec2(size * 0.01, size * 0.23), 2.0, theme::blue());
}

// -----------------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------------

fn tint(color: Color32, amount: f32) -> Color32 {
    theme::blend(theme::panel_alt(), color, amount.clamp(0.0, 1.0))
}

/// egui doesn't expose a dedicated ellipse helper on `Painter` in the same way
/// as circles, so this tiny extension builds one from a convex mesh-like shape.
trait PainterEllipseExt {
    fn ellipse_filled(&self, rect: Rect, color: Color32);
}

impl PainterEllipseExt for Painter {
    fn ellipse_filled(&self, rect: Rect, color: Color32) {
        const POINTS: usize = 24;

        let mut points = Vec::with_capacity(POINTS);

        for i in 0..POINTS {
            let angle = i as f32 / POINTS as f32 * std::f32::consts::TAU;

            points.push(Pos2::new(
                rect.center().x + rect.width() * 0.5 * angle.cos(),
                rect.center().y + rect.height() * 0.5 * angle.sin(),
            ));
        }

        self.add(Shape::convex_polygon(points, color, Stroke::NONE));
    }
}

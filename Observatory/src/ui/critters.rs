use eframe::egui::{
    self, Align2, Color32, CursorIcon, FontId, Id, LayerId, Order, Painter, Pos2, Rect, Sense,
    Shape, Stroke, Vec2, vec2,
};

use crate::{
    config::UiPreferences,
    model::{ComponentId, SystemSnapshot},
    theme,
};

#[derive(Debug, Clone, Copy)]
enum FamiliarKind {
    Cat,
    Bat,
    Bunny,
    Puppy,
}

impl FamiliarKind {
    fn key(self) -> &'static str {
        match self {
            Self::Cat => "cat",
            Self::Bat => "bat",
            Self::Bunny => "bunny",
            Self::Puppy => "puppy",
        }
    }

    fn chase_window(self) -> (f32, f32, f32) {
        match self {
            Self::Cat => (17.0, 5.0, 7.0),
            Self::Bat => (13.0, 4.0, 8.0),
            Self::Bunny => (19.0, 5.0, 10.0),
            Self::Puppy => (11.0, 6.0, 6.0),
        }
    }

    fn sleep_window(self) -> (f32, f32, f32) {
        match self {
            Self::Cat => (47.0, 6.5, 11.0),
            Self::Bat => (53.0, 7.0, 19.0),
            Self::Bunny => (43.0, 6.0, 8.0),
            Self::Puppy => (58.0, 8.0, 22.0),
        }
    }

    fn responsiveness(self, chasing: bool, sleeping: bool) -> (f32, f32, f32) {
        if sleeping {
            return (10.0, 11.0, 90.0);
        }

        match (self, chasing) {
            (Self::Cat, true) => (26.0, 8.0, 460.0),
            (Self::Bat, true) => (31.0, 7.0, 620.0),
            (Self::Bunny, true) => (23.0, 8.5, 430.0),
            (Self::Puppy, true) => (30.0, 8.5, 520.0),
            (Self::Cat, false) => (14.0, 9.5, 260.0),
            (Self::Bat, false) => (17.0, 7.5, 360.0),
            (Self::Bunny, false) => (15.0, 10.0, 300.0),
            (Self::Puppy, false) => (18.0, 9.5, 340.0),
        }
    }

    fn hitbox(self, position: Pos2, size: f32) -> Rect {
        let dimensions = match self {
            Self::Bat => vec2(size * 2.25, size * 1.45),
            _ => vec2(size * 1.95, size * 1.75),
        };

        Rect::from_center_size(position, dimensions)
    }
}

#[derive(Debug, Clone, Copy)]
struct FamiliarState {
    initialized: bool,
    position: Pos2,
    velocity: Vec2,
    feed_started_at: f32,
    fed_until: f32,
    sleep_anchor: Pos2,
    sleep_pose_time: f32,
    was_sleeping: bool,
}

impl Default for FamiliarState {
    fn default() -> Self {
        Self {
            initialized: false,
            position: Pos2::new(0.0, 0.0),
            velocity: Vec2::ZERO,
            feed_started_at: -1000.0,
            fed_until: -1000.0,
            sleep_anchor: Pos2::new(0.0, 0.0),
            sleep_pose_time: 0.0,
            was_sleeping: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct FamiliarMemory {
    cat: FamiliarState,
    bat: FamiliarState,
    bunny: FamiliarState,
    puppy: FamiliarState,
}

impl FamiliarMemory {
    fn state_mut(&mut self, kind: FamiliarKind) -> &mut FamiliarState {
        match kind {
            FamiliarKind::Cat => &mut self.cat,
            FamiliarKind::Bat => &mut self.bat,
            FamiliarKind::Bunny => &mut self.bunny,
            FamiliarKind::Puppy => &mut self.puppy,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct FamiliarFrame {
    position: Pos2,
    pose_time: f32,
    sleeping: bool,
    fed: bool,
    hovered: bool,
    chasing: bool,
    feed_age: f32,
}

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

    // These are living UI decorations now, so keep the animation clock ticking
    // even when the rest of the dashboard has nothing new to paint.
    ctx.request_repaint();

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

    const CAT_SIZE: f32 = 31.0;
    const BAT_SIZE: f32 = 27.0;
    const BUNNY_SIZE: f32 = 29.0;
    const PUPPY_SIZE: f32 = 31.0;

    let memory_id = Id::new("observatory_familiar_interaction_memory");
    let mut memory = ctx.data(|data| {
        data.get_temp::<FamiliarMemory>(memory_id)
            .unwrap_or_default()
    });

    let pointer = ctx.input(|input| input.pointer.hover_pos());
    let dt = ctx
        .input(|input| input.stable_dt)
        .clamp(1.0 / 240.0, 1.0 / 20.0);

    if preferences.catgirl {
        let mut target = cat_position(rect, cat_time);

        if selected == ComponentId::Network {
            target.y -= 18.0;
        }

        let frame = update_familiar(
            ctx,
            rect,
            FamiliarKind::Cat,
            memory.state_mut(FamiliarKind::Cat),
            target,
            CAT_SIZE,
            elapsed,
            cat_time,
            dt,
            pointer,
            true,
            true,
        );

        draw_cat(&painter, frame.position, CAT_SIZE, frame.pose_time);
        draw_interaction_effects(&painter, FamiliarKind::Cat, frame, CAT_SIZE, theme::pink());
    }

    if preferences.batgirl {
        let frame = update_familiar(
            ctx,
            rect,
            FamiliarKind::Bat,
            memory.state_mut(FamiliarKind::Bat),
            bat_position(rect, bat_time),
            BAT_SIZE,
            elapsed,
            bat_time,
            dt,
            pointer,
            !thermal_alert,
            !thermal_alert,
        );

        draw_bat(&painter, frame.position, BAT_SIZE, frame.pose_time);
        draw_interaction_effects(
            &painter,
            FamiliarKind::Bat,
            frame,
            BAT_SIZE,
            theme::violet(),
        );
    }

    if preferences.bunnygirl {
        let target = if incident_active {
            Pos2::new(rect.right() - 105.0, rect.bottom() - 62.0)
        } else {
            bunny_position(rect, bunny_time)
        };

        let frame = update_familiar(
            ctx,
            rect,
            FamiliarKind::Bunny,
            memory.state_mut(FamiliarKind::Bunny),
            target,
            BUNNY_SIZE,
            elapsed,
            bunny_time,
            dt,
            pointer,
            !incident_active,
            !incident_active,
        );

        draw_bunny(&painter, frame.position, BUNNY_SIZE, frame.pose_time);
        draw_interaction_effects(
            &painter,
            FamiliarKind::Bunny,
            frame,
            BUNNY_SIZE,
            theme::blue(),
        );
    }

    if preferences.puppygirl {
        let target = if fleet_has_offline {
            Pos2::new(rect.left() + 92.0, rect.bottom() - 52.0)
        } else {
            puppy_position(rect, puppy_time)
        };

        let frame = update_familiar(
            ctx,
            rect,
            FamiliarKind::Puppy,
            memory.state_mut(FamiliarKind::Puppy),
            target,
            PUPPY_SIZE,
            elapsed,
            puppy_time,
            dt,
            pointer,
            !fleet_has_offline,
            !fleet_has_offline,
        );

        draw_puppy(&painter, frame.position, PUPPY_SIZE, frame.pose_time);
        draw_interaction_effects(
            &painter,
            FamiliarKind::Puppy,
            frame,
            PUPPY_SIZE,
            theme::gold(),
        );
    }

    ctx.data_mut(|data| data.insert_temp(memory_id, memory));
}

fn update_familiar(
    ctx: &egui::Context,
    rect: Rect,
    kind: FamiliarKind,
    state: &mut FamiliarState,
    base_target: Pos2,
    size: f32,
    elapsed: f32,
    pose_time: f32,
    dt: f32,
    pointer: Option<Pos2>,
    chase_allowed: bool,
    sleep_allowed: bool,
) -> FamiliarFrame {
    let base_target = clamp_position(rect, base_target, size * 1.45);

    if !state.initialized {
        state.initialized = true;
        state.position = base_target;
        state.sleep_anchor = base_target;
    }

    let current_hitbox = kind.hitbox(state.position, size);
    let pointer_inside_workspace = pointer.is_some_and(|position| rect.contains(position));
    let hovered_before_move = pointer.is_some_and(|position| current_hitbox.contains(position));
    let fed_before_click = elapsed < state.fed_until;

    let (chase_period, chase_duration, chase_phase) = kind.chase_window();
    let chase_strength =
        if chase_allowed && pointer_inside_workspace && !fed_before_click && !hovered_before_move {
            activity_envelope(elapsed, chase_period, chase_duration, chase_phase)
        } else {
            0.0
        };

    let chasing = chase_strength > 0.04;

    let (sleep_period, sleep_duration, sleep_phase) = kind.sleep_window();
    let wants_sleep = sleep_allowed
        && !fed_before_click
        && !hovered_before_move
        && !chasing
        && activity_envelope(elapsed, sleep_period, sleep_duration, sleep_phase) > 0.55;

    if wants_sleep && !state.was_sleeping {
        state.sleep_anchor = sleep_target(kind, rect, state.position);
        state.sleep_pose_time = pose_time;
        state.velocity = Vec2::ZERO;
    }

    if !wants_sleep && state.was_sleeping {
        // A tiny wake-up twitch keeps waking from looking like a teleport from
        // "frozen" to "moving".
        state.velocity.y -= if matches!(kind, FamiliarKind::Bat) {
            28.0
        } else {
            18.0
        };
    }

    state.was_sleeping = wants_sleep;

    let mut target = if wants_sleep {
        state.sleep_anchor
    } else {
        base_target
    };

    if chasing {
        if let Some(cursor) = pointer {
            let cursor_target = chase_target(kind, rect, base_target, cursor, size);
            target = lerp_pos(target, cursor_target, chase_strength);
        }
    }

    let (spring, damping, max_speed) = kind.responsiveness(chasing, wants_sleep);
    spring_toward(state, target, dt, spring, damping, max_speed);

    state.position = clamp_position(rect, state.position, size * 1.35);

    let response = familiar_response(ctx, kind, kind.hitbox(state.position, size));
    let clicked = response.clicked();
    let hovered = response.hovered();

    if clicked {
        state.feed_started_at = elapsed;
        state.fed_until = elapsed + 3.6;
        state.was_sleeping = false;

        // Every familiar has a slightly different "SNACK!" reaction.
        state.velocity.y -= match kind {
            FamiliarKind::Bat => 95.0,
            FamiliarKind::Bunny => 150.0,
            FamiliarKind::Cat => 82.0,
            FamiliarKind::Puppy => 105.0,
        };
    }

    let fed = elapsed < state.fed_until;
    let sleeping = wants_sleep && !fed && !hovered;

    FamiliarFrame {
        position: state.position,
        pose_time: if sleeping {
            state.sleep_pose_time
        } else {
            pose_time
        },
        sleeping,
        fed,
        hovered,
        chasing: chasing && !fed,
        feed_age: elapsed - state.feed_started_at,
    }
}

fn familiar_response(ctx: &egui::Context, kind: FamiliarKind, hitbox: Rect) -> egui::Response {
    let response = egui::Area::new(Id::new(("observatory_familiar_hitbox", kind.key())))
        .order(Order::Foreground)
        .fixed_pos(hitbox.min)
        .show(ctx, |ui| {
            ui.allocate_response(hitbox.size(), Sense::click())
        })
        .inner;

    response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text("Click to feed :3")
}

fn spring_toward(
    state: &mut FamiliarState,
    target: Pos2,
    dt: f32,
    spring: f32,
    damping: f32,
    max_speed: f32,
) {
    let displacement = target - state.position;
    state.velocity += displacement * spring * dt;
    state.velocity *= (-damping * dt).exp();

    let speed = state.velocity.length();
    if speed > max_speed {
        state.velocity *= max_speed / speed;
    }

    state.position += state.velocity * dt;
}

fn chase_target(kind: FamiliarKind, rect: Rect, base: Pos2, cursor: Pos2, size: f32) -> Pos2 {
    let target = match kind {
        FamiliarKind::Bat => cursor + vec2(0.0, -size * 0.55),
        FamiliarKind::Cat => {
            let lift = ((base.y - cursor.y).max(0.0) * 0.08).min(size * 0.65);
            Pos2::new(cursor.x - size * 0.45, base.y - lift)
        }
        FamiliarKind::Bunny => Pos2::new(cursor.x + size * 0.30, base.y),
        FamiliarKind::Puppy => Pos2::new(cursor.x - size * 0.20, base.y),
    };

    clamp_position(rect, target, size * 1.45)
}

fn sleep_target(kind: FamiliarKind, rect: Rect, current: Pos2) -> Pos2 {
    let target = match kind {
        FamiliarKind::Cat => Pos2::new(current.x, rect.bottom() - 54.0),
        FamiliarKind::Bat => Pos2::new(rect.right() - 92.0, rect.top() + 78.0),
        FamiliarKind::Bunny => Pos2::new(current.x, rect.bottom() - 60.0),
        FamiliarKind::Puppy => Pos2::new(current.x, rect.bottom() - 50.0),
    };

    clamp_position(rect, target, 44.0)
}

fn activity_envelope(t: f32, period: f32, duration: f32, phase: f32) -> f32 {
    let local = (t + phase).rem_euclid(period);

    if local >= duration {
        return 0.0;
    }

    let fade = 0.85_f32.min(duration * 0.35);
    let fade_in = smoothstep((local / fade).clamp(0.0, 1.0));
    let fade_out = smoothstep(((duration - local) / fade).clamp(0.0, 1.0));

    fade_in * fade_out
}

fn lerp_pos(a: Pos2, b: Pos2, t: f32) -> Pos2 {
    Pos2::new(lerp_f32(a.x, b.x, t), lerp_f32(a.y, b.y, t))
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
// Interaction overlays
// -----------------------------------------------------------------------------

fn draw_interaction_effects(
    painter: &Painter,
    kind: FamiliarKind,
    frame: FamiliarFrame,
    size: f32,
    accent: Color32,
) {
    if frame.hovered {
        let pulse = 0.5 + 0.5 * (frame.pose_time * 5.0).sin();
        painter.circle_stroke(
            frame.position,
            size * (0.72 + pulse * 0.05),
            Stroke::new(1.2, tint(accent, 0.58)),
        );
    }

    if frame.chasing {
        let streak = match kind {
            FamiliarKind::Bat => size * 0.95,
            _ => size * 0.62,
        };

        painter.line_segment(
            [
                frame.position + vec2(-streak, size * 0.10),
                frame.position + vec2(-streak * 0.48, size * 0.04),
            ],
            Stroke::new(1.0, tint(accent, 0.34)),
        );
    }

    if frame.sleeping {
        draw_sleep_face(painter, kind, frame.position, size, frame.pose_time);
        draw_sleep_marks(painter, frame.position, size, accent, frame.pose_time);
    }

    if frame.fed {
        draw_feed_effect(
            painter,
            frame.position,
            size,
            frame.feed_age.max(0.0),
            accent,
        );
    }
}

fn draw_sleep_face(painter: &Painter, kind: FamiliarKind, origin: Pos2, size: f32, pose_time: f32) {
    let (head, eye_dx, eye_y, cover, line_color) = match kind {
        FamiliarKind::Cat => {
            let facing = if (pose_time * 0.095).fract() < 0.5 {
                1.0
            } else {
                -1.0
            };

            (
                origin + vec2(size * 0.50 * facing, -size * 0.18),
                size * 0.085,
                -size * 0.025,
                tint(theme::pink(), 0.21),
                theme::white(),
            )
        }
        FamiliarKind::Bat => (
            origin + vec2(0.0, -size * 0.12),
            size * 0.055,
            0.0,
            tint(theme::violet(), 0.20),
            theme::pink(),
        ),
        FamiliarKind::Bunny => (
            origin + vec2(size * 0.43, -size * 0.17),
            size * 0.075,
            -size * 0.02,
            tint(theme::blue(), 0.20),
            theme::white(),
        ),
        FamiliarKind::Puppy => (
            origin + vec2(size * 0.52, -size * 0.13),
            size * 0.072,
            -size * 0.035,
            tint(theme::gold(), 0.19),
            theme::white(),
        ),
    };

    for direction in [-1.0_f32, 1.0] {
        let eye = head + vec2(eye_dx * direction, eye_y);
        painter.circle_filled(eye, 3.2, cover);
        painter.line_segment(
            [eye + vec2(-2.2, 0.2), eye + vec2(2.2, 0.2)],
            Stroke::new(1.15, line_color),
        );
    }
}

fn draw_sleep_marks(painter: &Painter, origin: Pos2, size: f32, accent: Color32, elapsed: f32) {
    let bob = (elapsed * 1.8).sin() * 2.0;

    painter.text(
        origin + vec2(size * 0.68, -size * 0.76 + bob),
        Align2::CENTER_CENTER,
        "z",
        FontId::proportional(10.0),
        tint(accent, 0.74),
    );

    painter.text(
        origin + vec2(size * 0.90, -size * 1.02 - bob * 0.35),
        Align2::CENTER_CENTER,
        "z",
        FontId::proportional(13.0),
        tint(accent, 0.52),
    );
}

fn draw_feed_effect(painter: &Painter, origin: Pos2, size: f32, age: f32, accent: Color32) {
    if age < 0.72 {
        let t = smoothstep((age / 0.72).clamp(0.0, 1.0));
        let start = origin + vec2(size * 0.15, -size * 1.20);
        let end = origin + vec2(size * 0.35, -size * 0.18);
        let snack = lerp_pos(start, end, t);

        painter.circle_filled(snack, 3.8, theme::gold());
        painter.circle_stroke(snack, 3.8, Stroke::new(1.0, theme::white()));
    }

    if age > 0.35 {
        let heart_age = age - 0.35;

        for index in 0..3 {
            let delay = index as f32 * 0.24;
            let local = heart_age - delay;

            if !(0.0..=1.6).contains(&local) {
                continue;
            }

            let rise = local * size * 0.62;
            let sway = (local * 5.5 + index as f32 * 1.7).sin() * size * 0.12;
            let scale = (1.0 - local / 1.6).clamp(0.35, 1.0);

            draw_heart(
                painter,
                origin
                    + vec2(
                        size * (0.05 + index as f32 * 0.16) + sway,
                        -size * 0.62 - rise,
                    ),
                4.8 * scale,
                tint(accent, 0.82),
            );
        }
    }

    if age < 1.05 {
        painter.text(
            origin + vec2(0.0, -size * 1.08),
            Align2::CENTER_BOTTOM,
            "nom!",
            FontId::proportional(10.0),
            theme::white(),
        );
    }
}

fn draw_heart(painter: &Painter, center: Pos2, size: f32, color: Color32) {
    let radius = size * 0.34;

    painter.circle_filled(center + vec2(-radius * 0.62, -radius * 0.18), radius, color);
    painter.circle_filled(center + vec2(radius * 0.62, -radius * 0.18), radius, color);

    painter.add(Shape::convex_polygon(
        vec![
            center + vec2(-size * 0.56, -size * 0.02),
            center + vec2(size * 0.56, -size * 0.02),
            center + vec2(0.0, size * 0.72),
        ],
        color,
        Stroke::NONE,
    ));
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

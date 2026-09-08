use eframe::egui::{
    self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2,
};

use crate::{
    model::{ComponentId, SystemSnapshot, TruthLevel},
    theme,
};

fn lerp(a: Pos2, b: Pos2, t: f32) -> Pos2 {
    Pos2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}

fn activity_color(value: f32, base: Color32) -> Color32 {
    let amount = (value / 100.0).clamp(0.0, 1.0);
    Color32::from_rgb(
        (base.r() as f32 * (0.36 + amount * 0.64)) as u8,
        (base.g() as f32 * (0.36 + amount * 0.64)) as u8,
        (base.b() as f32 * (0.36 + amount * 0.64)) as u8,
    )
}

fn node(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    rect: Rect,
    label: &str,
    subtitle: &str,
    selected: bool,
    accent: Color32,
) -> egui::Response {
    let response = ui.allocate_rect(rect, Sense::click());

    painter.rect_filled(
        rect,
        7.0,
        if selected { theme::selected_panel() } else { theme::panel() },
    );
    painter.rect_stroke(
        rect,
        7.0,
        Stroke::new(if selected { 2.0 } else { 1.0 }, if selected { accent } else { theme::border() }),
        StrokeKind::Inside,
    );

    painter.line_segment(
        [
            Pos2::new(rect.left() + 12.0, rect.top() + 11.0),
            Pos2::new(rect.right() - 12.0, rect.top() + 11.0),
        ],
        Stroke::new(2.0, accent),
    );

    painter.text(
        Pos2::new(rect.left() + 12.0, rect.top() + 24.0),
        Align2::LEFT_TOP,
        label,
        FontId::monospace(15.0),
        theme::white(),
    );
    painter.text(
        Pos2::new(rect.left() + 12.0, rect.top() + 44.0),
        Align2::LEFT_TOP,
        subtitle,
        FontId::monospace(10.0),
        theme::muted(),
    );

    response
}

fn flow_particles(
    painter: &egui::Painter,
    start: Pos2,
    end: Pos2,
    elapsed: f32,
    rate: f32,
    color: Color32,
) {
    let count = (3.0 + rate.sqrt() * 0.42).clamp(3.0, 12.0) as usize;
    painter.line_segment([start, end], Stroke::new(1.0, theme::border()));

    for i in 0..count {
        let phase = (elapsed * (0.15 + rate * 0.0007) + i as f32 / count as f32).fract();
        let p = lerp(start, end, phase);
        let radius = 1.8 + (rate / 1500.0).clamp(0.0, 1.0) * 1.8;
        painter.circle_filled(p, radius, color);
    }
}

pub fn machine_view(
    ui: &mut egui::Ui,
    snapshot: &SystemSnapshot,
    elapsed: f32,
    selected: &mut ComponentId,
    descend: bool,
) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("THE MACHINE")
                .size(18.0)
                .strong()
                .color(theme::white()),
        );
        ui.label(
            egui::RichText::new("◇ LOGICAL / SCHEMATIC")
                .monospace()
                .size(10.0)
                .color(theme::muted()),
        );
    });
    ui.label(
        egui::RichText::new("Observed activity animates the schematic. Physical placement is not implied.")
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(8.0);

    let height = if descend { 650.0 } else { 555.0 };
    let width = ui.available_width().max(680.0);
    let (response, painter) = ui.allocate_painter(Vec2::new(width, height), Sense::hover());
    let area = response.rect.shrink(4.0);

    painter.rect_filled(area, 10.0, theme::deep_bg());
    painter.rect_stroke(area, 10.0, Stroke::new(1.0, theme::border()), StrokeKind::Inside);

    let cpu = Rect::from_min_size(
        Pos2::new(area.left() + area.width() * 0.25, area.top() + 30.0),
        Vec2::new(area.width() * 0.50, if descend { 205.0 } else { 174.0 }),
    );
    let ram = Rect::from_min_size(
        Pos2::new(area.left() + 34.0, area.top() + if descend { 302.0 } else { 268.0 }),
        Vec2::new(area.width() * 0.29, 150.0),
    );
    let gpu = Rect::from_min_size(
        Pos2::new(area.right() - area.width() * 0.29 - 34.0, ram.top()),
        Vec2::new(area.width() * 0.29, 150.0),
    );
    let storage = Rect::from_min_size(
        Pos2::new(area.left() + area.width() * 0.19, area.bottom() - 100.0),
        Vec2::new(area.width() * 0.23, 72.0),
    );
    let network = Rect::from_min_size(
        Pos2::new(area.right() - area.width() * 0.23 - area.width() * 0.19, storage.top()),
        Vec2::new(area.width() * 0.23, 72.0),
    );

    let cpu_bottom = Pos2::new(cpu.center().x, cpu.bottom());
    let hub = Pos2::new(cpu.center().x, ram.top() - 30.0);
    let ram_top = Pos2::new(ram.center().x, ram.top());
    let gpu_top = Pos2::new(gpu.center().x, gpu.top());
    let storage_top = Pos2::new(storage.center().x, storage.top());
    let network_top = Pos2::new(network.center().x, network.top());

    flow_particles(&painter, cpu_bottom, hub, elapsed, snapshot.cpu.package_usage * 10.0, theme::pink());
    flow_particles(&painter, hub, ram_top, elapsed + 0.5, snapshot.memory.page_faults_per_second as f32, theme::blue());
    flow_particles(&painter, hub, gpu_top, elapsed + 0.2, snapshot.gpu.pcie_rx_mib_s + snapshot.gpu.pcie_tx_mib_s, theme::violet());
    flow_particles(&painter, storage_top, ram.center_bottom(), elapsed + 0.8, snapshot.storage.read_mib_s + snapshot.storage.write_mib_s, theme::white());
    flow_particles(&painter, network_top, gpu.center_bottom(), elapsed + 1.2, (snapshot.network.rx_mib_s + snapshot.network.tx_mib_s) * 12.0, theme::green());

    let cpu_response = node(
        ui, &painter, cpu, "CPU // LOGICAL TOPOLOGY",
        &format!("{:.1}% package  •  {:.1}°C  •  {} logical CPUs", snapshot.cpu.package_usage, snapshot.cpu.package_temperature_c, snapshot.cpu.logical_cpus.len()),
        *selected == ComponentId::Cpu, theme::pink(),
    );
    let ram_response = node(
        ui, &painter, ram, "RAM // MEMORY",
        &format!("{} used", crate::model::format_bytes(snapshot.memory.used_bytes)),
        *selected == ComponentId::Memory, theme::blue(),
    );
    let gpu_response = node(
        ui, &painter, gpu, "GPU // COMPUTE",
        &format!("{:.1}%  •  {:.1}°C", snapshot.gpu.utilization, snapshot.gpu.temperature_c),
        *selected == ComponentId::Gpu, theme::violet(),
    );
    let storage_response = node(
        ui, &painter, storage, "NVMe // STORAGE",
        &format!("R {:>6.1}  W {:>6.1} MiB/s", snapshot.storage.read_mib_s, snapshot.storage.write_mib_s),
        *selected == ComponentId::Storage, theme::white(),
    );
    let network_response = node(
        ui, &painter, network, "NETWORK",
        &format!("RX {:>5.1}  TX {:>5.1} MiB/s", snapshot.network.rx_mib_s, snapshot.network.tx_mib_s),
        *selected == ComponentId::Network, theme::green(),
    );

    if cpu_response.clicked() { *selected = ComponentId::Cpu; }
    if ram_response.clicked() { *selected = ComponentId::Memory; }
    if gpu_response.clicked() { *selected = ComponentId::Gpu; }
    if storage_response.clicked() { *selected = ComponentId::Storage; }
    if network_response.clicked() { *selected = ComponentId::Network; }

    let cols = 8;
    let gap = 5.0;
    let inner = Rect::from_min_max(
        Pos2::new(cpu.left() + 12.0, cpu.top() + 68.0),
        Pos2::new(cpu.right() - 12.0, cpu.bottom() - 12.0),
    );
    let cell_w = (inner.width() - gap * (cols as f32 - 1.0)) / cols as f32;
    let rows = (snapshot.cpu.logical_cpus.len() + cols - 1) / cols;
    let cell_h = ((inner.height() - gap * (rows.saturating_sub(1)) as f32) / rows.max(1) as f32).max(26.0);

    for (i, core) in snapshot.cpu.logical_cpus.iter().enumerate() {
        let row = i / cols;
        let col = i % cols;
        let rect = Rect::from_min_size(
            Pos2::new(inner.left() + col as f32 * (cell_w + gap), inner.top() + row as f32 * (cell_h + gap)),
            Vec2::new(cell_w, cell_h),
        );
        let color = activity_color(core.usage, theme::pink());
        painter.rect_filled(rect, 3.0, theme::cell_bg());
        let filled = Rect::from_min_max(
            Pos2::new(rect.left(), rect.bottom() - rect.height() * (core.usage / 100.0)),
            rect.right_bottom(),
        );
        painter.rect_filled(filled, 3.0, color);
        painter.rect_stroke(rect, 3.0, Stroke::new(1.0, color), StrokeKind::Inside);
        painter.text(rect.center(), Align2::CENTER_CENTER, format!("{:02}\n{:02.0}%", core.logical_id, core.usage), FontId::monospace(if descend { 9.0 } else { 8.0 }), theme::white());
    }

    let ram_inner = Rect::from_min_max(
        Pos2::new(ram.left() + 12.0, ram.top() + 76.0),
        Pos2::new(ram.right() - 12.0, ram.bottom() - 16.0),
    );
    let total = snapshot.memory.total_bytes.max(1) as f32;
    let used = snapshot.memory.used_bytes as f32 / total;
    painter.rect_filled(ram_inner, 3.0, theme::track_bg());
    let used_rect = Rect::from_min_size(ram_inner.min, Vec2::new(ram_inner.width() * used, ram_inner.height()));
    painter.rect_filled(used_rect, 3.0, theme::blue());
    painter.rect_stroke(ram_inner, 3.0, Stroke::new(1.0, theme::border()), StrokeKind::Inside);

    let grid = Rect::from_min_max(
        Pos2::new(gpu.left() + 12.0, gpu.top() + 76.0),
        Pos2::new(gpu.right() - 12.0, gpu.bottom() - 16.0),
    );
    let gcols = 12;
    let grows = 4;
    let cg = 3.0;
    let cw = (grid.width() - cg * (gcols - 1) as f32) / gcols as f32;
    let ch = (grid.height() - cg * (grows - 1) as f32) / grows as f32;
    let active_cells = ((gcols * grows) as f32 * snapshot.gpu.utilization / 100.0) as usize;

    for i in 0..gcols * grows {
        let row = i / gcols;
        let col = i % gcols;
        let rect = Rect::from_min_size(
            Pos2::new(grid.left() + col as f32 * (cw + cg), grid.top() + row as f32 * (ch + cg)),
            Vec2::new(cw, ch),
        );
        let pulse = ((elapsed * 2.0 + i as f32 * 0.19).sin() * 0.5 + 0.5) * 0.35 + 0.65;
        let color = if i < active_cells {
            Color32::from_rgb((theme::violet().r() as f32 * pulse) as u8, (theme::violet().g() as f32 * pulse) as u8, (theme::violet().b() as f32 * pulse) as u8)
        } else {
            theme::grid_inactive()
        };
        painter.rect_filled(rect, 2.0, color);
    }

    if descend {
        for (i, process) in snapshot.processes.iter().take(4).enumerate() {
            let target_index = process.current_cpu.min(snapshot.cpu.logical_cpus.len().saturating_sub(1));
            let row = target_index / cols;
            let col = target_index % cols;
            let target = Pos2::new(
                inner.left() + col as f32 * (cell_w + gap) + cell_w * 0.5,
                inner.top() + row as f32 * (cell_h + gap) + cell_h * 0.5,
            );
            let orbit = Pos2::new(cpu.left() + 24.0 + i as f32 * 115.0, cpu.bottom() - 17.0);
            painter.line_segment([orbit, target], Stroke::new(1.0, theme::blue()));
            painter.circle_filled(orbit, 4.0, theme::blue());
            painter.text(Pos2::new(orbit.x + 7.0, orbit.y), Align2::LEFT_CENTER, &process.name, FontId::monospace(9.0), theme::muted());
        }
    }

    let truth_y = area.bottom() - 13.0;
    let truths = [
        (TruthLevel::Observed, "events"),
        (TruthLevel::Sampled, "utilization"),
        (TruthLevel::Inferred, "flow"),
        (TruthLevel::Schematic, "layout"),
    ];
    let mut x = area.left() + 12.0;
    for (level, noun) in truths {
        painter.text(Pos2::new(x, truth_y), Align2::LEFT_CENTER, format!("{} {} {}", level.glyph(), level.label(), noun), FontId::monospace(9.0), theme::truth_color(level));
        x += 136.0;
    }
}

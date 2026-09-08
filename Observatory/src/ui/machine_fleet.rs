use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};

use crate::{
    fleet::{CpuPackageTopology, FleetMachine},
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

    let text_painter = painter.with_clip_rect(rect.shrink(8.0));

    painter.rect_filled(
        rect,
        7.0,
        if selected {
            theme::selected_panel()
        } else {
            theme::panel()
        },
    );

    painter.rect_stroke(
        rect,
        7.0,
        Stroke::new(
            if selected { 2.0 } else { 1.0 },
            if selected { accent } else { theme::border() },
        ),
        StrokeKind::Inside,
    );

    painter.line_segment(
        [
            Pos2::new(rect.left() + 12.0, rect.top() + 11.0),
            Pos2::new(rect.right() - 12.0, rect.top() + 11.0),
        ],
        Stroke::new(2.0, accent),
    );

    text_painter.text(
        Pos2::new(rect.left() + 12.0, rect.top() + 24.0),
        Align2::LEFT_TOP,
        label,
        FontId::monospace(15.0),
        theme::white(),
    );

    text_painter.text(
        Pos2::new(rect.left() + 12.0, rect.top() + 42.0),
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

        painter.circle_filled(
            lerp(start, end, phase),
            1.8 + (rate / 1500.0).clamp(0.0, 1.0) * 1.8,
            color,
        );
    }
}

pub fn machine_view(
    ui: &mut egui::Ui,
    machine: &FleetMachine,
    elapsed: f32,
    selected: &mut ComponentId,
    descend: bool,
) {
    let snapshot = &machine.system;
    let package_count = machine.cpu_packages.len().max(1);
    let multi_package = package_count > 1;

    ui.horizontal_wrapped(|ui| {
        ui.label(
            egui::RichText::new("THE MACHINE")
                .size(18.0)
                .strong()
                .color(theme::white()),
        );

        ui.label(
            egui::RichText::new(format!(
                "◇ {} // {}",
                machine.name.to_uppercase(),
                machine.origin.label()
            ))
            .monospace()
            .size(10.0)
            .color(theme::muted()),
        );
    });

    ui.label(
        egui::RichText::new(
            "Logical topology follows the selected Fleet target. Package placement is schematic.",
        )
        .size(11.0)
        .color(theme::muted()),
    );

    ui.add_space(8.0);

    let height = if multi_package {
        if descend { 720.0 } else { 625.0 }
    } else if descend {
        650.0
    } else {
        555.0
    };

    let width = ui.available_width().max(680.0);

    let (response, painter) = ui.allocate_painter(Vec2::new(width, height), Sense::hover());

    let area = response.rect.shrink(4.0);

    painter.rect_filled(area, 10.0, theme::deep_bg());

    painter.rect_stroke(
        area,
        10.0,
        Stroke::new(1.0, theme::border()),
        StrokeKind::Inside,
    );

    let cpu_width = if multi_package {
        area.width() * 0.72
    } else {
        area.width() * 0.50
    };

    let cpu_height = if multi_package {
        if package_count > 2 { 290.0 } else { 230.0 }
    } else if descend {
        205.0
    } else {
        174.0
    };

    let cpu = Rect::from_min_size(
        Pos2::new(area.center().x - cpu_width * 0.5, area.top() + 30.0),
        Vec2::new(cpu_width, cpu_height),
    );

    let ram_top_y = cpu.bottom() + if multi_package { 58.0 } else { 64.0 };

    let ram = Rect::from_min_size(
        Pos2::new(area.left() + 34.0, ram_top_y),
        Vec2::new(area.width() * 0.29, 150.0),
    );

    let gpu = Rect::from_min_size(
        Pos2::new(area.right() - area.width() * 0.29 - 34.0, ram.top()),
        Vec2::new(area.width() * 0.29, 150.0),
    );

    let npu = snapshot.npu.available.then(|| {
        Rect::from_min_size(
            Pos2::new(area.center().x - area.width() * 0.11, ram.top()),
            Vec2::new(area.width() * 0.22, 150.0),
        )
    });

    let storage = Rect::from_min_size(
        Pos2::new(area.left() + area.width() * 0.19, area.bottom() - 112.0),
        Vec2::new(area.width() * 0.23, 84.0),
    );

    let network = Rect::from_min_size(
        Pos2::new(
            area.right() - area.width() * 0.23 - area.width() * 0.19,
            storage.top(),
        ),
        Vec2::new(area.width() * 0.23, 84.0),
    );

    let cpu_bottom = Pos2::new(cpu.center().x, cpu.bottom());

    let hub = Pos2::new(cpu.center().x, ram.top() - 30.0);

    let ram_top = Pos2::new(ram.center().x, ram.top());

    let gpu_top = Pos2::new(gpu.center().x, gpu.top());

    let npu_top = npu.map(|rect| Pos2::new(rect.center().x, rect.top()));

    let storage_top = Pos2::new(storage.center().x, storage.top());

    let network_top = Pos2::new(network.center().x, network.top());

    flow_particles(
        &painter,
        cpu_bottom,
        hub,
        elapsed,
        snapshot.cpu.package_usage * 10.0,
        theme::pink(),
    );

    flow_particles(
        &painter,
        hub,
        ram_top,
        elapsed + 0.5,
        snapshot.memory.page_faults_per_second as f32,
        theme::blue(),
    );

    if snapshot.gpu.available {
        let pcie_activity =
            snapshot.gpu.pcie_rx_mib_s.unwrap_or(0.0) + snapshot.gpu.pcie_tx_mib_s.unwrap_or(0.0);

        flow_particles(
            &painter,
            hub,
            gpu_top,
            elapsed + 0.2,
            pcie_activity,
            theme::violet(),
        );
    }

    if let Some(npu_top) = npu_top {
        flow_particles(
            &painter,
            hub,
            npu_top,
            elapsed + 0.35,
            snapshot.npu.utilization * 10.0,
            theme::gold(),
        );
    }

    if snapshot.storage.available && snapshot.storage.rates_available {
        flow_particles(
            &painter,
            storage_top,
            ram.center_bottom(),
            elapsed + 0.8,
            snapshot.storage.read_mib_s + snapshot.storage.write_mib_s,
            theme::white(),
        );
    }

    if snapshot.network.available && snapshot.network.rates_available {
        flow_particles(
            &painter,
            network_top,
            gpu.center_bottom(),
            elapsed + 1.2,
            (snapshot.network.rx_mib_s + snapshot.network.tx_mib_s) * 12.0,
            theme::green(),
        );
    }

    let cpu_label = if multi_package {
        "CPU // MULTI-PACKAGE TOPOLOGY"
    } else {
        "CPU // LOGICAL TOPOLOGY"
    };

    let cpu_subtitle = if multi_package {
        format!(
            "{}\n{:.1}% aggregate  •  {} packages  •  {} logical CPUs",
            snapshot.cpu.model,
            snapshot.cpu.package_usage,
            package_count,
            snapshot.cpu.logical_cpus.len()
        )
    } else {
        format!(
            "{}\n{:.1}% package  •  {} logical CPUs",
            snapshot.cpu.model,
            snapshot.cpu.package_usage,
            snapshot.cpu.logical_cpus.len()
        )
    };

    let cpu_response = node(
        ui,
        &painter,
        cpu,
        cpu_label,
        &cpu_subtitle,
        *selected == ComponentId::Cpu,
        theme::pink(),
    );

    let ram_response = node(
        ui,
        &painter,
        ram,
        "RAM // MEMORY",
        &format!(
            "{} used",
            crate::model::format_bytes(snapshot.memory.used_bytes)
        ),
        *selected == ComponentId::Memory,
        theme::blue(),
    );

    // Bind the formatted subtitle so its String lives through the node() call.
    // Returning `&format!(...)` from the `if` expression creates a reference to
    // a temporary String that is dropped before node() can borrow it.
    let gpu_subtitle = if snapshot.gpu.available {
        format!(
            "{}\n{:.1}%  •  {:.1}°C  •  {} VRAM",
            snapshot.gpu.model,
            snapshot.gpu.utilization,
            snapshot.gpu.temperature_c,
            crate::model::format_bytes(snapshot.gpu.vram_used_bytes),
        )
    } else {
        "not present / not exposed".to_string()
    };

    let gpu_response = node(
        ui,
        &painter,
        gpu,
        "GPU // COMPUTE",
        &gpu_subtitle,
        *selected == ComponentId::Gpu,
        theme::violet(),
    );

    let npu_response = npu.map(|rect| {
        let subtitle = format!(
            "{:.1}%  •  {} / {} MHz",
            snapshot.npu.utilization,
            snapshot.npu.current_frequency_mhz,
            snapshot.npu.max_frequency_mhz
        );

        node(
            ui,
            &painter,
            rect,
            "NPU // AI ENGINE",
            &subtitle,
            *selected == ComponentId::Npu,
            theme::gold(),
        )
    });

    let storage_subtitle = if !snapshot.storage.available {
        format!("{}\ntelemetry unavailable", snapshot.storage.model,)
    } else if snapshot.storage.rates_available {
        format!(
            "{}\nR {:.2} / W {:.2} MiB/s",
            snapshot.storage.model, snapshot.storage.read_mib_s, snapshot.storage.write_mib_s,
        )
    } else {
        format!("{}\nsampling baseline", snapshot.storage.model,)
    };

    let storage_response = node(
        ui,
        &painter,
        storage,
        "STORAGE",
        &storage_subtitle,
        *selected == ComponentId::Storage,
        theme::white(),
    );

    let network_subtitle = if !snapshot.network.available {
        format!("{}\ntelemetry unavailable", snapshot.network.interface,)
    } else if snapshot.network.rates_available {
        format!(
            "{}\nRX {:.2} / TX {:.2} MiB/s",
            snapshot.network.interface, snapshot.network.rx_mib_s, snapshot.network.tx_mib_s,
        )
    } else {
        format!("{}\nsampling baseline", snapshot.network.interface,)
    };

    let network_response = node(
        ui,
        &painter,
        network,
        "NETWORK",
        &network_subtitle,
        *selected == ComponentId::Network,
        theme::green(),
    );

    if cpu_response.clicked() {
        *selected = ComponentId::Cpu;
    }
    if ram_response.clicked() {
        *selected = ComponentId::Memory;
    }
    if gpu_response.clicked() {
        *selected = ComponentId::Gpu;
    }
    if npu_response
        .as_ref()
        .map(|response| response.clicked())
        .unwrap_or(false)
    {
        *selected = ComponentId::Npu;
    }
    if storage_response.clicked() {
        *selected = ComponentId::Storage;
    }
    if network_response.clicked() {
        *selected = ComponentId::Network;
    }

    draw_cpu_packages(&painter, cpu, snapshot, &machine.cpu_packages, descend);

    draw_ram(&painter, ram, snapshot);
    draw_gpu(&painter, gpu, snapshot, elapsed);

    if let Some(npu_rect) = npu {
        draw_npu(&painter, npu_rect, snapshot);
    }

    if descend {
        draw_process_tokens(&painter, cpu, snapshot, &machine.cpu_packages);
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
        painter.text(
            Pos2::new(x, truth_y),
            Align2::LEFT_CENTER,
            format!("{} {} {}", level.glyph(), level.label(), noun),
            FontId::monospace(9.0),
            theme::truth_color(level),
        );

        x += 136.0;
    }
}

fn draw_npu(painter: &egui::Painter, rect: Rect, snapshot: &SystemSnapshot) {
    let meter = Rect::from_min_max(
        Pos2::new(rect.left() + 12.0, rect.bottom() - 34.0),
        Pos2::new(rect.right() - 12.0, rect.bottom() - 16.0),
    );

    painter.rect_filled(meter, 3.0, theme::track_bg());

    let fill = Rect::from_min_size(
        meter.min,
        Vec2::new(
            meter.width() * (snapshot.npu.utilization / 100.0).clamp(0.0, 1.0),
            meter.height(),
        ),
    );

    painter.rect_filled(fill, 3.0, theme::gold());
    painter.rect_stroke(
        meter,
        3.0,
        Stroke::new(1.0, theme::border()),
        StrokeKind::Inside,
    );
}

fn draw_cpu_packages(
    painter: &egui::Painter,
    cpu_rect: Rect,
    snapshot: &SystemSnapshot,
    packages: &[CpuPackageTopology],
    descend: bool,
) {
    let fallback;

    let packages = if packages.is_empty() {
        fallback = vec![CpuPackageTopology {
            package_id: 0,
            model: snapshot.cpu.model.clone(),
            physical_cores: snapshot.cpu.logical_cpus.len(),
            logical_cpu_ids: snapshot
                .cpu
                .logical_cpus
                .iter()
                .map(|cpu| cpu.logical_id)
                .collect(),
        }];

        &fallback
    } else {
        packages
    };

    let inner = Rect::from_min_max(
        Pos2::new(cpu_rect.left() + 12.0, cpu_rect.top() + 82.0),
        Pos2::new(cpu_rect.right() - 12.0, cpu_rect.bottom() - 12.0),
    );

    let package_cols = if packages.len() <= 1 { 1 } else { 2 };

    let package_rows = (packages.len() + package_cols - 1) / package_cols;

    let gap = 8.0;

    let package_width =
        (inner.width() - gap * (package_cols.saturating_sub(1)) as f32) / package_cols as f32;

    let package_height = (inner.height() - gap * (package_rows.saturating_sub(1)) as f32)
        / package_rows.max(1) as f32;

    for (index, package) in packages.iter().enumerate() {
        let col = index % package_cols;
        let row = index / package_cols;

        let rect = Rect::from_min_size(
            Pos2::new(
                inner.left() + col as f32 * (package_width + gap),
                inner.top() + row as f32 * (package_height + gap),
            ),
            Vec2::new(package_width, package_height),
        );

        let package_usage = average_package_usage(snapshot, &package.logical_cpu_ids);

        painter.rect_filled(rect, 4.0, theme::cell_bg());

        painter.rect_stroke(
            rect,
            4.0,
            Stroke::new(1.0, activity_color(package_usage, theme::pink())),
            StrokeKind::Inside,
        );

        painter.text(
            rect.left_top() + Vec2::new(8.0, 7.0),
            Align2::LEFT_TOP,
            format!(
                "PKG {} // {}C // {}T // {:.0}%",
                package.package_id,
                package.physical_cores,
                package.logical_cpu_ids.len(),
                package_usage
            ),
            FontId::monospace(8.8),
            theme::white(),
        );

        draw_package_cores(painter, rect, snapshot, package, descend);
    }
}

fn draw_package_cores(
    painter: &egui::Painter,
    package_rect: Rect,
    snapshot: &SystemSnapshot,
    package: &CpuPackageTopology,
    descend: bool,
) {
    let ids = &package.logical_cpu_ids;

    if ids.is_empty() {
        return;
    }

    let core_area = Rect::from_min_max(
        Pos2::new(package_rect.left() + 7.0, package_rect.top() + 28.0),
        Pos2::new(package_rect.right() - 7.0, package_rect.bottom() - 7.0),
    );

    let cols = if ids.len() <= 8 { ids.len().max(1) } else { 8 };

    let rows = (ids.len() + cols - 1) / cols;

    let gap = 3.0;

    let cell_w = (core_area.width() - gap * (cols.saturating_sub(1)) as f32) / cols as f32;

    let cell_h = (core_area.height() - gap * (rows.saturating_sub(1)) as f32) / rows.max(1) as f32;

    for (index, logical_id) in ids.iter().enumerate() {
        let Some(core) = snapshot
            .cpu
            .logical_cpus
            .iter()
            .find(|core| core.logical_id == *logical_id)
        else {
            continue;
        };

        let col = index % cols;
        let row = index / cols;

        let rect = Rect::from_min_size(
            Pos2::new(
                core_area.left() + col as f32 * (cell_w + gap),
                core_area.top() + row as f32 * (cell_h + gap),
            ),
            Vec2::new(cell_w, cell_h),
        );

        let color = activity_color(core.usage, theme::pink());

        painter.rect_filled(rect, 2.0, theme::track_bg());

        let filled = Rect::from_min_max(
            Pos2::new(
                rect.left(),
                rect.bottom() - rect.height() * (core.usage / 100.0),
            ),
            rect.right_bottom(),
        );

        painter.rect_filled(filled, 2.0, color);

        painter.rect_stroke(rect, 2.0, Stroke::new(0.8, color), StrokeKind::Inside);

        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            if descend && cell_h >= 20.0 {
                format!("{:02}\n{:02.0}%", core.logical_id, core.usage)
            } else {
                format!("{:02}", core.logical_id)
            },
            FontId::monospace(if cell_h < 15.0 { 6.5 } else { 7.5 }),
            theme::white(),
        );
    }
}

fn average_package_usage(snapshot: &SystemSnapshot, ids: &[usize]) -> f32 {
    let mut total = 0.0;
    let mut count = 0;

    for id in ids {
        if let Some(cpu) = snapshot
            .cpu
            .logical_cpus
            .iter()
            .find(|cpu| cpu.logical_id == *id)
        {
            total += cpu.usage;
            count += 1;
        }
    }

    if count == 0 {
        0.0
    } else {
        total / count as f32
    }
}

fn draw_ram(painter: &egui::Painter, ram: Rect, snapshot: &SystemSnapshot) {
    let inner = Rect::from_min_max(
        Pos2::new(ram.left() + 12.0, ram.top() + 76.0),
        Pos2::new(ram.right() - 12.0, ram.bottom() - 16.0),
    );

    let total = snapshot.memory.total_bytes.max(1) as f32;

    let used = snapshot.memory.used_bytes as f32 / total;

    painter.rect_filled(inner, 3.0, theme::track_bg());

    painter.rect_filled(
        Rect::from_min_size(inner.min, Vec2::new(inner.width() * used, inner.height())),
        3.0,
        theme::blue(),
    );

    painter.rect_stroke(
        inner,
        3.0,
        Stroke::new(1.0, theme::border()),
        StrokeKind::Inside,
    );
}

fn draw_gpu(painter: &egui::Painter, gpu: Rect, snapshot: &SystemSnapshot, elapsed: f32) {
    let grid = Rect::from_min_max(
        Pos2::new(gpu.left() + 12.0, gpu.top() + 76.0),
        Pos2::new(gpu.right() - 12.0, gpu.bottom() - 16.0),
    );

    if !snapshot.gpu.available {
        painter.text(
            grid.center(),
            Align2::CENTER_CENTER,
            "NO GPU\nTELEMETRY",
            FontId::monospace(10.0),
            theme::muted(),
        );

        return;
    }

    let cols = 12;
    let rows = 4;
    let gap = 3.0;

    let cell_w = (grid.width() - gap * (cols - 1) as f32) / cols as f32;

    let cell_h = (grid.height() - gap * (rows - 1) as f32) / rows as f32;

    let active = ((cols * rows) as f32 * snapshot.gpu.utilization / 100.0) as usize;

    for i in 0..cols * rows {
        let row = i / cols;
        let col = i % cols;

        let rect = Rect::from_min_size(
            Pos2::new(
                grid.left() + col as f32 * (cell_w + gap),
                grid.top() + row as f32 * (cell_h + gap),
            ),
            Vec2::new(cell_w, cell_h),
        );

        let pulse = ((elapsed * 2.0 + i as f32 * 0.19).sin() * 0.5 + 0.5) * 0.35 + 0.65;

        let color = if i < active {
            Color32::from_rgb(
                (theme::violet().r() as f32 * pulse) as u8,
                (theme::violet().g() as f32 * pulse) as u8,
                (theme::violet().b() as f32 * pulse) as u8,
            )
        } else {
            theme::grid_inactive()
        };

        painter.rect_filled(rect, 2.0, color);
    }
}

fn draw_process_tokens(
    painter: &egui::Painter,
    cpu: Rect,
    snapshot: &SystemSnapshot,
    packages: &[CpuPackageTopology],
) {
    if snapshot.processes.is_empty() {
        return;
    }

    // For multi-package systems, annotate the process with the package
    // containing its last observed logical CPU rather than pretending to know a
    // physical silicon coordinate.
    for (index, process) in snapshot.processes.iter().take(5).enumerate() {
        let package_id = packages
            .iter()
            .find(|package| package.logical_cpu_ids.contains(&process.last_cpu))
            .map(|package| package.package_id)
            .unwrap_or(0);

        let origin = Pos2::new(
            cpu.left() + 24.0 + index as f32 * 102.0,
            cpu.bottom() - 12.0,
        );

        painter.circle_filled(origin, 3.5, theme::blue());

        painter.text(
            Pos2::new(origin.x + 6.0, origin.y),
            Align2::LEFT_CENTER,
            format!(
                "{} // P{} CPU{:02}",
                process.name, package_id, process.last_cpu
            ),
            FontId::monospace(7.5),
            theme::muted(),
        );
    }
}

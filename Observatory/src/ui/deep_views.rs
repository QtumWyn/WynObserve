use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};

use crate::{
    analysis::AnalysisSnapshot,
    fleet::FleetState,
    model::{SystemSnapshot, TruthLevel, format_bytes},
    theme,
};

fn page_heading(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.label(
        egui::RichText::new(title)
            .size(19.0)
            .strong()
            .color(theme::white()),
    );

    ui.label(egui::RichText::new(subtitle).color(theme::muted()));

    ui.add_space(10.0);
}

fn truth(ui: &mut egui::Ui, level: TruthLevel) {
    ui.label(
        egui::RichText::new(format!("{} {}", level.glyph(), level.label()))
            .monospace()
            .size(10.0)
            .color(theme::truth_color(level)),
    );
}

fn bar(ui: &mut egui::Ui, fraction: f32, label: &str, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 21.0), Sense::hover());

    ui.painter().rect_filled(rect, 3.0, theme::track_bg());

    let fill = Rect::from_min_size(
        rect.min,
        Vec2::new(rect.width() * fraction.clamp(0.0, 1.0), rect.height()),
    );

    ui.painter().rect_filled(fill, 3.0, color);

    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::monospace(10.0),
        theme::white(),
    );
}

pub fn replay_view(ui: &mut egui::Ui, system: &SystemSnapshot, elapsed: f32) {
    page_heading(
        ui,
        "TIME MACHINE // TELEMETRY REPLAY",
        "rolling history ready for pause, scrub, replay, bookmarks and incident capture",
    );

    let data = AnalysisSnapshot::mock(system, elapsed);

    ui.horizontal(|ui| {
        ui.button("◀ 5s");
        ui.button("◀ FRAME");
        ui.button("Ⅱ PAUSE");
        ui.button("FRAME ▶");
        ui.button("5s ▶");
        ui.separator();
        ui.label(
            egui::RichText::new("LIVE BUFFER // 36.0s")
                .monospace()
                .color(theme::blue()),
        );
        ui.button("★ BOOKMARK");
    });

    ui.add_space(8.0);

    let (response, painter) =
        ui.allocate_painter(Vec2::new(ui.available_width(), 420.0), Sense::hover());

    let area = response.rect.shrink(8.0);

    painter.rect_filled(area, 6.0, theme::terminal_bg());
    painter.rect_stroke(
        area,
        6.0,
        Stroke::new(1.0, theme::border()),
        StrokeKind::Inside,
    );

    let plot = Rect::from_min_max(
        Pos2::new(area.left() + 45.0, area.top() + 34.0),
        Pos2::new(area.right() - 15.0, area.bottom() - 45.0),
    );

    for i in 0..5 {
        let y = plot.top() + i as f32 * plot.height() / 4.0;
        painter.line_segment(
            [Pos2::new(plot.left(), y), Pos2::new(plot.right(), y)],
            Stroke::new(0.6, theme::border()),
        );
    }

    let mut cpu_points = Vec::new();
    let mut mem_points = Vec::new();

    for (i, point) in data.timeline.iter().enumerate() {
        let x = plot.left() + i as f32 / (data.timeline.len() - 1).max(1) as f32 * plot.width();

        cpu_points.push(Pos2::new(
            x,
            plot.bottom() - point.cpu_percent / 100.0 * plot.height(),
        ));

        mem_points.push(Pos2::new(
            x,
            plot.bottom() - point.memory_percent / 100.0 * plot.height(),
        ));

        if let Some(event) = &point.event {
            let y = plot.top() + 12.0;
            painter.circle_filled(Pos2::new(x, y), 3.0, theme::gold());

            painter.text(
                Pos2::new(x + 5.0, y),
                egui::Align2::LEFT_CENTER,
                event,
                egui::FontId::monospace(8.5),
                theme::gold(),
            );
        }
    }

    painter.add(egui::Shape::line(
        cpu_points,
        Stroke::new(1.8, theme::pink()),
    ));

    painter.add(egui::Shape::line(
        mem_points,
        Stroke::new(1.8, theme::blue()),
    ));

    painter.text(
        area.left_top() + Vec2::new(12.0, 10.0),
        egui::Align2::LEFT_TOP,
        "CPU",
        egui::FontId::monospace(10.0),
        theme::pink(),
    );

    painter.text(
        area.left_top() + Vec2::new(58.0, 10.0),
        egui::Align2::LEFT_TOP,
        "MEMORY",
        egui::FontId::monospace(10.0),
        theme::blue(),
    );

    painter.text(
        Pos2::new(plot.right(), plot.bottom() + 18.0),
        egui::Align2::RIGHT_TOP,
        "NOW",
        egui::FontId::monospace(9.0),
        theme::white(),
    );

    painter.text(
        Pos2::new(plot.left(), plot.bottom() + 18.0),
        egui::Align2::LEFT_TOP,
        "-36s",
        egui::FontId::monospace(9.0),
        theme::muted(),
    );
}

pub fn causality_view(ui: &mut egui::Ui, system: &SystemSnapshot, elapsed: f32) {
    page_heading(
        ui,
        "CAUSALITY // WHY DID THE MACHINE DO THAT?",
        "mock reconstructed event chain ready for scheduler, fault, storage and syscall correlation",
    );

    let data = AnalysisSnapshot::mock(system, elapsed);

    for (index, step) in data.causal_chain.iter().enumerate() {
        egui::Frame::new()
            .fill(theme::panel())
            .stroke(Stroke::new(1.0, theme::border()))
            .corner_radius(5.0)
            .inner_margin(10.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("{:02}", index + 1))
                            .monospace()
                            .strong()
                            .color(theme::pink()),
                    );

                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(&step.title)
                                .monospace()
                                .strong()
                                .color(theme::white()),
                        );
                        ui.label(egui::RichText::new(&step.detail).color(theme::muted()));
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        truth(ui, step.truth)
                    });
                });
            });

        if index + 1 < data.causal_chain.len() {
            ui.label(
                egui::RichText::new("                    ↓")
                    .monospace()
                    .color(theme::violet()),
            );
        }
    }
}

pub fn syscalls_view(ui: &mut egui::Ui, system: &SystemSnapshot, elapsed: f32) {
    page_heading(
        ui,
        "SYSCALL MICROSCOPE",
        "live syscall counts, latency and error surface for the selected process",
    );

    let data = AnalysisSnapshot::mock(system, elapsed);

    egui::Grid::new("syscall_table")
        .striped(true)
        .min_col_width(130.0)
        .show(ui, |ui| {
            for header in ["SYSCALL", "CALLS / S", "AVG LATENCY", "ERRORS / S"] {
                ui.label(
                    egui::RichText::new(header)
                        .monospace()
                        .strong()
                        .color(theme::muted()),
                );
            }
            ui.end_row();

            for syscall in &data.syscalls {
                ui.label(
                    egui::RichText::new(&syscall.name)
                        .monospace()
                        .color(theme::white()),
                );
                ui.label(
                    egui::RichText::new(syscall.calls_per_second.to_string())
                        .monospace()
                        .color(theme::pink()),
                );
                ui.label(
                    egui::RichText::new(format!("{:.1} µs", syscall.avg_latency_us))
                        .monospace()
                        .color(theme::blue()),
                );
                ui.label(
                    egui::RichText::new(syscall.errors_per_second.to_string())
                        .monospace()
                        .color(if syscall.errors_per_second > 0 {
                            theme::gold()
                        } else {
                            theme::green()
                        }),
                );
                ui.end_row();
            }
        });
}

pub fn flamegraph_view(ui: &mut egui::Ui, system: &SystemSnapshot, elapsed: f32) {
    page_heading(
        ui,
        "LIVE FLAMEGRAPH",
        "rolling CPU profile surface ready for perf stacks and clickable symbol descent",
    );

    let data = AnalysisSnapshot::mock(system, elapsed);

    let max_depth = data.flamegraph.iter().map(|f| f.depth).max().unwrap_or(0);

    let height = (max_depth as f32 + 1.0) * 56.0 + 30.0;

    let (response, painter) =
        ui.allocate_painter(Vec2::new(ui.available_width(), height), Sense::hover());

    let area = response.rect.shrink(6.0);
    painter.rect_filled(area, 5.0, theme::terminal_bg());

    for frame in &data.flamegraph {
        let row_height = 48.0;
        let rect = Rect::from_min_size(
            Pos2::new(
                area.left() + frame.start * area.width(),
                area.bottom() - (frame.depth as f32 + 1.0) * row_height,
            ),
            Vec2::new(frame.width * area.width() - 3.0, row_height - 4.0),
        );

        let color = theme::blend(
            theme::wine(),
            theme::pink(),
            (frame.depth as f32 * 0.18).min(0.8),
        );

        painter.rect_filled(rect, 3.0, color);
        painter.rect_stroke(
            rect,
            3.0,
            Stroke::new(1.0, theme::border()),
            StrokeKind::Inside,
        );

        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("{} // {}", frame.label, frame.samples),
            egui::FontId::monospace(9.5),
            theme::white(),
        );
    }
}

pub fn memory_map_view(ui: &mut egui::Ui, system: &SystemSnapshot, elapsed: f32) {
    page_heading(
        ui,
        "MEMORY CARTOGRAPHY",
        "virtual address-space map ready for /proc/<pid>/maps, smaps and page-fault correlation",
    );

    let data = AnalysisSnapshot::mock(system, elapsed);

    for region in &data.memory_regions {
        egui::Frame::new()
            .fill(theme::panel())
            .stroke(Stroke::new(1.0, theme::border()))
            .corner_radius(5.0)
            .inner_margin(9.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(&region.label)
                                .monospace()
                                .strong()
                                .color(theme::white()),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "0x{:016X} → 0x{:016X} // {}",
                                region.start, region.end, region.permissions
                            ))
                            .monospace()
                            .size(10.0)
                            .color(theme::muted()),
                        );
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!("dirty {:>5.1}%", region.dirty_percent))
                                .monospace()
                                .color(theme::gold()),
                        );
                    });
                });

                bar(
                    ui,
                    region.resident_percent / 100.0,
                    &format!("resident {:>5.1}%", region.resident_percent),
                    theme::blue(),
                );
            });

        ui.add_space(5.0);
    }
}

pub fn scheduler_view(ui: &mut egui::Ui, system: &SystemSnapshot, elapsed: f32) {
    page_heading(
        ui,
        "SCHEDULER WEATHER MAP",
        "run queues, wakeups, migrations and pressure per logical CPU",
    );

    let data = AnalysisSnapshot::mock(system, elapsed);

    egui::ScrollArea::vertical().show(ui, |ui| {
        for chunk in data.scheduler.chunks(4) {
            ui.columns(4, |cols| {
                for (col, cpu) in cols.iter_mut().zip(chunk.iter()) {
                    egui::Frame::new()
                        .fill(theme::panel())
                        .stroke(Stroke::new(1.0, theme::border()))
                        .corner_radius(5.0)
                        .inner_margin(9.0)
                        .show(col, |ui| {
                            ui.label(
                                egui::RichText::new(format!("CPU {:02}", cpu.cpu))
                                    .monospace()
                                    .strong()
                                    .color(theme::white()),
                            );

                            bar(
                                ui,
                                cpu.pressure / 100.0,
                                &format!("pressure {:.0}%", cpu.pressure),
                                if cpu.pressure > 70.0 {
                                    theme::gold()
                                } else {
                                    theme::violet()
                                },
                            );

                            ui.label(
                                egui::RichText::new(format!(
                                    "runq {}  wake {} /s  mig {} /s",
                                    cpu.run_queue,
                                    cpu.wakeups_per_second,
                                    cpu.migrations_per_second,
                                ))
                                .monospace()
                                .size(9.5)
                                .color(theme::muted()),
                            );
                        });
                }
            });
            ui.add_space(6.0);
        }
    });
}

pub fn cache_view(ui: &mut egui::Ui, system: &SystemSnapshot, elapsed: f32) {
    page_heading(
        ui,
        "CACHE / PMU CHAMBER",
        "hardware-counter surface for IPC, misses, stalls, branches and future NASM helpers",
    );

    let data = AnalysisSnapshot::mock(system, elapsed);
    let cache = &data.cache;

    ui.horizontal(|ui| {
        metric_tile(ui, "IPC", format!("{:.2}", cache.ipc), theme::pink());
        metric_tile(
            ui,
            "L1D MISS",
            format!("{:.1}%", cache.l1d_miss_percent),
            theme::blue(),
        );
        metric_tile(
            ui,
            "LLC MISS",
            format!("{:.1}%", cache.llc_miss_percent),
            theme::gold(),
        );
        metric_tile(
            ui,
            "BRANCH MISS",
            format!("{:.1}%", cache.branch_miss_percent),
            theme::violet(),
        );
    });

    ui.add_space(10.0);

    for (label, value, color) in [
        ("L1D misses", cache.l1d_miss_percent, theme::blue()),
        ("L1I misses", cache.l1i_miss_percent, theme::green()),
        ("L2 misses", cache.l2_miss_percent, theme::violet()),
        ("LLC misses", cache.llc_miss_percent, theme::gold()),
        ("branch misses", cache.branch_miss_percent, theme::pink()),
        ("stalled cycles", cache.stalled_cycle_percent, theme::gold()),
    ] {
        bar(ui, value / 100.0, &format!("{label} // {value:.1}%"), color);
        ui.add_space(5.0);
    }

    ui.label(
        egui::RichText::new(format!(
            "cycles/s ≈ {:.2} billion",
            cache.cycles_per_second / 1_000_000_000.0
        ))
        .monospace()
        .color(theme::muted()),
    );
}

fn metric_tile(ui: &mut egui::Ui, label: &str, value: String, color: Color32) {
    egui::Frame::new()
        .fill(theme::panel())
        .stroke(Stroke::new(1.0, theme::border()))
        .corner_radius(5.0)
        .inner_margin(10.0)
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(label)
                    .monospace()
                    .size(9.5)
                    .color(theme::muted()),
            );
            ui.label(
                egui::RichText::new(value)
                    .monospace()
                    .size(18.0)
                    .strong()
                    .color(color),
            );
        });
}

pub fn irq_view(ui: &mut egui::Ui, system: &SystemSnapshot, elapsed: f32) {
    page_heading(
        ui,
        "IRQ / SOFTIRQ CATHEDRAL",
        "hardware interrupt ownership and CPU affinity surface",
    );

    let data = AnalysisSnapshot::mock(system, elapsed);

    egui::Grid::new("irq_table")
        .striped(true)
        .min_col_width(125.0)
        .show(ui, |ui| {
            for h in ["IRQ", "SOURCE", "CPU", "INTERRUPTS / S"] {
                ui.label(
                    egui::RichText::new(h)
                        .monospace()
                        .strong()
                        .color(theme::muted()),
                );
            }
            ui.end_row();

            for irq in &data.irqs {
                ui.label(
                    egui::RichText::new(&irq.irq)
                        .monospace()
                        .color(theme::pink()),
                );
                ui.label(
                    egui::RichText::new(&irq.source)
                        .monospace()
                        .color(theme::white()),
                );
                ui.label(
                    egui::RichText::new(format!("CPU{:02}", irq.cpu))
                        .monospace()
                        .color(theme::blue()),
                );
                ui.label(
                    egui::RichText::new(irq.interrupts_per_second.to_string())
                        .monospace()
                        .color(theme::green()),
                );
                ui.end_row();
            }
        });
}

pub fn binary_view(ui: &mut egui::Ui, system: &SystemSnapshot, elapsed: f32) {
    page_heading(
        ui,
        "BINARY ARCHAEOLOGY",
        "ELF / symbol / section specimen table ready for readelf, objdump and debug-info integration",
    );

    let data = AnalysisSnapshot::mock(system, elapsed);
    let bin = &data.binary;

    ui.horizontal_wrapped(|ui| {
        metric_tile(ui, "FORMAT", bin.format.clone(), theme::pink());
        metric_tile(ui, "ARCH", bin.architecture.clone(), theme::blue());
        metric_tile(
            ui,
            "ENTRY",
            format!("0x{:X}", bin.entry_point),
            theme::violet(),
        );
    });

    ui.add_space(10.0);

    ui.label(
        egui::RichText::new(format!("path // {}", bin.path))
            .monospace()
            .color(theme::white()),
    );
    ui.label(
        egui::RichText::new(format!("build-id // {}", bin.build_id))
            .monospace()
            .color(theme::muted()),
    );

    ui.add_space(10.0);

    ui.columns(2, |cols| {
        cols[0].label(
            egui::RichText::new("SECTIONS")
                .monospace()
                .strong()
                .color(theme::pink()),
        );

        egui::Grid::new("binary_sections")
            .striped(true)
            .show(&mut cols[0], |ui| {
                for section in &bin.sections {
                    ui.label(
                        egui::RichText::new(&section.name)
                            .monospace()
                            .color(theme::white()),
                    );
                    ui.label(
                        egui::RichText::new(format_bytes(section.size_bytes))
                            .monospace()
                            .color(theme::blue()),
                    );
                    ui.label(
                        egui::RichText::new(&section.flags)
                            .monospace()
                            .color(theme::gold()),
                    );
                    ui.end_row();
                }
            });

        cols[1].label(
            egui::RichText::new("DYNAMIC LIBRARIES")
                .monospace()
                .strong()
                .color(theme::violet()),
        );

        for lib in &bin.libraries {
            cols[1].label(
                egui::RichText::new(format!("• {lib}"))
                    .monospace()
                    .color(theme::muted()),
            );
        }
    });
}

pub fn autopsy_view(ui: &mut egui::Ui, system: &SystemSnapshot, elapsed: f32) {
    page_heading(
        ui,
        "PROCESS AUTOPSY",
        "frozen pre-death telemetry surface for crashes, exits and forensic replay",
    );

    let data = AnalysisSnapshot::mock(system, elapsed);
    let dead = &data.autopsy;

    egui::Frame::new()
        .fill(theme::panel())
        .stroke(Stroke::new(1.0, theme::gold()))
        .corner_radius(6.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(format!("{} // PID {}", dead.process, dead.pid))
                    .monospace()
                    .strong()
                    .size(17.0)
                    .color(theme::white()),
            );

            ui.label(
                egui::RichText::new(&dead.exit_reason)
                    .monospace()
                    .color(theme::gold()),
            );

            ui.label(
                egui::RichText::new(format!(
                    "peak RAM {}  //  last CPU{:02}  //  {}",
                    format_bytes(dead.peak_memory_bytes),
                    dead.last_cpu,
                    dead.last_instruction
                ))
                .monospace()
                .color(theme::muted()),
            );
        });

    ui.add_space(10.0);

    for event in &dead.events {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                egui::RichText::new(format!("-{:04.1}s", event.age_seconds))
                    .monospace()
                    .color(theme::muted()),
            );
            truth(ui, event.truth);
            ui.label(
                egui::RichText::new(&event.message)
                    .monospace()
                    .color(theme::white()),
            );
        });
        ui.separator();
    }
}

pub fn packet_flow_view(ui: &mut egui::Ui, system: &SystemSnapshot, elapsed: f32) {
    page_heading(
        ui,
        "PACKET FLOW // PROCESS → SOCKET → WIRE",
        "connection ownership, bandwidth and latency surface ready for netlink/eBPF attribution",
    );

    let data = AnalysisSnapshot::mock(system, elapsed);

    for flow in &data.flows {
        egui::Frame::new()
            .fill(theme::panel())
            .stroke(Stroke::new(1.0, theme::border()))
            .corner_radius(5.0)
            .inner_margin(10.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(&flow.process)
                            .monospace()
                            .strong()
                            .color(theme::pink()),
                    );
                    ui.label(egui::RichText::new("→").color(theme::muted()));
                    ui.label(
                        egui::RichText::new(&flow.remote)
                            .monospace()
                            .color(theme::white()),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!("{:.1} ms", flow.latency_ms))
                                .monospace()
                                .color(theme::gold()),
                        );
                    });
                });

                ui.label(
                    egui::RichText::new(format!(
                        "{}  //  RX {:.2} MiB/s  //  TX {:.2} MiB/s",
                        flow.protocol, flow.rx_mib_s, flow.tx_mib_s
                    ))
                    .monospace()
                    .color(theme::muted()),
                );
            });

        ui.add_space(5.0);
    }
}

pub fn fleet_view(ui: &mut egui::Ui, fleet: &FleetState, selected_machine_id: &mut String) {
    page_heading(
        ui,
        "FLEET // DISTRIBUTED OBSERVATORY",
        "select any connected agent and the entire Observatory retargets to that machine",
    );

    ui.horizontal_wrapped(|ui| {
        ui.label(
            egui::RichText::new(
                "Selecting OPEN MACHINE changes CPU, Memory, GPU, Processes, Network, The Machine, and deep-analysis tabs.",
            )
            .color(theme::muted()),
        );
    });

    ui.add_space(8.0);

    ui.columns(2, |cols| {
        for (index, machine) in fleet.machines.iter().enumerate() {
            let col = &mut cols[index % 2];

            let selected = selected_machine_id == &machine.id;

            egui::Frame::new()
                .fill(if selected {
                    theme::selected_panel()
                } else {
                    theme::panel()
                })
                .stroke(Stroke::new(
                    if selected { 2.0 } else { 1.0 },
                    if !machine.online {
                        theme::muted()
                    } else if selected {
                        theme::pink()
                    } else {
                        theme::green()
                    },
                ))
                .corner_radius(6.0)
                .inner_margin(12.0)
                .show(col, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(if machine.online { "●" } else { "○" }).color(
                                if machine.online {
                                    theme::green()
                                } else {
                                    theme::muted()
                                },
                            ),
                        );

                        ui.label(
                            egui::RichText::new(&machine.name)
                                .monospace()
                                .strong()
                                .size(16.0)
                                .color(theme::white()),
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if selected {
                                ui.label(
                                    egui::RichText::new("◆ ACTIVE TARGET")
                                        .monospace()
                                        .size(9.0)
                                        .color(theme::pink()),
                                );
                            }
                        });
                    });

                    ui.label(
                        egui::RichText::new(format!(
                            "{} // {}",
                            machine.role,
                            machine.origin.label()
                        ))
                        .monospace()
                        .size(10.0)
                        .color(theme::muted()),
                    );

                    let memory_percent = machine.system.memory.used_bytes as f32
                        / machine.system.memory.total_bytes.max(1) as f32
                        * 100.0;

                    bar(
                        ui,
                        machine.system.cpu.package_usage / 100.0,
                        &format!(
                            "CPU {:.1}% // {} package{} // {} logical",
                            machine.system.cpu.package_usage,
                            machine.cpu_packages.len(),
                            if machine.cpu_packages.len() == 1 {
                                ""
                            } else {
                                "s"
                            },
                            machine.system.cpu.logical_cpus.len()
                        ),
                        theme::pink(),
                    );

                    bar(
                        ui,
                        memory_percent / 100.0,
                        &format!(
                            "RAM {:.1}% // {}",
                            memory_percent,
                            format_bytes(machine.system.memory.total_bytes)
                        ),
                        theme::blue(),
                    );

                    ui.label(
                        egui::RichText::new(format!(
                            "network {:.1} MiB/s // {} connections",
                            machine.system.network.rx_mib_s + machine.system.network.tx_mib_s,
                            machine.system.network.connections
                        ))
                        .monospace()
                        .color(theme::green()),
                    );

                    if let Some(version) = &machine.agent_version {
                        ui.label(
                            egui::RichText::new(version)
                                .monospace()
                                .size(9.0)
                                .color(theme::muted()),
                        );
                    }

                    ui.add_space(7.0);

                    if machine.online {
                        if ui
                            .add_sized(
                                [ui.available_width(), 34.0],
                                egui::Button::new(if selected {
                                    "◆ VIEWING THIS MACHINE"
                                } else {
                                    "OPEN MACHINE"
                                })
                                .selected(selected),
                            )
                            .clicked()
                        {
                            *selected_machine_id = machine.id.clone();
                        }
                    } else {
                        ui.add_enabled(false, egui::Button::new("OFFLINE // LAST KNOWN SNAPSHOT"));
                    }
                });

            col.add_space(9.0);
        }
    });

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);

    ui.label(
        egui::RichText::new(
            "FUTURE AGENT CONTRACT // authenticated read-only telemetry by default; process-control commands live on a separately authorized channel.",
        )
        .monospace()
        .size(9.5)
        .color(theme::muted()),
    );
}

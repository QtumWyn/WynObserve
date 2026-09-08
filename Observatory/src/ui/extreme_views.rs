use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};

use crate::{
    extended_analysis::ExtremeSnapshot,
    fleet::{FleetMachine, FleetState},
    model::{TruthLevel, format_bytes},
    theme,
};

fn heading(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.label(
        egui::RichText::new(title)
            .size(19.0)
            .strong()
            .color(theme::white()),
    );

    ui.label(egui::RichText::new(subtitle).color(theme::muted()));

    ui.add_space(10.0);
}

fn truth(ui: &mut egui::Ui, value: TruthLevel) {
    ui.label(
        egui::RichText::new(format!("{} {}", value.glyph(), value.label()))
            .monospace()
            .size(9.5)
            .color(theme::truth_color(value)),
    );
}

fn bar(ui: &mut egui::Ui, fraction: f32, label: &str, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 21.0), Sense::hover());

    ui.painter().rect_filled(rect, 3.0, theme::track_bg());

    ui.painter().rect_filled(
        Rect::from_min_size(
            rect.min,
            Vec2::new(rect.width() * fraction.clamp(0.0, 1.0), rect.height()),
        ),
        3.0,
        color,
    );

    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::monospace(10.0),
        theme::white(),
    );
}

fn card(ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(theme::panel())
        .stroke(Stroke::new(1.0, theme::border()))
        .corner_radius(6.0)
        .inner_margin(10.0)
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(title)
                    .monospace()
                    .strong()
                    .color(theme::white()),
            );

            ui.add_space(5.0);
            body(ui);
        });
}

pub fn fabric_view(ui: &mut egui::Ui, machine: &FleetMachine, fleet: &FleetState, elapsed: f32) {
    heading(
        ui,
        "PCIe / MOTHERBOARD FABRIC",
        "root complexes, devices, IOMMU groups, NUMA locality and link topology",
    );

    let data = ExtremeSnapshot::mock(machine, fleet, elapsed);

    let (response, painter) =
        ui.allocate_painter(Vec2::new(ui.available_width(), 430.0), Sense::hover());

    let area = response.rect.shrink(8.0);

    painter.rect_filled(area, 6.0, theme::terminal_bg());

    let cpu = Rect::from_center_size(
        Pos2::new(area.left() + area.width() * 0.17, area.center().y),
        Vec2::new(190.0, 105.0),
    );

    painter.rect_filled(cpu, 5.0, theme::selected_panel());
    painter.rect_stroke(
        cpu,
        5.0,
        Stroke::new(1.5, theme::pink()),
        StrokeKind::Inside,
    );
    painter.text(
        cpu.center(),
        egui::Align2::CENTER_CENTER,
        format!(
            "{} CPU package{}\n{} logical CPUs",
            machine.cpu_packages.len(),
            if machine.cpu_packages.len() == 1 {
                ""
            } else {
                "s"
            },
            machine.system.cpu.logical_cpus.len()
        ),
        egui::FontId::monospace(11.0),
        theme::white(),
    );

    let x = area.left() + area.width() * 0.48;

    for (index, device) in data.pcie_devices.iter().enumerate() {
        let y = area.top() + 30.0 + index as f32 * 92.0;

        let rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(area.width() * 0.43, 72.0));

        let accent = match index {
            1 => theme::violet(),
            2 => theme::blue(),
            3 => theme::green(),
            _ => theme::pink(),
        };

        painter.line_segment(
            [cpu.right_center(), rect.left_center()],
            Stroke::new(1.0, theme::border()),
        );

        painter.rect_filled(rect, 5.0, theme::panel());

        painter.rect_stroke(rect, 5.0, Stroke::new(1.0, accent), StrokeKind::Inside);

        painter.text(
            rect.left_top() + Vec2::new(9.0, 8.0),
            egui::Align2::LEFT_TOP,
            format!("{} // {}", device.address, device.name),
            egui::FontId::monospace(9.5),
            theme::white(),
        );

        painter.text(
            rect.left_top() + Vec2::new(9.0, 31.0),
            egui::Align2::LEFT_TOP,
            format!(
                "{} // {} // NUMA {:?} // IOMMU {:?}",
                device.class, device.link, device.numa_node, device.iommu_group
            ),
            egui::FontId::monospace(8.5),
            theme::muted(),
        );
    }
}

pub fn numa_view(ui: &mut egui::Ui, machine: &FleetMachine, fleet: &FleetState, elapsed: f32) {
    heading(
        ui,
        "NUMA HEATMAP",
        "CPU package locality, memory placement, local/remote access and cross-node migrations",
    );

    let data = ExtremeSnapshot::mock(machine, fleet, elapsed);

    if data.numa_nodes.len() <= 1 {
        ui.label(
            egui::RichText::new(
                "Single NUMA node detected in this mock topology. Locality penalties are minimal.",
            )
            .color(theme::muted()),
        );
    }

    ui.columns(data.numa_nodes.len().clamp(1, 2), |columns| {
        for (column, node) in columns.iter_mut().zip(data.numa_nodes.iter()) {
            card(
                column,
                &format!("NUMA NODE {} // {} CPUs", node.node_id, node.cpu_ids.len()),
                |ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} / {} used",
                            format_bytes(node.used_memory_bytes),
                            format_bytes(node.total_memory_bytes)
                        ))
                        .monospace()
                        .color(theme::blue()),
                    );

                    bar(
                        ui,
                        node.local_access_percent / 100.0,
                        &format!("local access {:.1}%", node.local_access_percent),
                        theme::green(),
                    );

                    bar(
                        ui,
                        node.remote_access_percent / 100.0,
                        &format!("remote access {:.1}%", node.remote_access_percent),
                        if node.remote_access_percent > 20.0 {
                            theme::gold()
                        } else {
                            theme::violet()
                        },
                    );

                    ui.label(
                        egui::RichText::new(format!(
                            "{} cross-node migrations / s",
                            node.migrations_per_second
                        ))
                        .monospace()
                        .color(theme::muted()),
                    );
                },
            );
        }
    });
}

pub fn storage_io_view(
    ui: &mut egui::Ui,
    machine: &FleetMachine,
    fleet: &FleetState,
    elapsed: f32,
) {
    heading(
        ui,
        "STORAGE I/O MICROSCOPE",
        "process → filesystem → block queue → NVMe surface for IOPS, latency and writeback",
    );

    let data = ExtremeSnapshot::mock(machine, fleet, elapsed);

    for io in &data.storage_io {
        card(ui, &format!("{} // {}", io.device, io.filesystem), |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new(format!("R {:>8.0} IOPS", io.read_iops))
                        .monospace()
                        .color(theme::blue()),
                );
                ui.label(
                    egui::RichText::new(format!("W {:>8.0} IOPS", io.write_iops))
                        .monospace()
                        .color(theme::pink()),
                );
                ui.label(
                    egui::RichText::new(format!("QD {:.1}", io.queue_depth))
                        .monospace()
                        .color(theme::violet()),
                );
            });

            bar(
                ui,
                io.utilization_percent / 100.0,
                &format!("device utilization {:.1}%", io.utilization_percent),
                theme::gold(),
            );

            ui.label(
                egui::RichText::new(format!(
                    "read {:.2} ms // write {:.2} ms // dirty writeback {:.1} MiB/s",
                    io.read_latency_ms, io.write_latency_ms, io.dirty_writeback_mib_s
                ))
                .monospace()
                .color(theme::muted()),
            );
        });
    }
}

pub fn power_view(ui: &mut egui::Ui, machine: &FleetMachine, fleet: &FleetState, elapsed: f32) {
    heading(
        ui,
        "POWER / THERMAL TOPOLOGY",
        "temperature zones, package power, fan behavior and throttle-state surface",
    );

    let data = ExtremeSnapshot::mock(machine, fleet, elapsed);

    for zone in &data.thermal_zones {
        card(ui, &zone.name, |ui| {
            let ratio = if zone.trip_c > 0.0 {
                zone.temperature_c / zone.trip_c
            } else {
                0.0
            };

            bar(
                ui,
                ratio,
                &format!("{:.1}°C / {:.1}°C trip", zone.temperature_c, zone.trip_c),
                if zone.throttling {
                    theme::gold()
                } else {
                    theme::pink()
                },
            );

            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new(format!("{:.1} W", zone.power_watts))
                        .monospace()
                        .color(theme::blue()),
                );

                if let Some(rpm) = zone.fan_rpm {
                    ui.label(
                        egui::RichText::new(format!("{rpm} RPM"))
                            .monospace()
                            .color(theme::green()),
                    );
                }

                if zone.throttling {
                    ui.label(
                        egui::RichText::new("▲ THROTTLING")
                            .monospace()
                            .strong()
                            .color(theme::gold()),
                    );
                }
            });
        });

        ui.add_space(6.0);
    }
}

pub fn locks_view(ui: &mut egui::Ui, machine: &FleetMachine, fleet: &FleetState, elapsed: f32) {
    heading(
        ui,
        "LOCK CONTENTION CHAMBER",
        "mutex/futex ownership, waiter pressure and blocked-thread surface",
    );

    let data = ExtremeSnapshot::mock(machine, fleet, elapsed);

    for lock in &data.locks {
        card(ui, &lock.lock_name, |ui| {
            ui.label(
                egui::RichText::new(format!(
                    "owner {} // {} waiter{} // {:.2} ms wait",
                    lock.owner,
                    lock.waiter_count,
                    if lock.waiter_count == 1 { "" } else { "s" },
                    lock.wait_ms
                ))
                .monospace()
                .color(theme::muted()),
            );

            bar(
                ui,
                lock.contention_percent / 100.0,
                &format!(
                    "contention {:.1}% // {} acquisitions/s",
                    lock.contention_percent, lock.acquisitions_per_second
                ),
                if lock.contention_percent > 20.0 {
                    theme::gold()
                } else {
                    theme::violet()
                },
            );
        });

        ui.add_space(6.0);
    }
}

pub fn containers_view(
    ui: &mut egui::Ui,
    machine: &FleetMachine,
    fleet: &FleetState,
    elapsed: f32,
) {
    heading(
        ui,
        "CGROUP / CONTAINER TOPOLOGY",
        "host → cgroup → container → process resource boundaries",
    );

    let data = ExtremeSnapshot::mock(machine, fleet, elapsed);

    for group in &data.cgroups {
        card(ui, &format!("{} // {}", group.path, group.kind), |ui| {
            ui.label(
                egui::RichText::new(format!(
                    "{} processes // CPU {:.1}% // RAM {}",
                    group.processes,
                    group.cpu_percent,
                    format_bytes(group.memory_bytes)
                ))
                .monospace()
                .color(theme::white()),
            );

            if let Some(limit) = group.memory_limit_bytes {
                bar(
                    ui,
                    group.memory_bytes as f32 / limit.max(1) as f32,
                    &format!("memory limit {}", format_bytes(limit)),
                    theme::blue(),
                );
            }

            if let Some(quota) = group.cpu_quota_percent {
                ui.label(
                    egui::RichText::new(format!(
                        "CPU quota {:.0}% // OOM events {}",
                        quota, group.oom_events
                    ))
                    .monospace()
                    .color(if group.oom_events > 0 {
                        theme::gold()
                    } else {
                        theme::muted()
                    }),
                );
            }
        });

        ui.add_space(6.0);
    }
}

pub fn database_view(ui: &mut egui::Ui, machine: &FleetMachine, fleet: &FleetState, elapsed: f32) {
    heading(
        ui,
        "DATABASE MICROSCOPE",
        "connections, transactions, cache, WAL, locks and active-query surface",
    );

    let data = ExtremeSnapshot::mock(machine, fleet, elapsed);

    let db = &data.database;

    ui.horizontal_wrapped(|ui| {
        tile(
            ui,
            "CONNECTIONS",
            db.active_connections.to_string(),
            theme::blue(),
        );
        tile(
            ui,
            "TX / S",
            format!("{:.0}", db.transactions_per_second),
            theme::pink(),
        );
        tile(
            ui,
            "CACHE HIT",
            format!("{:.2}%", db.cache_hit_percent),
            theme::green(),
        );
        tile(
            ui,
            "WAL",
            format!("{:.1} MiB/s", db.wal_mib_s),
            theme::violet(),
        );
        tile(
            ui,
            "BLOCKED",
            db.blocked_queries.to_string(),
            if db.blocked_queries > 0 {
                theme::gold()
            } else {
                theme::green()
            },
        );
    });

    ui.add_space(10.0);

    ui.label(
        egui::RichText::new(&db.name)
            .monospace()
            .strong()
            .color(theme::white()),
    );

    egui::Grid::new("database_queries")
        .striped(true)
        .min_col_width(120.0)
        .show(ui, |ui| {
            for header in ["PID", "AGE", "STATE", "QUERY"] {
                ui.label(
                    egui::RichText::new(header)
                        .monospace()
                        .strong()
                        .color(theme::muted()),
                );
            }
            ui.end_row();

            for query in &db.active_queries {
                ui.label(
                    egui::RichText::new(query.pid.to_string())
                        .monospace()
                        .color(theme::pink()),
                );
                ui.label(
                    egui::RichText::new(format!("{:.1} ms", query.age_ms))
                        .monospace()
                        .color(theme::blue()),
                );
                ui.label(egui::RichText::new(&query.state).monospace().color(
                    if query.state.contains("waiting") {
                        theme::gold()
                    } else {
                        theme::green()
                    },
                ));
                ui.label(
                    egui::RichText::new(&query.summary)
                        .monospace()
                        .color(theme::white()),
                );
                ui.end_row();
            }
        });
}

fn tile(ui: &mut egui::Ui, label: &str, value: String, color: Color32) {
    egui::Frame::new()
        .fill(theme::panel())
        .stroke(Stroke::new(1.0, theme::border()))
        .corner_radius(5.0)
        .inner_margin(9.0)
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(label)
                    .monospace()
                    .size(9.0)
                    .color(theme::muted()),
            );
            ui.label(
                egui::RichText::new(value)
                    .monospace()
                    .strong()
                    .size(16.0)
                    .color(color),
            );
        });
}

pub fn traces_view(ui: &mut egui::Ui, machine: &FleetMachine, fleet: &FleetState, elapsed: f32) {
    heading(
        ui,
        "DISTRIBUTED REQUEST TRACE",
        "follow one request through services, database, storage and physical machines",
    );

    let data = ExtremeSnapshot::mock(machine, fleet, elapsed);

    let total_ms = data
        .trace_spans
        .iter()
        .map(|span| span.start_ms + span.duration_ms)
        .fold(1.0_f32, f32::max);

    for span in &data.trace_spans {
        ui.horizontal(|ui| {
            ui.add_sized(
                [180.0, 28.0],
                egui::Label::new(
                    egui::RichText::new(format!("{} // {}", span.service, span.machine))
                        .monospace()
                        .color(theme::white()),
                ),
            );

            let (rect, _) = ui.allocate_exact_size(
                Vec2::new(ui.available_width().max(240.0), 28.0),
                Sense::hover(),
            );

            ui.painter().rect_filled(rect, 3.0, theme::track_bg());

            let left = span.start_ms / total_ms * rect.width();
            let width = span.duration_ms / total_ms * rect.width();

            let span_rect = Rect::from_min_size(
                Pos2::new(rect.left() + left, rect.top() + 3.0),
                Vec2::new(width.max(4.0), rect.height() - 6.0),
            );

            ui.painter().rect_filled(
                span_rect,
                3.0,
                if span.status == "OK" {
                    theme::violet()
                } else {
                    theme::gold()
                },
            );

            ui.painter().text(
                span_rect.center(),
                egui::Align2::CENTER_CENTER,
                format!("{} // {:.1}ms", span.operation, span.duration_ms),
                egui::FontId::monospace(8.5),
                theme::white(),
            );
        });

        ui.add_space(4.0);
    }
}

pub fn compare_view(
    ui: &mut egui::Ui,
    fleet: &FleetState,
    selected_machine_id: &str,
    comparison_machine_id: &mut String,
) {
    heading(
        ui,
        "MACHINE COMPARISON",
        "side-by-side hardware and telemetry comparison for 'works on my machine' exorcisms",
    );

    let Some(primary) = fleet.machine(selected_machine_id) else {
        ui.label("Primary Fleet target unavailable.");
        return;
    };

    if comparison_machine_id == selected_machine_id
        || fleet.machine(comparison_machine_id).is_none()
    {
        if let Some(other) = fleet
            .machines
            .iter()
            .find(|machine| machine.online && machine.id != selected_machine_id)
        {
            *comparison_machine_id = other.id.clone();
        }
    }

    ui.horizontal_wrapped(|ui| {
        ui.label(
            egui::RichText::new("COMPARE AGAINST")
                .monospace()
                .color(theme::muted()),
        );

        for machine in fleet
            .machines
            .iter()
            .filter(|machine| machine.online && machine.id != selected_machine_id)
        {
            let selected = comparison_machine_id == &machine.id;

            if ui
                .add(egui::Button::new(&machine.name).selected(selected))
                .clicked()
            {
                *comparison_machine_id = machine.id.clone();
            }
        }
    });

    ui.add_space(10.0);

    let Some(other) = fleet.machine(comparison_machine_id) else {
        ui.label("No comparison target available.");
        return;
    };

    egui::Grid::new("machine_compare_grid")
        .striped(true)
        .min_col_width(180.0)
        .show(ui, |ui| {
            ui.label("");
            ui.label(
                egui::RichText::new(&primary.name)
                    .monospace()
                    .strong()
                    .color(theme::pink()),
            );
            ui.label(
                egui::RichText::new(&other.name)
                    .monospace()
                    .strong()
                    .color(theme::blue()),
            );
            ui.end_row();

            compare_row(
                ui,
                "CPU PACKAGES",
                primary.cpu_packages.len().to_string(),
                other.cpu_packages.len().to_string(),
            );

            compare_row(
                ui,
                "LOGICAL CPUs",
                primary.system.cpu.logical_cpus.len().to_string(),
                other.system.cpu.logical_cpus.len().to_string(),
            );

            compare_row(
                ui,
                "CPU LOAD",
                format!("{:.1}%", primary.system.cpu.package_usage),
                format!("{:.1}%", other.system.cpu.package_usage),
            );

            compare_row(
                ui,
                "RAM",
                format_bytes(primary.system.memory.total_bytes),
                format_bytes(other.system.memory.total_bytes),
            );

            compare_row(
                ui,
                "GPU",
                primary.system.gpu.model.clone(),
                other.system.gpu.model.clone(),
            );

            compare_row(
                ui,
                "NETWORK",
                primary.system.network.interface.clone(),
                other.system.network.interface.clone(),
            );
        });
}

fn compare_row(ui: &mut egui::Ui, label: &str, left: String, right: String) {
    ui.label(egui::RichText::new(label).monospace().color(theme::muted()));
    ui.label(egui::RichText::new(left).monospace().color(theme::white()));
    ui.label(egui::RichText::new(right).monospace().color(theme::white()));
    ui.end_row();
}

pub fn diff_view(ui: &mut egui::Ui, machine: &FleetMachine, fleet: &FleetState, elapsed: f32) {
    heading(
        ui,
        "SNAPSHOT DIFF / CONFIG DRIFT",
        "compare this host against a previous snapshot and surface meaningful changes",
    );

    let data = ExtremeSnapshot::mock(machine, fleet, elapsed);

    egui::Grid::new("snapshot_diff")
        .striped(true)
        .min_col_width(130.0)
        .show(ui, |ui| {
            for header in ["CATEGORY", "ITEM", "BEFORE", "AFTER", "STATUS"] {
                ui.label(
                    egui::RichText::new(header)
                        .monospace()
                        .strong()
                        .color(theme::muted()),
                );
            }
            ui.end_row();

            for diff in &data.diff {
                ui.label(
                    egui::RichText::new(&diff.category)
                        .monospace()
                        .color(theme::blue()),
                );
                ui.label(
                    egui::RichText::new(&diff.item)
                        .monospace()
                        .color(theme::white()),
                );
                ui.label(
                    egui::RichText::new(&diff.before)
                        .monospace()
                        .color(theme::muted()),
                );
                ui.label(
                    egui::RichText::new(&diff.after)
                        .monospace()
                        .color(theme::white()),
                );
                ui.label(egui::RichText::new(&diff.severity).monospace().color(
                    if diff.severity == "REVIEW" {
                        theme::gold()
                    } else {
                        theme::green()
                    },
                ));
                ui.end_row();
            }
        });
}

pub fn firmware_view(ui: &mut egui::Ui, machine: &FleetMachine, fleet: &FleetState, elapsed: f32) {
    heading(
        ui,
        "FIRMWARE / PLATFORM ARCHAEOLOGY",
        "UEFI, SMBIOS, microcode, Secure Boot, IOMMU and device firmware inventory",
    );

    let data = ExtremeSnapshot::mock(machine, fleet, elapsed);

    for item in &data.firmware {
        card(ui, &item.component, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new(&item.value)
                        .monospace()
                        .color(theme::white()),
                );
                truth(ui, item.truth);
            });
        });
        ui.add_space(5.0);
    }
}

pub fn security_view(ui: &mut egui::Ui, machine: &FleetMachine, fleet: &FleetState, elapsed: f32) {
    heading(
        ui,
        "SECURITY ANATOMY",
        "capabilities, namespaces, confinement, listeners and suspicious-runtime observations",
    );

    let data = ExtremeSnapshot::mock(machine, fleet, elapsed);

    for item in &data.security {
        card(ui, &item.subject, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new(&item.detail).color(theme::white()));

                truth(ui, item.truth);

                ui.label(
                    egui::RichText::new(&item.severity)
                        .monospace()
                        .strong()
                        .color(if item.severity == "REVIEW" {
                            theme::gold()
                        } else {
                            theme::green()
                        }),
                );
            });
        });

        ui.add_space(5.0);
    }
}

pub fn source_silicon_view(
    ui: &mut egui::Ui,
    machine: &FleetMachine,
    fleet: &FleetState,
    elapsed: f32,
) {
    heading(
        ui,
        "SOURCE → SILICON DESCENT",
        "debug symbols + disassembly + sampled execution + PMU + memory locality",
    );

    let data = ExtremeSnapshot::mock(machine, fleet, elapsed);

    for (index, step) in data.source_silicon.iter().enumerate() {
        card(ui, &step.layer, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new(&step.value)
                        .monospace()
                        .color(theme::white()),
                );
                truth(ui, step.truth);
            });
        });

        if index + 1 < data.source_silicon.len() {
            ui.label(
                egui::RichText::new("                       ↓")
                    .monospace()
                    .color(theme::violet()),
            );
        }
    }
}

pub fn anomalies_view(ui: &mut egui::Ui, machine: &FleetMachine, fleet: &FleetState, elapsed: f32) {
    heading(
        ui,
        "ANOMALY BRAIN",
        "explainable machine-specific baselines, not fortune-telling",
    );

    let data = ExtremeSnapshot::mock(machine, fleet, elapsed);

    for item in &data.anomalies {
        card(
            ui,
            &format!("{:>3.0}% // {}", item.score * 100.0, item.title),
            |ui| {
                bar(
                    ui,
                    item.score,
                    &item.detail,
                    if item.score > 0.8 {
                        theme::gold()
                    } else if item.score > 0.55 {
                        theme::pink()
                    } else {
                        theme::blue()
                    },
                );

                truth(ui, item.truth);
            },
        );

        ui.add_space(6.0);
    }
}

pub fn incidents_view(
    ui: &mut egui::Ui,
    machine: &FleetMachine,
    fleet: &FleetState,
    elapsed: f32,
    captured_incidents: &mut u64,
) {
    heading(
        ui,
        "INCIDENT DOSSIERS",
        "freeze pre/post-event telemetry into portable replayable investigation bundles",
    );

    let data = ExtremeSnapshot::mock(machine, fleet, elapsed);

    ui.horizontal_wrapped(|ui| {
        if ui
            .button("◉ CAPTURE INCIDENT // 60s PRE + 30s POST")
            .clicked()
        {
            *captured_incidents += 1;
        }

        ui.label(
            egui::RichText::new(format!("session captures // {}", captured_incidents))
                .monospace()
                .color(theme::blue()),
        );
    });

    ui.add_space(10.0);

    for incident in &data.incidents {
        card(
            ui,
            &format!(
                "{} // {} // {}",
                incident.id, incident.severity, incident.title
            ),
            |ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "{} // {:.0}s ago",
                        incident.machine, incident.age_seconds
                    ))
                    .monospace()
                    .color(theme::muted()),
                );

                ui.label(egui::RichText::new(&incident.summary).color(theme::white()));

                ui.horizontal(|ui| {
                    ui.button("OPEN REPLAY");
                    ui.button("EXPORT .wynobs");
                    ui.button("COMPARE");
                });
            },
        );

        ui.add_space(6.0);
    }
}

pub fn cross_machine_view(
    ui: &mut egui::Ui,
    machine: &FleetMachine,
    fleet: &FleetState,
    elapsed: f32,
) {
    heading(
        ui,
        "CROSS-MACHINE CAUSALITY",
        "align Fleet events around one timestamp to reconstruct distributed failures",
    );

    let data = ExtremeSnapshot::mock(machine, fleet, elapsed);

    for event in &data.cross_machine {
        ui.horizontal_wrapped(|ui| {
            ui.add_sized(
                [92.0, 24.0],
                egui::Label::new(
                    egui::RichText::new(format!("{:+} ms", event.offset_ms))
                        .monospace()
                        .color(if event.offset_ms == 0 {
                            theme::pink()
                        } else {
                            theme::muted()
                        }),
                ),
            );

            ui.add_sized(
                [160.0, 24.0],
                egui::Label::new(
                    egui::RichText::new(&event.machine)
                        .monospace()
                        .strong()
                        .color(theme::blue()),
                ),
            );

            truth(ui, event.truth);

            ui.label(
                egui::RichText::new(&event.event)
                    .monospace()
                    .color(theme::white()),
            );
        });

        ui.separator();
    }
}

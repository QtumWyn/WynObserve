use eframe::egui::{self, Color32, Stroke, StrokeKind};

use crate::{
    model::{
        ComponentId, InstructionSample, ProcessSnapshot, SystemSnapshot, TruthLevel, format_bytes,
    },
    theme,
};
use wyn_protocol::process_memory::ProcessMemoryMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessTab {
    Running,
    Sleeping,
}

impl Default for ProcessTab {
    fn default() -> Self {
        Self::Running
    }
}

fn heading(ui: &mut egui::Ui, text: &str, subtitle: &str) {
    ui.label(
        egui::RichText::new(text)
            .size(19.0)
            .strong()
            .color(theme::white()),
    );
    ui.label(
        egui::RichText::new(subtitle)
            .size(12.0)
            .color(theme::muted()),
    );
    ui.add_space(8.0);
}

fn metric(ui: &mut egui::Ui, label: &str, value: impl Into<String>, color: Color32) {
    egui::Frame::new()
        .fill(theme::panel())
        .stroke(Stroke::new(1.0, theme::border()))
        .corner_radius(6.0)
        .inner_margin(9.0)
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(label)
                    .size(10.5)
                    .monospace()
                    .color(theme::muted()),
            );
            ui.label(
                egui::RichText::new(value.into())
                    .size(16.0)
                    .monospace()
                    .strong()
                    .color(color),
            );
        });
}

fn optional_u32(value: Option<u32>, suffix: &str) -> String {
    value
        .map(|value| format!("{value} {suffix}"))
        .unwrap_or_else(|| "N/A".to_string())
}

fn optional_f32(value: Option<f32>, suffix: &str) -> String {
    value
        .map(|value| format!("{value:.1} {suffix}"))
        .unwrap_or_else(|| "N/A".to_string())
}

fn optional_pcie(rx: Option<f32>, tx: Option<f32>) -> String {
    match (rx, tx) {
        (Some(rx), Some(tx)) => {
            format!("RX {rx:.2} / TX {tx:.2} MiB/s")
        }
        _ => "N/A".to_string(),
    }
}

fn progress(ui: &mut egui::Ui, fraction: f32, label: &str, color: Color32) {
    let fraction = fraction.clamp(0.0, 1.0);
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 20.0), egui::Sense::hover());
    let painter = ui.painter();

    painter.rect_filled(rect, 3.0, theme::track_bg());

    let fill =
        egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * fraction, rect.height()));

    painter.rect_filled(fill, 3.0, color);
    painter.rect_stroke(
        rect,
        3.0,
        Stroke::new(1.0, theme::border()),
        StrokeKind::Inside,
    );

    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::monospace(10.5),
        theme::white(),
    );
}

fn table_cell(ui: &mut egui::Ui, width: f32, text: impl Into<String>, color: Color32) {
    ui.add_sized(
        [width, 22.0],
        egui::Label::new(egui::RichText::new(text.into()).monospace().color(color)).truncate(),
    );
}

fn table_header(ui: &mut egui::Ui, width: f32, text: &str) {
    ui.add_sized(
        [width, 22.0],
        egui::Label::new(
            egui::RichText::new(text)
                .monospace()
                .strong()
                .color(theme::muted()),
        )
        .truncate(),
    );
}

fn truth_badge(ui: &mut egui::Ui, truth: TruthLevel) {
    ui.label(
        egui::RichText::new(format!("{} {}", truth.glyph(), truth.label()))
            .monospace()
            .size(10.5)
            .color(theme::truth_color(truth)),
    );
}

pub fn inspector(
    ui: &mut egui::Ui,
    snapshot: &SystemSnapshot,
    selected: ComponentId,
    descend: bool,
) {
    heading(
        ui,
        selected.label(),
        "SELECTED COMPONENT // click the schematic to inspect",
    );

    match selected {
        ComponentId::Cpu => {
            metric(
                ui,
                "PACKAGE LOAD",
                format!("{:.1}%", snapshot.cpu.package_usage),
                theme::pink(),
            );
            metric(
                ui,
                "TEMPERATURE",
                format!("{:.1} °C", snapshot.cpu.package_temperature_c),
                theme::gold(),
            );
            metric(
                ui,
                "CONTEXT SWITCHES",
                format!("{} / s", snapshot.cpu.context_switches_per_second),
                theme::blue(),
            );
            metric(
                ui,
                "MIGRATIONS",
                format!("{} / s", snapshot.cpu.migrations_per_second),
                theme::violet(),
            );
            truth_badge(ui, TruthLevel::Sampled);
        }
        ComponentId::Memory => {
            metric(
                ui,
                "TOTAL",
                format_bytes(snapshot.memory.total_bytes),
                theme::white(),
            );
            metric(
                ui,
                "USED",
                format_bytes(snapshot.memory.used_bytes),
                theme::pink(),
            );
            metric(
                ui,
                "AVAILABLE",
                format_bytes(snapshot.memory.available_bytes),
                theme::blue(),
            );
            metric(
                ui,
                "PAGE FAULTS",
                format!("{} / s", snapshot.memory.page_faults_per_second),
                theme::gold(),
            );
            truth_badge(ui, TruthLevel::Sampled);
        }
        ComponentId::Gpu => {
            metric(ui, "MODEL", &snapshot.gpu.model, theme::white());
            metric(
                ui,
                "GPU LOAD",
                format!("{:.1}%", snapshot.gpu.utilization),
                theme::violet(),
            );
            metric(
                ui,
                "VRAM",
                format!(
                    "{} / {} // {:.1}%",
                    format_bytes(snapshot.gpu.vram_used_bytes),
                    format_bytes(snapshot.gpu.vram_total_bytes),
                    snapshot.gpu.vram_used_percent(),
                ),
                theme::pink(),
            );
            metric(
                ui,
                "VRAM ACTIVITY",
                format!("{:.1}%", snapshot.gpu.memory_activity_percent),
                theme::violet(),
            );
            metric(
                ui,
                "THERMAL",
                format!("{:.1} °C", snapshot.gpu.temperature_c),
                theme::gold(),
            );
            metric(
                ui,
                "POWER",
                format!("{:.1} W", snapshot.gpu.power_watts),
                theme::gold(),
            );
            metric(
                ui,
                "GRAPHICS CLOCK",
                optional_u32(snapshot.gpu.graphics_clock_mhz, "MHz"),
                theme::blue(),
            );
            metric(
                ui,
                "MEMORY CLOCK",
                optional_u32(snapshot.gpu.memory_clock_mhz, "MHz"),
                theme::blue(),
            );
            metric(
                ui,
                "P-STATE",
                snapshot.gpu.performance_state.as_deref().unwrap_or("N/A"),
                theme::violet(),
            );
            metric(
                ui,
                "FAN",
                snapshot
                    .gpu
                    .fan_percent
                    .map(|value| format!("{value}%"))
                    .unwrap_or_else(|| "N/A".to_string()),
                theme::green(),
            );
            metric(
                ui,
                "POWER LIMIT",
                optional_f32(snapshot.gpu.power_limit_watts, "W"),
                theme::pink(),
            );
            metric(
                ui,
                "PCIe",
                optional_pcie(snapshot.gpu.pcie_rx_mib_s, snapshot.gpu.pcie_tx_mib_s),
                theme::blue(),
            );
            truth_badge(ui, TruthLevel::Sampled);
        }
        ComponentId::Npu => {
            metric(ui, "MODEL", &snapshot.npu.model, theme::white());
            metric(
                ui,
                "NPU LOAD",
                format!("{:.1}%", snapshot.npu.utilization),
                theme::violet(),
            );
            metric(
                ui,
                "FREQUENCY",
                format!(
                    "{} / {} MHz",
                    snapshot.npu.current_frequency_mhz, snapshot.npu.max_frequency_mhz
                ),
                theme::blue(),
            );
            metric(
                ui,
                "RESIDENT MEMORY",
                format_bytes(snapshot.npu.memory_used_bytes),
                theme::pink(),
            );
            metric(ui, "POWER STATE", &snapshot.npu.power_state, theme::gold());
            truth_badge(ui, TruthLevel::Observed);
        }
        ComponentId::Storage => {
            metric(ui, "MODEL", &snapshot.storage.model, theme::white());
            metric(
                ui,
                "DEVICE",
                if snapshot.storage.device.is_empty() {
                    "N/A"
                } else {
                    &snapshot.storage.device
                },
                theme::violet(),
            );
            metric(
                ui,
                "THROUGHPUT",
                if snapshot.storage.rates_available {
                    format!(
                        "R {:.2} / W {:.2} MiB/s",
                        snapshot.storage.read_mib_s, snapshot.storage.write_mib_s,
                    )
                } else {
                    "awaiting rate baseline".to_string()
                },
                theme::blue(),
            );
            metric(
                ui,
                "IOPS",
                if snapshot.storage.rates_available {
                    format!(
                        "R {:.0} / W {:.0}",
                        snapshot.storage.read_iops, snapshot.storage.write_iops,
                    )
                } else {
                    "N/A".to_string()
                },
                theme::pink(),
            );
            metric(
                ui,
                "UTILIZATION",
                if snapshot.storage.rates_available {
                    format!("{:.1}%", snapshot.storage.utilization)
                } else {
                    "N/A".to_string()
                },
                theme::gold(),
            );
            metric(
                ui,
                "QUEUE",
                if snapshot.storage.rates_available {
                    format!(
                        "avg {:.2} // {} in flight",
                        snapshot.storage.average_queue_depth, snapshot.storage.io_in_progress,
                    )
                } else {
                    format!("{} in flight", snapshot.storage.io_in_progress)
                },
                theme::green(),
            );
            truth_badge(ui, TruthLevel::Sampled);
        }
        ComponentId::Network => {
            metric(ui, "INTERFACE", &snapshot.network.interface, theme::white());
            metric(
                ui,
                "THROUGHPUT",
                if snapshot.network.rates_available {
                    format!(
                        "RX {:.2} / TX {:.2} MiB/s",
                        snapshot.network.rx_mib_s, snapshot.network.tx_mib_s,
                    )
                } else {
                    "awaiting rate baseline".to_string()
                },
                theme::green(),
            );
            metric(
                ui,
                "PACKETS",
                if snapshot.network.rates_available {
                    format!(
                        "RX {} / TX {} pps",
                        snapshot.network.rx_packets_per_second,
                        snapshot.network.tx_packets_per_second,
                    )
                } else {
                    "N/A".to_string()
                },
                theme::blue(),
            );
            metric(
                ui,
                "ERRORS / DROPS",
                if snapshot.network.rates_available {
                    format!(
                        "ERR {} / {}  DROP {} / {}",
                        snapshot.network.rx_errors_per_second,
                        snapshot.network.tx_errors_per_second,
                        snapshot.network.rx_drops_per_second,
                        snapshot.network.tx_drops_per_second,
                    )
                } else {
                    "N/A".to_string()
                },
                theme::gold(),
            );
            metric(
                ui,
                "TCP ESTABLISHED",
                if snapshot.network.connections_available {
                    snapshot.network.connections.to_string()
                } else {
                    "N/A".to_string()
                },
                theme::violet(),
            );
            truth_badge(ui, TruthLevel::Sampled);
        }
    }

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);

    ui.label(
        egui::RichText::new(if descend {
            "EVENT VEIN // DESCENDED"
        } else {
            "EVENT VEIN"
        })
        .size(12.0)
        .monospace()
        .color(theme::muted()),
    );

    for event in snapshot.events.iter().take(if descend { 5 } else { 3 }) {
        egui::Frame::new()
            .fill(theme::panel_alt())
            .corner_radius(4.0)
            .inner_margin(7.0)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    truth_badge(ui, event.truth);
                    ui.label(
                        egui::RichText::new(&event.message)
                            .monospace()
                            .size(11.5)
                            .color(theme::text()),
                    );
                });
            });
    }
}

fn terminal_sample_text(sample: &InstructionSample) -> String {
    let cpu = sample
        .cpu_id
        .map(|id| format!("CPU{id:02}"))
        .unwrap_or_else(|| "CPU??".to_string());

    format!(
        "◉ +{:05.2}s  {}  {}[{}:{}]\n  0x{:016X}  {:<20}  {}",
        sample.age_seconds,
        cpu,
        sample.process_name,
        sample.pid,
        sample.tid,
        sample.address,
        sample.bytes_hex(),
        sample.assembly(),
    )
}

fn sample_inspector(ui: &mut egui::Ui, sample: &InstructionSample, descend: bool) {
    ui.label(
        egui::RichText::new("INSPECT SAMPLE")
            .size(12.5)
            .monospace()
            .strong()
            .color(theme::pink()),
    );

    ui.add_space(4.0);

    egui::Frame::new()
        .fill(theme::panel())
        .stroke(Stroke::new(1.0, theme::border()))
        .corner_radius(6.0)
        .inner_margin(9.0)
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(format!(
                    "{} // PID {} // TID {}",
                    sample.process_name, sample.pid, sample.tid,
                ))
                .monospace()
                .size(12.0)
                .strong()
                .color(theme::white()),
            );

            ui.horizontal_wrapped(|ui| {
                truth_badge(ui, sample.truth);

                ui.label(
                    egui::RichText::new(sample.architecture.label())
                        .monospace()
                        .size(10.5)
                        .color(theme::blue()),
                );

                if let Some(cpu_id) = sample.cpu_id {
                    ui.label(
                        egui::RichText::new(format!("CPU{cpu_id:02}"))
                            .monospace()
                            .size(10.5)
                            .color(theme::violet()),
                    );
                }
            });

            ui.add_space(5.0);

            ui.label(
                egui::RichText::new(format!("0x{:016X}", sample.address))
                    .monospace()
                    .size(11.5)
                    .color(theme::muted()),
            );

            ui.label(
                egui::RichText::new(sample.bytes_hex())
                    .monospace()
                    .size(13.0)
                    .strong()
                    .color(theme::blue()),
            );

            ui.label(
                egui::RichText::new(sample.assembly())
                    .monospace()
                    .size(14.0)
                    .strong()
                    .color(theme::pink()),
            );

            if let Some(note) = &sample.note {
                ui.add_space(5.0);
                ui.label(egui::RichText::new(note).size(11.0).color(theme::muted()));
            }

            if descend {
                ui.add_space(6.0);
                ui.separator();

                ui.label(
                    egui::RichText::new(format!(
                        "sequence     {}\nage          {:.2}s\ncomponent    {}\narchitecture {}",
                        sample.sequence,
                        sample.age_seconds,
                        sample.component.label(),
                        sample.architecture.label(),
                    ))
                    .monospace()
                    .size(10.5)
                    .color(theme::muted()),
                );
            }
        });
}

pub fn instruction_feed(
    ui: &mut egui::Ui,
    snapshot: &SystemSnapshot,
    component: ComponentId,
    selected_sequence: &mut Option<u64>,
    descend: bool,
    _viewport_height: f32,
) {
    // IMPORTANT:
    //
    // Do not estimate heights from the whole window anymore.
    // We ask egui what is actually left in THIS column after every header,
    // label, margin, zoom factor, and parent panel has consumed its space.

    heading(
        ui,
        "INSTRUCTION VEIN",
        &format!(
            "{} CONTEXT // live selected-process x86-64 samples",
            component.label()
        ),
    );

    ui.horizontal_wrapped(|ui| {
        truth_badge(ui, TruthLevel::Sampled);

        ui.label(
            egui::RichText::new("LIVE PERF STREAM")
                .size(10.5)
                .monospace()
                .strong()
                .color(theme::pink()),
        );
    });

    ui.add(
        egui::Label::new(
            egui::RichText::new(
                "Host CPU execution sampled from the selected process. Component selection is visualization context.",
            )
                .size(10.5)
                .color(theme::muted()),
        )
            .wrap(),
    );

    ui.add_space(8.0);

    let filtered = snapshot.instruction_samples.iter().collect::<Vec<_>>();

    if filtered.is_empty() {
        ui.add(
            egui::Label::new(
                egui::RichText::new(
                    "No live instruction samples yet. Select a process in PROCESSES, then return to THE MACHINE."
                )
                    .monospace()
                    .color(theme::muted()),
            )
                .wrap(),
        );

        return;
    }

    let newest = filtered[0];

    let selection_is_valid = selected_sequence
        .map(|sequence| filtered.iter().any(|sample| sample.sequence == sequence))
        .unwrap_or(false);

    if !selection_is_valid {
        *selected_sequence = Some(newest.sequence);
    }

    let selected_sample = filtered
        .iter()
        .copied()
        .find(|sample| Some(sample.sequence) == *selected_sequence)
        .unwrap_or(newest);

    // We now split the ACTUAL REMAINING SPACE, rather than guessing.
    //
    // In normal mode the terminal gets ~58% and inspector ~42%.
    // DESCEND gives the inspector even more room because it contains
    // additional metadata.
    let remaining = ui.available_height().max(260.0);

    let inspector_fraction = if descend { 0.48 } else { 0.40 };
    let gap_and_separator = 18.0;

    let inspector_height = (remaining * inspector_fraction).clamp(145.0, 330.0);

    let terminal_height = (remaining - inspector_height - gap_and_separator).max(120.0);

    // TOP: live terminal stream.
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), terminal_height),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            egui::Frame::new()
                .fill(theme::terminal_bg())
                .stroke(Stroke::new(1.0, theme::border()))
                .corner_radius(5.0)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt(format!("instruction_terminal_{component:?}"))
                        .auto_shrink([false, false])
                        .stick_to_bottom(true)
                        .animated(true)
                        .show(ui, |ui| {
                            for sample in filtered.iter().rev() {
                                let selected = *selected_sequence == Some(sample.sequence);

                                let color = if selected {
                                    theme::white()
                                } else if sample.sequence == newest.sequence {
                                    theme::pink()
                                } else {
                                    theme::text()
                                };

                                let response = ui.selectable_label(
                                    selected,
                                    egui::RichText::new(terminal_sample_text(sample))
                                        .monospace()
                                        .size(12.5)
                                        .color(color),
                                );

                                if response.clicked() {
                                    *selected_sequence = Some(sample.sequence);
                                }

                                ui.add_space(3.0);
                            }
                        });
                });
        },
    );

    ui.add_space(6.0);
    ui.separator();
    ui.add_space(6.0);

    // BOTTOM: sample inspector.
    //
    // The inspector now owns a guaranteed viewport. If its content grows
    // beyond that viewport, IT scrolls instead of disappearing below the
    // application window.
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), inspector_height),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            egui::ScrollArea::vertical()
                .id_salt(format!("instruction_inspector_{component:?}"))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    sample_inspector(ui, selected_sample, descend);
                });
        },
    );
}

pub fn cpu_view(ui: &mut egui::Ui, snapshot: &SystemSnapshot, descend: bool) {
    heading(
        ui,
        "CPU // SCHEDULER CHAMBER",
        "logical CPUs, active work, frequency and migration pressure",
    );

    ui.horizontal(|ui| {
        metric(
            ui,
            "PACKAGE",
            format!("{:.1}%", snapshot.cpu.package_usage),
            theme::pink(),
        );
        metric(
            ui,
            "THERMAL",
            format!("{:.1} °C", snapshot.cpu.package_temperature_c),
            theme::gold(),
        );
        metric(
            ui,
            "CTX SWITCH",
            format!("{}/s", snapshot.cpu.context_switches_per_second),
            theme::blue(),
        );
        metric(
            ui,
            "MIGRATIONS",
            format!("{}/s", snapshot.cpu.migrations_per_second),
            theme::violet(),
        );
    });

    ui.add_space(8.0);

    egui::ScrollArea::vertical().show(ui, |ui| {
        for chunk in snapshot.cpu.logical_cpus.chunks(4) {
            ui.columns(4, |cols| {
                for (col, core) in cols.iter_mut().zip(chunk.iter()) {
                    egui::Frame::new()
                        .fill(theme::panel())
                        .stroke(Stroke::new(1.0, theme::border()))
                        .corner_radius(5.0)
                        .inner_margin(8.0)
                        .show(col, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("CPU {:02}", core.logical_id))
                                        .monospace()
                                        .strong()
                                        .color(theme::white()),
                                );

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            egui::RichText::new(format!("{:.0}%", core.usage))
                                                .monospace()
                                                .color(theme::pink()),
                                        );
                                    },
                                );
                            });

                            progress(ui, core.usage / 100.0, &core.process, theme::wine());

                            ui.label(
                                egui::RichText::new(format!(
                                    "{} MHz  •  {:.1} °C",
                                    core.frequency_mhz, core.temperature_c
                                ))
                                .size(10.5)
                                .monospace()
                                .color(theme::muted()),
                            );

                            if descend {
                                ui.label(
                                    egui::RichText::new(
                                        "◇ affinity / cache / IRQ details hook here",
                                    )
                                    .size(10.0)
                                    .monospace()
                                    .color(theme::blue()),
                                );
                            }
                        });
                }
            });

            ui.add_space(6.0);
        }
    });
}

pub fn memory_view(
    ui: &mut egui::Ui,
    snapshot: &SystemSnapshot,
    descend: bool,
    process_name: Option<&str>,
    pid: Option<u32>,
    map: Option<&ProcessMemoryMap>,
    loading: bool,
    error: Option<&str>,
) {
    heading(
        ui,
        "MEMORY // PAGE CATHEDRAL",
        "system memory composition, page behavior and future virtual-memory descent",
    );

    let used_ratio = snapshot.memory.used_bytes as f32 / snapshot.memory.total_bytes.max(1) as f32;

    progress(
        ui,
        used_ratio,
        &format!(
            "{} / {}",
            format_bytes(snapshot.memory.used_bytes),
            format_bytes(snapshot.memory.total_bytes)
        ),
        theme::blue(),
    );

    ui.add_space(10.0);

    ui.horizontal(|ui| {
        metric(
            ui,
            "ACTIVE",
            format_bytes(snapshot.memory.active_bytes),
            theme::pink(),
        );
        metric(
            ui,
            "CACHE",
            format_bytes(snapshot.memory.cached_bytes),
            theme::blue(),
        );
        metric(
            ui,
            "DIRTY",
            format_bytes(snapshot.memory.dirty_bytes),
            theme::gold(),
        );
        metric(
            ui,
            "FAULTS",
            format!("{}/s", snapshot.memory.page_faults_per_second),
            theme::violet(),
        );
    });

    ui.add_space(12.0);

    egui::Frame::new()
        .fill(theme::panel())
        .stroke(Stroke::new(1.0, theme::border()))
        .corner_radius(6.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new("GLOBAL MEMORY MAP // PHYSICAL MEMORY")
                    .monospace()
                    .strong()
                    .color(theme::white()),
            );

            ui.label(
                egui::RichText::new(
                    "live machine-wide memory telemetry // rows are relative to total RAM and may overlap",
                )
                    .size(10.5)
                    .color(theme::muted()),
            );

            ui.add_space(8.0);

            let total = snapshot.memory.total_bytes.max(1);

            for (label, bytes, color) in [
                (
                    "used",
                    snapshot.memory.used_bytes,
                    theme::pink(),
                ),
                (
                    "available",
                    snapshot.memory.available_bytes,
                    theme::green(),
                ),
                (
                    "active",
                    snapshot.memory.active_bytes,
                    theme::violet(),
                ),
                (
                    "cached",
                    snapshot.memory.cached_bytes,
                    theme::blue(),
                ),
                (
                    "dirty",
                    snapshot.memory.dirty_bytes,
                    theme::gold(),
                ),
            ] {
                let fraction =
                    bytes as f32 / total as f32;

                progress(
                    ui,
                    fraction,
                    &format!(
                        "{label:<12} {} // {:>5.1}%",
                        format_bytes(bytes),
                        fraction * 100.0,
                    ),
                    color,
                );

                ui.add_space(4.0);
            }

            ui.add_space(4.0);

            ui.label(
                egui::RichText::new(format!(
                    "{} page faults / s",
                    snapshot.memory.page_faults_per_second,
                ))
                    .monospace()
                    .color(theme::muted()),
            );
        });

    egui::Frame::new()
        .fill(theme::panel())
        .stroke(Stroke::new(1.0, theme::border()))
        .corner_radius(6.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new("PROCESS MEMORY MAP")
                        .monospace()
                        .strong()
                        .color(theme::white()),
                );

                if let (Some(name), Some(pid)) = (process_name, pid) {
                    ui.label(
                        egui::RichText::new(format!("// {name} // PID {pid}"))
                            .monospace()
                            .color(theme::blue()),
                    );
                }
            });

            ui.add_space(6.0);

            if loading {
                ui.horizontal(|ui| {
                    ui.spinner();

                    ui.label(
                        egui::RichText::new("requesting live address space...")
                            .monospace()
                            .color(theme::blue()),
                    );
                });

                return;
            }

            if let Some(error) = error {
                ui.label(
                    egui::RichText::new(format!("INSPECTION ERROR // {error}"))
                        .monospace()
                        .color(theme::gold()),
                );

                return;
            }

            let Some(map) = map else {
                ui.label(
                    egui::RichText::new("Select a process from PROCESSES and click MAP.")
                        .monospace()
                        .color(theme::muted()),
                );

                return;
            };

            let total = map.total_virtual_bytes.max(1);

            let rows = [
                ("heap", map.overview.heap_bytes, theme::pink()),
                ("anonymous", map.overview.anonymous_bytes, theme::violet()),
                (
                    "shared libraries",
                    map.overview.shared_library_bytes,
                    theme::blue(),
                ),
                (
                    "mapped files",
                    map.overview.mapped_file_bytes,
                    theme::green(),
                ),
                ("stacks", map.overview.stack_bytes, theme::gold()),
                (
                    "executable image",
                    map.overview.executable_image_bytes,
                    theme::white(),
                ),
                ("special", map.overview.special_bytes, theme::muted()),
            ];

            for (label, bytes, color) in rows {
                let fraction = bytes as f32 / total as f32;

                progress(
                    ui,
                    fraction,
                    &format!(
                        "{label:<18} {} // {:>5.1}%",
                        format_bytes(bytes),
                        fraction * 100.0,
                    ),
                    color,
                );

                ui.add_space(4.0);
            }

            ui.add_space(5.0);

            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "{} virtual",
                        format_bytes(map.total_virtual_bytes)
                    ))
                    .monospace()
                    .color(theme::blue()),
                );

                ui.label(
                    egui::RichText::new(format!("{} regions", map.region_count))
                        .monospace()
                        .color(theme::pink()),
                );

                ui.label(
                    egui::RichText::new(format!("{} W+X", map.writable_executable_regions))
                        .monospace()
                        .color(if map.writable_executable_regions > 0 {
                            theme::gold()
                        } else {
                            theme::green()
                        }),
                );
            });

            if descend {
                ui.add_space(8.0);

                ui.label(
                    egui::RichText::new("DESCEND // detailed regions available in MEMORY MAP")
                        .monospace()
                        .size(11.0)
                        .color(theme::blue()),
                );
            }
        });
}

pub fn gpu_view(ui: &mut egui::Ui, snapshot: &SystemSnapshot, descend: bool) {
    heading(
        ui,
        "GPU // COMPUTE NAVE",
        "NVML identity, compute, VRAM, clocks, thermals, power and PCIe telemetry",
    );

    if !snapshot.gpu.available {
        egui::Frame::new()
            .fill(theme::panel())
            .stroke(Stroke::new(1.0, theme::border()))
            .corner_radius(6.0)
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("NO GPU TELEMETRY AVAILABLE")
                        .monospace()
                        .strong()
                        .color(theme::muted()),
                );
            });
        return;
    }

    egui::Frame::new()
        .fill(theme::panel())
        .stroke(Stroke::new(1.0, theme::border()))
        .corner_radius(6.0)
        .inner_margin(10.0)
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(&snapshot.gpu.model)
                    .monospace()
                    .strong()
                    .color(theme::white()),
            );
            ui.label(
                egui::RichText::new(format!(
                    "{} // {}",
                    snapshot.gpu.pci_address, snapshot.gpu.uuid,
                ))
                .monospace()
                .size(10.0)
                .color(theme::muted()),
            );
        });

    ui.add_space(10.0);

    ui.horizontal_wrapped(|ui| {
        metric(
            ui,
            "UTILIZATION",
            format!("{:.1}%", snapshot.gpu.utilization),
            theme::violet(),
        );
        metric(
            ui,
            "THERMAL",
            format!("{:.1} °C", snapshot.gpu.temperature_c),
            theme::gold(),
        );
        metric(
            ui,
            "POWER",
            format!("{:.1} W", snapshot.gpu.power_watts),
            theme::pink(),
        );
        metric(
            ui,
            "P-STATE",
            snapshot.gpu.performance_state.as_deref().unwrap_or("N/A"),
            theme::blue(),
        );
    });

    ui.add_space(10.0);

    progress(
        ui,
        snapshot.gpu.vram_used_percent() / 100.0,
        &format!(
            "VRAM {} / {} // {:.1}%",
            format_bytes(snapshot.gpu.vram_used_bytes),
            format_bytes(snapshot.gpu.vram_total_bytes),
            snapshot.gpu.vram_used_percent(),
        ),
        theme::violet(),
    );

    ui.add_space(5.0);

    progress(
        ui,
        snapshot.gpu.memory_activity_percent / 100.0,
        &format!("VRAM ACTIVITY {:.1}%", snapshot.gpu.memory_activity_percent,),
        theme::blue(),
    );

    ui.add_space(12.0);

    ui.columns(2, |columns| {
        egui::Frame::new()
            .fill(theme::panel())
            .stroke(Stroke::new(1.0, theme::border()))
            .corner_radius(6.0)
            .inner_margin(10.0)
            .show(&mut columns[0], |ui| {
                ui.label(
                    egui::RichText::new("CLOCK DOMAINS")
                        .monospace()
                        .strong()
                        .color(theme::white()),
                );

                for (label, value) in [
                    ("GRAPHICS", snapshot.gpu.graphics_clock_mhz),
                    ("SM", snapshot.gpu.sm_clock_mhz),
                    ("MEMORY", snapshot.gpu.memory_clock_mhz),
                    ("VIDEO", snapshot.gpu.video_clock_mhz),
                ] {
                    ui.label(
                        egui::RichText::new(format!("{label:<9} {}", optional_u32(value, "MHz"),))
                            .monospace()
                            .color(theme::blue()),
                    );
                }
            });

        egui::Frame::new()
            .fill(theme::panel())
            .stroke(Stroke::new(1.0, theme::border()))
            .corner_radius(6.0)
            .inner_margin(10.0)
            .show(&mut columns[1], |ui| {
                ui.label(
                    egui::RichText::new("BOARD / TRANSPORT")
                        .monospace()
                        .strong()
                        .color(theme::white()),
                );

                ui.label(
                    egui::RichText::new(format!(
                        "FAN          {}",
                        snapshot
                            .gpu
                            .fan_percent
                            .map(|value| format!("{value}%"))
                            .unwrap_or_else(|| "N/A".into()),
                    ))
                    .monospace()
                    .color(theme::green()),
                );

                ui.label(
                    egui::RichText::new(format!(
                        "POWER LIMIT  {}",
                        optional_f32(snapshot.gpu.power_limit_watts, "W"),
                    ))
                    .monospace()
                    .color(theme::pink()),
                );

                ui.label(
                    egui::RichText::new(format!(
                        "PCIe         {}",
                        optional_pcie(snapshot.gpu.pcie_rx_mib_s, snapshot.gpu.pcie_tx_mib_s,),
                    ))
                    .monospace()
                    .color(theme::violet()),
                );
            });
    });

    ui.add_space(12.0);

    egui::Frame::new()
        .fill(theme::panel())
        .stroke(Stroke::new(1.0, theme::border()))
        .corner_radius(6.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new("COMPUTE GRID")
                    .monospace()
                    .strong()
                    .color(theme::white()),
            );

            ui.label(
                egui::RichText::new(
                    "Aggregate utilization only. Cells do not claim literal physical SM ownership.",
                )
                .size(11.0)
                .color(theme::muted()),
            );

            ui.add_space(8.0);

            let cols = 18;
            let rows = 7;
            let active = ((cols * rows) as f32 * snapshot.gpu.utilization / 100.0) as usize;

            for row in 0..rows {
                ui.horizontal(|ui| {
                    for col in 0..cols {
                        let i = row * cols + col;

                        let color = if i < active {
                            theme::violet()
                        } else {
                            theme::grid_inactive()
                        };

                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(22.0, 18.0), egui::Sense::hover());

                        ui.painter().rect_filled(rect, 2.0, color);
                    }
                });
            }

            if descend {
                ui.add_space(8.0);

                ui.label(
                    egui::RichText::new(
                        "DESCEND target: NVML engines → per-process VRAM → PCIe → profiler modes",
                    )
                    .monospace()
                    .size(11.0)
                    .color(theme::blue()),
                );
            }
        });
}

pub fn npu_view(ui: &mut egui::Ui, snapshot: &SystemSnapshot, descend: bool) {
    heading(
        ui,
        "NPU // NEURAL ENGINE",
        "live accelerator activity and device identity",
    );

    if !snapshot.npu.available {
        ui.label(
            egui::RichText::new("No NPU is reported by the selected machine.")
                .monospace()
                .color(theme::muted()),
        );
        return;
    }

    ui.horizontal_wrapped(|ui| {
        metric(ui, "MODEL", &snapshot.npu.model, theme::white());
        metric(
            ui,
            "UTILIZATION",
            format!("{:.1}%", snapshot.npu.utilization),
            theme::violet(),
        );
        metric(
            ui,
            "FREQUENCY",
            format!(
                "{} / {} MHz",
                snapshot.npu.current_frequency_mhz, snapshot.npu.max_frequency_mhz
            ),
            theme::blue(),
        );
        metric(ui, "POWER STATE", &snapshot.npu.power_state, theme::gold());
    });

    ui.add_space(10.0);
    progress(
        ui,
        snapshot.npu.utilization / 100.0,
        &format!("NPU activity // {:.1}%", snapshot.npu.utilization),
        theme::violet(),
    );

    ui.add_space(10.0);
    metric(
        ui,
        "RESIDENT NPU BUFFERS",
        format_bytes(snapshot.npu.memory_used_bytes),
        theme::pink(),
    );
    metric(
        ui,
        "PCI ADDRESS",
        if snapshot.npu.pci_address.is_empty() {
            "not reported"
        } else {
            &snapshot.npu.pci_address
        },
        theme::green(),
    );

    if descend {
        ui.add_space(10.0);
        ui.label(
            egui::RichText::new(
                "DESCEND target: OpenVINO execution queue → model inference → command attribution",
            )
            .monospace()
            .color(theme::blue()),
        );
    }
}

pub fn npu_memory_view(ui: &mut egui::Ui, snapshot: &SystemSnapshot, descend: bool) {
    heading(
        ui,
        "NPU MEMORY // RESIDENT BUFFERS",
        "device-resident buffer objects reported by the NPU driver",
    );

    if !snapshot.npu.available {
        ui.label("No NPU is reported by the selected machine.");
        return;
    }

    metric(
        ui,
        "CURRENT RESIDENT MEMORY",
        format_bytes(snapshot.npu.memory_used_bytes),
        theme::pink(),
    );
    metric(ui, "DEVICE", &snapshot.npu.model, theme::white());
    metric(
        ui,
        "PCI ADDRESS",
        if snapshot.npu.pci_address.is_empty() {
            "not reported"
        } else {
            &snapshot.npu.pci_address
        },
        theme::green(),
    );

    ui.add_space(10.0);
    ui.label(
        egui::RichText::new(
            "The current backend reports resident NPU buffer bytes, not a total-capacity percentage.",
        )
        .monospace()
        .size(11.0)
        .color(theme::muted()),
    );

    if descend {
        ui.label(
            egui::RichText::new(
                "Future: model allocation history // per-inference buffers // OpenVINO model attribution",
            )
            .monospace()
            .color(theme::blue()),
        );
    }
}

pub fn npu_runtime_view(ui: &mut egui::Ui, snapshot: &SystemSnapshot, descend: bool) {
    heading(
        ui,
        "NPU RUNTIME // POWER + CLOCK",
        "runtime state for the selected neural accelerator",
    );

    if !snapshot.npu.available {
        ui.label("No NPU is reported by the selected machine.");
        return;
    }

    let frequency_fraction = if snapshot.npu.max_frequency_mhz == 0 {
        0.0
    } else {
        snapshot.npu.current_frequency_mhz as f32 / snapshot.npu.max_frequency_mhz as f32
    };

    metric(ui, "POWER STATE", &snapshot.npu.power_state, theme::gold());
    metric(
        ui,
        "CURRENT CLOCK",
        format!("{} MHz", snapshot.npu.current_frequency_mhz),
        theme::blue(),
    );
    metric(
        ui,
        "MAX CLOCK",
        format!("{} MHz", snapshot.npu.max_frequency_mhz),
        theme::white(),
    );

    ui.add_space(8.0);
    progress(
        ui,
        frequency_fraction,
        &format!(
            "clock // {} / {} MHz",
            snapshot.npu.current_frequency_mhz, snapshot.npu.max_frequency_mhz
        ),
        theme::blue(),
    );
    ui.add_space(6.0);
    progress(
        ui,
        snapshot.npu.utilization / 100.0,
        &format!("busy // {:.1}%", snapshot.npu.utilization),
        theme::violet(),
    );

    if descend {
        ui.add_space(10.0);
        ui.label(
            egui::RichText::new(
                "Future: inference latency // queue depth // model name // voice-command workload",
            )
            .monospace()
            .color(theme::blue()),
        );
    }
}

pub fn processes_view(
    ui: &mut egui::Ui,
    snapshot: &SystemSnapshot,
    descend: bool,
    tab: &mut ProcessTab,
) -> Option<ProcessSnapshot> {
    let mut inspect_process = None;
    heading(
        ui,
        "PROCESSES // LIVING WORKLOAD",
        "live M6 process telemetry // CPU deltas, RSS, scheduler placement and process I/O",
    );

    if !snapshot.processes_available {
        ui.label(
            egui::RichText::new("No process telemetry is available for the selected machine.")
                .monospace()
                .color(theme::muted()),
        );
        return None;
    }

    ui.columns(3, |columns| {
        metric(
            &mut columns[0],
            "PROCESSES",
            snapshot.process_count.to_string(),
            theme::pink(),
        );
        metric(
            &mut columns[1],
            "THREADS",
            snapshot.thread_count.to_string(),
            theme::blue(),
        );
        metric(
            &mut columns[2],
            "EXPORTED",
            snapshot.processes.len().to_string(),
            theme::green(),
        );
    });

    ui.add_space(10.0);

    let running_count = snapshot
        .processes
        .iter()
        .filter(|process| process.state == "running")
        .count();

    let sleeping_count = snapshot
        .processes
        .iter()
        .filter(|process| process.state != "running")
        .count();

    ui.horizontal(|ui| {
        if ui
            .add(
                egui::Button::new(format!("RUNNING // {running_count}"))
                    .selected(*tab == ProcessTab::Running),
            )
            .clicked()
        {
            *tab = ProcessTab::Running;
        }

        if ui
            .add(
                egui::Button::new(format!("SLEEPING // {sleeping_count}"))
                    .selected(*tab == ProcessTab::Sleeping),
            )
            .clicked()
        {
            *tab = ProcessTab::Sleeping;
        }
    });

    ui.add_space(10.0);

    let table_width = ui.available_width();

    /*
     * Reserve a little room for inter-column
     * spacing. The remaining width is divided
     * proportionally.
     */
    let usable = (table_width - 90.0).max(900.0);

    let pid_w = usable * 0.065;
    let process_w = usable * 0.225;
    let state_w = usable * 0.095;
    let cpu_w = usable * 0.070;
    let ram_w = usable * 0.095;
    let last_cpu_w = usable * 0.075;
    let threads_w = usable * 0.075;
    let read_w = usable * 0.105;
    let write_w = usable * 0.105;
    let map_w = usable * 0.065;

    egui::Frame::new()
        .fill(theme::panel())
        .stroke(Stroke::new(1.0, theme::border()))
        .corner_radius(6.0)
        .inner_margin(8.0)
        .show(ui, |ui| {
            /*
             * Header.
             */
            ui.horizontal(|ui| {
                table_header(ui, pid_w, "PID");
                table_header(ui, process_w, "PROCESS");
                table_header(ui, state_w, "STATE");
                table_header(ui, cpu_w, "CPU");
                table_header(ui, ram_w, "RAM");
                table_header(ui, last_cpu_w, "LAST CPU");
                table_header(ui, threads_w, "THREADS");
                table_header(ui, read_w, "READ");
                table_header(ui, write_w, "WRITE");
                table_header(ui, map_w, "MEMORY");
            });

            ui.separator();

            let mut visible_processes = snapshot
                .processes
                .iter()
                .filter(|process| match tab {
                    ProcessTab::Running => process.state == "running",

                    ProcessTab::Sleeping => process.state != "running",
                })
                .collect::<Vec<_>>();

            visible_processes.sort_by(|a, b| {
                a.name
                    .to_ascii_lowercase()
                    .cmp(&b.name.to_ascii_lowercase())
                    .then_with(|| a.pid.cmp(&b.pid))
            });

            for process in visible_processes {
                let executable = process
                    .executable
                    .as_deref()
                    .unwrap_or("executable path unavailable");

                let state_color = match process.state.as_str() {
                    "running" => theme::green(),

                    "disk-sleep" | "zombie" => theme::gold(),

                    _ => theme::muted(),
                };

                let cpu_text = if process.cpu_rate_available {
                    format!("{:.1}%", process.cpu_usage)
                } else {
                    "baseline".to_string()
                };

                let memory_text = if process.memory_available {
                    format_bytes(process.memory_bytes)
                } else {
                    "N/A".to_string()
                };

                let read_text = if process.io_rates_available {
                    format!("{:.3} MiB/s", process.read_mib_s)
                } else {
                    "N/A".to_string()
                };

                let write_text = if process.io_rates_available {
                    format!("{:.3} MiB/s", process.write_mib_s)
                } else {
                    "N/A".to_string()
                };

                ui.horizontal(|ui| {
                    table_cell(ui, pid_w, process.pid.to_string(), theme::text());

                    let process_response = ui.add_sized(
                        [process_w, 22.0],
                        egui::Label::new(
                            egui::RichText::new(&process.name)
                                .monospace()
                                .color(theme::white()),
                        )
                        .truncate(),
                    );

                    process_response.on_hover_text(format!("{}\n{}", process.name, executable,));

                    table_cell(ui, state_w, &process.state, state_color);

                    table_cell(
                        ui,
                        cpu_w,
                        cpu_text,
                        if process.cpu_rate_available {
                            theme::pink()
                        } else {
                            theme::muted()
                        },
                    );

                    table_cell(ui, ram_w, memory_text, theme::text());

                    table_cell(
                        ui,
                        last_cpu_w,
                        format!("{:02}", process.last_cpu),
                        theme::blue(),
                    );

                    table_cell(ui, threads_w, process.threads.to_string(), theme::text());

                    table_cell(ui, read_w, read_text, theme::green());

                    table_cell(ui, write_w, write_text, theme::violet());

                    if ui
                        .add_sized([map_w, 22.0], egui::Button::new("MAP"))
                        .on_hover_text("Open this process's virtual memory map")
                        .clicked()
                    {
                        inspect_process = Some(process.clone());
                    }
                });

                if descend {
                    ui.horizontal(|ui| {
                        ui.add_space(pid_w);

                        let start = process
                            .started_at_unix_ms
                            .map(|value| format!("start {value} ms"))
                            .unwrap_or_else(|| "start unavailable".into());

                        ui.label(
                            egui::RichText::new(format!(
                                "↳ PPID {}   //   {}   //   {}   //   {}",
                                process.parent_pid,
                                if process.cpu_rate_available {
                                    "CPU LIVE"
                                } else {
                                    "CPU baseline"
                                },
                                if process.io_rates_available {
                                    "I/O LIVE"
                                } else {
                                    "I/O unavailable"
                                },
                                start,
                            ))
                            .monospace()
                            .size(10.0)
                            .color(theme::muted()),
                        );
                    });
                }

                ui.separator();
            }
        });

    inspect_process
}

pub fn kernel_view(ui: &mut egui::Ui, _snapshot: &SystemSnapshot, descend: bool) {
    heading(
        ui,
        "KERNEL // LOWER CHAMBERS",
        "platform-specific kernel, interrupt, driver and scheduler telemetry",
    );

    for (name, value, truth) in [
        ("Kernel", "backend not connected", TruthLevel::Sampled),
        (
            "Boot configuration",
            "awaiting platform collector",
            TruthLevel::Observed,
        ),
        (
            "Modules / drivers",
            "awaiting platform collector",
            TruthLevel::Observed,
        ),
        (
            "Interrupts",
            "awaiting platform collector",
            TruthLevel::Observed,
        ),
        (
            "Deferred work",
            "awaiting softirq / DPC collector",
            TruthLevel::Observed,
        ),
        (
            "NUMA",
            "awaiting platform topology collector",
            TruthLevel::Observed,
        ),
        ("IOMMU", "awaiting platform collector", TruthLevel::Inferred),
    ] {
        egui::Frame::new()
            .fill(theme::panel())
            .stroke(Stroke::new(1.0, theme::border()))
            .corner_radius(5.0)
            .inner_margin(9.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(name)
                            .monospace()
                            .strong()
                            .color(theme::white()),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        truth_badge(ui, truth);
                    });
                });

                ui.label(
                    egui::RichText::new(value)
                        .size(11.0)
                        .monospace()
                        .color(theme::muted()),
                );
            });

        ui.add_space(5.0);
    }

    if descend {
        ui.label(
            egui::RichText::new(
                "DESCEND target: platform tracing → perf/ETW → scheduler event timeline",
            )
            .monospace()
            .color(theme::blue()),
        );
    }
}

pub fn network_view(ui: &mut egui::Ui, snapshot: &SystemSnapshot, descend: bool) {
    heading(
        ui,
        "NETWORK // PACKET TRANSEPT",
        "live default-route interface throughput, packet rates, errors, drops and TCP state",
    );

    if !snapshot.network.available {
        ui.label(
            egui::RichText::new("No network telemetry is reported by the selected machine.")
                .monospace()
                .color(theme::muted()),
        );
        return;
    }

    ui.horizontal_wrapped(|ui| {
        metric(ui, "INTERFACE", &snapshot.network.interface, theme::white());

        metric(
            ui,
            "RX",
            if snapshot.network.rates_available {
                format!("{:.2} MiB/s", snapshot.network.rx_mib_s)
            } else {
                "baseline".to_string()
            },
            theme::green(),
        );

        metric(
            ui,
            "TX",
            if snapshot.network.rates_available {
                format!("{:.2} MiB/s", snapshot.network.tx_mib_s)
            } else {
                "baseline".to_string()
            },
            theme::blue(),
        );

        metric(
            ui,
            "TCP ESTABLISHED",
            if snapshot.network.connections_available {
                snapshot.network.connections.to_string()
            } else {
                "N/A".to_string()
            },
            theme::violet(),
        );
    });

    ui.add_space(12.0);

    egui::Frame::new()
        .fill(theme::panel())
        .stroke(Stroke::new(1.0, theme::border()))
        .corner_radius(6.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new("INTERFACE COUNTERS")
                    .monospace()
                    .strong()
                    .color(theme::white()),
            );

            ui.add_space(6.0);

            if snapshot.network.rates_available {
                ui.label(
                    egui::RichText::new(format!(
                        "PACKETS      RX {:>8} /s    TX {:>8} /s",
                        snapshot.network.rx_packets_per_second,
                        snapshot.network.tx_packets_per_second,
                    ))
                    .monospace()
                    .color(theme::blue()),
                );

                ui.label(
                    egui::RichText::new(format!(
                        "ERRORS       RX {:>8} /s    TX {:>8} /s",
                        snapshot.network.rx_errors_per_second,
                        snapshot.network.tx_errors_per_second,
                    ))
                    .monospace()
                    .color(
                        if snapshot.network.rx_errors_per_second > 0
                            || snapshot.network.tx_errors_per_second > 0
                        {
                            theme::gold()
                        } else {
                            theme::green()
                        },
                    ),
                );

                ui.label(
                    egui::RichText::new(format!(
                        "DROPS        RX {:>8} /s    TX {:>8} /s",
                        snapshot.network.rx_drops_per_second, snapshot.network.tx_drops_per_second,
                    ))
                    .monospace()
                    .color(
                        if snapshot.network.rx_drops_per_second > 0
                            || snapshot.network.tx_drops_per_second > 0
                        {
                            theme::gold()
                        } else {
                            theme::green()
                        },
                    ),
                );
            } else {
                ui.label(
                    egui::RichText::new(
                        "Waiting for a second counter sample before deriving rates.",
                    )
                    .monospace()
                    .color(theme::muted()),
                );
            }

            ui.add_space(8.0);
            truth_badge(ui, TruthLevel::Sampled);
        });

    ui.add_space(12.0);

    egui::Frame::new()
        .fill(theme::panel())
        .stroke(Stroke::new(1.0, theme::border()))
        .corner_radius(6.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new("SOCKET OWNERSHIP // next layer")
                    .monospace()
                    .strong()
                    .color(theme::white()),
            );

            for line in [
                "TCP ESTABLISHED   → live aggregate count now",
                "TCP LISTEN        → future owning PID",
                "UDP               → future owning PID",
                "process traffic   → future socket/process attribution",
            ] {
                ui.label(
                    egui::RichText::new(line)
                        .size(11.0)
                        .monospace()
                        .color(theme::muted()),
                );
            }

            if descend {
                ui.add_space(6.0);

                ui.label(
                    egui::RichText::new(
                        "DESCEND target: netlink → sockets → process → packet/event tracing",
                    )
                    .monospace()
                    .size(11.0)
                    .color(theme::blue()),
                );
            }
        });
}

pub fn logs_view(ui: &mut egui::Ui, snapshot: &SystemSnapshot) {
    heading(
        ui,
        "EVENT VEIN // LOGS",
        "normalized Observatory events with explicit telemetry provenance",
    );

    for (index, event) in snapshot.events.iter().enumerate() {
        egui::Frame::new()
            .fill(if index % 2 == 0 {
                theme::panel()
            } else {
                theme::panel_alt()
            })
            .stroke(Stroke::new(1.0, theme::border()))
            .corner_radius(4.0)
            .inner_margin(9.0)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new(format!("-{:04.1}s", event.age_seconds))
                            .size(10.5)
                            .monospace()
                            .color(theme::muted()),
                    );

                    truth_badge(ui, event.truth);

                    ui.label(
                        egui::RichText::new(&event.message)
                            .monospace()
                            .color(theme::text()),
                    );
                });
            });

        ui.add_space(4.0);
    }

    ui.add_space(10.0);

    ui.label(
        egui::RichText::new(
            "Future sources: scheduler migrations, page faults, storage bursts, network events, NVML, perf, eBPF, instruction samples.",
        )
        .size(11.0)
        .color(theme::muted()),
    );
}

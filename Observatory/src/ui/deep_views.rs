use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};

use crate::{
    analysis::AnalysisSnapshot,
    fleet::FleetState,
    model::{SystemSnapshot, TruthLevel, format_bytes},
    theme,
};

use wyn_protocol::process_memory::{
    ProcessMemoryMap, ProcessMemoryPermissions, ProcessMemoryRegion, ProcessMemoryRegionKind,
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

fn memory_kind_label(kind: ProcessMemoryRegionKind) -> &'static str {
    match kind {
        ProcessMemoryRegionKind::Heap => "HEAP",

        ProcessMemoryRegionKind::Anonymous => "ANONYMOUS",

        ProcessMemoryRegionKind::SharedLibrary => "SHARED LIB",

        ProcessMemoryRegionKind::MappedFile => "MAPPED FILE",

        ProcessMemoryRegionKind::Stack => "STACK",

        ProcessMemoryRegionKind::ExecutableImage => "EXECUTABLE",

        ProcessMemoryRegionKind::Special => "SPECIAL",

        ProcessMemoryRegionKind::Other => "OTHER",
    }
}

fn memory_kind_color(kind: ProcessMemoryRegionKind) -> Color32 {
    match kind {
        ProcessMemoryRegionKind::Heap => theme::pink(),

        ProcessMemoryRegionKind::Anonymous => theme::violet(),

        ProcessMemoryRegionKind::SharedLibrary => theme::blue(),

        ProcessMemoryRegionKind::MappedFile => theme::green(),

        ProcessMemoryRegionKind::Stack => theme::gold(),

        ProcessMemoryRegionKind::ExecutableImage => theme::white(),

        ProcessMemoryRegionKind::Special => theme::muted(),

        ProcessMemoryRegionKind::Other => theme::muted(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MemoryRegionFilter {
    #[default]
    All,
    Heap,
    Anonymous,
    SharedLibrary,
    MappedFile,
    Stack,
    Executable,
    Special,
    Other,
    WritableExecutable,
}

impl MemoryRegionFilter {
    pub const ALL: [Self; 10] = [
        Self::All,
        Self::Heap,
        Self::Anonymous,
        Self::SharedLibrary,
        Self::MappedFile,
        Self::Stack,
        Self::Executable,
        Self::Special,
        Self::Other,
        Self::WritableExecutable,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::All => "ALL",
            Self::Heap => "HEAP",
            Self::Anonymous => "ANONYMOUS",
            Self::SharedLibrary => "SHARED LIB",
            Self::MappedFile => "MAPPED FILE",
            Self::Stack => "STACK",
            Self::Executable => "EXECUTABLE",
            Self::Special => "SPECIAL",
            Self::Other => "OTHER",
            Self::WritableExecutable => "W+X ONLY",
        }
    }

    fn matches(self, region: &ProcessMemoryRegion) -> bool {
        match self {
            Self::All => true,

            Self::Heap => {
                matches!(region.kind, ProcessMemoryRegionKind::Heap)
            }

            Self::Anonymous => {
                matches!(region.kind, ProcessMemoryRegionKind::Anonymous)
            }

            Self::SharedLibrary => {
                matches!(region.kind, ProcessMemoryRegionKind::SharedLibrary)
            }

            Self::MappedFile => {
                matches!(region.kind, ProcessMemoryRegionKind::MappedFile)
            }

            Self::Stack => {
                matches!(region.kind, ProcessMemoryRegionKind::Stack)
            }

            Self::Executable => {
                matches!(region.kind, ProcessMemoryRegionKind::ExecutableImage)
            }

            Self::Special => {
                matches!(region.kind, ProcessMemoryRegionKind::Special)
            }

            Self::Other => {
                matches!(region.kind, ProcessMemoryRegionKind::Other)
            }

            Self::WritableExecutable => {
                region.permissions.writable && region.permissions.executable
            }
        }
    }
}

fn region_matches_search(region: &ProcessMemoryRegion, query: &str) -> bool {
    let query = query.trim();

    if query.is_empty() {
        return true;
    }

    let query = query.to_ascii_lowercase();

    let backing = region
        .pathname
        .as_deref()
        .unwrap_or("(anonymous)")
        .to_ascii_lowercase();

    let kind = memory_kind_label(region.kind).to_ascii_lowercase();

    let permissions = memory_permissions(&region.permissions).to_ascii_lowercase();

    let start_hex = format!("{:x}", region.start_address);

    let end_hex = format!("{:x}", region.end_address);

    let inode = region.inode.to_string();

    backing.contains(&query)
        || kind.contains(&query)
        || permissions.contains(&query)
        || start_hex.contains(&query)
        || end_hex.contains(&query)
        || inode.contains(&query)
}

fn region_inspector(ui: &mut egui::Ui, region: &ProcessMemoryRegion) {
    let wx = region.permissions.writable && region.permissions.executable;

    let accent = if wx {
        theme::gold()
    } else {
        memory_kind_color(region.kind)
    };

    ui.add_space(14.0);

    egui::Frame::new()
        .fill(theme::panel())
        .stroke(Stroke::new(
            if wx { 2.0 } else { 1.0 },
            if wx { theme::gold() } else { theme::border() },
        ))
        .corner_radius(6.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("REGION INSPECTOR")
                        .monospace()
                        .size(13.0)
                        .strong()
                        .color(theme::white()),
                );

                ui.label(
                    egui::RichText::new(memory_kind_label(region.kind))
                        .monospace()
                        .strong()
                        .color(accent),
                );
            });

            if wx {
                ui.label(
                    egui::RichText::new("▲ WRITABLE + EXECUTABLE REGION")
                        .monospace()
                        .strong()
                        .color(theme::gold()),
                );
            }

            ui.add_space(8.0);

            egui::Grid::new("memory_region_inspector")
                .num_columns(2)
                .spacing([24.0, 5.0])
                .show(ui, |ui| {
                    for (label, value) in [
                        ("START", format!("0x{:016X}", region.start_address)),
                        ("END", format!("0x{:016X}", region.end_address)),
                        ("SIZE", format_bytes(region.size_bytes)),
                        ("PERMISSIONS", memory_permissions(&region.permissions)),
                        ("OFFSET", format!("0x{:X}", region.offset)),
                        ("INODE", region.inode.to_string()),
                        (
                            "BACKING",
                            region
                                .pathname
                                .clone()
                                .unwrap_or_else(|| "(anonymous)".to_string()),
                        ),
                    ] {
                        ui.label(egui::RichText::new(label).monospace().color(theme::muted()));

                        ui.label(egui::RichText::new(value).monospace().color(theme::white()));

                        ui.end_row();
                    }
                });

            ui.add_space(8.0);

            ui.horizontal_wrapped(|ui| {
                for (label, enabled) in [
                    ("READ", region.permissions.readable),
                    ("WRITE", region.permissions.writable),
                    ("EXECUTE", region.permissions.executable),
                    ("PRIVATE", region.permissions.private),
                    ("SHARED", region.permissions.shared),
                ] {
                    ui.label(
                        egui::RichText::new(format!("{} {label}", if enabled { "✓" } else { "×" }))
                            .monospace()
                            .color(if enabled {
                                theme::green()
                            } else {
                                theme::muted()
                            }),
                    );
                }
            });
        });
}

fn memory_permissions(permissions: &ProcessMemoryPermissions) -> String {
    let read = if permissions.readable { 'r' } else { '-' };

    let write = if permissions.writable { 'w' } else { '-' };

    let execute = if permissions.executable { 'x' } else { '-' };

    let mapping = if permissions.private {
        'p'
    } else if permissions.shared {
        's'
    } else {
        '-'
    };

    format!("{read}{write}{execute}{mapping}")
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

pub fn memory_map_view(
    ui: &mut egui::Ui,
    machine_name: Option<&str>,
    process_name: Option<&str>,
    pid: Option<u32>,
    map: Option<&ProcessMemoryMap>,
    loading: bool,
    error: Option<&str>,
    page: &mut usize,
    search: &mut String,
    filter: &mut MemoryRegionFilter,
    selected_region: &mut Option<usize>,
) -> bool {
    let mut refresh_requested = false;

    page_heading(
        ui,
        "MEMORY CARTOGRAPHY // PROCESS ADDRESS SPACE",
        "live Agent inspection // /proc/<pid>/maps // virtual regions, permissions and backing objects",
    );

    let (Some(machine_name), Some(process_name), Some(pid)) = (machine_name, process_name, pid)
    else {
        egui::Frame::new()
            .fill(theme::panel())
            .stroke(Stroke::new(1.0, theme::border()))
            .corner_radius(6.0)
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("NO PROCESS SELECTED")
                        .monospace()
                        .strong()
                        .color(theme::muted()),
                );

                ui.label("Open PROCESSES and click MAP beside a process.");
            });

        return false;
    };

    /*
     * Target header.
     */
    egui::Frame::new()
        .fill(theme::panel())
        .stroke(Stroke::new(1.0, theme::border()))
        .corner_radius(6.0)
        .inner_margin(10.0)
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new(process_name)
                        .monospace()
                        .size(15.0)
                        .strong()
                        .color(theme::white()),
                );

                ui.label(
                    egui::RichText::new(format!("PID {pid}"))
                        .monospace()
                        .color(theme::pink()),
                );

                ui.label(
                    egui::RichText::new(format!("// {machine_name}"))
                        .monospace()
                        .color(theme::blue()),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("↻ REFRESH MAP").clicked() {
                        refresh_requested = true;
                    }
                });
            });
        });

    ui.add_space(10.0);

    if loading {
        ui.horizontal(|ui| {
            ui.spinner();

            ui.label(
                egui::RichText::new("Requesting live virtual memory map from Agent...")
                    .monospace()
                    .color(theme::blue()),
            );
        });

        return false;
    }

    if let Some(error) = error {
        egui::Frame::new()
            .fill(theme::panel())
            .stroke(Stroke::new(1.0, theme::gold()))
            .corner_radius(6.0)
            .inner_margin(10.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("INSPECTION FAILED")
                        .monospace()
                        .strong()
                        .color(theme::gold()),
                );

                ui.label(egui::RichText::new(error).monospace().color(theme::muted()));
            });

        return false;
    }

    let Some(map) = map else {
        ui.label(
            egui::RichText::new("Waiting for memory-map data.")
                .monospace()
                .color(theme::muted()),
        );

        return false;
    };

    /*
     * Summary.
     */
    ui.horizontal_wrapped(|ui| {
        for (label, value, color) in [
            (
                "VIRTUAL",
                format_bytes(map.total_virtual_bytes),
                theme::blue(),
            ),
            ("REGIONS", map.region_count.to_string(), theme::pink()),
            (
                "W+X",
                map.writable_executable_regions.to_string(),
                if map.writable_executable_regions > 0 {
                    theme::gold()
                } else {
                    theme::green()
                },
            ),
        ] {
            egui::Frame::new()
                .fill(theme::panel())
                .stroke(Stroke::new(1.0, theme::border()))
                .corner_radius(5.0)
                .inner_margin(9.0)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(label)
                            .monospace()
                            .size(9.5)
                            .color(theme::muted()),
                    );

                    ui.label(egui::RichText::new(value).monospace().strong().color(color));
                });
        }
    });

    if map.writable_executable_regions > 0 {
        ui.label(
            egui::RichText::new(format!(
                "▲ {} writable + executable mapping(s) detected",
                map.writable_executable_regions
            ))
            .monospace()
            .strong()
            .color(theme::gold()),
        );
    }

    ui.add_space(14.0);

    /*
     * THE OVERVIEW GRAPH.
     *
     * This is the live replacement for
     * the mock bars you wanted to keep.
     */
    egui::Frame::new()
        .fill(theme::panel())
        .stroke(Stroke::new(1.0, theme::border()))
        .corner_radius(6.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new("ADDRESS SPACE // COMPOSITION")
                    .monospace()
                    .strong()
                    .color(theme::white()),
            );

            ui.label(
                egui::RichText::new("proportion of total mapped virtual address space")
                    .size(10.5)
                    .color(theme::muted()),
            );

            ui.add_space(8.0);

            let total = map.total_virtual_bytes.max(1);

            let overview = [
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

            let known_bytes = overview
                .iter()
                .fold(0_u64, |total, (_, bytes, _)| total.saturating_add(*bytes));

            for (label, bytes, color) in overview {
                let fraction = bytes as f32 / total as f32;

                let percent = fraction * 100.0;

                bar(
                    ui,
                    fraction,
                    &format!("{label:<18} {} // {:>5.1}%", format_bytes(bytes), percent,),
                    color,
                );

                ui.add_space(4.0);
            }

            let other = total.saturating_sub(known_bytes);

            if other > 0 {
                let fraction = other as f32 / total as f32;

                bar(
                    ui,
                    fraction,
                    &format!(
                        "other              {} // {:>5.1}%",
                        format_bytes(other),
                        fraction * 100.0,
                    ),
                    theme::muted(),
                );
            }
        });

    ui.add_space(14.0);

    /*
     * Detailed region explorer.
     */
    ui.label(
        egui::RichText::new("REGION EXPLORER")
            .monospace()
            .size(13.0)
            .strong()
            .color(theme::white()),
    );

    ui.label(
        egui::RichText::new(format!("{} mappings captured", map.regions.len()))
            .monospace()
            .size(10.0)
            .color(theme::muted()),
    );

    ui.add_space(6.0);

    ui.horizontal_wrapped(|ui| {
        ui.label(
            egui::RichText::new("SEARCH")
                .monospace()
                .strong()
                .color(theme::muted()),
        );

        let search_changed = ui
            .add_sized(
                [360.0, 28.0],
                egui::TextEdit::singleline(search)
                    .hint_text("path, type, perms, address, inode..."),
            )
            .changed();

        let old_filter = *filter;

        egui::ComboBox::from_id_salt("memory_region_filter")
            .selected_text(filter.label())
            .show_ui(ui, |ui| {
                for candidate in MemoryRegionFilter::ALL {
                    ui.selectable_value(filter, candidate, candidate.label());
                }
            });

        let filter_changed = *filter != old_filter;

        if ui.button("CLEAR").clicked() {
            search.clear();
            *filter = MemoryRegionFilter::All;
            *page = 0;
            *selected_region = None;
        }

        if search_changed || filter_changed {
            *page = 0;
            *selected_region = None;
        }
    });

    let filtered_indices = map
        .regions
        .iter()
        .enumerate()
        .filter(|(_, region)| filter.matches(region) && region_matches_search(region, search))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();

    const REGIONS_PER_PAGE: usize = 100;

    let region_count = filtered_indices.len();

    let page_count = if region_count == 0 {
        1
    } else {
        (region_count + REGIONS_PER_PAGE - 1) / REGIONS_PER_PAGE
    };

    *page = (*page).min(page_count - 1);

    /*
     * Pagination controls live OUTSIDE the Grid.
     */
    ui.horizontal(|ui| {
        if ui
            .add_enabled(*page > 0, egui::Button::new("⏮ FIRST"))
            .clicked()
        {
            *page = 0;
        }

        if ui
            .add_enabled(*page > 0, egui::Button::new("◀ PREVIOUS"))
            .clicked()
        {
            *page -= 1;
        }

        ui.label(
            egui::RichText::new(format!("PAGE {} / {}", *page + 1, page_count,))
                .monospace()
                .strong()
                .color(theme::white()),
        );

        if ui
            .add_enabled(*page + 1 < page_count, egui::Button::new("NEXT ▶"))
            .clicked()
        {
            *page += 1;
        }

        if ui
            .add_enabled(*page + 1 < page_count, egui::Button::new("LAST ⏭"))
            .clicked()
        {
            *page = page_count - 1;
        }
    });

    /*
     * Recalculate these AFTER the buttons.
     *
     * That means clicking NEXT immediately renders
     * the new page instead of waiting one frame.
     */
    let start = *page * REGIONS_PER_PAGE;

    let end = (start + REGIONS_PER_PAGE).min(region_count);

    ui.label(
        egui::RichText::new(if region_count == 0 {
            "0 mappings".to_string()
        } else {
            format!("showing {}–{} of {} mappings", start + 1, end, region_count,)
        })
        .monospace()
        .size(10.0)
        .color(theme::muted()),
    );

    ui.add_space(6.0);

    /*
     * NOW enter the Grid.
     *
     * Only table cells and end_row() belong in here.
     */
    egui::ScrollArea::horizontal()
        .id_salt("process_memory_map_regions")
        .auto_shrink([false, true])
        .show(ui, |ui| {
            egui::Grid::new("process_memory_map_grid")
                .striped(true)
                .min_col_width(105.0)
                .show(ui, |ui| {
                    for header in [
                        "START", "END", "SIZE", "PERMS", "TYPE", "OFFSET", "INODE", "BACKING",
                        "INSPECT",
                    ] {
                        ui.label(
                            egui::RichText::new(header)
                                .monospace()
                                .strong()
                                .color(theme::muted()),
                        );
                    }

                    ui.end_row();

                    for &region_index in &filtered_indices[start..end] {
                        let region = &map.regions[region_index];
                        let wx = region.permissions.writable && region.permissions.executable;

                        let accent = if wx {
                            theme::gold()
                        } else {
                            memory_kind_color(region.kind)
                        };

                        ui.label(
                            egui::RichText::new(format!("0x{:016X}", region.start_address))
                                .monospace()
                                .color(theme::white()),
                        );

                        ui.label(
                            egui::RichText::new(format!("0x{:016X}", region.end_address))
                                .monospace()
                                .color(theme::white()),
                        );

                        ui.label(egui::RichText::new(format_bytes(region.size_bytes)).monospace());

                        ui.label(
                            egui::RichText::new(memory_permissions(&region.permissions))
                                .monospace()
                                .strong()
                                .color(if wx { theme::gold() } else { theme::blue() }),
                        );

                        ui.label(
                            egui::RichText::new(memory_kind_label(region.kind))
                                .monospace()
                                .color(accent),
                        );

                        ui.label(
                            egui::RichText::new(format!("0x{:X}", region.offset))
                                .monospace()
                                .color(theme::muted()),
                        );

                        ui.label(
                            egui::RichText::new(region.inode.to_string())
                                .monospace()
                                .color(theme::muted()),
                        );

                        let backing = region.pathname.as_deref().unwrap_or("(anonymous)");

                        ui.label(egui::RichText::new(backing).monospace().color(accent))
                            .on_hover_text(backing);

                        let selected = *selected_region == Some(region_index);

                        if ui
                            .selectable_label(selected, if selected { "◆ OPEN" } else { "◇ VIEW" })
                            .clicked()
                        {
                            *selected_region = Some(region_index);
                        }

                        ui.end_row();
                    }
                });
        });

    if let Some(index) = *selected_region {
        if let Some(region) = map.regions.get(index) {
            region_inspector(ui, region);
        }
    }
    refresh_requested
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

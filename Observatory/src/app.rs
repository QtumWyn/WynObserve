use std::time::{Duration, Instant};

use eframe::egui;

use crate::{
    config::{self, ThemePreset, UiPreferences},
    fleet::FleetState,
    hub::HubFleetClient,
    live::LiveTelemetry,
    model::{
        ComponentId, InstructionArchitecture, InstructionSample, ProcessSnapshot, SystemSnapshot,
        TelemetrySource, TruthLevel,
    },
    theme, ui,
    updater::{UpdateState, Updater},
};
use wyn_protocol::{ObservatoryResponseKind, process_memory::ProcessMemoryMap};

const INSTRUCTION_VEIN_INTERVAL: Duration = Duration::from_millis(250);

const INSTRUCTION_VEIN_BUFFER_SIZE: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Overview,
    Server,
    Fleet,

    Cpu,
    Memory,
    Gpu,
    Processes,
    Network,

    Npu,
    NpuMemory,
    NpuRuntime,

    Replay,
    Causality,
    Syscalls,
    Flamegraph,
    MemoryMap,
    Scheduler,
    Cache,
    Interrupts,
    PacketFlow,

    Fabric,
    Numa,
    StorageIo,
    PowerThermal,

    Locks,
    Containers,
    Database,

    Traces,
    Compare,
    Diff,
    CrossMachine,

    Anomalies,
    Incidents,
    SourceSilicon,
    Firmware,
    Security,

    Binary,
    Autopsy,
    Kernel,
    Logs,
}

impl View {
    pub const CORE: [Self; 3] = [Self::Overview, Self::Server, Self::Fleet];

    pub const LIVE: [Self; 5] = [
        Self::Cpu,
        Self::Memory,
        Self::Gpu,
        Self::Processes,
        Self::Network,
    ];

    pub const NPU: [Self; 3] = [Self::Npu, Self::NpuMemory, Self::NpuRuntime];

    pub const DEEP: [Self; 9] = [
        Self::Replay,
        Self::Causality,
        Self::Syscalls,
        Self::Flamegraph,
        Self::MemoryMap,
        Self::Scheduler,
        Self::Cache,
        Self::Interrupts,
        Self::PacketFlow,
    ];

    pub const HARDWARE: [Self; 4] = [
        Self::Fabric,
        Self::Numa,
        Self::StorageIo,
        Self::PowerThermal,
    ];

    pub const SERVER_STACK: [Self; 3] = [Self::Locks, Self::Containers, Self::Database];

    pub const DISTRIBUTED: [Self; 4] =
        [Self::Traces, Self::Compare, Self::Diff, Self::CrossMachine];

    pub const INTELLIGENCE: [Self; 5] = [
        Self::Anomalies,
        Self::Incidents,
        Self::SourceSilicon,
        Self::Firmware,
        Self::Security,
    ];

    pub const FORENSICS: [Self; 4] = [Self::Binary, Self::Autopsy, Self::Kernel, Self::Logs];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Overview => "THE MACHINE",
            Self::Server => "THE SERVER",
            Self::Fleet => "FLEET",

            Self::Cpu => "CPU",
            Self::Memory => "MEMORY",
            Self::Gpu => "GPU",
            Self::Processes => "PROCESSES",
            Self::Network => "NETWORK",

            Self::Npu => "NPU OVERVIEW",
            Self::NpuMemory => "NPU MEMORY",
            Self::NpuRuntime => "NPU RUNTIME",

            Self::Replay => "TIME MACHINE",
            Self::Causality => "CAUSALITY",
            Self::Syscalls => "SYSCALLS",
            Self::Flamegraph => "FLAMEGRAPH",
            Self::MemoryMap => "MEMORY MAP",
            Self::Scheduler => "SCHEDULER",
            Self::Cache => "CACHE / PMU",
            Self::Interrupts => "IRQ / SOFTIRQ",
            Self::PacketFlow => "PACKET FLOW",

            Self::Fabric => "PCIe FABRIC",
            Self::Numa => "NUMA",
            Self::StorageIo => "STORAGE I/O",
            Self::PowerThermal => "POWER / THERMAL",

            Self::Locks => "LOCKS",
            Self::Containers => "CGROUPS",
            Self::Database => "DATABASE",

            Self::Traces => "TRACES",
            Self::Compare => "COMPARE",
            Self::Diff => "SNAPSHOT DIFF",
            Self::CrossMachine => "CROSS-MACHINE",

            Self::Anomalies => "ANOMALIES",
            Self::Incidents => "INCIDENTS",
            Self::SourceSilicon => "SOURCE → SILICON",
            Self::Firmware => "FIRMWARE",
            Self::Security => "SECURITY",

            Self::Binary => "BINARY",
            Self::Autopsy => "AUTOPSY",
            Self::Kernel => "KERNEL",
            Self::Logs => "LOGS",
        }
    }

    pub const fn requires_npu(self) -> bool {
        matches!(self, Self::Npu | Self::NpuMemory | Self::NpuRuntime)
    }
}

#[derive(Debug, Clone)]
struct ProcessMemoryTarget {
    machine_id: String,
    machine_name: String,

    pid: u32,
    process_name: String,

    started_at_unix_ms: Option<u64>,
}

pub struct ObservatoryApp {
    source: Box<dyn TelemetrySource>,
    hub: HubFleetClient,

    /// Real local snapshot produced by the currently-connected local collector.
    local_snapshot: SystemSnapshot,

    /// Snapshot for the Fleet-selected target. Every normal Observatory tab
    /// reads this value.
    snapshot: SystemSnapshot,

    fleet: FleetState,
    selected_machine_id: String,

    memory_map_target: Option<ProcessMemoryTarget>,

    memory_map_request_id: Option<u64>,

    memory_map_result: Option<ProcessMemoryMap>,

    memory_map_error: Option<String>,

    memory_map_page: usize,
    memory_map_search: String,

    memory_map_filter: ui::MemoryRegionFilter,

    memory_map_selected_region: Option<usize>,

    instruction_vein_target: Option<ProcessMemoryTarget>,

    instruction_vein_request_id: Option<u64>,

    instruction_vein_request_target: Option<(String, u32)>,

    instruction_vein_samples: Vec<InstructionSample>,

    instruction_vein_last_request: Option<Instant>,

    instruction_vein_next_sequence: u64,

    started: Instant,
    paused_at: Option<f64>,
    paused: bool,
    playback_speed: f32,
    view: View,
    process_tab: ui::ProcessTab,
    selected: ComponentId,
    selected_instruction: Option<u64>,
    descend: bool,

    comparison_machine_id: String,
    captured_incidents: u64,

    preferences: UiPreferences,
    settings_open: bool,
    settings_dirty: bool,
    settings_status: Option<String>,
    updater: Updater,
}

impl ObservatoryApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        runtime_config: crate::runtime_config::ObservatoryConfig,
    ) -> Self {
        let preferences = UiPreferences::load();

        theme::install(&cc.egui_ctx, &preferences);
        cc.egui_ctx
            .send_viewport_cmd(egui::ViewportCommand::Title(preferences.title.clone()));

        let mut source: Box<dyn TelemetrySource> = Box::new(LiveTelemetry::connect(
            runtime_config.local.endpoint.clone(),
        ));

        let local_snapshot = source.poll(0.0);
        let fleet = FleetState::default();

        let selected_machine_id = String::new();

        let snapshot = fleet
            .machine(&selected_machine_id)
            .map(|machine| machine.system.clone())
            .unwrap_or_else(|| local_snapshot.clone());

        let hub = HubFleetClient::connect(runtime_config.hub.endpoint.clone());
        let updater = Updater::default();

        updater.check();

        Self {
            source,
            hub,
            local_snapshot,
            snapshot,
            fleet,
            selected_machine_id,
            memory_map_target: None,
            memory_map_request_id: None,
            memory_map_result: None,
            memory_map_error: None,
            memory_map_page: 0,
            memory_map_search: String::new(),

            memory_map_filter: ui::MemoryRegionFilter::All,

            memory_map_selected_region: None,

            instruction_vein_target: None,

            instruction_vein_request_id: None,

            instruction_vein_request_target: None,

            instruction_vein_samples: Vec::new(),

            instruction_vein_last_request: None,

            instruction_vein_next_sequence: 1,
            started: Instant::now(),
            paused_at: None,
            paused: false,
            playback_speed: 1.0,
            view: View::Overview,
            process_tab: ui::ProcessTab::Running,
            selected: ComponentId::Cpu,
            selected_instruction: None,
            descend: false,

            comparison_machine_id: String::new(),
            captured_incidents: 0,

            preferences,
            settings_open: false,
            settings_dirty: false,
            settings_status: None,

            updater,
        }
    }

    fn sync_selected_snapshot(&mut self) {
        if let Some(machine) = self.fleet.machine(&self.selected_machine_id) {
            if machine.online {
                self.snapshot = machine.system.clone();
                self.enforce_view_capabilities();
                return;
            }
        }

        if let Some(machine) = self.fleet.first_online() {
            self.selected_machine_id = machine.id.clone();
            self.snapshot = machine.system.clone();
        } else {
            self.snapshot = self.local_snapshot.clone();
        }

        self.enforce_view_capabilities();
    }

    fn enforce_view_capabilities(&mut self) {
        if self.view.requires_npu() && !self.snapshot.npu.available {
            self.view = View::Overview;
        }

        if self.selected == ComponentId::Npu && !self.snapshot.npu.available {
            self.selected = ComponentId::Cpu;
            self.selected_instruction = None;
        }
    }

    fn selected_machine_name(&self) -> &str {
        self.fleet
            .machine(&self.selected_machine_id)
            .map(|machine| machine.name.as_str())
            .unwrap_or("unknown")
    }

    fn elapsed(&self) -> f64 {
        if let Some(value) = self.paused_at {
            value
        } else {
            self.started.elapsed().as_secs_f64() * self.playback_speed as f64
        }
    }

    fn toggle_pause(&mut self) {
        if self.paused {
            let paused_value = self.paused_at.unwrap_or(0.0);

            self.started = Instant::now()
                - std::time::Duration::from_secs_f64(paused_value / self.playback_speed as f64);

            self.paused_at = None;
            self.paused = false;
        } else {
            self.paused_at = Some(self.elapsed());
            self.paused = true;
        }
    }

    fn set_speed(&mut self, speed: f32) {
        let current = self.elapsed();
        self.playback_speed = speed;

        if self.paused {
            self.paused_at = Some(current);
        } else {
            self.started =
                Instant::now() - std::time::Duration::from_secs_f64(current / speed as f64);
        }
    }

    fn top_bar(&mut self, root: &mut egui::Ui) {
        egui::Panel::top("observatory_top")
            .exact_size(66.0)
            .frame(egui::Frame::new().fill(theme::panel()).inner_margin(10.0))
            .show(root, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(&self.preferences.title)
                                .size(20.0)
                                .strong()
                                .color(theme::white()),
                        );

                        let familiar = self.preferences.familiar_signature();

                        let subtitle = if familiar.is_empty() {
                            "systems visualization laboratory".to_string()
                        } else {
                            format!("systems visualization laboratory  •  {familiar}")
                        };

                        ui.label(
                            egui::RichText::new(subtitle)
                                .size(11.0)
                                .color(theme::muted()),
                        );
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "TARGET // {}",
                                self.selected_machine_name().to_uppercase()
                            ))
                            .monospace()
                            .size(10.5)
                            .strong()
                            .color(theme::blue()),
                        );

                        if ui.button("⚙ SETTINGS").clicked() {
                            self.settings_open = true;
                        }

                        ui.label(
                            egui::RichText::new(" HYBRID LIVE ")
                                .monospace()
                                .size(11.0)
                                .color(theme::pink()),
                        );

                        let live_label = if self.paused {
                            "● PAUSED"
                        } else {
                            "● LIVE"
                        };

                        let color = if self.paused {
                            theme::gold()
                        } else {
                            theme::green()
                        };

                        ui.label(
                            egui::RichText::new(live_label)
                                .monospace()
                                .strong()
                                .color(color),
                        );
                    });
                });
            });
    }

    fn nav(&mut self, root: &mut egui::Ui) {
        egui::Panel::left("observatory_nav")
            .exact_size(156.0)
            .frame(egui::Frame::new().fill(theme::panel()).inner_margin(8.0))
            .show(root, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("observatory_nav_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_space(12.0);

                        nav_group(ui, &mut self.view, "CORE", &View::CORE);
                        nav_group(ui, &mut self.view, "LIVE", &View::LIVE);

                        if self.snapshot.npu.available {
                            nav_group(ui, &mut self.view, "NPU", &View::NPU);
                        }

                        nav_group(ui, &mut self.view, "DEEP ANALYSIS", &View::DEEP);
                        nav_group(ui, &mut self.view, "HARDWARE", &View::HARDWARE);
                        nav_group(ui, &mut self.view, "SERVER STACK", &View::SERVER_STACK);
                        nav_group(ui, &mut self.view, "DISTRIBUTED", &View::DISTRIBUTED);
                        nav_group(ui, &mut self.view, "INTELLIGENCE", &View::INTELLIGENCE);
                        nav_group(ui, &mut self.view, "FORENSICS", &View::FORENSICS);

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(10.0);

                        // Keep backend/UI metadata in the same scrollable
                        // sidebar so it can never be clipped by window height.
                        egui::Frame::new()
                            .fill(theme::deep_bg())
                            .stroke(egui::Stroke::new(1.0, theme::border()))
                            .corner_radius(6.0)
                            .inner_margin(8.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new("●").size(9.5).color(theme::green()),
                                    );

                                    ui.label(
                                        egui::RichText::new("M6 BACKEND")
                                            .size(9.5)
                                            .monospace()
                                            .strong()
                                            .color(theme::white()),
                                    );
                                });

                                ui.label(
                                    egui::RichText::new("127.0.0.1:4767")
                                        .size(9.0)
                                        .monospace()
                                        .color(theme::blue()),
                                );

                                ui.add_space(5.0);
                                ui.separator();
                                ui.add_space(5.0);

                                ui.label(
                                    egui::RichText::new("UI PROTOTYPE 0.5")
                                        .size(9.5)
                                        .monospace()
                                        .strong()
                                        .color(theme::pink()),
                                );

                                ui.label(
                                    egui::RichText::new("basement fully excavated")
                                        .size(8.5)
                                        .monospace()
                                        .color(theme::muted()),
                                );
                            });

                        ui.add_space(12.0);
                    });
            });
    }

    fn bottom_bar(&mut self, root: &mut egui::Ui) {
        egui::Panel::bottom("observatory_timeline")
            .exact_size(58.0)
            .frame(egui::Frame::new().fill(theme::panel()).inner_margin(10.0))
            .show(root, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .button(if self.paused {
                            "▶ RESUME"
                        } else {
                            "Ⅱ PAUSE"
                        })
                        .clicked()
                    {
                        self.toggle_pause();
                    }

                    ui.separator();

                    for speed in [0.25_f32, 1.0, 4.0] {
                        let selected = (self.playback_speed - speed).abs() < f32::EPSILON;

                        if ui
                            .add(egui::Button::new(format!("{speed}×")).selected(selected))
                            .clicked()
                        {
                            self.set_speed(speed);
                        }
                    }

                    ui.separator();

                    let descend_text = if self.descend {
                        "◆ ASCEND"
                    } else {
                        "◇ DESCEND"
                    };

                    if ui
                        .add(egui::Button::new(
                            egui::RichText::new(descend_text)
                                .monospace()
                                .strong()
                                .color(if self.descend {
                                    theme::blue()
                                } else {
                                    theme::violet()
                                }),
                        ))
                        .clicked()
                    {
                        self.descend = !self.descend;
                    }

                    ui.separator();

                    let time = self.elapsed();

                    ui.label(
                        egui::RichText::new(format!("T+{:07.2}s", time))
                            .monospace()
                            .color(theme::blue()),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new("◇ topology  •  ≈ flow  •  ◉ instruction drip")
                                .size(10.0)
                                .monospace()
                                .color(theme::muted()),
                        );
                    });
                });
            });
    }

    fn save_preferences(&mut self) {
        match self.preferences.save() {
            Ok(path) => {
                self.settings_dirty = false;
                self.settings_status = Some(format!("Saved to {}", path.display()));
            }
            Err(error) => {
                self.settings_status = Some(format!("Could not save settings: {error}"));
            }
        }
    }

    fn updater_section(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.separator();
        ui.add_space(10.0);

        ui.label(
            egui::RichText::new("UPDATES")
                .monospace()
                .strong()
                .color(theme::pink()),
        );

        ui.label(format!("Installed version: {}", env!("CARGO_PKG_VERSION")));

        ui.add_space(6.0);

        match self.updater.state() {
            UpdateState::Idle => {
                if ui.button("CHECK FOR UPDATES").clicked() {
                    self.updater.check();
                }
            }

            UpdateState::Checking => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Checking for updates...");
                });
            }

            UpdateState::Current => {
                ui.label(egui::RichText::new("✓ WynObserve is up to date").color(theme::green()));

                if ui.button("CHECK AGAIN").clicked() {
                    self.updater.check();
                }
            }

            UpdateState::Available(update) => {
                ui.label(
                    egui::RichText::new(format!("Update available: {}", update.version))
                        .strong()
                        .color(theme::blue()),
                );

                ui.label(
                    egui::RichText::new(format!("Release: {}", update.tag))
                        .monospace()
                        .size(10.0)
                        .color(theme::muted()),
                );

                if ui.button("UPDATE NOW").clicked() {
                    self.updater.install(update);
                }
            }

            UpdateState::Downloading => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Downloading update...");
                });
            }

            UpdateState::Installing => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Installing update...");
                });

                ui.label(
                    egui::RichText::new("KDE may ask for your administrator password.")
                        .size(10.0)
                        .color(theme::muted()),
                );
            }

            UpdateState::Installed(version) => {
                ui.label(
                    egui::RichText::new(format!("✓ WynObserve {} installed", version))
                        .strong()
                        .color(theme::green()),
                );

                ui.label("Restart WynObserve to use the new version.");

                if ui.button("RESTART NOW").clicked() {
                    match crate::updater::restart_installed() {
                        Ok(()) => {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }

                        Err(error) => {
                            eprintln!("WynObserve // restart failed: {error}");
                        }
                    }
                }
            }

            UpdateState::Error(error) => {
                ui.label(
                    egui::RichText::new(format!("Update failed: {error}")).color(theme::pink()),
                );

                if ui.button("TRY AGAIN").clicked() {
                    self.updater.check();
                }
            }
        }
    }

    fn settings_window(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }

        let was_open = self.settings_open;
        let mut open = self.settings_open;
        let mut changed = false;
        let mut save_clicked = false;

        egui::Window::new("WYNOBSERVE // SETTINGS")
            .open(&mut open)
            .default_width(560.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new("IDENTITY")
                        .monospace()
                        .strong()
                        .color(theme::pink()),
                );

                ui.label(
                    egui::RichText::new(
                        "Give this copy of Observatory its own name and familiar energy.",
                    )
                        .color(theme::muted()),
                );

                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    ui.label("Display name");

                    changed |= ui
                        .text_edit_singleline(
                            &mut self.preferences.title,
                        )
                        .changed();
                });

                ui.add_space(8.0);

                ui.label(
                    egui::RichText::new("FAMILIARS")
                        .monospace()
                        .strong()
                        .color(theme::violet()),
                );

                ui.horizontal_wrapped(|ui| {
                    changed |= ui
                        .checkbox(
                            &mut self.preferences.catgirl,
                            "Catgirl",
                        )
                        .changed();

                    changed |= ui
                        .checkbox(
                            &mut self.preferences.batgirl,
                            "Batgirl",
                        )
                        .changed();

                    changed |= ui
                        .checkbox(
                            &mut self.preferences.bunnygirl,
                            "Bunnygirl",
                        )
                        .changed();

                    changed |= ui
                        .checkbox(
                            &mut self.preferences.puppygirl,
                            "Puppygirl",
                        )
                        .changed();
                });

                let signature = self.preferences.familiar_signature();

                if !signature.is_empty() {
                    ui.label(
                        egui::RichText::new(format!(
                            "Selected: {signature}"
                        ))
                            .monospace()
                            .color(theme::blue()),
                    );
                }

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);

                ui.label(
                    egui::RichText::new("THEME")
                        .monospace()
                        .strong()
                        .color(theme::pink()),
                );

                ui.horizontal_wrapped(|ui| {
                    for preset in ThemePreset::ALL {
                        let selected =
                            self.preferences.theme == preset;

                        if ui
                            .add(
                                egui::Button::new(preset.label())
                                    .selected(selected),
                            )
                            .clicked()
                            && !selected
                        {
                            self.preferences.theme = preset;
                            changed = true;
                        }
                    }
                });

                if self.preferences.theme != ThemePreset::Custom {
                    if ui.button("CUSTOMIZE THIS PRESET").clicked() {
                        self.preferences.custom_theme =
                            theme::palette_for_preset(
                                self.preferences.theme,
                            );

                        self.preferences.theme = ThemePreset::Custom;
                        changed = true;
                    }
                }

                if self.preferences.theme == ThemePreset::Custom {
                    ui.add_space(10.0);

                    ui.label(
                        egui::RichText::new("CUSTOM THEME BUILDER")
                            .monospace()
                            .strong()
                            .color(theme::blue()),
                    );

                    ui.label(
                        egui::RichText::new(
                            "A small builder for the colors that matter most. Changes preview live.",
                        )
                            .color(theme::muted()),
                    );

                    ui.add_space(6.0);

                    egui::Grid::new("custom_theme_builder")
                        .num_columns(2)
                        .spacing([16.0, 7.0])
                        .show(ui, |ui| {
                            changed |= color_row(
                                ui,
                                "Background",
                                &mut self.preferences.custom_theme.bg,
                            );
                            changed |= color_row(
                                ui,
                                "Panel",
                                &mut self.preferences.custom_theme.panel,
                            );
                            changed |= color_row(
                                ui,
                                "Panel Alt",
                                &mut self.preferences.custom_theme.panel_alt,
                            );
                            changed |= color_row(
                                ui,
                                "Border",
                                &mut self.preferences.custom_theme.border,
                            );
                            changed |= color_row(
                                ui,
                                "Primary",
                                &mut self.preferences.custom_theme.primary,
                            );
                            changed |= color_row(
                                ui,
                                "Secondary",
                                &mut self.preferences.custom_theme.secondary,
                            );
                            changed |= color_row(
                                ui,
                                "Info / Data",
                                &mut self.preferences.custom_theme.info,
                            );
                            changed |= color_row(
                                ui,
                                "Success / Observed",
                                &mut self.preferences.custom_theme.success,
                            );
                            changed |= color_row(
                                ui,
                                "Warning / Thermal",
                                &mut self.preferences.custom_theme.warning,
                            );
                            changed |= color_row(
                                ui,
                                "Selection",
                                &mut self.preferences.custom_theme.selection,
                            );
                            changed |= color_row(
                                ui,
                                "Text",
                                &mut self.preferences.custom_theme.text,
                            );
                            changed |= color_row(
                                ui,
                                "Muted Text",
                                &mut self.preferences.custom_theme.muted,
                            );
                            changed |= color_row(
                                ui,
                                "Bright Text",
                                &mut self.preferences.custom_theme.bright,
                            );
                        });

                    ui.add_space(6.0);

                    if ui.button("RESET CUSTOM → GOTHIC").clicked() {
                        self.preferences.custom_theme =
                            theme::gothic_palette();
                        changed = true;
                    }
                }

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            self.settings_dirty || changed,
                            egui::Button::new("SAVE SETTINGS"),
                        )
                        .clicked()
                    {
                        save_clicked = true;
                    }

                    if ui.button("RESET ALL").clicked() {
                        self.preferences = UiPreferences::default();
                        changed = true;
                    }
                });

                ui.add_space(5.0);

                ui.add(
                    egui::Label::new(
                        egui::RichText::new(format!(
                            "config: {}",
                            config::config_path().display()
                        ))
                            .monospace()
                            .size(9.5)
                            .color(theme::muted()),
                    )
                        .wrap(),
                );

                if let Some(status) = &self.settings_status {
                    ui.label(
                        egui::RichText::new(status)
                            .size(10.0)
                            .color(theme::muted()),
                    );
                }

                ui.add_space(12.0);

                self.updater_section(
                    ui,
                    ctx,
                );
            });

        if changed {
            self.settings_dirty = true;
            theme::apply(ctx, &self.preferences);

            ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.preferences.title.clone()));
        }

        let closed = was_open && !open;
        self.settings_open = open;

        if save_clicked || (closed && self.settings_dirty) {
            self.save_preferences();
        }
    }

    fn render_current_view(
        &mut self,
        ui: &mut egui::Ui,
        selected_machine: &Option<crate::fleet::FleetMachine>,
    ) {
        match self.view {
            View::Overview => {
                if let Some(machine) = selected_machine {
                    let mut live_machine = machine.clone();

                    let target_matches = self
                        .instruction_vein_target
                        .as_ref()
                        .is_some_and(|target| target.machine_id == machine.id);

                    if target_matches {
                        live_machine.system.instruction_samples =
                            self.instruction_vein_samples.clone();
                    } else {
                        live_machine.system.instruction_samples.clear();
                    }

                    ui::overview(
                        ui,
                        &live_machine,
                        self.elapsed() as f32,
                        &mut self.selected,
                        &mut self.selected_instruction,
                        self.descend,
                    );
                } else {
                    ui.label("No Fleet machine is currently available.");
                }
            }
            View::Server => ui::server_view(ui, &self.snapshot, self.elapsed() as f32),
            View::Fleet => ui::fleet_view(ui, &self.fleet, &mut self.selected_machine_id),
            View::Cpu => ui::cpu_view(ui, &self.snapshot, self.descend),
            View::Memory => {
                let target = self.memory_map_target.as_ref();

                ui::memory_view(
                    ui,
                    &self.snapshot,
                    self.descend,
                    target.map(|target| target.process_name.as_str()),
                    target.map(|target| target.pid),
                    self.memory_map_result.as_ref(),
                    self.memory_map_request_id.is_some(),
                    self.memory_map_error.as_deref(),
                )
            }
            View::Gpu => ui::gpu_view(ui, &self.snapshot, self.descend),
            View::Processes => {
                if let Some(process) =
                    ui::processes_view(ui, &self.snapshot, self.descend, &mut self.process_tab)
                {
                    self.open_process_memory_map(process);
                }
            }
            View::Network => ui::network_view(ui, &self.snapshot, self.descend),

            View::Npu => ui::npu_view(ui, &self.snapshot, self.descend),
            View::NpuMemory => ui::npu_memory_view(ui, &self.snapshot, self.descend),
            View::NpuRuntime => ui::npu_runtime_view(ui, &self.snapshot, self.descend),

            View::Replay => ui::replay_view(ui, &self.snapshot, self.elapsed() as f32),
            View::Causality => ui::causality_view(ui, &self.snapshot, self.elapsed() as f32),
            View::Syscalls => ui::syscalls_view(ui, &self.snapshot, self.elapsed() as f32),
            View::Flamegraph => ui::flamegraph_view(ui, &self.snapshot, self.elapsed() as f32),
            View::MemoryMap => {
                let refresh_requested = {
                    let target = self.memory_map_target.as_ref();

                    ui::memory_map_view(
                        ui,
                        target.map(|target| target.machine_name.as_str()),
                        target.map(|target| target.process_name.as_str()),
                        target.map(|target| target.pid),
                        self.memory_map_result.as_ref(),
                        self.memory_map_request_id.is_some(),
                        self.memory_map_error.as_deref(),
                        &mut self.memory_map_page,
                        &mut self.memory_map_search,
                        &mut self.memory_map_filter,
                        &mut self.memory_map_selected_region,
                    )
                };

                if refresh_requested {
                    self.refresh_process_memory_map();
                }
            }
            View::Scheduler => ui::scheduler_view(ui, &self.snapshot, self.elapsed() as f32),
            View::Cache => ui::cache_view(ui, &self.snapshot, self.elapsed() as f32),
            View::Interrupts => ui::irq_view(ui, &self.snapshot, self.elapsed() as f32),
            View::PacketFlow => ui::packet_flow_view(ui, &self.snapshot, self.elapsed() as f32),

            View::Fabric => {
                if let Some(machine) = selected_machine {
                    ui::fabric_view(ui, machine, &self.fleet, self.elapsed() as f32);
                }
            }
            View::Numa => {
                if let Some(machine) = selected_machine {
                    ui::numa_view(ui, machine, &self.fleet, self.elapsed() as f32);
                }
            }
            View::StorageIo => {
                if let Some(machine) = selected_machine {
                    ui::storage_io_view(ui, machine, &self.fleet, self.elapsed() as f32);
                }
            }
            View::PowerThermal => {
                if let Some(machine) = selected_machine {
                    ui::power_view(ui, machine, &self.fleet, self.elapsed() as f32);
                }
            }

            View::Locks => {
                if let Some(machine) = selected_machine {
                    ui::locks_view(ui, machine, &self.fleet, self.elapsed() as f32);
                }
            }
            View::Containers => {
                if let Some(machine) = selected_machine {
                    ui::containers_view(ui, machine, &self.fleet, self.elapsed() as f32);
                }
            }
            View::Database => {
                if let Some(machine) = selected_machine {
                    ui::database_view(ui, machine, &self.fleet, self.elapsed() as f32);
                }
            }

            View::Traces => {
                if let Some(machine) = selected_machine {
                    ui::traces_view(ui, machine, &self.fleet, self.elapsed() as f32);
                }
            }
            View::Compare => ui::compare_view(
                ui,
                &self.fleet,
                &self.selected_machine_id,
                &mut self.comparison_machine_id,
            ),
            View::Diff => {
                if let Some(machine) = selected_machine {
                    ui::diff_view(ui, machine, &self.fleet, self.elapsed() as f32);
                }
            }
            View::CrossMachine => {
                if let Some(machine) = selected_machine {
                    ui::cross_machine_view(ui, machine, &self.fleet, self.elapsed() as f32);
                }
            }

            View::Anomalies => {
                if let Some(machine) = selected_machine {
                    ui::anomalies_view(ui, machine, &self.fleet, self.elapsed() as f32);
                }
            }
            View::Incidents => {
                if let Some(machine) = selected_machine {
                    ui::incidents_view(
                        ui,
                        machine,
                        &self.fleet,
                        self.elapsed() as f32,
                        &mut self.captured_incidents,
                    );
                }
            }
            View::SourceSilicon => {
                if let Some(machine) = selected_machine {
                    ui::source_silicon_view(ui, machine, &self.fleet, self.elapsed() as f32);
                }
            }
            View::Firmware => {
                if let Some(machine) = selected_machine {
                    ui::firmware_view(ui, machine, &self.fleet, self.elapsed() as f32);
                }
            }
            View::Security => {
                if let Some(machine) = selected_machine {
                    ui::security_view(ui, machine, &self.fleet, self.elapsed() as f32);
                }
            }

            View::Binary => ui::binary_view(ui, &self.snapshot, self.elapsed() as f32),
            View::Autopsy => ui::autopsy_view(ui, &self.snapshot, self.elapsed() as f32),
            View::Kernel => ui::kernel_view(ui, &self.snapshot, self.descend),
            View::Logs => ui::logs_view(ui, &self.snapshot),
        }
    }

    fn open_process_memory_map(&mut self, process: ProcessSnapshot) {
        let machine_id = self.selected_machine_id.clone();

        let machine_name = self.selected_machine_name().to_string();

        let target = ProcessMemoryTarget {
            machine_id: machine_id.clone(),

            machine_name,

            pid: process.pid,

            process_name: process.name.clone(),

            started_at_unix_ms: process.started_at_unix_ms,
        };

        self.memory_map_target = Some(target.clone());

        self.instruction_vein_target = Some(target);

        self.instruction_vein_samples.clear();

        self.instruction_vein_last_request = None;

        self.selected_instruction = None;

        self.memory_map_result = None;
        self.memory_map_error = None;

        self.memory_map_page = 0;

        self.memory_map_search.clear();

        self.memory_map_filter = ui::MemoryRegionFilter::All;

        self.memory_map_selected_region = None;

        /*
         * Move to the page immediately.
         *
         * The user sees the loading state
         * while the request travels through:
         *
         * Observatory -> Hub -> Agent.
         */
        self.view = View::MemoryMap;

        if machine_id.is_empty() {
            self.memory_map_request_id = None;

            self.memory_map_error = Some("No Fleet machine is selected.".to_string());

            return;
        }

        match self.hub.request_process_memory_map(
            &machine_id,
            process.pid,
            process.started_at_unix_ms,
        ) {
            Ok(request_id) => {
                self.memory_map_request_id = Some(request_id);
            }

            Err(error) => {
                self.memory_map_request_id = None;

                self.memory_map_error = Some(error);
            }
        }
    }

    fn refresh_process_memory_map(&mut self) {
        let Some(target) = self.memory_map_target.clone() else {
            return;
        };

        self.memory_map_result = None;

        self.memory_map_error = None;

        self.memory_map_page = 0;

        self.memory_map_selected_region = None;

        match self.hub.request_process_memory_map(
            &target.machine_id,
            target.pid,
            target.started_at_unix_ms,
        ) {
            Ok(request_id) => {
                self.memory_map_request_id = Some(request_id);
            }

            Err(error) => {
                self.memory_map_request_id = None;

                self.memory_map_error = Some(error);
            }
        }
    }

    fn poll_process_memory_map_response(&mut self) {
        let Some(request_id) = self.memory_map_request_id else {
            return;
        };

        let Some(response) = self.hub.take_response(request_id) else {
            return;
        };

        self.memory_map_request_id = None;

        match response {
            ObservatoryResponseKind::ProcessMemoryMap { machine_id, map } => {
                if let Some(target) = &self.memory_map_target {
                    if target.machine_id != machine_id {
                        self.memory_map_error = Some(format!(
                            "Memory map returned for machine `{machine_id}` instead of `{}`",
                            target.machine_id
                        ));

                        return;
                    }

                    if target.pid != map.pid {
                        self.memory_map_error = Some(format!(
                            "Memory map returned PID {} instead of PID {}",
                            map.pid, target.pid,
                        ));

                        return;
                    }
                }

                self.memory_map_error = None;
                self.memory_map_result = Some(map);
            }

            ObservatoryResponseKind::InstructionVein { .. } => {
                self.memory_map_result = None;

                self.memory_map_error =
                    Some("Received Instruction Vein data for a memory-map request".to_string());
            }

            ObservatoryResponseKind::Error { code, message } => {
                self.memory_map_result = None;

                self.memory_map_error = Some(format!("{code} // {message}"));
            }
        }
    }

    fn maybe_request_instruction_vein(&mut self) {
        /*
         * Only sample while the Vein is actually
         * visible.
         */
        if self.view != View::Overview || self.paused || self.instruction_vein_request_id.is_some()
        {
            return;
        }

        let Some(target) = self.instruction_vein_target.clone() else {
            return;
        };

        /*
         * Never inspect a process belonging to a
         * different Fleet machine than the one
         * currently selected.
         */
        if target.machine_id != self.selected_machine_id {
            return;
        }

        if self
            .instruction_vein_last_request
            .is_some_and(|last| last.elapsed() < INSTRUCTION_VEIN_INTERVAL)
        {
            return;
        }

        match self.hub.request_instruction_vein_sample(
            &target.machine_id,
            target.pid,
            target.started_at_unix_ms,
        ) {
            Ok(request_id) => {
                self.instruction_vein_request_id = Some(request_id);

                self.instruction_vein_request_target = Some((target.machine_id, target.pid));

                self.instruction_vein_last_request = Some(Instant::now());
            }

            Err(error) => {
                eprintln!("Observatory // Instruction Vein request failed // {error}");

                self.instruction_vein_last_request = Some(Instant::now());
            }
        }
    }

    fn poll_instruction_vein_response(&mut self) {
        let Some(request_id) = self.instruction_vein_request_id else {
            return;
        };

        let Some(response) = self.hub.take_response(request_id) else {
            return;
        };

        self.instruction_vein_request_id = None;

        let requested_target = self.instruction_vein_request_target.take();

        match response {
            ObservatoryResponseKind::InstructionVein { machine_id, batch } => {
                /*
                 * First validate the response against
                 * the request that produced it.
                 */
                if requested_target.as_ref() != Some(&(machine_id.clone(), batch.pid)) {
                    eprintln!("Observatory // Instruction Vein routing mismatch");

                    return;
                }

                let Some(target) = self.instruction_vein_target.clone() else {
                    return;
                };

                /*
                 * The user might have selected another
                 * process while this request was flying
                 * across the network.
                 *
                 * A stale response is harmless. Drop it.
                 */
                if target.machine_id != machine_id || target.pid != batch.pid {
                    return;
                }

                let newest_perf_time = batch
                    .samples
                    .iter()
                    .map(|sample| sample.perf_time)
                    .max()
                    .unwrap_or(0);

                let mut converted = Vec::with_capacity(batch.samples.len());

                for sample in batch.samples {
                    let mut assembly = sample.instruction.splitn(2, char::is_whitespace);

                    let mnemonic = assembly.next().unwrap_or_default().to_string();

                    let operands = assembly.next().unwrap_or_default().trim().to_string();

                    let sequence = self.instruction_vein_next_sequence;

                    self.instruction_vein_next_sequence =
                        self.instruction_vein_next_sequence.wrapping_add(1).max(1);

                    let age_seconds =
                        newest_perf_time.saturating_sub(sample.perf_time) as f64 / 1_000_000_000.0;

                    converted.push(InstructionSample {
                        sequence,

                        age_seconds: age_seconds as f32,

                        /*
                         * The current Vein samples
                         * host CPU execution.
                         */
                        component: ComponentId::Cpu,

                        truth: TruthLevel::Sampled,

                        pid: sample.pid,

                        tid: sample.tid,

                        process_name: target.process_name.clone(),

                        cpu_id: Some(sample.cpu as usize),

                        architecture: InstructionArchitecture::X86_64,

                        address: sample.ip,

                        bytes: sample.bytes,

                        mnemonic,

                        operands,

                        note: Some(
                            "live Linux perf sample // process_vm_readv // iced-x86".to_string(),
                        ),
                    });
                }

                /*
                 * ProcessInstructionSampler returns
                 * chronological order.
                 *
                 * The UI expects newest first.
                 */
                converted.reverse();

                /*
                 * New batch first, older retained
                 * samples afterward.
                 */
                converted.append(&mut self.instruction_vein_samples);

                converted.truncate(INSTRUCTION_VEIN_BUFFER_SIZE);

                self.instruction_vein_samples = converted;
            }

            ObservatoryResponseKind::Error { code, message } => {
                eprintln!("Observatory // Instruction Vein // {code} // {message}");
            }

            ObservatoryResponseKind::ProcessMemoryMap { .. } => {
                eprintln!(
                    "Observatory // received memory-map response for Instruction Vein request"
                );
            }
        }
    }
}

fn color_row(ui: &mut egui::Ui, label: &str, color: &mut egui::Color32) -> bool {
    ui.label(label);

    let changed = ui.color_edit_button_srgba(color).changed();

    ui.end_row();

    changed
}

fn nav_group(ui: &mut egui::Ui, current: &mut View, label: &str, views: &[View]) {
    ui.label(
        egui::RichText::new(label)
            .size(9.5)
            .monospace()
            .strong()
            .color(theme::muted()),
    );

    for &view in views {
        let selected = *current == view;

        let text = if selected {
            egui::RichText::new(format!("◆ {}", view.label()))
                .strong()
                .color(theme::pink())
        } else {
            egui::RichText::new(format!("  {}", view.label())).color(theme::text())
        };

        if ui
            .add_sized([138.0, 31.0], egui::Button::new(text).selected(selected))
            .clicked()
        {
            *current = view;
        }
    }

    ui.add_space(7.0);
}

impl eframe::App for ObservatoryApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_process_memory_map_response();

        self.poll_instruction_vein_response();

        if !self.paused {
            self.local_snapshot = self.source.poll(self.elapsed());

            self.fleet = self.hub.poll();

            self.sync_selected_snapshot();

            self.maybe_request_instruction_vein();

            ctx.request_repaint_after(Duration::from_millis(16));
        }
    }

    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Panel order matters in egui.
        //
        // Lay out the fixed top and bottom bars first so they reserve their
        // vertical space. The sidebar is created afterward and therefore only
        // receives the height between those two bars instead of extending
        // underneath the timeline panel.
        self.top_bar(root);
        self.bottom_bar(root);
        self.nav(root);

        let selected_machine = self
            .fleet
            .machine(&self.selected_machine_id)
            .cloned()
            .or_else(|| self.fleet.first_online().cloned());

        let central = egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::bg()).inner_margin(18.0))
            .show(root, |ui| {
                let view = self.view;

                // THE MACHINE already owns its own responsive scrolling.
                // Every other page shares one vertical page scroller so
                // content remains reachable when the viewport is short.
                if view == View::Overview {
                    self.render_current_view(ui, &selected_machine);
                } else {
                    egui::ScrollArea::vertical()
                        .id_salt(("observatory_page_scroll", view.label()))
                        .auto_shrink([false, false])
                        .animated(true)
                        .show(ui, |ui| {
                            self.render_current_view(ui, &selected_machine);

                            ui.add_space(14.0);
                        });
                }
            });

        self.sync_selected_snapshot();

        if !self.settings_open {
            ui::draw_familiars(
                root.ctx(),
                central.response.rect,
                &self.preferences,
                self.elapsed() as f32,
                &self.snapshot,
                self.selected,
                self.view == View::Autopsy || self.view == View::Incidents,
                self.fleet.machines.iter().any(|machine| !machine.online),
            );
        }

        self.settings_window(root.ctx());
    }
}

mod critters;
mod deep_views;
mod extreme_views;
mod machine_fleet;
mod panels;
mod server;

pub use critters::draw_familiars;
pub use deep_views::{
    autopsy_view, binary_view, cache_view, causality_view, flamegraph_view, fleet_view, irq_view,
    memory_map_view, packet_flow_view, replay_view, scheduler_view, syscalls_view,
};
pub use extreme_views::{
    anomalies_view, compare_view, containers_view, cross_machine_view, database_view, diff_view,
    fabric_view, firmware_view, incidents_view, locks_view, numa_view, power_view, security_view,
    source_silicon_view, storage_io_view, traces_view,
};
pub use panels::{
    cpu_view, gpu_view, kernel_view, logs_view, memory_view, network_view, npu_memory_view,
    npu_runtime_view, npu_view, processes_view,
};
pub use server::server_view;

use eframe::egui;

use crate::{fleet::FleetMachine, model::ComponentId};

pub fn overview(
    ui: &mut egui::Ui,
    machine: &FleetMachine,
    elapsed: f32,
    selected: &mut ComponentId,
    selected_instruction: &mut Option<u64>,
    descend: bool,
) {
    let snapshot = &machine.system;

    let available_width = ui.available_width();
    let available_height = ui.available_height().max(420.0);

    if available_width > 1220.0 {
        let machine_width = available_width * 0.54;
        let inspector_width = available_width * 0.17;
        let instruction_width =
            (available_width - machine_width - inspector_width - 24.0).max(285.0);

        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(machine_width, available_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("machine_column_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            machine_fleet::machine_view(ui, machine, elapsed, selected, descend);
                        });
                },
            );

            ui.separator();

            ui.allocate_ui_with_layout(
                egui::vec2(inspector_width, available_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("component_inspector_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            panels::inspector(ui, snapshot, *selected, descend);
                        });
                },
            );

            ui.separator();

            ui.allocate_ui_with_layout(
                egui::vec2(instruction_width, available_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    panels::instruction_feed(
                        ui,
                        snapshot,
                        *selected,
                        selected_instruction,
                        descend,
                        available_height,
                    );
                },
            );
        });
    } else {
        egui::ScrollArea::vertical()
            .id_salt("overview_narrow_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                machine_fleet::machine_view(ui, machine, elapsed, selected, descend);

                ui.add_space(24.0);
                ui.separator();
                ui.add_space(18.0);

                if ui.available_width() > 760.0 {
                    ui.columns(2, |columns| {
                        panels::inspector(&mut columns[0], snapshot, *selected, descend);

                        panels::instruction_feed(
                            &mut columns[1],
                            snapshot,
                            *selected,
                            selected_instruction,
                            descend,
                            available_height,
                        );
                    });
                } else {
                    panels::inspector(ui, snapshot, *selected, descend);

                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(12.0);

                    panels::instruction_feed(
                        ui,
                        snapshot,
                        *selected,
                        selected_instruction,
                        descend,
                        available_height,
                    );
                }
            });
    }
}

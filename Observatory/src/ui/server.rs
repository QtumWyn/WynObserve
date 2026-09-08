use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};

use crate::{
    analysis::AnalysisSnapshot,
    model::{SystemSnapshot, format_bytes},
    theme,
};

fn particle(painter: &egui::Painter, a: Pos2, b: Pos2, t: f32, offset: f32, color: Color32) {
    let p = Pos2::new(
        a.x + (b.x - a.x) * ((t + offset).fract()),
        a.y + (b.y - a.y) * ((t + offset).fract()),
    );

    painter.circle_filled(p, 2.4, color);
}

pub fn server_view(ui: &mut egui::Ui, system: &SystemSnapshot, elapsed: f32) {
    let deep = AnalysisSnapshot::mock(system, elapsed);
    let server = &deep.server;

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("THE SERVER")
                .size(19.0)
                .strong()
                .color(theme::white()),
        );

        ui.label(
            egui::RichText::new("◇ HOST / SERVICE TOPOLOGY")
                .monospace()
                .size(10.5)
                .color(theme::muted()),
        );
    });

    ui.label(
        egui::RichText::new(format!("{} // {}", server.hostname, server.role))
            .color(theme::muted()),
    );

    ui.add_space(8.0);

    let width = ui.available_width();
    let height = ui.available_height().max(620.0);

    let (response, painter) = ui.allocate_painter(Vec2::new(width, height), Sense::hover());

    let area = response.rect.shrink(5.0);

    painter.rect_filled(area, 10.0, theme::deep_bg());
    painter.rect_stroke(
        area,
        10.0,
        Stroke::new(1.0, theme::border()),
        StrokeKind::Inside,
    );

    let chassis = Rect::from_min_size(
        Pos2::new(area.left() + 28.0, area.top() + 28.0),
        Vec2::new(area.width() * 0.55, area.height() - 56.0),
    );

    painter.rect_filled(chassis, 7.0, theme::panel());
    painter.rect_stroke(
        chassis,
        7.0,
        Stroke::new(1.5, theme::pink()),
        StrokeKind::Inside,
    );

    painter.text(
        chassis.left_top() + Vec2::new(14.0, 14.0),
        Align2::LEFT_TOP,
        "PHYSICAL SERVER",
        FontId::monospace(15.0),
        theme::white(),
    );

    let cpu0 = Rect::from_min_size(
        chassis.left_top() + Vec2::new(30.0, 68.0),
        Vec2::new(chassis.width() * 0.34, 100.0),
    );

    let cpu1 = Rect::from_min_size(Pos2::new(cpu0.right() + 28.0, cpu0.top()), cpu0.size());

    for (index, rect) in [cpu0, cpu1].iter().enumerate() {
        painter.rect_filled(*rect, 5.0, theme::cell_bg());
        painter.rect_stroke(
            *rect,
            5.0,
            Stroke::new(1.0, theme::pink()),
            StrokeKind::Inside,
        );

        painter.text(
            rect.left_top() + Vec2::new(10.0, 9.0),
            Align2::LEFT_TOP,
            format!("CPU SOCKET {index}"),
            FontId::monospace(11.0),
            theme::white(),
        );

        let usage = if index == 0 {
            system.cpu.package_usage
        } else {
            (system.cpu.package_usage * 0.87 + 7.0).min(100.0)
        };

        let fill = Rect::from_min_size(
            Pos2::new(rect.left() + 10.0, rect.bottom() - 22.0),
            Vec2::new((rect.width() - 20.0) * usage / 100.0, 10.0),
        );

        painter.rect_filled(
            Rect::from_min_size(
                Pos2::new(rect.left() + 10.0, rect.bottom() - 22.0),
                Vec2::new(rect.width() - 20.0, 10.0),
            ),
            2.0,
            theme::track_bg(),
        );

        painter.rect_filled(fill, 2.0, theme::pink());
    }

    let ram = Rect::from_min_size(
        Pos2::new(chassis.left() + 30.0, cpu0.bottom() + 34.0),
        Vec2::new(chassis.width() - 60.0, 96.0),
    );

    painter.rect_filled(ram, 5.0, theme::cell_bg());
    painter.rect_stroke(
        ram,
        5.0,
        Stroke::new(1.0, theme::blue()),
        StrokeKind::Inside,
    );

    painter.text(
        ram.left_top() + Vec2::new(10.0, 9.0),
        Align2::LEFT_TOP,
        "ECC RAM BANKS",
        FontId::monospace(11.0),
        theme::white(),
    );

    let ram_pct = system.memory.used_bytes as f32 / system.memory.total_bytes.max(1) as f32;

    for bank in 0..12 {
        let bank_rect = Rect::from_min_size(
            Pos2::new(
                ram.left() + 12.0 + bank as f32 * ((ram.width() - 24.0) / 12.0),
                ram.top() + 38.0,
            ),
            Vec2::new((ram.width() - 34.0) / 12.0, 38.0),
        );

        let active = bank as f32 / 12.0 <= ram_pct;

        painter.rect_filled(
            bank_rect,
            2.0,
            if active {
                theme::blue()
            } else {
                theme::grid_inactive()
            },
        );
    }

    let storage = Rect::from_min_size(
        Pos2::new(chassis.left() + 30.0, ram.bottom() + 30.0),
        Vec2::new(chassis.width() * 0.44, 112.0),
    );

    let nic = Rect::from_min_size(
        Pos2::new(storage.right() + 24.0, storage.top()),
        Vec2::new(chassis.right() - storage.right() - 54.0, 112.0),
    );

    for (rect, label, accent) in [
        (storage, "STORAGE ARRAY", theme::white()),
        (nic, "NETWORK / NIC", theme::green()),
    ] {
        painter.rect_filled(rect, 5.0, theme::cell_bg());
        painter.rect_stroke(rect, 5.0, Stroke::new(1.0, accent), StrokeKind::Inside);
        painter.text(
            rect.left_top() + Vec2::new(10.0, 9.0),
            Align2::LEFT_TOP,
            label,
            FontId::monospace(11.0),
            theme::white(),
        );
    }

    painter.text(
        storage.left_top() + Vec2::new(10.0, 38.0),
        Align2::LEFT_TOP,
        format!(
            "R {:>7.1} MiB/s\nW {:>7.1} MiB/s",
            system.storage.read_mib_s, system.storage.write_mib_s,
        ),
        FontId::monospace(10.5),
        theme::muted(),
    );

    painter.text(
        nic.left_top() + Vec2::new(10.0, 38.0),
        Align2::LEFT_TOP,
        format!(
            "RX {:>6.1} MiB/s\nTX {:>6.1} MiB/s",
            system.network.rx_mib_s, system.network.tx_mib_s,
        ),
        FontId::monospace(10.5),
        theme::muted(),
    );

    let ingress = Pos2::new(area.right() - 20.0, area.top() + 90.0);
    let ingress_box = Rect::from_center_size(
        Pos2::new(area.right() - 170.0, area.top() + 92.0),
        Vec2::new(250.0, 88.0),
    );

    painter.rect_filled(ingress_box, 5.0, theme::panel());
    painter.rect_stroke(
        ingress_box,
        5.0,
        Stroke::new(1.0, theme::green()),
        StrokeKind::Inside,
    );

    painter.text(
        ingress_box.left_top() + Vec2::new(10.0, 10.0),
        Align2::LEFT_TOP,
        "REQUEST INGRESS",
        FontId::monospace(11.0),
        theme::white(),
    );

    painter.text(
        ingress_box.left_top() + Vec2::new(10.0, 35.0),
        Align2::LEFT_TOP,
        format!(
            "{:.0} req/s\n{} active connections",
            server.requests_per_second, server.active_connections
        ),
        FontId::monospace(10.5),
        theme::muted(),
    );

    let service_top = ingress_box.bottom() + 24.0;
    let service_width = 250.0;

    for (index, service) in server.services.iter().enumerate() {
        let rect = Rect::from_min_size(
            Pos2::new(
                area.right() - service_width - 45.0,
                service_top + index as f32 * 86.0,
            ),
            Vec2::new(service_width, 68.0),
        );

        painter.rect_filled(rect, 5.0, theme::panel());
        painter.rect_stroke(
            rect,
            5.0,
            Stroke::new(
                1.0,
                if service.error_percent > 1.0 {
                    theme::gold()
                } else {
                    theme::green()
                },
            ),
            StrokeKind::Inside,
        );

        painter.text(
            rect.left_top() + Vec2::new(10.0, 8.0),
            Align2::LEFT_TOP,
            format!("{} // {}", service.name.to_uppercase(), service.status),
            FontId::monospace(10.5),
            theme::white(),
        );

        painter.text(
            rect.left_top() + Vec2::new(10.0, 31.0),
            Align2::LEFT_TOP,
            format!(
                "{:>6.0} req/s   {:>5.1} ms   {:>4.2}% err",
                service.requests_per_second, service.latency_ms, service.error_percent,
            ),
            FontId::monospace(9.5),
            theme::muted(),
        );

        let start = if index == 0 {
            ingress_box.center_bottom()
        } else {
            Pos2::new(
                area.right() - service_width - 45.0 + service_width * 0.5,
                service_top + (index - 1) as f32 * 86.0 + 68.0,
            )
        };

        let end = rect.center_top();

        painter.line_segment([start, end], Stroke::new(1.0, theme::border()));

        particle(
            &painter,
            start,
            end,
            elapsed * 0.20,
            index as f32 * 0.19,
            theme::green(),
        );
    }

    let nic_center = nic.center();
    let web_center = Pos2::new(
        area.right() - service_width * 0.5 - 45.0,
        service_top + 34.0,
    );

    painter.line_segment([nic_center, web_center], Stroke::new(1.0, theme::border()));

    for offset in [0.0, 0.33, 0.66] {
        particle(
            &painter,
            web_center,
            nic_center,
            elapsed * 0.12,
            offset,
            theme::green(),
        );
    }

    // Bottom server analytics band.
    let band = Rect::from_min_size(
        Pos2::new(area.left() + 28.0, area.bottom() - 92.0),
        Vec2::new(area.width() - 56.0, 64.0),
    );

    painter.rect_filled(band, 5.0, theme::panel_alt());

    let metrics = [
        format!("P95 {:.1} ms", server.p95_latency_ms),
        format!("ERR {:.2}%", server.error_rate_percent),
        format!("RAM {}", format_bytes(system.memory.used_bytes)),
        format!("PROC {}", system.processes.len()),
        format!("CONTAINERS {}", server.containers.len()),
    ];

    let step = band.width() / metrics.len() as f32;

    for (index, metric) in metrics.iter().enumerate() {
        painter.text(
            Pos2::new(band.left() + index as f32 * step + 10.0, band.center().y),
            Align2::LEFT_CENTER,
            metric,
            FontId::monospace(10.5),
            if index == 1 {
                theme::gold()
            } else {
                theme::white()
            },
        );
    }

    let _ = ingress;
}

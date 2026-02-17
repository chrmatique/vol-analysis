use eframe::egui;

use crate::app::AppState;

pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Cross-Sector Correlation Matrix");
    ui.add_space(8.0);

    let corr = match &state.analysis.correlation {
        Some(c) if !c.symbols.is_empty() => c,
        _ => {
            ui.label("No correlation data available. Load market data first.");
            return;
        }
    };

    ui.label(format!(
        "Average cross-sector correlation: {:.3}",
        state.analysis.avg_cross_correlation
    ));
    ui.add_space(8.0);

    // Render the correlation matrix as a colored grid
    let n = corr.symbols.len();
    let cell_size = 48.0;

    egui::ScrollArea::both().show(ui, |ui| {
        egui::Grid::new("corr_matrix")
            .min_col_width(cell_size)
            .max_col_width(cell_size)
            .spacing(egui::vec2(2.0, 2.0))
            .show(ui, |ui| {
                // Header row
                ui.label(""); // empty corner cell
                for sym in &corr.symbols {
                    ui.vertical_centered(|ui| {
                        ui.small(sym);
                    });
                }
                ui.end_row();

                // Data rows
                for i in 0..n {
                    ui.small(&corr.symbols[i]);
                    for j in 0..n {
                        let val = corr.matrix[i][j];
                        let color = correlation_color(val);
                        let text_color = if val.abs() > 0.5 {
                            egui::Color32::WHITE
                        } else {
                            egui::Color32::BLACK
                        };

                        let (rect, _resp) = ui.allocate_exact_size(
                            egui::vec2(cell_size, 24.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().rect_filled(rect, 2.0, color);
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            format!("{:.2}", val),
                            egui::FontId::proportional(11.0),
                            text_color,
                        );
                    }
                    ui.end_row();
                }
            });
    });

    ui.add_space(16.0);
    ui.separator();
    ui.add_space(8.0);

    // Color legend
    ui.horizontal(|ui| {
        ui.label("Legend: ");
        color_swatch(ui, egui::Color32::from_rgb(220, 50, 50), "-1.0");
        color_swatch(ui, egui::Color32::from_rgb(240, 240, 240), " 0.0");
        color_swatch(ui, egui::Color32::from_rgb(50, 50, 220), "+1.0");
    });
}

fn correlation_color(val: f64) -> egui::Color32 {
    let clamped = val.clamp(-1.0, 1.0);
    if clamped >= 0.0 {
        // White to blue
        let t = clamped as f32;
        egui::Color32::from_rgb(
            (240.0 * (1.0 - t)) as u8,
            (240.0 * (1.0 - t)) as u8,
            (240.0 * (1.0 - t) + 220.0 * t) as u8,
        )
    } else {
        // White to red
        let t = (-clamped) as f32;
        egui::Color32::from_rgb(
            (240.0 * (1.0 - t) + 220.0 * t) as u8,
            (240.0 * (1.0 - t)) as u8,
            (240.0 * (1.0 - t)) as u8,
        )
    }
}

fn color_swatch(ui: &mut egui::Ui, color: egui::Color32, label: &str) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(20.0, 16.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 2.0, color);
    ui.label(label);
}

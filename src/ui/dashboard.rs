use eframe::egui;

use crate::app::AppState;
use crate::config;

pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Market Structure Dashboard");
    ui.add_space(8.0);

    if state.market_data.sectors.is_empty() {
        ui.label("No data loaded. Click 'Refresh Data' to fetch market data.");
        return;
    }

    // Key metrics row
    ui.horizontal(|ui| {
        let n_sectors = state.market_data.sectors.len();
        metric_card(ui, "Sectors Loaded", &format!("{}", n_sectors));

        if let Some(ref bench) = state.market_data.benchmark {
            if let Some(last) = bench.bars.last() {
                metric_card(ui, "SPY Last Close", &format!("${:.2}", last.close));
            }
        }

        metric_card(
            ui,
            "Avg Cross-Correlation",
            &format!("{:.3}", state.analysis.avg_cross_correlation),
        );

        if let Some(spread) = state.analysis.bond_spreads.first() {
            metric_card(
                ui,
                "10Y-2Y Spread",
                &format!("{:.2} bps", spread.spread_10y_2y * 100.0),
            );
        }

        let n_rates = state.market_data.treasury_rates.len();
        metric_card(ui, "Treasury Data Points", &format!("{}", n_rates));
    });

    ui.add_space(16.0);
    ui.separator();
    ui.add_space(8.0);

    // Sector heatmap
    ui.heading("Sector Volatility Heatmap");
    ui.add_space(8.0);

    egui::Grid::new("sector_heatmap")
        .striped(true)
        .min_col_width(100.0)
        .show(ui, |ui| {
            ui.strong("Sector");
            ui.strong("Symbol");
            ui.strong("Last Close");
            ui.strong("21D Vol");
            ui.strong("63D Vol");
            ui.strong("Vol Ratio");
            ui.strong("Bars");
            ui.end_row();

            for (i, sector) in state.market_data.sectors.iter().enumerate() {
                let name = config::SECTOR_ETFS
                    .iter()
                    .find(|(s, _)| *s == sector.symbol)
                    .map(|(_, n)| *n)
                    .unwrap_or("Unknown");

                ui.label(name);
                ui.label(&sector.symbol);

                if let Some(last) = sector.bars.last() {
                    ui.label(format!("${:.2}", last.close));
                } else {
                    ui.label("-");
                }

                // Show latest vol metrics
                if let Some(vm) = state.analysis.volatility.iter().find(|v| v.symbol == sector.symbol) {
                    let sv = vm.short_window_vol.last().copied().unwrap_or(0.0);
                    let lv = vm.long_window_vol.last().copied().unwrap_or(0.0);
                    let vr = vm.vol_ratio.last().copied().unwrap_or(0.0);

                    let vol_color = vol_to_color(sv);
                    ui.colored_label(vol_color, format!("{:.1}%", sv * 100.0));
                    ui.colored_label(vol_to_color(lv), format!("{:.1}%", lv * 100.0));

                    let ratio_color = if vr > 1.2 {
                        egui::Color32::from_rgb(220, 50, 50)
                    } else if vr < 0.8 {
                        egui::Color32::from_rgb(50, 180, 50)
                    } else {
                        ui.visuals().text_color()
                    };
                    ui.colored_label(ratio_color, format!("{:.2}", vr));
                } else {
                    ui.label("-");
                    ui.label("-");
                    ui.label("-");
                }

                ui.label(format!("{}", sector.bars.len()));
                ui.end_row();

                // Highlight selected sector
                if i == state.selected_sector_idx {
                    // selection indicator is handled by grid striping
                }
            }
        });

    // FMP sector performance
    if !state.market_data.sector_performance.is_empty() {
        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);
        ui.heading("FMP Sector Performance (Real-Time)");
        ui.add_space(8.0);

        egui::Grid::new("fmp_sector_perf")
            .striped(true)
            .min_col_width(120.0)
            .show(ui, |ui| {
                ui.strong("Sector");
                ui.strong("Change %");
                ui.end_row();

                for sp in &state.market_data.sector_performance {
                    ui.label(&sp.sector);
                    let color = if sp.changes_percentage >= 0.0 {
                        egui::Color32::from_rgb(50, 180, 50)
                    } else {
                        egui::Color32::from_rgb(220, 50, 50)
                    };
                    ui.colored_label(color, format!("{:+.2}%", sp.changes_percentage));
                    ui.end_row();
                }
            });
    }
}

fn metric_card(ui: &mut egui::Ui, label: &str, value: &str) {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(8.0))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.small(label);
                ui.strong(value);
            });
        });
}

fn vol_to_color(vol: f64) -> egui::Color32 {
    let pct = vol * 100.0;
    if pct > 30.0 {
        egui::Color32::from_rgb(220, 50, 50)
    } else if pct > 20.0 {
        egui::Color32::from_rgb(220, 150, 50)
    } else if pct > 10.0 {
        egui::Color32::from_rgb(200, 200, 50)
    } else {
        egui::Color32::from_rgb(50, 180, 50)
    }
}

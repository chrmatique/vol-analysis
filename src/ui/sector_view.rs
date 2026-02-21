use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};

use crate::app::AppState;
use crate::config;
use crate::ui::chart_utils::{self, height_control, HoverSeries};

pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Sector Volatility Analysis");
    ui.add_space(8.0);

    if state.market_data.sectors.is_empty() {
        ui.label("No data loaded. Click 'Refresh Data' to fetch market data.");
        return;
    }

    // Sector selector
    ui.horizontal(|ui| {
        ui.label("Select Sector:");
        egui::ComboBox::from_id_salt("sector_selector")
            .selected_text(
                state
                    .market_data
                    .sectors
                    .get(state.selected_sector_idx)
                    .map(|s| {
                        let name = config::SECTOR_ETFS
                            .iter()
                            .find(|(sym, _)| *sym == s.symbol)
                            .map(|(_, n)| *n)
                            .unwrap_or("Unknown");
                        format!("{} ({})", name, s.symbol)
                    })
                    .unwrap_or_else(|| "Select...".to_string()),
            )
            .show_ui(ui, |ui| {
                for (i, sector) in state.market_data.sectors.iter().enumerate() {
                    let name = config::SECTOR_ETFS
                        .iter()
                        .find(|(sym, _)| *sym == sector.symbol)
                        .map(|(_, n)| *n)
                        .unwrap_or("Unknown");
                    ui.selectable_value(
                        &mut state.selected_sector_idx,
                        i,
                        format!("{} ({})", name, sector.symbol),
                    );
                }
            });
    });

    ui.add_space(8.0);

    let sector = match state.market_data.sectors.get(state.selected_sector_idx) {
        Some(s) => s,
        None => return,
    };

    let vol_metrics = state
        .analysis
        .volatility
        .iter()
        .find(|v| v.symbol == sector.symbol);

    // Price chart
    ui.collapsing("Price Chart", |ui| {
        height_control(ui, &mut state.chart_heights.sector_price, "Price Chart Height");

        let price_data: Vec<[f64; 2]> = sector
            .bars
            .iter()
            .enumerate()
            .map(|(i, b)| [i as f64, b.close])
            .collect();
        let prices: PlotPoints = price_data.iter().copied().collect();
        let hover = [HoverSeries { name: &sector.symbol, data: &price_data, decimals: 2, suffix: "" }];

        Plot::new("price_plot")
            .height(state.chart_heights.sector_price)
            .allow_drag(true)
            .allow_scroll(false)
            .allow_zoom(false)
            .x_axis_label("Trading Day")
            .y_axis_label("Price ($)")
            .coordinates_formatter(chart_utils::HOVER_CORNER, chart_utils::hover_formatter(&hover))
            .label_formatter(chart_utils::no_hover_label)
            .show(ui, |plot_ui| {
                plot_ui.line(
                    Line::new(prices)
                        .name(&sector.symbol)
                        .color(egui::Color32::from_rgb(100, 150, 255)),
                );
            });
    });

    ui.add_space(8.0);

    // Volatility chart
    if let Some(vm) = vol_metrics {
        ui.label(format!(
            "Showing {}-day and {}-day rolling volatility",
            config::SHORT_VOL_WINDOW,
            config::LONG_VOL_WINDOW
        ));

        let short_data: Vec<[f64; 2]> = vm
            .short_window_vol
            .iter()
            .enumerate()
            .map(|(i, v)| [i as f64, *v * 100.0])
            .collect();
        let short_points: PlotPoints = short_data.iter().copied().collect();

        let long_data: Vec<[f64; 2]> = vm
            .long_window_vol
            .iter()
            .enumerate()
            .map(|(i, v)| [i as f64, *v * 100.0])
            .collect();
        let long_points: PlotPoints = long_data.iter().copied().collect();

        let park_data: Vec<[f64; 2]> = vm
            .parkinson_vol
            .iter()
            .enumerate()
            .map(|(i, v)| [i as f64, *v * 100.0])
            .collect();
        let park_points: PlotPoints = park_data.iter().copied().collect();

        let short_name = format!("{}D Vol", config::SHORT_VOL_WINDOW);
        let long_name = format!("{}D Vol", config::LONG_VOL_WINDOW);
        let vol_hover = [
            HoverSeries { name: &short_name, data: &short_data, decimals: 1, suffix: "%" },
            HoverSeries { name: &long_name, data: &long_data, decimals: 1, suffix: "%" },
            HoverSeries { name: "Parkinson Vol", data: &park_data, decimals: 1, suffix: "%" },
        ];

        height_control(ui, &mut state.chart_heights.sector_vol, "Volatility Chart Height");
        Plot::new("vol_plot")
            .height(state.chart_heights.sector_vol)
            .allow_drag(true)
            .allow_scroll(false)
            .allow_zoom(false)
            .x_axis_label("Trading Day (aligned)")
            .y_axis_label("Annualized Vol (%)")
            .legend(egui_plot::Legend::default())
            .coordinates_formatter(chart_utils::HOVER_CORNER, chart_utils::hover_formatter(&vol_hover))
            .label_formatter(chart_utils::no_hover_label)
            .show(ui, |plot_ui| {
                plot_ui.line(
                    Line::new(short_points)
                        .name(format!("{}D Vol", config::SHORT_VOL_WINDOW))
                        .color(egui::Color32::from_rgb(255, 100, 100)),
                );
                plot_ui.line(
                    Line::new(long_points)
                        .name(format!("{}D Vol", config::LONG_VOL_WINDOW))
                        .color(egui::Color32::from_rgb(100, 100, 255)),
                );
                plot_ui.line(
                    Line::new(park_points)
                        .name("Parkinson Vol")
                        .color(egui::Color32::from_rgb(100, 220, 100)),
                );
            });

        // Vol ratio chart
        ui.add_space(8.0);
        ui.label("Volatility Ratio (Short / Long) - above 1.0 indicates rising vol regime");

        let ratio_data: Vec<[f64; 2]> = vm
            .vol_ratio
            .iter()
            .enumerate()
            .map(|(i, v)| [i as f64, *v])
            .collect();
        let ratio_points: PlotPoints = ratio_data.iter().copied().collect();

        let baseline: PlotPoints = PlotPoints::from_iter(
            (0..vm.vol_ratio.len()).map(|i| [i as f64, 1.0]),
        );

        let ratio_hover = [HoverSeries { name: "Vol Ratio", data: &ratio_data, decimals: 2, suffix: "" }];

        height_control(ui, &mut state.chart_heights.sector_ratio, "Vol Ratio Chart Height");
        Plot::new("ratio_plot")
            .height(state.chart_heights.sector_ratio)
            .allow_drag(true)
            .allow_scroll(false)
            .allow_zoom(false)
            .x_axis_label("Trading Day (aligned)")
            .y_axis_label("Vol Ratio")
            .coordinates_formatter(chart_utils::HOVER_CORNER, chart_utils::hover_formatter(&ratio_hover))
            .label_formatter(chart_utils::no_hover_label)
            .show(ui, |plot_ui| {
                plot_ui.line(
                    Line::new(ratio_points)
                        .name("Vol Ratio")
                        .color(egui::Color32::from_rgb(255, 180, 50)),
                );
                plot_ui.line(
                    Line::new(baseline)
                        .name("Baseline (1.0)")
                        .color(egui::Color32::from_rgb(150, 150, 150))
                        .style(egui_plot::LineStyle::dashed_dense()),
                );
            });

        // Summary stats
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);

        if let (Some(sv), Some(lv), Some(vr)) = (
            vm.short_window_vol.last(),
            vm.long_window_vol.last(),
            vm.vol_ratio.last(),
        ) {
            ui.horizontal(|ui| {
                ui.label(format!(
                    "Latest: {}D Vol = {:.1}% | {}D Vol = {:.1}% | Ratio = {:.2}",
                    config::SHORT_VOL_WINDOW,
                    sv * 100.0,
                    config::LONG_VOL_WINDOW,
                    lv * 100.0,
                    vr
                ));
            });
        }
    } else {
        ui.label("No volatility data computed for this sector yet.");
    }
}

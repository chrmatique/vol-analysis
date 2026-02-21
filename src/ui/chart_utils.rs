/// Shared chart utilities for all UI views that render plots.

use eframe::egui;
use egui_plot::{CoordinatesFormatter, Corner, PlotBounds, PlotPoint};

// ── Hover label utilities ───────────────────────────────────────────────────

/// A named data series for hover display. Borrows the underlying data so no
/// heap allocation is needed beyond what the view already holds.
pub struct HoverSeries<'a> {
    pub name: &'a str,
    pub data: &'a [[f64; 2]],
    pub decimals: usize,
    pub suffix: &'a str,
}

/// Build a `CoordinatesFormatter` that shows the nearest Y value for each
/// series at the cursor's X position.  Use with
/// `Plot::coordinates_formatter(Corner::RightBottom, hover_formatter(&series))`.
pub fn hover_formatter<'a>(series: &'a [HoverSeries<'a>]) -> CoordinatesFormatter<'a> {
    CoordinatesFormatter::new(move |cursor: &PlotPoint, _bounds: &PlotBounds| {
        let x = cursor.x;
        let mut text = format!("x: {:.0}", x);
        for s in series {
            if let Some(idx) = nearest_x_index(s.data, x) {
                use std::fmt::Write;
                let _ = write!(
                    text,
                    "\n{}: {:.prec$}{}",
                    s.name,
                    s.data[idx][1],
                    s.suffix,
                    prec = s.decimals
                );
            }
        }
        text
    })
}

/// Variant of [`hover_formatter`] for charts with discrete, labelled X
/// positions (e.g. yield-curve maturity names).  `x_labels[i]` is shown
/// instead of the numeric X value when the cursor is nearest to index `i`.
pub fn hover_formatter_labeled_x<'a>(
    series: &'a [HoverSeries<'a>],
    x_labels: &'a [String],
) -> CoordinatesFormatter<'a> {
    CoordinatesFormatter::new(move |cursor: &PlotPoint, _bounds: &PlotBounds| {
        let x = cursor.x;
        let x_idx = x.round().max(0.0) as usize;
        let x_display = x_labels
            .get(x_idx)
            .map(|s| s.as_str())
            .unwrap_or("?");
        let mut text = x_display.to_string();
        for s in series {
            if let Some(idx) = nearest_x_index(s.data, x) {
                use std::fmt::Write;
                let _ = write!(
                    text,
                    "\n{}: {:.prec$}{}",
                    s.name,
                    s.data[idx][1],
                    s.suffix,
                    prec = s.decimals
                );
            }
        }
        text
    })
}

/// Pass to `Plot::label_formatter` to suppress the default per-line hover
/// tooltip (we show data in the corner instead).
pub fn no_hover_label(_name: &str, _point: &PlotPoint) -> String {
    String::new()
}

/// The fixed corner where hover labels are displayed.
pub const HOVER_CORNER: Corner = Corner::RightBottom;

/// Binary-search for the index of the data point whose X is closest to
/// `target_x`.  Assumes `data` is sorted ascending by `[0]` (X).
fn nearest_x_index(data: &[[f64; 2]], target_x: f64) -> Option<usize> {
    if data.is_empty() {
        return None;
    }
    let idx = data.partition_point(|p| p[0] < target_x);
    if idx == 0 {
        return Some(0);
    }
    if idx >= data.len() {
        return Some(data.len() - 1);
    }
    let left_dist = (data[idx - 1][0] - target_x).abs();
    let right_dist = (data[idx][0] - target_x).abs();
    if left_dist <= right_dist {
        Some(idx - 1)
    } else {
        Some(idx)
    }
}

/// Inline height-adjustment drag control placed immediately above a chart.
/// Allows all drawn charts to be vertically resized via a shared implementation.
pub fn height_control(ui: &mut egui::Ui, height: &mut f32, label: &str) {
    egui::Frame::none()
        .fill(egui::Color32::from_rgba_unmultiplied(80, 120, 200, 18))
        .inner_margin(egui::Margin::symmetric(8.0, 3.0))
        .rounding(egui::Rounding::same(4.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.colored_label(egui::Color32::from_rgb(100, 160, 255), "⇕");
                ui.colored_label(egui::Color32::from_gray(170), label);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(
                        egui::DragValue::new(height)
                            .speed(2.0)
                            .range(80.0..=800.0)
                            .suffix(" px"),
                    );
                    ui.colored_label(egui::Color32::from_gray(130), "drag to resize ·");
                });
            });
        });
    ui.add_space(2.0);
}

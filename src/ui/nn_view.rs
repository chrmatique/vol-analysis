use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};

use crate::app::AppState;
use crate::data::models::TrainingStatus;
use crate::nn::training::TrainingProgress;

pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Neural Network - Volatility Regime Prediction");
    ui.add_space(8.0);

    if state.market_data.sectors.is_empty() {
        ui.label("Load market data first before training the neural network.");
        return;
    }

    // Model info
    ui.group(|ui| {
        ui.label("Model Architecture: LSTM (hidden=64) -> Linear");
        ui.label("Input: 26 features (11 sector vols + 11 returns + cross-corr + spread + slope + VIX-proxy)");
        ui.label("Output: 5-day forward realized volatility prediction");
        ui.label(format!("Lookback: {} trading days per sample", crate::config::NN_LOOKBACK_DAYS));
    });

    ui.add_space(8.0);

    // Check training progress from background thread
    if let Some(ref progress) = state.training_progress {
        if let Ok(status) = progress.status.lock() {
            state.training_status = status.clone();
        }
        if let Ok(losses) = progress.losses.lock() {
            state.training_losses = losses.clone();
        }
        if let Ok(preds) = progress.predictions.lock() {
            state.nn_predictions = preds.clone();
        }
    }

    // Training controls
    ui.horizontal(|ui| {
        match &state.training_status {
            TrainingStatus::Idle => {
                if ui.button("Train Model").clicked() {
                    start_training(state);
                }
            }
            TrainingStatus::Training {
                epoch,
                total_epochs,
                loss,
            } => {
                ui.spinner();
                ui.label(format!(
                    "Training... Epoch {}/{} | Loss: {:.6}",
                    epoch, total_epochs, loss
                ));
                let progress = *epoch as f32 / *total_epochs as f32;
                ui.add(egui::ProgressBar::new(progress).show_percentage());
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
            }
            TrainingStatus::Complete { final_loss } => {
                ui.colored_label(
                    egui::Color32::from_rgb(50, 180, 50),
                    format!("Training complete! Final loss: {:.6}", final_loss),
                );
                if ui.button("Retrain").clicked() {
                    state.training_status = TrainingStatus::Idle;
                    state.training_losses.clear();
                    state.nn_predictions.clear();
                    state.training_progress = None;
                }
            }
            TrainingStatus::Error(msg) => {
                ui.colored_label(
                    egui::Color32::from_rgb(220, 50, 50),
                    format!("Error: {}", msg),
                );
                if ui.button("Retry").clicked() {
                    state.training_status = TrainingStatus::Idle;
                    state.training_progress = None;
                }
            }
        }
    });

    ui.add_space(8.0);

    // Loss curve
    if !state.training_losses.is_empty() {
        ui.heading("Training Loss");
        let loss_points: PlotPoints = state
            .training_losses
            .iter()
            .enumerate()
            .map(|(i, l)| [i as f64, *l])
            .collect();

        Plot::new("loss_plot")
            .height(200.0)
            .allow_drag(true)
            .allow_zoom(true)
            .x_axis_label("Epoch")
            .y_axis_label("MSE Loss")
            .show(ui, |plot_ui| {
                plot_ui.line(
                    Line::new(loss_points)
                        .name("Training Loss")
                        .color(egui::Color32::from_rgb(255, 100, 100)),
                );
            });
    }

    ui.add_space(8.0);

    // Predictions table
    if !state.nn_predictions.is_empty() {
        ui.heading("Predictions (5-Day Forward Vol)");
        ui.add_space(4.0);

        egui::Grid::new("predictions_table")
            .striped(true)
            .min_col_width(100.0)
            .show(ui, |ui| {
                ui.strong("Sector");
                ui.strong("Predicted Vol (%)");
                ui.end_row();

                for (sector, vol) in &state.nn_predictions {
                    ui.label(sector);
                    let vol_pct = vol * 100.0;
                    let color = if vol_pct > 30.0 {
                        egui::Color32::from_rgb(220, 50, 50)
                    } else if vol_pct > 20.0 {
                        egui::Color32::from_rgb(220, 150, 50)
                    } else {
                        egui::Color32::from_rgb(50, 180, 50)
                    };
                    ui.colored_label(color, format!("{:.2}%", vol_pct));
                    ui.end_row();
                }
            });
    } else if matches!(state.training_status, TrainingStatus::Idle) {
        ui.add_space(8.0);
        ui.label("No predictions yet. Train the model to generate predictions.");
    }

    ui.add_space(16.0);
    ui.separator();
    ui.add_space(4.0);
    ui.small("Neural network powered by the Burn deep learning framework (NdArray backend with autodiff).");
}

fn start_training(state: &mut AppState) {
    let progress = TrainingProgress::new();
    state.training_progress = Some(progress.clone());
    state.training_status = TrainingStatus::Training {
        epoch: 0,
        total_epochs: crate::config::NN_EPOCHS,
        loss: f64::NAN,
    };
    state.training_losses.clear();
    state.nn_predictions.clear();

    let market_data = state.market_data.clone();

    std::thread::spawn(move || {
        crate::nn::training::train(&market_data, &progress);
    });
}

use eframe::egui;

use crate::app::AppState;

pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Settings");
    ui.add_space(8.0);

    let mut prev_visible = false;

    // NN Training Settings section
    render_nn_training_section(ui, state, &mut prev_visible);
}

fn render_nn_training_section(
    ui: &mut egui::Ui,
    state: &mut AppState,
    prev_visible: &mut bool,
) {
    // Add divider before this section if previous section was visible
    if *prev_visible {
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);
    }

    ui.heading("Neural Network Training");
    ui.add_space(4.0);

    ui.group(|ui| {
        ui.label("Select input features to use during model training:");
        ui.add_space(4.0);

        // Sector Volatility checkbox
        let mut vol_enabled = state.nn_feature_flags.sector_volatility;
        ui.checkbox(&mut vol_enabled, "Sector Volatility (11 features)");
        if vol_enabled != state.nn_feature_flags.sector_volatility {
            state.nn_feature_flags.sector_volatility = vol_enabled;
        }

        // Market Randomness checkbox
        let mut rand_enabled = state.nn_feature_flags.market_randomness;
        ui.checkbox(&mut rand_enabled, "Market Randomness (22 features)");
        if rand_enabled != state.nn_feature_flags.market_randomness {
            state.nn_feature_flags.market_randomness = rand_enabled;
        }

        // Kurtosis checkbox
        let mut kurt_enabled = state.nn_feature_flags.kurtosis;
        ui.checkbox(&mut kurt_enabled, "Kurtosis (22 features)");
        if kurt_enabled != state.nn_feature_flags.kurtosis {
            state.nn_feature_flags.kurtosis = kurt_enabled;
        }

        ui.add_space(8.0);

        if ui.button("Save Settings").clicked() {
            if let Ok(_) = crate::data::cache::save_json("nn_feature_flags.json", &state.nn_feature_flags) {
                state.status_message = "Settings saved successfully.".to_string();
            } else {
                state.status_message = "Failed to save settings.".to_string();
            }
        }

        ui.label("Settings are applied when you start a new training session.");
    });

    *prev_visible = true;
}

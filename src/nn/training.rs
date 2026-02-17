use std::sync::{Arc, Mutex};

use burn::{
    backend::{Autodiff, NdArray},
    data::dataloader::DataLoaderBuilder,
    module::AutodiffModule,
    optim::{AdamConfig, GradientsParams, Optimizer},
    tensor::backend::AutodiffBackend,
};

use crate::config;
use crate::data::models::{MarketData, TrainingStatus};
use crate::nn::dataset::{build_dataset, VolBatcher};
use crate::nn::model::{VolPredictionModelConfig, NUM_FEATURES, OUTPUT_SIZE};

/// Training backend: NdArray with autodiff (CPU-based, reliable)
pub type TrainingBackend = Autodiff<NdArray>;

/// Shared state for communicating training progress to the UI
#[derive(Clone)]
pub struct TrainingProgress {
    pub status: Arc<Mutex<TrainingStatus>>,
    pub losses: Arc<Mutex<Vec<f64>>>,
    pub predictions: Arc<Mutex<Vec<(String, f64)>>>,
}

impl TrainingProgress {
    pub fn new() -> Self {
        Self {
            status: Arc::new(Mutex::new(TrainingStatus::Idle)),
            losses: Arc::new(Mutex::new(Vec::new())),
            predictions: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

/// Run the full training pipeline
pub fn train(market_data: &MarketData, progress: &TrainingProgress) {
    let device = <NdArray as burn::tensor::backend::Backend>::Device::default();

    // Update status
    set_status(progress, TrainingStatus::Training {
        epoch: 0,
        total_epochs: config::NN_EPOCHS,
        loss: f64::NAN,
    });

    // Build dataset
    let dataset = build_dataset(market_data, config::NN_LOOKBACK_DAYS, config::NN_FORWARD_DAYS);

    if dataset.samples.is_empty() {
        set_status(progress, TrainingStatus::Error(
            "Not enough data to build training dataset. Load more market data.".into(),
        ));
        return;
    }

    let total = dataset.samples.len();
    let train_size = (total as f64 * 0.8) as usize;

    if train_size < config::NN_BATCH_SIZE || total - train_size < 1 {
        set_status(progress, TrainingStatus::Error(
            format!("Dataset too small ({} samples). Need more data.", total),
        ));
        return;
    }

    // Split chronologically
    let train_samples = dataset.samples[..train_size].to_vec();
    let _val_samples = dataset.samples[train_size..].to_vec();

    let train_dataset = crate::nn::dataset::VolDataset { samples: train_samples };

    let batcher = VolBatcher::<TrainingBackend>::new(device.clone());

    let dataloader = DataLoaderBuilder::new(batcher)
        .batch_size(config::NN_BATCH_SIZE)
        .shuffle(42)
        .build(train_dataset);

    // Initialize model
    let model_config = VolPredictionModelConfig {
        input_size: NUM_FEATURES,
        hidden_size: config::NN_HIDDEN_SIZE,
        output_size: OUTPUT_SIZE,
    };
    let mut model = model_config.init::<TrainingBackend>(&device);

    // Optimizer
    let mut optim = AdamConfig::new().init();

    // Training loop
    let mut best_loss = f64::INFINITY;
    for epoch in 0..config::NN_EPOCHS {
        let mut epoch_loss = 0.0;
        let mut batch_count = 0;

        for batch in dataloader.iter() {
            let output = model.forward(batch.inputs);
            let loss = mse_loss(output, batch.targets);

            let loss_val = loss.clone().into_data().to_vec::<f32>().unwrap_or_default();
            let loss_scalar = loss_val.first().copied().unwrap_or(f32::NAN) as f64;

            // Backward pass
            let grads = loss.backward();
            let grads = GradientsParams::from_grads(grads, &model);
            model = optim.step(config::NN_LEARNING_RATE, model, grads);

            epoch_loss += loss_scalar;
            batch_count += 1;
        }

        let avg_loss = if batch_count > 0 {
            epoch_loss / batch_count as f64
        } else {
            f64::NAN
        };

        // Track best
        if avg_loss < best_loss {
            best_loss = avg_loss;
        }

        // Update progress
        if let Ok(mut losses) = progress.losses.lock() {
            losses.push(avg_loss);
        }
        set_status(progress, TrainingStatus::Training {
            epoch: epoch + 1,
            total_epochs: config::NN_EPOCHS,
            loss: avg_loss,
        });
    }

    // Generate predictions using the trained model's inference mode
    let inference_device = <NdArray as burn::tensor::backend::Backend>::Device::default();
    let valid_model = model.valid();
    generate_predictions(&valid_model, market_data, &inference_device, progress);

    set_status(progress, TrainingStatus::Complete { final_loss: best_loss });
}

/// Mean squared error loss
fn mse_loss<B: AutodiffBackend>(
    predictions: burn::tensor::Tensor<B, 2>,
    targets: burn::tensor::Tensor<B, 2>,
) -> burn::tensor::Tensor<B, 1> {
    let diff = predictions - targets;
    let sq = diff.clone() * diff;
    sq.mean().unsqueeze()
}

/// Generate predictions for each sector using the trained model
fn generate_predictions<B: burn::tensor::backend::Backend>(
    model: &crate::nn::model::VolPredictionModel<B>,
    market_data: &MarketData,
    device: &B::Device,
    progress: &TrainingProgress,
) {
    let dataset = build_dataset(market_data, config::NN_LOOKBACK_DAYS, config::NN_FORWARD_DAYS);

    if let Some(last_sample) = dataset.samples.last() {
        let seq_len = last_sample.features.len();
        let num_features = last_sample.features.first().map(|f| f.len()).unwrap_or(0);

        let mut input_data: Vec<f32> = Vec::with_capacity(seq_len * num_features);
        for step in &last_sample.features {
            for &f in step {
                input_data.push(f as f32);
            }
        }

        let input = burn::tensor::Tensor::<B, 1>::from_floats(input_data.as_slice(), device)
            .reshape([1_usize, seq_len, num_features]);

        let pred = model.forward(input);
        let pred_val = pred.into_data().to_vec::<f32>().unwrap_or_default();
        let predicted_vol = pred_val.first().copied().unwrap_or(0.0) as f64;

        let mut predictions = Vec::new();
        for sector in &market_data.sectors {
            predictions.push((sector.symbol.clone(), predicted_vol));
        }

        if let Ok(mut preds) = progress.predictions.lock() {
            *preds = predictions;
        }
    }
}

fn set_status(progress: &TrainingProgress, status: TrainingStatus) {
    if let Ok(mut s) = progress.status.lock() {
        *s = status;
    }
}

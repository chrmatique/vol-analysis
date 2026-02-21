/// Sector ETF ticker symbols (SPDR S&P 500 sector ETFs)
pub const SECTOR_ETFS: &[(&str, &str)] = &[
    ("XLK", "Technology"),
    ("XLF", "Financials"),
    ("XLE", "Energy"),
    ("XLV", "Healthcare"),
    ("XLI", "Industrials"),
    ("XLP", "Consumer Staples"),
    ("XLY", "Consumer Discretionary"),
    ("XLU", "Utilities"),
    ("XLRE", "Real Estate"),
    ("XLC", "Communication Services"),
    ("XLB", "Materials"),
];

/// Market benchmark
pub const BENCHMARK_SYMBOL: &str = "SPY";

/// Financial Modeling Prep API key.
/// Reads FMP_API_KEY from the environment (set in a gitignored .env file).
/// Call `load_env()` once at startup to populate the environment from .env.
pub fn fmp_api_key() -> String {
    dotenvy::var("FMP_API_KEY").unwrap_or_default()
}

/// Load variables from a `.env` file in the working directory into the process
/// environment. Silent no-op if the file is absent (production builds may rely
/// on the environment being set externally).
pub fn load_env() {
    let Ok(contents) = std::fs::read_to_string(".env") else { return };
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            if std::env::var(key).is_err() {
                // Only set if not already present in the environment
                unsafe { std::env::set_var(key, value); }
            }
        }
    }
}

/// Default historical lookback in calendar days (~2 years)
pub const DEFAULT_LOOKBACK_DAYS: u32 = 730;

/// Rolling volatility window sizes (trading days)
pub const SHORT_VOL_WINDOW: usize = 21;  // ~1 month
pub const LONG_VOL_WINDOW: usize = 63;   // ~3 months

/// Neural network configuration
pub const NN_LOOKBACK_DAYS: usize = 60;
pub const NN_FORWARD_DAYS: usize = 5;
pub const NN_HIDDEN_SIZE: usize = 64;
pub const NN_LEARNING_RATE: f64 = 1e-3;
pub const NN_EPOCHS: usize = 1000;
pub const NN_BATCH_SIZE: usize = 32;

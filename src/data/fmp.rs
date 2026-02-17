use anyhow::{Context, Result};

use crate::data::cache;
use crate::data::models::{SectorPerformance, TreasuryRate};

/// Fetch treasury rates from FMP API
pub async fn fetch_treasury_rates(api_key: &str) -> Result<Vec<TreasuryRate>> {
    let cache_file = "fmp_treasury_rates.json";
    if cache::is_cache_fresh(cache_file, 12) {
        if let Ok(cached) = cache::load_json::<Vec<TreasuryRate>>(cache_file) {
            tracing::info!("Using cached treasury rates");
            return Ok(cached);
        }
    }

    tracing::info!("Fetching FMP treasury rates");
    let url = format!(
        "https://financialmodelingprep.com/stable/treasury-rates?apikey={}",
        api_key
    );

    let resp = reqwest::get(&url)
        .await
        .context("Failed to fetch treasury rates")?;

    let rates: Vec<TreasuryRate> = resp
        .json()
        .await
        .context("Failed to parse treasury rates JSON")?;

    if let Err(e) = cache::save_json(cache_file, &rates) {
        tracing::warn!("Failed to cache treasury rates: {}", e);
    }

    Ok(rates)
}

/// Fetch sector performance from FMP API (v3 endpoint)
pub async fn fetch_sector_performance(api_key: &str) -> Result<Vec<SectorPerformance>> {
    let cache_file = "fmp_sector_performance.json";
    if cache::is_cache_fresh(cache_file, 1) {
        if let Ok(cached) = cache::load_json::<Vec<SectorPerformance>>(cache_file) {
            tracing::info!("Using cached sector performance");
            return Ok(cached);
        }
    }

    tracing::info!("Fetching FMP sector performance");
    let url = format!(
        "https://financialmodelingprep.com/api/v3/sector-performance?apikey={}",
        api_key
    );

    let resp = reqwest::get(&url)
        .await
        .context("Failed to fetch sector performance")?;

    let text = resp.text().await.context("Failed to read response body")?;

    let performance: Vec<SectorPerformance> = serde_json::from_str(&text)
        .unwrap_or_else(|e| {
            tracing::warn!("Failed to parse sector performance: {}. Response: {}", e, &text[..200.min(text.len())]);
            vec![]
        });

    if !performance.is_empty() {
        if let Err(e) = cache::save_json(cache_file, &performance) {
            tracing::warn!("Failed to cache sector performance: {}", e);
        }
    }

    Ok(performance)
}

use anyhow::{Context, Result};
use chrono::NaiveDate;
use time::OffsetDateTime;
use yahoo_finance_api as yahoo;

use crate::data::cache;
use crate::data::models::{OhlcvBar, SectorTimeSeries};

/// Fetch historical OHLCV data for a given symbol from Yahoo Finance
pub async fn fetch_symbol_history(
    symbol: &str,
    name: &str,
    lookback_days: u32,
) -> Result<SectorTimeSeries> {
    let cache_file = format!("yahoo_{}.json", symbol);
    if cache::is_cache_fresh(&cache_file, 12) {
        if let Ok(cached) = cache::load_json::<SectorTimeSeries>(&cache_file) {
            tracing::info!("Using cached data for {}", symbol);
            return Ok(cached);
        }
    }

    tracing::info!("Fetching Yahoo Finance data for {}", symbol);
    let provider = yahoo::YahooConnector::new()
        .context("Failed to create Yahoo connector")?;

    let now = OffsetDateTime::now_utc();
    let start = now - time::Duration::days(lookback_days as i64);

    let resp = provider
        .get_quote_history(symbol, start, now)
        .await
        .with_context(|| format!("Failed to fetch history for {}", symbol))?;

    let quotes = resp
        .quotes()
        .with_context(|| format!("Failed to parse quotes for {}", symbol))?;

    let bars: Vec<OhlcvBar> = quotes
        .iter()
        .filter_map(|q| {
            let dt = OffsetDateTime::from_unix_timestamp(q.timestamp as i64).ok()?;
            let date = NaiveDate::from_ymd_opt(
                dt.year(),
                dt.month() as u32,
                dt.day() as u32,
            )?;
            Some(OhlcvBar {
                date,
                open: q.open,
                high: q.high,
                low: q.low,
                close: q.close,
                volume: q.volume,
            })
        })
        .collect();

    let series = SectorTimeSeries {
        symbol: symbol.to_string(),
        name: name.to_string(),
        bars,
    };

    if let Err(e) = cache::save_json(&cache_file, &series) {
        tracing::warn!("Failed to cache data for {}: {}", symbol, e);
    }

    Ok(series)
}

/// Fetch data for all sector ETFs concurrently
pub async fn fetch_all_sectors(
    symbols: &[(&str, &str)],
    lookback_days: u32,
) -> Vec<(String, Result<SectorTimeSeries>)> {
    let mut handles = Vec::new();

    for &(symbol, name) in symbols {
        let sym = symbol.to_string();
        let nm = name.to_string();
        let handle = tokio::spawn(async move {
            let result = fetch_symbol_history(&sym, &nm, lookback_days).await;
            (sym, result)
        });
        handles.push(handle);
    }

    let mut results = Vec::new();
    for handle in handles {
        match handle.await {
            Ok(result) => results.push(result),
            Err(e) => {
                tracing::error!("Task join error: {}", e);
            }
        }
    }

    results
}

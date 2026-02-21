use anyhow::{Context, Result};

use crate::data::cache;
use crate::data::models::TreasuryRate;
use crate::data::models::SectorPerformance;

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

/// Test for fetch_treasury_rates: fetches, prints JSON to debug terminal.
/// `cargo test -- --nocapture fetch_treasury_rates_dump_json` to see output.
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fetch_treasury_rates_dump_json() {
        let api_key = std::env::var("FMP_API_KEY")
            .or_else(|_| dotenvy::var("FMP_API_KEY"))
            .expect("FMP_API_KEY not set in environment or .env");
        let res = fetch_treasury_rates(&api_key).await;
        match res {
            Ok(rates) => {
                let json = serde_json::to_string_pretty(&rates).unwrap();
                // Print to debug terminal
                println!("{}", json);
            }
            Err(e) => panic!("fetch_treasury_rates failed: {:?}", e),
        }
    }
}

/// Fetch sector performance from FMP stable sector-performance-snapshot endpoint.
/// Tries recent business days until data is found.
pub async fn fetch_sector_performance(api_key: &str) -> Result<Vec<SectorPerformance>> {
    let cache_file = "fmp_sector_performance.json";
    if cache::is_cache_fresh(cache_file, 1) {
        if let Ok(cached) = cache::load_json::<Vec<SectorPerformance>>(cache_file) {
            tracing::info!("Using cached sector performance");
            return Ok(cached);
        }
    }

    tracing::info!("Fetching FMP sector performance snapshot");

    let today = chrono::Local::now().date_naive();

    for offset in 1..=7 {
        let date = today - chrono::Duration::days(offset);
        let date_str = date.format("%Y-%m-%d");
        let url = format!(
            "https://financialmodelingprep.com/stable/sector-performance-snapshot?date={}&apikey={}",
            date_str, api_key
        );

        let resp = match reqwest::get(&url).await {
            Ok(r) => r,
            Err(e) => {
                tracing::debug!("Request failed for {}: {}", date_str, e);
                continue;
            }
        };

        let text = match resp.text().await {
            Ok(t) => t,
            Err(_) => continue,
        };

        if text.contains("Error") || text.contains("error") {
            tracing::debug!("FMP error for {}: {}", date_str, &text[..200.min(text.len())]);
            continue;
        }

        match serde_json::from_str::<Vec<SectorPerformance>>(&text) {
            Ok(perf) if !perf.is_empty() => {
                // Deduplicate by sector (keep first occurrence per sector — typically NASDAQ)
                let mut seen = std::collections::HashSet::new();
                let deduped: Vec<SectorPerformance> = perf
                    .into_iter()
                    .filter(|p| seen.insert(p.sector.clone()))
                    .collect();

                tracing::info!(
                    "Got sector performance for {} ({} sectors)",
                    date_str,
                    deduped.len()
                );

                if let Err(e) = cache::save_json(cache_file, &deduped) {
                    tracing::warn!("Failed to cache sector performance: {}", e);
                }
                return Ok(deduped);
            }
            Ok(_) => continue,
            Err(e) => {
                tracing::debug!("Parse error for {}: {}", date_str, e);
                continue;
            }
        }
    }

    tracing::warn!("Could not fetch sector performance for any recent date");
    Ok(vec![])
}

use crate::data::models::VolatilityMetrics;

const TRADING_DAYS_PER_YEAR: f64 = 252.0;

/// Compute rolling historical volatility (annualized std dev of log returns)
pub fn rolling_volatility(log_returns: &[f64], window: usize) -> Vec<f64> {
    if log_returns.len() < window || window < 2 {
        return vec![];
    }
    log_returns
        .windows(window)
        .map(|w| {
            let mean = w.iter().sum::<f64>() / w.len() as f64;
            let variance =
                w.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (w.len() - 1) as f64;
            variance.sqrt() * TRADING_DAYS_PER_YEAR.sqrt()
        })
        .collect()
}

/// Parkinson volatility estimator using high/low range (more efficient than close-to-close)
pub fn parkinson_volatility(highs: &[f64], lows: &[f64], window: usize) -> Vec<f64> {
    if highs.len() != lows.len() || highs.len() < window || window < 1 {
        return vec![];
    }

    let hl_log_sq: Vec<f64> = highs
        .iter()
        .zip(lows.iter())
        .map(|(h, l)| {
            if *l <= 0.0 || *h <= 0.0 {
                return 0.0;
            }
            (h / l).ln().powi(2)
        })
        .collect();

    let factor = 1.0 / (4.0 * std::f64::consts::LN_2);
    hl_log_sq
        .windows(window)
        .map(|w| {
            let avg = w.iter().sum::<f64>() / w.len() as f64;
            (factor * avg).sqrt() * TRADING_DAYS_PER_YEAR.sqrt()
        })
        .collect()
}

/// Compute volatility ratio (short-term / long-term) aligned by their trailing ends
pub fn volatility_ratio(short_vol: &[f64], long_vol: &[f64]) -> Vec<f64> {
    let len = short_vol.len().min(long_vol.len());
    let s_off = short_vol.len() - len;
    let l_off = long_vol.len() - len;
    short_vol[s_off..]
        .iter()
        .zip(&long_vol[l_off..])
        .map(|(s, l)| if l.abs() > 1e-10 { s / l } else { 1.0 })
        .collect()
}

/// Compute full VolatilityMetrics for a sector
pub fn compute_sector_volatility(
    symbol: &str,
    log_returns: &[f64],
    highs: &[f64],
    lows: &[f64],
    short_window: usize,
    long_window: usize,
) -> VolatilityMetrics {
    let short_vol = rolling_volatility(log_returns, short_window);
    let long_vol = rolling_volatility(log_returns, long_window);
    let park_vol = parkinson_volatility(highs, lows, short_window);
    let vol_rat = volatility_ratio(&short_vol, &long_vol);

    // Trim all series to match the shortest (long_vol)
    let n = long_vol.len();
    let trim = |v: &[f64]| -> Vec<f64> {
        if v.len() >= n {
            v[v.len() - n..].to_vec()
        } else {
            v.to_vec()
        }
    };

    VolatilityMetrics {
        symbol: symbol.to_string(),
        short_window_vol: trim(&short_vol),
        long_window_vol: long_vol,
        parkinson_vol: trim(&park_vol),
        vol_ratio: vol_rat,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_returns() -> Vec<f64> {
        vec![
            0.01, -0.005, 0.008, -0.012, 0.003, 0.007, -0.002, 0.015, -0.01, 0.006,
            0.002, -0.008, 0.011, -0.004, 0.009, -0.001, 0.005, -0.007, 0.013, -0.003,
            0.004, -0.006, 0.01, -0.009, 0.002,
        ]
    }

    #[test]
    fn test_rolling_volatility_length() {
        let returns = sample_returns();
        let vol = rolling_volatility(&returns, 5);
        assert_eq!(vol.len(), returns.len() - 5 + 1);
    }

    #[test]
    fn test_rolling_volatility_positive() {
        let returns = sample_returns();
        let vol = rolling_volatility(&returns, 5);
        for v in &vol {
            assert!(*v > 0.0, "Volatility should be positive, got {}", v);
        }
    }

    #[test]
    fn test_rolling_volatility_insufficient_data() {
        let returns = vec![0.01, 0.02];
        let vol = rolling_volatility(&returns, 5);
        assert!(vol.is_empty());
    }

    #[test]
    fn test_parkinson_volatility() {
        let highs = vec![101.0, 102.0, 100.5, 103.0, 101.5, 104.0, 102.0];
        let lows = vec![99.0, 100.0, 98.5, 101.0, 99.5, 102.0, 100.0];
        let vol = parkinson_volatility(&highs, &lows, 3);
        assert_eq!(vol.len(), 5);
        for v in &vol {
            assert!(*v > 0.0);
        }
    }

    #[test]
    fn test_volatility_ratio() {
        let short = vec![0.15, 0.20, 0.18, 0.22];
        let long = vec![0.16, 0.19];
        let ratio = volatility_ratio(&short, &long);
        assert_eq!(ratio.len(), 2);
        assert!((ratio[0] - 0.18 / 0.16).abs() < 1e-10);
        assert!((ratio[1] - 0.22 / 0.19).abs() < 1e-10);
    }
}

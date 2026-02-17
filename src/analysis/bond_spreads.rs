use chrono::NaiveDate;

use crate::data::models::{BondSpread, TreasuryRate};

/// Compute term spread (10Y - 2Y) and curve slope (30Y - 3M) from treasury rate data
pub fn compute_term_spreads(rates: &[TreasuryRate]) -> Vec<BondSpread> {
    rates
        .iter()
        .filter_map(|r| {
            let date = r.parsed_date()?;
            let y10 = r.year10?;
            let y2 = r.year2?;
            let y30 = r.year30.unwrap_or(y10);
            let m3 = r.month3.unwrap_or(y2);

            Some(BondSpread {
                date,
                spread_10y_2y: y10 - y2,
                curve_slope: y30 - m3,
            })
        })
        .collect()
}

/// Detect yield curve inversion dates (where 10Y < 2Y)
pub fn detect_inversions(rates: &[TreasuryRate]) -> Vec<NaiveDate> {
    rates
        .iter()
        .filter_map(|r| {
            let date = r.parsed_date()?;
            let y10 = r.year10?;
            let y2 = r.year2?;
            if y10 < y2 {
                Some(date)
            } else {
                None
            }
        })
        .collect()
}

/// Extract the yield curve for a specific date as ordered (maturity_label, rate) pairs
pub fn yield_curve_for_date(rate: &TreasuryRate) -> Vec<(&'static str, f64)> {
    let mut curve = Vec::new();
    if let Some(v) = rate.month1 { curve.push(("1M", v)); }
    if let Some(v) = rate.month2 { curve.push(("2M", v)); }
    if let Some(v) = rate.month3 { curve.push(("3M", v)); }
    if let Some(v) = rate.month6 { curve.push(("6M", v)); }
    if let Some(v) = rate.year1 { curve.push(("1Y", v)); }
    if let Some(v) = rate.year2 { curve.push(("2Y", v)); }
    if let Some(v) = rate.year3 { curve.push(("3Y", v)); }
    if let Some(v) = rate.year5 { curve.push(("5Y", v)); }
    if let Some(v) = rate.year7 { curve.push(("7Y", v)); }
    if let Some(v) = rate.year10 { curve.push(("10Y", v)); }
    if let Some(v) = rate.year20 { curve.push(("20Y", v)); }
    if let Some(v) = rate.year30 { curve.push(("30Y", v)); }
    curve
}

/// Compute correlation between spread changes and sector volatility changes
pub fn spread_vol_correlation(
    spreads: &[f64],
    volatilities: &[f64],
) -> f64 {
    let n = spreads.len().min(volatilities.len());
    if n < 3 {
        return 0.0;
    }

    // Compute changes (first differences)
    let spread_changes: Vec<f64> = spreads.windows(2).map(|w| w[1] - w[0]).collect();
    let vol_changes: Vec<f64> = volatilities.windows(2).map(|w| w[1] - w[0]).collect();

    let m = spread_changes.len().min(vol_changes.len());
    if m < 2 {
        return 0.0;
    }

    let mean_s = spread_changes[..m].iter().sum::<f64>() / m as f64;
    let mean_v = vol_changes[..m].iter().sum::<f64>() / m as f64;

    let mut cov = 0.0;
    let mut var_s = 0.0;
    let mut var_v = 0.0;

    for i in 0..m {
        let ds = spread_changes[i] - mean_s;
        let dv = vol_changes[i] - mean_v;
        cov += ds * dv;
        var_s += ds * ds;
        var_v += dv * dv;
    }

    let denom = (var_s * var_v).sqrt();
    if denom < 1e-15 { 0.0 } else { cov / denom }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rate(date: &str, y2: f64, y10: f64, y30: f64, m3: f64) -> TreasuryRate {
        TreasuryRate {
            date: date.to_string(),
            month1: None,
            month2: None,
            month3: Some(m3),
            month6: None,
            year1: None,
            year2: Some(y2),
            year3: None,
            year5: None,
            year7: None,
            year10: Some(y10),
            year20: None,
            year30: Some(y30),
        }
    }

    #[test]
    fn test_compute_term_spreads() {
        let rates = vec![
            make_rate("2025-01-01", 3.5, 4.2, 4.8, 3.6),
            make_rate("2025-01-02", 3.4, 4.1, 4.7, 3.5),
        ];
        let spreads = compute_term_spreads(&rates);
        assert_eq!(spreads.len(), 2);
        assert!((spreads[0].spread_10y_2y - 0.7).abs() < 1e-10);
        assert!((spreads[0].curve_slope - 1.2).abs() < 1e-10);
    }

    #[test]
    fn test_detect_inversions() {
        let rates = vec![
            make_rate("2025-01-01", 4.5, 4.2, 4.8, 3.6), // inverted: 10Y < 2Y
            make_rate("2025-01-02", 3.4, 4.1, 4.7, 3.5), // normal
        ];
        let inversions = detect_inversions(&rates);
        assert_eq!(inversions.len(), 1);
        assert_eq!(
            inversions[0],
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()
        );
    }

    #[test]
    fn test_yield_curve_for_date() {
        let rate = make_rate("2025-01-01", 3.5, 4.2, 4.8, 3.6);
        let curve = yield_curve_for_date(&rate);
        assert_eq!(curve.len(), 4); // m3, y2, y10, y30
        assert_eq!(curve[0].0, "3M");
    }

    #[test]
    fn test_spread_vol_correlation() {
        let spreads = vec![0.5, 0.6, 0.4, 0.7, 0.3, 0.8];
        let vols = vec![0.15, 0.16, 0.14, 0.17, 0.13, 0.18];
        let corr = spread_vol_correlation(&spreads, &vols);
        assert!(corr > 0.9, "Expected high positive correlation, got {}", corr);
    }
}

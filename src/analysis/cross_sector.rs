use crate::data::models::CorrelationMatrix;

/// Compute Pearson correlation between two equal-length slices
fn pearson_correlation(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len().min(b.len());
    if n < 2 {
        return 0.0;
    }

    let mean_a = a[..n].iter().sum::<f64>() / n as f64;
    let mean_b = b[..n].iter().sum::<f64>() / n as f64;

    let mut cov = 0.0;
    let mut var_a = 0.0;
    let mut var_b = 0.0;

    for i in 0..n {
        let da = a[i] - mean_a;
        let db = b[i] - mean_b;
        cov += da * db;
        var_a += da * da;
        var_b += db * db;
    }

    let denom = (var_a * var_b).sqrt();
    if denom < 1e-15 {
        0.0
    } else {
        cov / denom
    }
}

/// Compute pairwise Pearson correlation matrix for multiple return series
pub fn compute_correlation_matrix(
    symbols: &[String],
    returns: &[Vec<f64>],
) -> CorrelationMatrix {
    let n = symbols.len();
    let mut matrix = vec![vec![0.0; n]; n];

    // Align all series to the same length (shortest)
    let min_len = returns.iter().map(|r| r.len()).min().unwrap_or(0);
    if min_len < 2 {
        return CorrelationMatrix {
            symbols: symbols.to_vec(),
            matrix,
        };
    }

    let aligned: Vec<&[f64]> = returns
        .iter()
        .map(|r| &r[r.len() - min_len..])
        .collect();

    for i in 0..n {
        matrix[i][i] = 1.0;
        for j in (i + 1)..n {
            let corr = pearson_correlation(aligned[i], aligned[j]);
            matrix[i][j] = corr;
            matrix[j][i] = corr;
        }
    }

    CorrelationMatrix {
        symbols: symbols.to_vec(),
        matrix,
    }
}


/// Compute average cross-sector correlation from a correlation matrix
pub fn average_cross_correlation(matrix: &CorrelationMatrix) -> f64 {
    let n = matrix.symbols.len();
    if n < 2 {
        return 0.0;
    }
    let mut sum = 0.0;
    let mut count = 0;
    for i in 0..n {
        for j in (i + 1)..n {
            sum += matrix.matrix[i][j];
            count += 1;
        }
    }
    if count == 0 { 0.0 } else { sum / count as f64 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pearson_perfect_positive() {
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let b = vec![2.0, 4.0, 6.0, 8.0, 10.0];
        let corr = pearson_correlation(&a, &b);
        assert!((corr - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_pearson_perfect_negative() {
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let b = vec![10.0, 8.0, 6.0, 4.0, 2.0];
        let corr = pearson_correlation(&a, &b);
        assert!((corr + 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_correlation_matrix_diagonal() {
        let symbols = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let returns = vec![
            vec![0.01, -0.02, 0.03, 0.01, -0.01],
            vec![0.02, -0.01, 0.02, 0.015, -0.005],
            vec![-0.01, 0.03, -0.02, 0.005, 0.01],
        ];
        let cm = compute_correlation_matrix(&symbols, &returns);
        for i in 0..3 {
            assert!((cm.matrix[i][i] - 1.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_correlation_matrix_symmetric() {
        let symbols = vec!["A".to_string(), "B".to_string()];
        let returns = vec![
            vec![0.01, -0.02, 0.03, 0.01],
            vec![0.02, -0.01, 0.02, 0.015],
        ];
        let cm = compute_correlation_matrix(&symbols, &returns);
        assert!((cm.matrix[0][1] - cm.matrix[1][0]).abs() < 1e-10);
    }

    #[test]
    fn test_average_cross_correlation() {
        let cm = CorrelationMatrix {
            symbols: vec!["A".to_string(), "B".to_string(), "C".to_string()],
            matrix: vec![
                vec![1.0, 0.8, 0.6],
                vec![0.8, 1.0, 0.7],
                vec![0.6, 0.7, 1.0],
            ],
        };
        let avg = average_cross_correlation(&cm);
        let expected = (0.8 + 0.6 + 0.7) / 3.0;
        assert!((avg - expected).abs() < 1e-10);
    }
}

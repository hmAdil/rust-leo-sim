/// Hungarian algorithm for optimal assignment (minimum weight bipartite matching).
/// Used for OSPA metric computation.
/// 
/// This is a simple O(n³) implementation suitable for small to medium-sized problems.

/// Result of the Hungarian algorithm: pairs of (row, col) indices for optimal assignment
pub type Assignment = Vec<(usize, usize)>;

/// Solve the assignment problem using the Hungarian algorithm.
/// 
/// Given a cost matrix, finds the minimum cost assignment.
/// Returns pairs of (row_index, col_index) for matched elements.
pub fn solve(cost_matrix: &[Vec<f64>]) -> Assignment {
    if cost_matrix.is_empty() {
        return Vec::new();
    }
    
    let n = cost_matrix.len();
    let m = cost_matrix[0].len();
    
    if n == 0 || m == 0 {
        return Vec::new();
    }
    
    // For rectangular matrices, we pad to make it square
    let size = n.max(m);
    let mut matrix = vec![vec![f64::INFINITY; size]; size];
    for i in 0..n {
        for j in 0..m {
            matrix[i][j] = cost_matrix[i][j];
        }
    }
    
    // Hungarian algorithm implementation
    let mut u = vec![0.0; size + 1];
    let mut v = vec![0.0; size + 1];
    let mut p = vec![0; size + 1];
    let mut way = vec![0; size + 1];
    
    for i in 1..=size {
        p[0] = i;
        let mut j0 = 0;
        let mut minv = vec![f64::INFINITY; size + 1];
        let mut used = vec![false; size + 1];
        let mut iters = 0usize;
        loop {
            iters += 1;
            if iters > size + 2 { break; }
            used[j0] = true;
            let i0 = p[j0];
            let mut delta = f64::INFINITY;
            let mut j1 = 0;

            for j in 1..=size {
                if !used[j] {
                    let cur = matrix[i0 - 1][j - 1] - u[i0] - v[j];
                    if cur < minv[j] {
                        minv[j] = cur;
                        way[j] = j0;
                    }
                    if minv[j] < delta {
                        delta = minv[j];
                        j1 = j;
                    }
                }
            }

            // Safety check for infinite loop due to NaN from INF-INF
            if j1 == 0 || delta == f64::INFINITY {
                break;
            }

            for j in 0..=size {
                if used[j] {
                    u[p[j]] += delta;
                    v[j] -= delta;
                } else {
                    minv[j] -= delta;
                }
            }

            j0 = j1;

            if p[j0] == 0 {
                break;
            }
        }

        // Augmenting path
        while j0 != 0 {
            let j1 = way[j0];
            p[j0] = p[j1];
            j0 = j1;
        }
    }
    
    // Extract assignment
    let mut result = Vec::new();
    for j in 1..=size {
        if p[j] != 0 && p[j] <= n && j <= m {
            result.push((p[j] - 1, j - 1));
        }
    }
    
    result
}

/// Compute OSPA distance between predicted and ground truth positions.
/// 
/// OSPA = (1/max(|P|,|G|) * (sum of min(d, cutoff)^p for matched pairs + cutoff^p * |unmatched|))^(1/p)
/// 
/// # Arguments
/// * `predicted` - Vector of predicted positions
/// * `ground_truth` - Vector of ground truth positions  
/// * `cutoff` - Maximum distance considered (default 100.0 km)
/// * `p` - Order of the metric (default 2)
pub fn compute_ospa(predicted: &[[f64; 3]], ground_truth: &[[f64; 3]], cutoff: f64, p: f64) -> f64 {
    if predicted.is_empty() && ground_truth.is_empty() {
        return 0.0;
    }
    
    let n = predicted.len();
    let m = ground_truth.len();
    
    if n == 0 || m == 0 {
        // All unmatched
        return cutoff;
    }
    
    // Build cost matrix with distances
    let mut cost_matrix = vec![vec![0.0; m]; n];
    for i in 0..n {
        for j in 0..m {
            let dx = predicted[i][0] - ground_truth[j][0];
            let dy = predicted[i][1] - ground_truth[j][1];
            let dz = predicted[i][2] - ground_truth[j][2];
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();
            cost_matrix[i][j] = (dist.min(cutoff)).powf(p);
        }
    }
    
    // Solve assignment
    let assignment = solve(&cost_matrix);
    
    // Count matched pairs and sum costs
    let mut total_cost = 0.0;
    for (i, j) in &assignment {
        if *i < n && *j < m {
            total_cost += cost_matrix[*i][*j];
        }
    }
    
    // Add penalty for unmatched
    let unmatched = (n as isize - m as isize).abs() as usize;
    total_cost += unmatched as f64 * cutoff.powf(p);
    
    // Normalize and take p-th root
    let norm = (n.max(m) as f64).max(1.0);
    (total_cost / norm).powf(1.0 / p)
}
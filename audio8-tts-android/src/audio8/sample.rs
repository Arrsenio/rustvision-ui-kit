//! DualAR nucleus / Gumbel sampling ported from `arktts_runtime.runtime._sample`.

use rand::Rng;

pub fn sample_token(
    logits: &[f32],
    temperature: f32,
    top_p: f32,
    top_k: usize,
    greedy: bool,
    rng: &mut impl Rng,
) -> usize {
    if logits.is_empty() {
        return 0;
    }
    if greedy {
        return logits
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(i, _)| i)
            .unwrap_or(0);
    }

    let mut order: Vec<usize> = (0..logits.len()).collect();
    order.sort_by(|&a, &b| logits[b].total_cmp(&logits[a]));
    let mut sorted: Vec<f64> = order.iter().map(|&i| logits[i] as f64).collect();
    let max = sorted.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let mut base: Vec<f64> = sorted.iter().map(|v| (*v - max).exp()).collect();
    let sum: f64 = base.iter().sum();
    if sum > 0.0 {
        for v in &mut base {
            *v /= sum;
        }
    }
    let mut cumulative = 0.0;
    let k = top_k.max(1);
    let mut remove = vec![false; base.len()];
    for i in 0..base.len() {
        cumulative += base[i];
        remove[i] = cumulative > top_p as f64 || i >= k;
    }
    if !remove.is_empty() {
        remove[0] = false;
    }

    let mut masked: Vec<f64> = logits.iter().map(|&v| v as f64).collect();
    for (rank, &idx) in order.iter().enumerate() {
        if remove[rank] {
            masked[idx] = f64::NEG_INFINITY;
        }
    }
    let temp = temperature.max(1e-5) as f64;
    for v in &mut masked {
        *v /= temp;
    }
    let max = masked.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let mut probs: Vec<f64> = masked.iter().map(|v| (*v - max).exp()).collect();
    let sum: f64 = probs.iter().sum::<f64>().max(1e-12);
    for p in &mut probs {
        *p /= sum;
    }
    // Gumbel-max as in the Python runtime: argmax(probs / -log(U)).
    let mut best = 0usize;
    let mut best_score = f64::NEG_INFINITY;
    for (i, p) in probs.iter().enumerate() {
        let u = rng.gen::<f64>().clamp(1e-12, 1.0);
        let score = *p / -u.ln();
        if score > best_score {
            best_score = score;
            best = i;
        }
    }
    best
}

pub fn sample_semantic(
    logits: &[f32],
    allowed_ids: &[i64],
    previous: &[i64],
    begin: i64,
    end: i64,
    temperature: f32,
    top_p: f32,
    top_k: usize,
    greedy: bool,
    rng: &mut impl Rng,
) -> i64 {
    if allowed_ids.is_empty() {
        return begin;
    }
    let allowed_logits: Vec<f32> = if logits.len() == allowed_ids.len() {
        logits.to_vec()
    } else {
        allowed_ids
            .iter()
            .map(|&id| logits.get(id as usize).copied().unwrap_or(f32::NEG_INFINITY))
            .collect()
    };
    let normal_index = sample_token(&allowed_logits, temperature, top_p, top_k, greedy, rng);
    let normal = allowed_ids[normal_index.min(allowed_ids.len() - 1)];
    if greedy {
        return normal;
    }
    let high_index = sample_token(&allowed_logits, 1.0, 0.9, top_k, false, rng);
    let high = allowed_ids[high_index.min(allowed_ids.len() - 1)];
    if (begin..=end).contains(&normal) && previous.contains(&normal) {
        high
    } else {
        normal
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn greedy_picks_argmax() {
        let mut rng = StdRng::seed_from_u64(1);
        assert_eq!(
            sample_token(&[0.1, 3.0, 0.2], 0.8, 0.9, 50, true, &mut rng),
            1
        );
    }

    #[test]
    fn sampling_is_seed_stable() {
        let logits = [0.1f32, 2.0, 0.3, 0.05];
        let mut a = StdRng::seed_from_u64(42);
        let mut b = StdRng::seed_from_u64(42);
        assert_eq!(
            sample_token(&logits, 0.7, 0.9, 50, false, &mut a),
            sample_token(&logits, 0.7, 0.9, 50, false, &mut b)
        );
    }
}

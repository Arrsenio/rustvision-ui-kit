//! DualAR generate loop in Rust (Slow AR → Fast AR codebooks → codec decode).
//!
//! Sessions are a trait so ONNX Runtime (`ort`) or a test double can drive the
//! same algorithm the Python `ArkTtsRuntime` uses.

use super::npy::{CodecCodes, CODEBOOK_SIZE, NUM_CODEBOOKS};
use super::params::SynthParams;
use super::sample::{sample_semantic, sample_token};
use super::text::system_prompt_parts;
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct DualArManifest {
    pub sample_rate: u32,
    pub num_layers: usize,
    pub num_fast_layers: usize,
    pub n_local_heads: usize,
    pub fast_n_local_heads: usize,
    pub head_dim: usize,
    pub fast_head_dim: usize,
    pub max_seq_len: usize,
    pub num_codebooks: usize,
    pub codebook_size: i64,
    pub semantic_begin_id: i64,
    pub semantic_end_id: i64,
    pub im_end_id: i64,
    pub codec_hop_length: usize,
    pub slow_logits_layout_semantic_then_eos: bool,
}

impl Default for DualArManifest {
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            num_layers: 24,
            num_fast_layers: 4,
            n_local_heads: 2,
            fast_n_local_heads: 2,
            head_dim: 64,
            fast_head_dim: 64,
            max_seq_len: 2048,
            num_codebooks: NUM_CODEBOOKS,
            codebook_size: CODEBOOK_SIZE,
            semantic_begin_id: 0,
            semantic_end_id: 4095,
            im_end_id: 2,
            codec_hop_length: 2048,
            slow_logits_layout_semantic_then_eos: true,
        }
    }
}

pub trait TokenEncoder {
    fn encode(&self, text: &str) -> Result<Vec<i64>, String>;
}

pub trait DualArBackend {
    fn slow_step(
        &mut self,
        codes: &[i64],
        positions: &[i64],
        cache_keys: &mut [Vec<f32>],
        cache_values: &mut [Vec<f32>],
    ) -> Result<(Vec<f32>, Vec<f32>), String>;
    fn fast_step(
        &mut self,
        slow_hidden: &[f32],
        token_id: i64,
        use_slow_hidden: bool,
        position: i64,
        cache_keys: &mut [Vec<f32>],
        cache_values: &mut [Vec<f32>],
    ) -> Result<Vec<f32>, String>;
    fn decode_codes(&mut self, codes: &CodecCodes) -> Result<Vec<f32>, String>;
}

/// Pack DualAR prompt tensor `[1, num_codebooks+1, T]` like `PromptBuilder.build`.
pub fn build_prompt(
    encoder: &dyn TokenEncoder,
    target_text: &str,
    reference_text: &str,
    reference_codes: &CodecCodes,
    semantic_begin_id: i64,
) -> Result<Vec<Vec<i64>>, String> {
    let [prefix_s, suffix_s] = system_prompt_parts(reference_text, target_text)?;
    let prefix = encoder.encode(&prefix_s)?;
    let suffix = encoder.encode(&suffix_s)?;
    let semantic: Vec<i64> = (0..reference_codes.frames)
        .map(|t| reference_codes.at(0, t) + semantic_begin_id)
        .collect();
    let t = prefix.len() + semantic.len() + suffix.len();
    let rows = reference_codes.rows + 1;
    let mut values = vec![vec![0i64; t]; rows];
    let mut row0 = prefix.clone();
    row0.extend_from_slice(&semantic);
    row0.extend_from_slice(&suffix);
    values[0] = row0;
    let begin = prefix.len();
    for r in 0..reference_codes.rows {
        for f in 0..reference_codes.frames {
            values[r + 1][begin + f] = reference_codes.at(r, f);
        }
    }
    Ok(values)
}

pub fn generate_codes(
    backend: &mut dyn DualArBackend,
    manifest: &DualArManifest,
    prompt: &[Vec<i64>],
    params: &SynthParams,
    cancel: Arc<AtomicBool>,
    mut on_frame: impl FnMut(&[i64], usize),
) -> Result<CodecCodes, String> {
    let prompt_len = prompt.first().map(|r| r.len()).unwrap_or(0);
    if prompt_len == 0 {
        return Err("empty prompt".into());
    }
    if prompt_len >= manifest.max_seq_len {
        return Err(format!(
            "prompt length {prompt_len} exceeds max sequence length {}",
            manifest.max_seq_len
        ));
    }
    let max_new = (params.max_new_tokens as usize).min(manifest.max_seq_len - prompt_len);
    let mut rng = StdRng::seed_from_u64(params.seed as u64);
    let mut slow_k = vec![Vec::new(); manifest.num_layers];
    let mut slow_v = vec![Vec::new(); manifest.num_layers];
    let mut codes_feed = flatten_prompt(prompt);
    let positions: Vec<i64> = (0..prompt_len as i64).collect();
    let (mut logits, mut hidden) =
        backend.slow_step(&codes_feed, &positions, &mut slow_k, &mut slow_v)?;

    let mut allowed: Vec<i64> = (manifest.semantic_begin_id..=manifest.semantic_end_id).collect();
    allowed.push(manifest.im_end_id);
    let mut previous: Vec<i64> = Vec::new();
    let mut frames: Vec<Vec<i64>> = Vec::new();

    for step in 0..max_new {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let allowed_logits: Vec<f32> = if manifest.slow_logits_layout_semantic_then_eos
            && logits.len() == allowed.len()
        {
            logits.clone()
        } else {
            allowed
                .iter()
                .map(|&id| logits.get(id as usize).copied().unwrap_or(f32::NEG_INFINITY))
                .collect()
        };
        let semantic = sample_semantic(
            &allowed_logits,
            &allowed,
            &previous,
            manifest.semantic_begin_id,
            manifest.semantic_end_id,
            params.temperature,
            params.top_p,
            params.top_k.max(1) as usize,
            params.greedy,
            &mut rng,
        );
        if semantic == manifest.im_end_id {
            break;
        }
        previous.push(semantic);
        if previous.len() > 10 {
            previous.remove(0);
        }
        let mut fast_k = vec![Vec::new(); manifest.num_fast_layers];
        let mut fast_v = vec![Vec::new(); manifest.num_fast_layers];
        backend.fast_step(&hidden, 0, true, 0, &mut fast_k, &mut fast_v)?;
        let mut token = (semantic - manifest.semantic_begin_id).clamp(0, manifest.codebook_size - 1);
        let mut codebooks = vec![token];
        for fast_pos in 1..manifest.num_codebooks as i64 {
            let fast_logits =
                backend.fast_step(&hidden, token, false, fast_pos, &mut fast_k, &mut fast_v)?;
            token = sample_token(
                &fast_logits,
                params.temperature,
                params.top_p,
                params.top_k.max(1) as usize,
                params.greedy,
                &mut rng,
            ) as i64;
            token = token.clamp(0, manifest.codebook_size - 1);
            codebooks.push(token);
        }
        on_frame(&codebooks, step);
        frames.push(codebooks);
        if step + 1 >= max_new {
            break;
        }
        let mut column = vec![semantic];
        column.extend_from_slice(frames.last().unwrap());
        codes_feed = column;
        let pos = [prompt_len as i64 + step as i64];
        let step_out = backend.slow_step(&codes_feed, &pos, &mut slow_k, &mut slow_v)?;
        logits = step_out.0;
        hidden = step_out.1;
    }

    if frames.is_empty() {
        return Err("model produced no codec frames".into());
    }
    let t = frames.len();
    let rows = frames[0].len();
    let mut values = vec![0i64; rows * t];
    for (f, frame) in frames.iter().enumerate() {
        for (r, v) in frame.iter().enumerate() {
            values[r * t + f] = *v;
        }
    }
    CodecCodes::new(rows, t, values)
}

fn flatten_prompt(prompt: &[Vec<i64>]) -> Vec<i64> {
    // runtime feeds `codes` as the full packed tensor; backends may reshape.
    prompt.iter().flatten().copied().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio8::npy::CodecCodes;

    struct DummyEnc;
    impl TokenEncoder for DummyEnc {
        fn encode(&self, text: &str) -> Result<Vec<i64>, String> {
            Ok(text.chars().map(|c| c as u32 as i64 % 100).collect())
        }
    }

    #[test]
    fn prompt_layout_has_codebook_rows() {
        let codes = CodecCodes::new(10, 4, (0..40).map(|i| i % 100).collect()).unwrap();
        let packed = build_prompt(&DummyEnc, "hello", "ref text", &codes, 100).unwrap();
        assert_eq!(packed.len(), 11);
        assert!(packed[0].len() > 4);
    }
}

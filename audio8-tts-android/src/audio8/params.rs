//! Every Audio8 inference / serving / SFT knob the apps and CLIs expose.

use serde::{Deserialize, Serialize};

pub const SAMPLE_RATE: u32 = 44100;
pub const CODEC_HOP: u32 = 2048;
pub const MAX_SEQ_LEN: u32 = 2048;
pub const NUM_CODEBOOKS: usize = 10;
pub const CODEBOOK_SIZE: i64 = 4096;
pub const MAX_TEXT_CHARS: usize = 1000;
pub const MAX_AUDIO_BYTES: usize = 50 * 1024 * 1024;
pub const MIN_REF_SECS: f32 = 0.5;
pub const MAX_REF_SECS: f32 = 30.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineKind {
    OnnxHttp,
    Sglang,
    LocalOnnx,
}

impl EngineKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::OnnxHttp => "ONNX HTTP",
            Self::Sglang => "SGLANG",
            Self::LocalOnnx => "LOCAL DUALAR",
        }
    }
    pub fn all() -> [Self; 3] {
        [Self::OnnxHttp, Self::Sglang, Self::LocalOnnx]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResponseFormat {
    Wav,
    Pcm,
    Codes,
}

impl ResponseFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Wav => "wav",
            Self::Pcm => "pcm",
            Self::Codes => "codes",
        }
    }
    pub fn all() -> [Self; 3] {
        [Self::Wav, Self::Pcm, Self::Codes]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DType {
    Auto,
    BFloat16,
    Float16,
    Float32,
}

impl DType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::BFloat16 => "bfloat16",
            Self::Float16 => "float16",
            Self::Float32 => "float32",
        }
    }
    pub fn all() -> [Self; 4] {
        [Self::Auto, Self::BFloat16, Self::Float16, Self::Float32]
    }
}

#[derive(Clone, Debug)]
pub struct Reference {
    pub audio_path: String,
    pub text: String,
    pub audio: Option<Vec<u8>>,
}

#[derive(Clone, Debug)]
pub struct SynthParams {
    pub text: String,
    pub voice: String,
    pub language: String,
    pub max_new_tokens: i32,
    pub retry_max_new_tokens: i32,
    pub temperature: f32,
    pub top_p: f32,
    pub top_k: i32,
    pub seed: i64,
    pub greedy: bool,
    pub save_codes: bool,
    pub overwrite: bool,
    pub stream: bool,
    pub chunk_frames: i32,
    pub stream_context_frames: i32,
    pub stream_guard_frames: i32,
    pub response_format: ResponseFormat,
    pub model: String,
    pub reference_text: String,
    pub reference_audio_name: String,
    pub reference_audio: Option<Vec<u8>>,
    pub reference_path: String,
    pub sample_id: String,
}

impl Default for SynthParams {
    fn default() -> Self {
        Self {
            text: "Welcome to Audio8 TTS.".into(),
            voice: String::new(),
            language: "AUTO".into(),
            max_new_tokens: 1024,
            retry_max_new_tokens: 2000,
            temperature: 0.8,
            top_p: 0.95,
            top_k: 50,
            seed: 42,
            greedy: false,
            save_codes: true,
            overwrite: false,
            stream: false,
            chunk_frames: 12,
            stream_context_frames: 128,
            stream_guard_frames: 1,
            response_format: ResponseFormat::Wav,
            model: "audio8/tts-0.6b".into(),
            reference_text: String::new(),
            reference_audio_name: String::new(),
            reference_audio: None,
            reference_path: String::new(),
            sample_id: "utt_001".into(),
        }
    }
}

impl SynthParams {
    pub fn validate(&self) -> Result<(), String> {
        if self.text.trim().is_empty() {
            return Err("text must not be empty".into());
        }
        if self.text.chars().count() > MAX_TEXT_CHARS {
            return Err(format!("text longer than {MAX_TEXT_CHARS} characters"));
        }
        if self.max_new_tokens < 1 {
            return Err("--max-new-tokens must be positive".into());
        }
        if self.retry_max_new_tokens < self.max_new_tokens {
            return Err("--retry-max-new-tokens must be >= --max-new-tokens".into());
        }
        if self.temperature <= 0.0 {
            return Err("--temperature must be positive".into());
        }
        if !(self.top_p > 0.0 && self.top_p <= 1.0) {
            return Err("--top-p must be in (0, 1]".into());
        }
        if self.top_k < 0 {
            return Err("--top-k must be non-negative".into());
        }
        let has_audio = self.reference_audio.is_some() || !self.reference_path.trim().is_empty();
        let has_text = !self.reference_text.trim().is_empty();
        if has_audio != has_text {
            return Err("reference_audio and reference_text must be provided together".into());
        }
        Ok(())
    }

    pub fn do_sample(&self) -> bool {
        !self.greedy
    }
}

#[derive(Clone, Debug)]
pub struct EngineConfig {
    pub kind: EngineKind,
    pub base_url: String,
    pub timeout_secs: u64,
    pub model_dir: String,
    pub voices_dir: String,
    pub registration_dir: String,
    pub precision: String,
    pub codec_precision: String,
    pub threads: i32,
    pub device: String,
    pub dtype: DType,
    pub model_id: String,
    pub hf_model: String,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            kind: EngineKind::OnnxHttp,
            base_url: "http://10.0.2.2:8024".into(),
            timeout_secs: 180,
            model_dir: "model".into(),
            voices_dir: "voices".into(),
            registration_dir: "model/registration".into(),
            precision: "int4".into(),
            codec_precision: "fp16".into(),
            threads: 5,
            device: "auto".into(),
            dtype: DType::Auto,
            model_id: "audio8/tts-0.6b".into(),
            hf_model: "Audio8/Audio8-TTS-Preview-0.6b".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SftConfig {
    pub model_name_or_path: String,
    pub train_jsonl: String,
    pub eval_jsonl: String,
    pub output_dir: String,
    pub export_dir: String,
    pub max_length: u32,
    pub freeze_slow_ar: bool,
    pub freeze_fast_ar: bool,
    pub batch_size: u32,
    pub eval_batch_size: u32,
    pub gradient_accumulation_steps: u32,
    pub learning_rate: f64,
    pub num_train_epochs: f32,
    pub warmup_ratio: f32,
    pub lr_scheduler_type: String,
    pub weight_decay: f32,
    pub max_grad_norm: f32,
    pub logging_steps: u32,
    pub save_steps: u32,
    pub save_total_limit: u32,
    pub dataloader_num_workers: u32,
    pub bf16: bool,
    pub gradient_checkpointing: bool,
    pub resume_mode: String,
    pub slow_loss_weight: f32,
    pub fast_loss_weight: f32,
    pub nnodes: u32,
    pub node_rank: u32,
    pub nproc_per_node: u32,
    pub master_addr: String,
    pub master_port: u32,
    pub report_to: String,
}

impl Default for SftConfig {
    fn default() -> Self {
        Self {
            model_name_or_path: "model/audio8_tts_0_6B_preview".into(),
            train_jsonl: String::new(),
            eval_jsonl: String::new(),
            output_dir: "outputs/audio8_tts_sft".into(),
            export_dir: "outputs/audio8_tts_sft/export".into(),
            max_length: 2048,
            freeze_slow_ar: false,
            freeze_fast_ar: false,
            batch_size: 2,
            eval_batch_size: 2,
            gradient_accumulation_steps: 8,
            learning_rate: 1e-5,
            num_train_epochs: 1.0,
            warmup_ratio: 0.01,
            lr_scheduler_type: "cosine".into(),
            weight_decay: 0.0,
            max_grad_norm: 1.0,
            logging_steps: 10,
            save_steps: 500,
            save_total_limit: 3,
            dataloader_num_workers: 0,
            bf16: true,
            gradient_checkpointing: true,
            resume_mode: "none".into(),
            slow_loss_weight: 1.0,
            fast_loss_weight: 1.0,
            nnodes: 1,
            node_rank: 0,
            nproc_per_node: 1,
            master_addr: "127.0.0.1".into(),
            master_port: 29500,
            report_to: "tensorboard".into(),
        }
    }
}

impl SftConfig {
    pub fn to_pretty_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".into())
    }

    pub fn to_shell(&self) -> String {
        format!(
            "TRAIN_JSONL={train} \\\nEVAL_JSONL={eval} \\\nMODEL={model} \\\nOUTPUT_DIR={out} \\\nEXPORT_DIR={export} \\\nMAX_LENGTH={max_len} \\\nBATCH_SIZE={bs} \\\nEVAL_BATCH_SIZE={ebs} \\\nGRADIENT_ACCUMULATION_STEPS={ga} \\\nLEARNING_RATE={lr} \\\nNUM_TRAIN_EPOCHS={ep} \\\nWARMUP_RATIO={wr} \\\nLR_SCHEDULER_TYPE={sched} \\\nWEIGHT_DECAY={wd} \\\nMAX_GRAD_NORM={gn} \\\nLOGGING_STEPS={ls} \\\nSAVE_STEPS={ss} \\\nSAVE_TOTAL_LIMIT={stl} \\\nDATALOADER_NUM_WORKERS={dw} \\\nBF16={bf} \\\nGRADIENT_CHECKPOINTING={gc} \\\nFREEZE_SLOW_AR={fs} \\\nFREEZE_FAST_AR={ff} \\\nRESUME_MODE={rm} \\\nREPORT_TO={rt} \\\nNNODES={nn} \\\nNODE_RANK={nr} \\\nNPROC_PER_NODE={np} \\\nMASTER_ADDR={ma} \\\nMASTER_PORT={mp} \\\nbash audio8_tts_sft.sh",
            train = shell(&self.train_jsonl),
            eval = shell(&self.eval_jsonl),
            model = shell(&self.model_name_or_path),
            out = shell(&self.output_dir),
            export = shell(&self.export_dir),
            max_len = self.max_length,
            bs = self.batch_size,
            ebs = self.eval_batch_size,
            ga = self.gradient_accumulation_steps,
            lr = self.learning_rate,
            ep = self.num_train_epochs,
            wr = self.warmup_ratio,
            sched = self.lr_scheduler_type,
            wd = self.weight_decay,
            gn = self.max_grad_norm,
            ls = self.logging_steps,
            ss = self.save_steps,
            stl = self.save_total_limit,
            dw = self.dataloader_num_workers,
            bf = self.bf16,
            gc = self.gradient_checkpointing,
            fs = self.freeze_slow_ar,
            ff = self.freeze_fast_ar,
            rm = self.resume_mode,
            rt = self.report_to,
            nn = self.nnodes,
            nr = self.node_rank,
            np = self.nproc_per_node,
            ma = self.master_addr,
            mp = self.master_port,
        )
    }
}

fn shell(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

#[derive(Clone, Debug, Default)]
pub struct VoiceMeta {
    pub name: String,
    pub reference_text: String,
    pub frames: usize,
    pub extra: String,
}

#[derive(Clone, Debug)]
pub struct HealthInfo {
    pub ok: bool,
    pub precision: String,
    pub codec_precision: String,
    pub provider: String,
    pub raw: String,
}

#[derive(Clone, Debug, Default)]
pub struct SystemInfo {
    pub current_mb: Option<f64>,
    pub peak_mb: Option<f64>,
    pub uptime_seconds: f64,
}

#[derive(Clone, Debug)]
pub struct SynthResult {
    pub wav: Vec<u8>,
    pub pcm: Option<Vec<u8>>,
    pub codes: Option<crate::audio8::npy::CodecCodes>,
    pub sample_rate: u32,
    pub finished: bool,
    pub code_frames: usize,
    pub elapsed_secs: f32,
}

#[derive(Clone, Debug)]
pub struct BatchRow {
    pub id: String,
    pub text: String,
    pub voice: String,
    pub reference_text: String,
    pub reference_path: String,
}

impl Default for BatchRow {
    fn default() -> Self {
        Self {
            id: "sample_001".into(),
            text: String::new(),
            voice: String::new(),
            reference_text: String::new(),
            reference_path: String::new(),
        }
    }
}

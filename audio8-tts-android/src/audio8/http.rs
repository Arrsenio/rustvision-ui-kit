//! Complete Audio8 HTTP surface: ONNX Runtime (`:8024`) and SGLang Omni (`:8010`).

use super::npy::{load_npy_codes, CodecCodes};
use super::params::{
    EngineConfig, EngineKind, HealthInfo, ResponseFormat, SynthParams, SynthResult, SystemInfo,
    VoiceMeta,
};
use super::wav::{looks_like_wav, WavClip};
use serde::Serialize;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct HttpEngine {
    pub cfg: EngineConfig,
}

#[derive(Serialize)]
struct OnnxTtsBody<'a> {
    text: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    voice_name: Option<&'a str>,
    max_new_tokens: i32,
    temperature: f32,
    top_p: f32,
    top_k: i32,
    seed: i64,
}

impl HttpEngine {
    pub fn new(cfg: EngineConfig) -> Self {
        Self { cfg }
    }

    fn agent(&self) -> ureq::Agent {
        ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(self.cfg.timeout_secs.max(5)))
            .build()
    }

    fn url(&self, path: &str) -> String {
        let base = self.cfg.base_url.trim().trim_end_matches('/');
        if path.starts_with('/') {
            format!("{base}{path}")
        } else {
            format!("{base}/{path}")
        }
    }

    pub fn health(&self) -> Result<HealthInfo, String> {
        let resp = self
            .agent()
            .get(&self.url("/api/health"))
            .call()
            .map_err(|e| format!("HEALTH: {e}"))?;
        let text = resp.into_string().map_err(|e| e.to_string())?;
        let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        Ok(HealthInfo {
            ok: v.get("ok").and_then(|x| x.as_bool()).unwrap_or(true),
            precision: v
                .get("precision")
                .and_then(|x| x.as_str())
                .unwrap_or("-")
                .to_string(),
            codec_precision: v
                .get("codec_precision")
                .and_then(|x| x.as_str())
                .unwrap_or("-")
                .to_string(),
            provider: v
                .pointer("/providers/slow/0")
                .and_then(|x| x.as_str())
                .unwrap_or("-")
                .to_string(),
            raw: text,
        })
    }

    pub fn system(&self) -> Result<SystemInfo, String> {
        let text = self
            .agent()
            .get(&self.url("/api/system"))
            .call()
            .map_err(|e| format!("SYSTEM: {e}"))?
            .into_string()
            .map_err(|e| e.to_string())?;
        let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        Ok(SystemInfo {
            current_mb: v.pointer("/memory/current_mb").and_then(|x| x.as_f64()),
            peak_mb: v.pointer("/memory/peak_mb").and_then(|x| x.as_f64()),
            uptime_seconds: v
                .get("uptime_seconds")
                .and_then(|x| x.as_f64())
                .unwrap_or(0.0),
        })
    }

    pub fn registration_status(&self) -> Result<(bool, String), String> {
        let text = self
            .agent()
            .get(&self.url("/api/registration/status"))
            .call()
            .map_err(|e| format!("REG STATUS: {e}"))?
            .into_string()
            .map_err(|e| e.to_string())?;
        let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        Ok((
            v.get("available").and_then(|x| x.as_bool()).unwrap_or(false),
            v.get("reason")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
        ))
    }

    pub fn reload(&self) -> Result<String, String> {
        let text = self
            .agent()
            .post(&self.url("/api/runtime/reload"))
            .call()
            .map_err(|e| format!("RELOAD: {e}"))?
            .into_string()
            .map_err(|e| e.to_string())?;
        Ok(text)
    }

    pub fn cancel(&self) -> Result<String, String> {
        let text = self
            .agent()
            .post(&self.url("/api/tts/cancel"))
            .call()
            .map_err(|e| format!("CANCEL: {e}"))?
            .into_string()
            .unwrap_or_default();
        Ok(text)
    }

    pub fn list_voices(&self) -> Result<Vec<VoiceMeta>, String> {
        let text = self
            .agent()
            .get(&self.url("/api/voices"))
            .call()
            .map_err(|e| format!("VOICES: {e}"))?
            .into_string()
            .map_err(|e| e.to_string())?;
        parse_voices(&text)
    }

    pub fn register_voice(
        &self,
        name: &str,
        transcript: &str,
        wav: &[u8],
        filename: &str,
        overwrite: bool,
    ) -> Result<VoiceMeta, String> {
        if name.trim().is_empty() {
            return Err("voice name must not be empty".into());
        }
        if transcript.trim().is_empty() {
            return Err("reference text must not be empty".into());
        }
        if wav.is_empty() {
            return Err("NO REFERENCE AUDIO".into());
        }
        let boundary = "----Audio8TtsBoundary7MA4YWxkTrZu0gW";
        let mut body = Vec::new();
        write_field(&mut body, boundary, "text", transcript.as_bytes(), None);
        write_field(&mut body, boundary, "name", name.trim().as_bytes(), None);
        write_field(
            &mut body,
            boundary,
            "overwrite",
            if overwrite { b"true" } else { b"false" },
            None,
        );
        let fname = if filename.is_empty() {
            "reference.wav"
        } else {
            filename
        };
        write_field(
            &mut body,
            boundary,
            "audio",
            wav,
            Some((fname, "application/octet-stream")),
        );
        body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
        let resp = self
            .agent()
            .post(&self.url("/api/voices/register"))
            .set(
                "Content-Type",
                &format!("multipart/form-data; boundary={boundary}"),
            )
            .send_bytes(&body)
            .map_err(|e| format!("REGISTER: {e}"))?;
        let status = resp.status();
        let text = resp.into_string().unwrap_or_default();
        if !(200..300).contains(&status) {
            return Err(format!("REGISTER HTTP {status}: {}", trunc(&text, 200)));
        }
        let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        let voice = v.get("voice").cloned().unwrap_or(v);
        Ok(VoiceMeta {
            name: voice
                .get("name")
                .and_then(|x| x.as_str())
                .unwrap_or(name)
                .to_string(),
            reference_text: voice
                .get("reference_text")
                .and_then(|x| x.as_str())
                .unwrap_or(transcript)
                .to_string(),
            frames: voice
                .pointer("/shape/1")
                .and_then(|x| x.as_u64())
                .unwrap_or(0) as usize,
            extra: text,
        })
    }

    pub fn synthesize(
        &self,
        req: &SynthParams,
        cancel: Arc<AtomicBool>,
        mut on_event: impl FnMut(EngineEvent),
    ) -> Result<SynthResult, String> {
        req.validate()?;
        if self.cfg.base_url.trim().is_empty() {
            return Err("NO ENGINE URL".into());
        }
        let started = Instant::now();
        match self.cfg.kind {
            EngineKind::OnnxHttp => {
                if req.stream {
                    self.onnx_stream(req, cancel, &mut on_event, started)
                } else {
                    self.onnx_tts(req, started)
                }
            }
            EngineKind::Sglang => self.sglang_speech(req, cancel, &mut on_event, started),
            EngineKind::LocalOnnx => Err(
                "LOCAL DUALAR needs ONNX sessions on device. Point ENGINE at ONNX HTTP or SGLANG, or load slow_ar_int4.onnx + fast_ar_int4.onnx + codec_decoder_fp16.onnx.".into(),
            ),
        }
    }

    fn onnx_tts(&self, req: &SynthParams, started: Instant) -> Result<SynthResult, String> {
        let voice = empty_to_none(&req.voice);
        let body = OnnxTtsBody {
            text: &req.text,
            voice_name: voice,
            max_new_tokens: req.max_new_tokens,
            temperature: if req.greedy { 0.05 } else { req.temperature },
            top_p: req.top_p,
            top_k: req.top_k.max(1),
            seed: req.seed,
        };
        let bytes = post_bytes(&self.agent(), &self.url("/api/tts"), &body)?;
        wav_result(bytes, req, started)
    }

    fn onnx_stream(
        &self,
        req: &SynthParams,
        cancel: Arc<AtomicBool>,
        on_event: &mut impl FnMut(EngineEvent),
        started: Instant,
    ) -> Result<SynthResult, String> {
        let voice = empty_to_none(&req.voice);
        let body = OnnxTtsBody {
            text: &req.text,
            voice_name: voice,
            max_new_tokens: req.max_new_tokens,
            temperature: if req.greedy { 0.05 } else { req.temperature },
            top_p: req.top_p,
            top_k: req.top_k.max(1),
            seed: req.seed,
        };
        let resp = self
            .agent()
            .post(&self.url("/api/tts/stream"))
            .set("Content-Type", "application/json")
            .send_json(&body)
            .map_err(|e| format!("STREAM: {e}"))?;
        let reader = BufReader::new(resp.into_reader());
        let mut pcm = Vec::new();
        let mut sample_rate = 44100u32;
        let mut frames = 0usize;
        for line in reader.lines() {
            if cancel.load(Ordering::Relaxed) {
                let _ = self.cancel();
                return Err("CANCELLED".into());
            }
            let line = line.map_err(|e| e.to_string())?;
            if line.trim().is_empty() {
                continue;
            }
            let v: Value = serde_json::from_str(&line).map_err(|e| e.to_string())?;
            match v.get("event").and_then(|x| x.as_str()).unwrap_or("") {
                "start" => {
                    sample_rate = v
                        .get("sample_rate")
                        .and_then(|x| x.as_u64())
                        .unwrap_or(44100) as u32;
                    on_event(EngineEvent::Stage("STREAM START".into()));
                }
                "audio_chunk" => {
                    frames = v
                        .get("frame_count")
                        .and_then(|x| x.as_u64())
                        .unwrap_or(frames as u64) as usize;
                    if let Some(b64) = v.get("pcm_b64").and_then(|x| x.as_str()) {
                        let chunk = base64::Engine::decode(
                            &base64::engine::general_purpose::STANDARD,
                            b64,
                        )
                        .map_err(|e| e.to_string())?;
                        pcm.extend_from_slice(&chunk);
                        on_event(EngineEvent::Pcm {
                            sample_rate,
                            bytes: chunk,
                            frames,
                        });
                    }
                }
                "complete" => {
                    frames = v
                        .get("frame_count")
                        .and_then(|x| x.as_u64())
                        .unwrap_or(frames as u64) as usize;
                    on_event(EngineEvent::Stage("STREAM COMPLETE".into()));
                }
                "cancelled" => return Err("CANCELLED".into()),
                _ => {}
            }
        }
        let clip = WavClip::from_s16le_mono(&pcm, sample_rate)?;
        Ok(SynthResult {
            wav: clip.bytes,
            pcm: Some(pcm),
            codes: None,
            sample_rate,
            finished: true,
            code_frames: frames,
            elapsed_secs: started.elapsed().as_secs_f32(),
        })
    }

    fn sglang_speech(
        &self,
        req: &SynthParams,
        cancel: Arc<AtomicBool>,
        on_event: &mut impl FnMut(EngineEvent),
        started: Instant,
    ) -> Result<SynthResult, String> {
        let mut body = json!({
            "model": req.model,
            "input": req.text,
            "response_format": req.response_format.as_str(),
            "max_new_tokens": req.max_new_tokens,
            "temperature": if req.greedy { 0.0 } else { req.temperature },
            "top_p": req.top_p,
            "top_k": req.top_k,
            "do_sample": req.do_sample(),
            "stream": req.stream,
        });
        if !req.voice.trim().is_empty() {
            body["voice"] = json!(req.voice);
        }
        if !req.reference_path.trim().is_empty() && !req.reference_text.trim().is_empty() {
            body["references"] = json!([{
                "audio_path": req.reference_path,
                "text": req.reference_text,
            }]);
        }
        if req.stream {
            let resp = self
                .agent()
                .post(&self.url("/v1/audio/speech"))
                .set("Content-Type", "application/json")
                .send_json(&body)
                .map_err(|e| format!("SGLANG STREAM: {e}"))?;
            let reader = BufReader::new(resp.into_reader());
            let mut pcm = Vec::new();
            let mut sample_rate = 44100u32;
            for line in reader.lines() {
                if cancel.load(Ordering::Relaxed) {
                    return Err("CANCELLED".into());
                }
                let line = line.map_err(|e| e.to_string())?;
                let line = line.trim();
                if !line.starts_with("data: ") {
                    continue;
                }
                let value = &line[6..];
                if value == "[DONE]" {
                    break;
                }
                let event: Value = serde_json::from_str(value).map_err(|e| e.to_string())?;
                if let Some(audio) = event.get("audio") {
                    sample_rate = audio
                        .get("sample_rate")
                        .and_then(|x| x.as_u64())
                        .unwrap_or(sample_rate as u64) as u32;
                    if let Some(data) = audio.get("data").and_then(|x| x.as_str()) {
                        let chunk = base64::Engine::decode(
                            &base64::engine::general_purpose::STANDARD,
                            data,
                        )
                        .map_err(|e| e.to_string())?;
                        pcm.extend_from_slice(&chunk);
                        on_event(EngineEvent::Pcm {
                            sample_rate,
                            bytes: chunk,
                            frames: 0,
                        });
                    }
                }
            }
            match req.response_format {
                ResponseFormat::Codes => Err("streamed codes are not assembled from SSE PCM".into()),
                _ => {
                    let clip = WavClip::from_s16le_mono(&pcm, sample_rate)?;
                    Ok(SynthResult {
                        wav: clip.bytes,
                        pcm: Some(pcm),
                        codes: None,
                        sample_rate,
                        finished: true,
                        code_frames: 0,
                        elapsed_secs: started.elapsed().as_secs_f32(),
                    })
                }
            }
        } else {
            let bytes = post_raw(&self.agent(), &self.url("/v1/audio/speech"), &body)?;
            match req.response_format {
                ResponseFormat::Codes => {
                    let codes = load_npy_codes(&bytes).or_else(|_| {
                        serde_json::from_slice::<Value>(&bytes)
                            .ok()
                            .and_then(|v| v.get("codes").cloned())
                            .ok_or_else(|| "not npy codes".to_string())
                            .and_then(|_| load_npy_codes(&bytes))
                    });
                    match codes {
                        Ok(c) => Ok(SynthResult {
                            wav: Vec::new(),
                            pcm: None,
                            codes: Some(c.clone()),
                            sample_rate: 44100,
                            finished: true,
                            code_frames: c.frames,
                            elapsed_secs: started.elapsed().as_secs_f32(),
                        }),
                        Err(e) => Err(e),
                    }
                }
                ResponseFormat::Pcm => {
                    let clip = WavClip::from_s16le_mono(&bytes, 44100)?;
                    Ok(SynthResult {
                        wav: clip.bytes,
                        pcm: Some(bytes),
                        codes: None,
                        sample_rate: 44100,
                        finished: true,
                        code_frames: 0,
                        elapsed_secs: started.elapsed().as_secs_f32(),
                    })
                }
                ResponseFormat::Wav => wav_result(bytes, req, started),
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum EngineEvent {
    Stage(String),
    Pcm {
        sample_rate: u32,
        bytes: Vec<u8>,
        frames: usize,
    },
}

fn wav_result(bytes: Vec<u8>, req: &SynthParams, started: Instant) -> Result<SynthResult, String> {
    if looks_like_wav(&bytes) {
        let clip = WavClip::from_wav_bytes(bytes.clone())?;
        Ok(SynthResult {
            wav: bytes,
            pcm: None,
            codes: None,
            sample_rate: clip.sample_rate,
            finished: true,
            code_frames: 0,
            elapsed_secs: started.elapsed().as_secs_f32(),
        })
    } else if req.save_codes {
        let codes = load_npy_codes(&bytes)?;
        Ok(SynthResult {
            wav: Vec::new(),
            pcm: None,
            codes: Some(codes.clone()),
            sample_rate: 44100,
            finished: true,
            code_frames: codes.frames,
            elapsed_secs: started.elapsed().as_secs_f32(),
        })
    } else {
        Err(format!("response was {} bytes, not WAV", bytes.len()))
    }
}

fn post_bytes<T: Serialize>(agent: &ureq::Agent, url: &str, body: &T) -> Result<Vec<u8>, String> {
    post_raw(agent, url, body)
}

fn post_raw<T: Serialize>(agent: &ureq::Agent, url: &str, body: &T) -> Result<Vec<u8>, String> {
    let resp = agent
        .post(url)
        .set("Content-Type", "application/json")
        .send_json(body)
        .map_err(|e| format!("{url}: {e}"))?;
    let status = resp.status();
    let mut bytes = Vec::new();
    resp.into_reader()
        .read_to_end(&mut bytes)
        .map_err(|e| format!("{url} READ: {e}"))?;
    if !(200..300).contains(&status) {
        return Err(format!(
            "{url} HTTP {status}: {}",
            trunc(&String::from_utf8_lossy(&bytes), 200)
        ));
    }
    Ok(bytes)
}

fn write_field(
    body: &mut Vec<u8>,
    boundary: &str,
    name: &str,
    data: &[u8],
    file: Option<(&str, &str)>,
) {
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    if let Some((filename, mime)) = file {
        body.extend_from_slice(
            format!(
                "Content-Disposition: form-data; name=\"{name}\"; filename=\"{filename}\"\r\nContent-Type: {mime}\r\n\r\n"
            )
            .as_bytes(),
        );
    } else {
        body.extend_from_slice(
            format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n").as_bytes(),
        );
    }
    body.extend_from_slice(data);
    body.extend_from_slice(b"\r\n");
}

fn empty_to_none(s: &str) -> Option<&str> {
    let s = s.trim();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

fn trunc(s: &str, n: usize) -> String {
    let s = s.replace('\n', " ");
    if s.len() <= n {
        s
    } else {
        format!("{}…", s.chars().take(n).collect::<String>())
    }
}

fn parse_voices(json: &str) -> Result<Vec<VoiceMeta>, String> {
    let value: Value = serde_json::from_str(json).map_err(|e| format!("VOICES JSON: {e}"))?;
    let mut out = Vec::new();
    let items = value
        .get("voices")
        .cloned()
        .or_else(|| value.get("data").cloned())
        .unwrap_or(value);
    match items {
        Value::Array(arr) => {
            for item in arr {
                out.push(voice_from_value(&item));
            }
        }
        other => out.push(voice_from_value(&other)),
    }
    out.retain(|v| !v.name.is_empty());
    Ok(out)
}

fn voice_from_value(item: &Value) -> VoiceMeta {
    match item {
        Value::String(s) => VoiceMeta {
            name: s.clone(),
            ..VoiceMeta::default()
        },
        Value::Object(map) => VoiceMeta {
            name: map
                .get("name")
                .or_else(|| map.get("id"))
                .or_else(|| map.get("voice"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            reference_text: map
                .get("reference_text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            frames: map
                .get("shape")
                .and_then(|v| v.as_array())
                .and_then(|a| a.get(1))
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize,
            extra: item.to_string(),
        },
        _ => VoiceMeta::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_voice_shapes() {
        let a = parse_voices(r#"{"voices":["speaker_a","speaker_b"]}"#).unwrap();
        assert_eq!(a.len(), 2);
        let b = parse_voices(r#"{"voices":[{"name":"neo","shape":[10,32]}]}"#).unwrap();
        assert_eq!(b[0].name, "neo");
        assert_eq!(b[0].frames, 32);
    }
}

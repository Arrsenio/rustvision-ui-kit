//! On-device voice profiles: `voices/<name>/{meta.json,codes.npy}`.

use super::npy::{load_npy_codes, CodecCodes};
use super::params::VoiceMeta;
use serde_json::{json, Value};

pub fn parse_voice_dir(meta_json: &str, npy: &[u8]) -> Result<(VoiceMeta, CodecCodes), String> {
    let codes = load_npy_codes(npy)?;
    let v: Value = serde_json::from_str(meta_json).map_err(|e| e.to_string())?;
    let meta = VoiceMeta {
        name: v.get("name").and_then(|x| x.as_str()).unwrap_or("").into(),
        reference_text: v
            .get("reference_text")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .into(),
        frames: codes.frames,
        extra: meta_json.to_string(),
    };
    if meta.reference_text.trim().is_empty() {
        return Err(format!("voice {} has no reference_text", meta.name));
    }
    Ok((meta, codes))
}

pub fn voice_meta_json(name: &str, reference_text: &str, codes: &CodecCodes, source: &str) -> String {
    json!({
        "name": name,
        "reference_text": reference_text,
        "shape": [codes.rows, codes.frames],
        "dtype": "uint16",
        "sample_rate": 44100,
        "source_audio": source,
        "source_kind": "audio8_tts_android",
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_profile() {
        let codes = CodecCodes::new(10, 2, (0..20).collect()).unwrap();
        let meta = voice_meta_json("neo", "hello from neo", &codes, "ref.wav");
        let npy = codes.to_npy_u16();
        let (m, c) = parse_voice_dir(&meta, &npy).unwrap();
        assert_eq!(m.name, "neo");
        assert_eq!(c.frames, 2);
    }
}

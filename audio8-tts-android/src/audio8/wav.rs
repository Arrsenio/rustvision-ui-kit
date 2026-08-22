//! PCM / WAV encode-decode and linear resample (codec hop 2048 @ 44.1 kHz).

#[derive(Clone, Debug, Default)]
pub struct WavClip {
    pub bytes: Vec<u8>,
    pub samples: Vec<i16>,
    pub sample_rate: u32,
    pub channels: u16,
}

impl WavClip {
    pub fn from_wav_bytes(bytes: Vec<u8>) -> Result<Self, String> {
        let parsed = parse_wav(&bytes)?;
        Ok(Self {
            bytes,
            samples: parsed.samples,
            sample_rate: parsed.sample_rate,
            channels: parsed.channels,
        })
    }

    pub fn from_f32_mono(samples: &[f32], sample_rate: u32) -> Self {
        let pcm: Vec<i16> = samples
            .iter()
            .map(|s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
            .collect();
        let bytes = write_wav_i16(&pcm, sample_rate, 1);
        Self {
            bytes,
            samples: pcm,
            sample_rate,
            channels: 1,
        }
    }

    pub fn from_s16le_mono(pcm: &[u8], sample_rate: u32) -> Result<Self, String> {
        if pcm.len() % 2 != 0 {
            return Err("PCM stream has an odd byte count".into());
        }
        let mut samples = Vec::with_capacity(pcm.len() / 2);
        let mut i = 0;
        while i + 1 < pcm.len() {
            samples.push(i16::from_le_bytes([pcm[i], pcm[i + 1]]));
            i += 2;
        }
        let bytes = write_wav_i16(&samples, sample_rate, 1);
        Ok(Self {
            bytes,
            samples,
            sample_rate,
            channels: 1,
        })
    }

    pub fn duration_secs(&self) -> f32 {
        if self.sample_rate == 0 || self.channels == 0 {
            return 0.0;
        }
        self.samples.len() as f32 / (self.sample_rate as f32 * self.channels as f32)
    }

    pub fn peaks(&self, buckets: usize) -> Vec<f32> {
        if buckets == 0 || self.samples.is_empty() {
            return vec![0.0; buckets];
        }
        let chunk = (self.samples.len() / buckets).max(1);
        (0..buckets)
            .map(|i| {
                let start = i * chunk;
                let end = (start + chunk).min(self.samples.len());
                if start >= end {
                    0.0
                } else {
                    let mut max = 0i32;
                    for s in &self.samples[start..end] {
                        max = max.max((*s as i32).unsigned_abs() as i32);
                    }
                    (max as f32) / (i16::MAX as f32)
                }
            })
            .collect()
    }
}

struct ParsedWav {
    samples: Vec<i16>,
    sample_rate: u32,
    channels: u16,
}

pub fn parse_wav(bytes: &[u8]) -> Result<ParsedWav, String> {
    if bytes.len() < 44 || !bytes.starts_with(b"RIFF") || &bytes[8..12] != b"WAVE" {
        return Err("NOT A WAV".into());
    }
    let mut offset = 12usize;
    let mut fmt: Option<(u16, u16, u32, u16)> = None;
    let mut data_range: Option<(usize, usize)> = None;
    while offset + 8 <= bytes.len() {
        let id = &bytes[offset..offset + 4];
        let size = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let start = offset + 8;
        let end = start.saturating_add(size).min(bytes.len());
        if id == b"fmt " && end - start >= 16 {
            let audio_format = u16::from_le_bytes(bytes[start..start + 2].try_into().unwrap());
            let channels = u16::from_le_bytes(bytes[start + 2..start + 4].try_into().unwrap());
            let sample_rate = u32::from_le_bytes(bytes[start + 4..start + 8].try_into().unwrap());
            let bits = u16::from_le_bytes(bytes[start + 14..start + 16].try_into().unwrap());
            fmt = Some((audio_format, channels, sample_rate, bits));
        } else if id == b"data" {
            data_range = Some((start, end));
        }
        offset = start + size + (size % 2);
    }
    let Some((format, channels, sample_rate, bits)) = fmt else {
        return Err("WAV MISSING FMT".into());
    };
    let Some((start, end)) = data_range else {
        return Err("WAV MISSING DATA".into());
    };
    if format != 1 {
        return Err(format!("WAV FORMAT {format} NOT PCM"));
    }
    if bits != 16 {
        return Err(format!("WAV {bits}-BIT NOT SUPPORTED"));
    }
    if channels == 0 || sample_rate == 0 {
        return Err("WAV BAD HEADER".into());
    }
    let pcm = &bytes[start..end];
    let mut samples = Vec::with_capacity(pcm.len() / 2);
    let mut i = 0;
    while i + 1 < pcm.len() {
        samples.push(i16::from_le_bytes([pcm[i], pcm[i + 1]]));
        i += 2;
    }
    Ok(ParsedWav {
        samples,
        sample_rate,
        channels,
    })
}

pub fn write_wav_i16(samples: &[i16], sample_rate: u32, channels: u16) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut b = Vec::with_capacity(44 + samples.len() * 2);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data_len).to_le_bytes());
    b.extend_from_slice(b"WAVE");
    b.extend_from_slice(b"fmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&channels.to_le_bytes());
    b.extend_from_slice(&sample_rate.to_le_bytes());
    let byte_rate = sample_rate * channels as u32 * 2;
    b.extend_from_slice(&byte_rate.to_le_bytes());
    b.extend_from_slice(&(channels * 2).to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        b.extend_from_slice(&s.to_le_bytes());
    }
    b
}

pub fn looks_like_wav(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WAVE"
}

pub fn resample_mono_i16(samples: &[i16], from: u32, to: u32) -> Vec<i16> {
    if from == 0 || to == 0 || samples.is_empty() || from == to {
        return samples.to_vec();
    }
    let ratio = to as f64 / from as f64;
    let out_len = ((samples.len() as f64) * ratio).round().max(1.0) as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src = i as f64 / ratio;
        let i0 = src.floor() as usize;
        let i1 = (i0 + 1).min(samples.len() - 1);
        let t = (src - i0 as f64) as f32;
        let a = samples[i0.min(samples.len() - 1)] as f32;
        let b = samples[i1] as f32;
        out.push((a + (b - a) * t) as i16);
    }
    out
}

pub fn pad_to_hop(samples: &[f32], hop: usize) -> Vec<f32> {
    if hop == 0 {
        return samples.to_vec();
    }
    let pad = (hop - (samples.len() % hop)) % hop;
    let mut out = samples.to_vec();
    out.extend(std::iter::repeat(0.0).take(pad));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_roundtrip_one_second() {
        let samples = vec![0i16; 44100];
        let bytes = write_wav_i16(&samples, 44100, 1);
        let clip = WavClip::from_wav_bytes(bytes).unwrap();
        assert!((clip.duration_secs() - 1.0).abs() < 0.001);
        assert!(looks_like_wav(&clip.bytes));
    }
}

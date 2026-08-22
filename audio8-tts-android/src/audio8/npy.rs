//! NumPy `.npy` codec arrays with shape `[num_codebooks, T]` (uint16 / int64).

use std::io::{Cursor, Read};

pub const NUM_CODEBOOKS: usize = 10;
pub const CODEBOOK_SIZE: i64 = 4096;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodecCodes {
    pub rows: usize,
    pub frames: usize,
    pub values: Vec<i64>,
}

impl CodecCodes {
    pub fn new(rows: usize, frames: usize, values: Vec<i64>) -> Result<Self, String> {
        if rows == 0 || frames == 0 {
            return Err("codec indices must have shape [N, T>0]".into());
        }
        if values.len() != rows * frames {
            return Err(format!(
                "codec buffer {} != {}x{}",
                values.len(),
                rows,
                frames
            ));
        }
        if values.iter().any(|v| *v < 0 || *v >= CODEBOOK_SIZE) {
            return Err(format!(
                "codec indices must be in [0, {}]",
                CODEBOOK_SIZE - 1
            ));
        }
        Ok(Self {
            rows,
            frames,
            values,
        })
    }

    pub fn at(&self, row: usize, frame: usize) -> i64 {
        self.values[row * self.frames + frame]
    }

    pub fn frame(&self, t: usize) -> Vec<i64> {
        (0..self.rows).map(|r| self.at(r, t)).collect()
    }

    pub fn to_npy_u16(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(self.values.len() * 2);
        for v in &self.values {
            data.extend_from_slice(&(*v as u16).to_le_bytes());
        }
        wrap_npy(&format!("'<u2'"), self.rows, self.frames, &data)
    }
}

fn wrap_npy(descr: &str, rows: usize, frames: usize, payload: &[u8]) -> Vec<u8> {
    let header = format!(
        "{{'descr': {descr}, 'fortran_order': False, 'shape': ({rows}, {frames}), }}"
    );
    let mut header_bytes = header.into_bytes();
    header_bytes.push(b'\n');
    // pad so MAGIC(6)+version(2)+hdrlen(2)+header is 64-byte aligned as CPython does.
    let mut total = 10 + header_bytes.len();
    let pad = (16 - (total % 16)) % 16;
    header_bytes.extend(std::iter::repeat(b' ').take(pad));
    total = 10 + header_bytes.len();
    let extra = (64 - (total % 64)) % 64;
    header_bytes.extend(std::iter::repeat(b' ').take(extra));
    if let Some(last) = header_bytes.last_mut() {
        *last = b'\n';
    }
    let mut out = b"\x93NUMPY\x01\x00".to_vec();
    let len = u16::try_from(header_bytes.len()).unwrap_or(u16::MAX);
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&header_bytes);
    out.extend_from_slice(payload);
    out
}

pub fn load_npy_codes(bytes: &[u8]) -> Result<CodecCodes, String> {
    if bytes.len() < 10 || &bytes[..6] != b"\x93NUMPY" {
        return Err("not an npy file".into());
    }
    let major = bytes[6];
    let (hdr_len, hdr_off) = if major == 1 {
        let n = u16::from_le_bytes([bytes[8], bytes[9]]) as usize;
        (n, 10usize)
    } else {
        if bytes.len() < 12 {
            return Err("npy header truncated".into());
        }
        let n = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
        (n, 12usize)
    };
    let hdr_end = hdr_off + hdr_len;
    if bytes.len() < hdr_end {
        return Err("npy header truncated".into());
    }
    let header = String::from_utf8_lossy(&bytes[hdr_off..hdr_end]);
    let descr = if header.contains("<u2") || header.contains("|u2") {
        DType::U16
    } else if header.contains("<i8") || header.contains("<i4") || header.contains("<i2") {
        DType::I64
    } else {
        return Err(format!("unsupported npy descr in {header}"));
    };
    let shape = parse_shape(&header)?;
    if shape.len() != 2 {
        return Err(format!("codec npy must be rank-2, got {shape:?}"));
    }
    let (rows, frames) = (shape[0], shape[1]);
    let payload = &bytes[hdr_end..];
    let mut values = Vec::with_capacity(rows * frames);
    let mut cur = Cursor::new(payload);
    match descr {
        DType::U16 => {
            let mut buf = [0u8; 2];
            for _ in 0..rows * frames {
                cur.read_exact(&mut buf).map_err(|e| e.to_string())?;
                values.push(u16::from_le_bytes(buf) as i64);
            }
        }
        DType::I64 => {
            if header.contains("<i2") {
                let mut buf = [0u8; 2];
                for _ in 0..rows * frames {
                    cur.read_exact(&mut buf).map_err(|e| e.to_string())?;
                    values.push(i16::from_le_bytes(buf) as i64);
                }
            } else if header.contains("<i4") {
                let mut buf = [0u8; 4];
                for _ in 0..rows * frames {
                    cur.read_exact(&mut buf).map_err(|e| e.to_string())?;
                    values.push(i32::from_le_bytes(buf) as i64);
                }
            } else {
                let mut buf = [0u8; 8];
                for _ in 0..rows * frames {
                    cur.read_exact(&mut buf).map_err(|e| e.to_string())?;
                    values.push(i64::from_le_bytes(buf));
                }
            }
        }
    }
    CodecCodes::new(rows, frames, values)
}

fn parse_shape(header: &str) -> Result<Vec<usize>, String> {
    let start = header.find("shape").ok_or("npy missing shape")?;
    let rest = &header[start..];
    let open = rest.find('(').ok_or("npy shape")?;
    let close = rest.find(')').ok_or("npy shape")?;
    let inner = &rest[open + 1..close];
    let dims: Result<Vec<usize>, _> = inner
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.parse::<usize>().map_err(|e| e.to_string()))
        .collect();
    dims
}

enum DType {
    U16,
    I64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_u16() {
        let codes = CodecCodes::new(10, 3, (0..30).map(|i| i % 4000).collect()).unwrap();
        let bytes = codes.to_npy_u16();
        let loaded = load_npy_codes(&bytes).unwrap();
        assert_eq!(loaded.rows, 10);
        assert_eq!(loaded.frames, 3);
        assert_eq!(loaded.values, codes.values);
    }
}

//! Audio8 text cleaning, IDs, languages, and DualAR prompt strings.
//! Port of `audio8_tts_data.clean_text` / `prompt.format_reference_text`.

use unicode_general_category::{get_general_category, GeneralCategory};

const LANGUAGES: &[(&str, &str)] = &[
    ("AUTO", "auto"),
    ("EN", "English"),
    ("ZH", "Chinese"),
    ("YUE", "Cantonese"),
    ("JA", "Japanese"),
    ("KO", "Korean"),
    ("FR", "French"),
    ("DE", "German"),
    ("ES", "Spanish"),
    ("IT", "Italian"),
    ("NL", "Dutch"),
    ("PL", "Polish"),
];

pub fn supported_languages() -> &'static [(&'static str, &'static str)] {
    LANGUAGES
}

fn is_unicode_other(c: char) -> bool {
    matches!(
        get_general_category(c),
        GeneralCategory::Control
            | GeneralCategory::Format
            | GeneralCategory::Surrogate
            | GeneralCategory::PrivateUse
            | GeneralCategory::Unassigned
    )
}

fn is_cjk(c: char) -> bool {
    matches!(
        c as u32,
        0x1100..=0x11FF
            | 0x2E80..=0x2FDF
            | 0x3000..=0x303F
            | 0x3040..=0x30FF
            | 0x3100..=0x31FF
            | 0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xA960..=0xA97F
            | 0xAC00..=0xD7A3
            | 0xD7B0..=0xD7FF
            | 0xF900..=0xFAFF
            | 0xFE30..=0xFE4F
            | 0xFF01..=0xFF9F
            | 0x20000..=0x2FA1F
    )
}

fn is_line_break(c: char) -> bool {
    matches!(
        c,
        '\r' | '\n' | '\u{000B}' | '\u{000C}' | '\u{001C}' | '\u{001D}' | '\u{001E}'
            | '\u{0085}' | '\u{2028}' | '\u{2029}'
    )
}

fn normalize_whitespace(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if !chars[i].is_whitespace() {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        let run = &chars[start..i];
        let left = if start == 0 {
            None
        } else {
            Some(chars[start - 1])
        };
        let right = chars.get(i).copied();
        let line = run.iter().copied().any(is_line_break);
        if line && left.is_some_and(is_cjk) && right.is_some_and(is_cjk) {
            continue;
        }
        out.push(' ');
    }
    out.trim().to_string()
}

pub fn clean_text(value: &str, field_name: &str) -> Result<String, String> {
    let filtered: String = value
        .chars()
        .filter_map(|c| {
            if c.is_whitespace() {
                Some(c)
            } else if is_unicode_other(c) {
                None
            } else {
                Some(c)
            }
        })
        .collect();
    let text = normalize_whitespace(&filtered);
    if text.is_empty() {
        return Err(format!("{field_name} must not be empty"));
    }
    Ok(text)
}

pub fn format_reference_text(text: &str) -> Result<String, String> {
    let text = clean_text(text, "reference_text")?;
    if speaker_tag_present(&text) {
        Ok(text)
    } else {
        Ok(format!("<|speaker:0|>{text}"))
    }
}

fn speaker_tag_present(text: &str) -> bool {
    let bytes = text.as_bytes();
    let needle = b"<|speaker:";
    bytes.windows(needle.len()).any(|w| w == needle)
        && text.contains("|>")
}

pub fn validate_sample_id(value: &str, fallback: Option<&str>) -> Result<String, String> {
    let sample_id = {
        let raw = if value.trim().is_empty() {
            fallback.unwrap_or("").trim()
        } else {
            value.trim()
        };
        raw.to_string()
    };
    if sample_id.is_empty() {
        return Err("sample id must not be empty".into());
    }
    if sample_id == "." || sample_id == ".." || sample_id.contains('/') || sample_id.contains('\\')
    {
        return Err(format!("unsafe sample id: {sample_id:?}"));
    }
    if !sample_id
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '-')
    {
        return Err(format!(
            "sample id contains unsupported characters: {sample_id:?}"
        ));
    }
    Ok(sample_id)
}

pub fn system_prompt_parts<'a>(
    reference_text: &'a str,
    target_text: &'a str,
) -> Result<[String; 2], String> {
    let prefix = format!(
        "<|im_start|>system\nconvert the provided text to speech reference to the following:\n\nText:\n{}\n\nSpeech:\n",
        format_reference_text(reference_text)?
    );
    let suffix = format!(
        "<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n<|voice|>",
        clean_text(target_text, "text")?
    );
    Ok([prefix, suffix])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_text_normalizes_whitespace_by_script() {
        let cases = [
            ("你\n好", "你好"),
            ("你 \t\n 好", "你好"),
            ("你 好", "你 好"),
            ("こんにちは。\n世界", "こんにちは。世界"),
            ("안녕하세요\n세계", "안녕하세요세계"),
            ("안녕하세요 세계", "안녕하세요 세계"),
            ("hello\nworld", "hello world"),
            ("你好\nworld", "你好 world"),
            ("こんにちは\nworld", "こんにちは world"),
            ("안녕하세요\nworld", "안녕하세요 world"),
            ("hello\n世界", "hello 世界"),
            ("甲\u{20000}乙", "甲\u{20000}乙"),
        ];
        for (value, expected) in cases {
            assert_eq!(clean_text(value, "text").unwrap(), expected, "{value:?}");
        }
    }

    #[test]
    fn clean_text_rejects_whitespace_only() {
        assert!(clean_text(" \n\t", "text").is_err());
    }

    #[test]
    fn reference_gets_speaker_tag() {
        assert_eq!(
            format_reference_text("hello there").unwrap(),
            "<|speaker:0|>hello there"
        );
        assert_eq!(
            format_reference_text("<|speaker:3|>already").unwrap(),
            "<|speaker:3|>already"
        );
    }

    #[test]
    fn sample_ids_are_path_safe() {
        assert_eq!(validate_sample_id("utt_001", None).unwrap(), "utt_001");
        assert!(validate_sample_id("../x", None).is_err());
        assert!(validate_sample_id("a/b", None).is_err());
    }
}

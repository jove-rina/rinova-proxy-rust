pub fn pad_base64(s: &str) -> String {
    let rem = s.len() % 4;
    if rem == 0 {
        s.to_string()
    } else {
        format!("{}{}", s, "=".repeat(4 - rem))
    }
}

pub fn safe_decode_uri(s: Option<&str>) -> String {
    match s {
        None | Some("") => String::new(),
        Some(raw) => urlencoding::decode(raw)
            .map(|decoded| decoded.into_owned())
            .unwrap_or_else(|_| raw.to_string()),
    }
}

pub fn decode_base64_utf8(s: &str) -> Result<String, base64::DecodeError> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD.decode(pad_base64(s))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pad_base64_adds_padding() {
        assert_eq!(pad_base64("YWJj"), "YWJj");
        assert_eq!(pad_base64("YWJjZA"), "YWJjZA==");
    }

    #[test]
    fn safe_decode_uri_handles_percent() {
        assert_eq!(safe_decode_uri(Some("%E7%BE%8E%E5%9B%BD")), "美国");
        assert_eq!(safe_decode_uri(None), "");
    }
}

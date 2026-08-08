//! QR-code decoding. Reads a QR from an image file on disk OR from a
//! `data:image/...;base64,...` data URL (e.g. a screenshot the agent just took),
//! and returns the decoded text payload(s). Pure-Rust (`rqrr` + `image`), no network.
//!
//! Legitimate uses: reading a login/pairing QR shown in a captured screenshot,
//! decoding a QR in a project asset, inspecting what a QR actually encodes.

/// Minimal standard-base64 decoder (no extra crate) for data-URL payloads.
fn b64_decode(s: &str) -> Result<Vec<u8>, String> {
    fn val(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    let mut buf = 0u32;
    let mut bits = 0u8;
    for &c in s.as_bytes() {
        if c == b'=' || c == b'\n' || c == b'\r' || c == b' ' {
            continue;
        }
        let v = match val(c) {
            Some(v) => v,
            None => return Err("非法 base64 字符".into()),
        };
        buf = (buf << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Ok(out)
}

/// Decode QR code(s) from an image. Provide `path` (a file on disk) OR `data_url`
/// (base64 data URL). Returns every QR payload found in the image.
#[tauri::command]
pub async fn decode_qr(
    path: Option<String>,
    data_url: Option<String>,
) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || decode_qr_inner(path, data_url))
        .await
        .map_err(|error| format!("二维码解码任务失败: {error}"))?
}

fn decode_qr_inner(path: Option<String>, data_url: Option<String>) -> Result<Vec<String>, String> {
    let img = if let Some(p) = path.as_deref().filter(|p| !p.is_empty()) {
        image::open(p).map_err(|e| format!("打不开图片 {p}: {e}"))?
    } else if let Some(d) = data_url.as_deref().filter(|d| !d.is_empty()) {
        let b64 = d.rsplit(',').next().unwrap_or(d);
        let bytes = b64_decode(b64.trim())?;
        image::load_from_memory(&bytes).map_err(|e| format!("解码图片失败: {e}"))?
    } else {
        return Err("decode_qr 需要 path（图片文件路径）或 data_url（base64 图片）".into());
    };

    let luma = img.to_luma8();
    let mut prepared = rqrr::PreparedImage::prepare(luma);
    let grids = prepared.detect_grids();
    let mut out = Vec::new();
    for g in grids {
        if let Ok((_meta, content)) = g.decode() {
            if !content.is_empty() {
                out.push(content);
            }
        }
    }
    if out.is_empty() {
        return Err(
            "没识别到二维码：图可能太糊/太小/有遮挡，或根本不是 QR。建议先裁剪到二维码那块区域、放大清晰些再试。"
                .into(),
        );
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{b64_decode, decode_qr_inner};

    #[test]
    fn base64_decoder_handles_standard_payload() {
        assert_eq!(b64_decode("aGVsbG8=").unwrap(), b"hello");
    }

    #[test]
    fn missing_qr_input_is_an_error() {
        assert!(decode_qr_inner(None, None).is_err());
    }
}

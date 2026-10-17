use alloc::{string::String, vec::Vec};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

const CUSTOM: &[u8] = b"_-9876543210abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
const STANDARD: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

pub fn decode(value: &str) -> Result<Vec<String>, &'static str> {
    let body = value
        .strip_prefix("qM9")
        .and_then(|body| body.strip_suffix("Z7"))
        .ok_or("Unsupported image payload")?;
    if !body.is_ascii() || body.len() <= 5 {
        return Err("Invalid image payload length");
    }
    let length = body.len() - 5;
    let last = length / 3;
    let first = (length - last) / 2;
    let middle = length - last - first;
    let marker_end = first + 2;
    let middle_end = marker_end + middle;
    let separator_end = middle_end + 3;
    if &body[first..marker_end] != "Vx" || &body[middle_end..separator_end] != "pL0" {
        return Err("Invalid image payload markers");
    }
    let mut reordered = Vec::with_capacity(length);
    reordered.extend_from_slice(&body.as_bytes()[separator_end..]);
    reordered.extend_from_slice(&body.as_bytes()[..first]);
    reordered.extend_from_slice(&body.as_bytes()[marker_end..middle_end]);
    for (index, block) in reordered.chunks_mut(7).enumerate() {
        if index % 2 == 1 {
            block.reverse();
        }
    }
    for character in &mut reordered {
        let index = CUSTOM
            .iter()
            .position(|value| value == character)
            .ok_or("Invalid image payload character")?;
        *character = STANDARD[index];
    }
    let decoded = URL_SAFE_NO_PAD
        .decode(reordered)
        .map_err(|_| "Invalid image base64")?;
    let images: Vec<String> = serde_json::from_slice(&decoded).map_err(|_| "Invalid image JSON")?;
    if images.is_empty() || images.iter().any(|image| image.trim().is_empty()) {
        return Err("Empty image list or path");
    }
    Ok(images)
}

pub fn remove_decoy(images: &mut Vec<String>, order_id: Option<i64>, sid: Option<i64>) {
    let (Some(order_id), Some(sid)) = (order_id, sid) else {
        return;
    };
    if images.is_empty() {
        return;
    }
    let size = images.len() as i64;
    let offset = (sid.wrapping_mul(2_654_435_761) ^ size.wrapping_mul(2_246_822_507)) % size;
    let index = order_id ^ offset;
    if index >= 0 && index < size {
        images.remove(index as usize);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn live_payload_matches_captured_paths_and_decoy() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/hipmh/chapter.json"
        ))
        .unwrap();
        let mut images = decode(fixture["images"].as_str().unwrap()).unwrap();
        let expected: Vec<String> = serde_json::from_value(fixture["decoded"].clone()).unwrap();
        assert_eq!(images, expected);
        remove_decoy(
            &mut images,
            fixture["order_id"].as_i64(),
            fixture["sid"].as_i64(),
        );
        let cleaned: Vec<String> = serde_json::from_value(fixture["cleaned"].clone()).unwrap();
        assert_eq!(images, cleaned);
    }

    #[test]
    fn malformed_input_returns_errors_without_panicking() {
        for value in [
            "",
            "qM9Z7",
            "qM9你好VxpL0Z7",
            "qM9aVxbpL0cZ7",
            "qM9!!!!!!!!!!!!Z7",
        ] {
            assert!(decode(value).is_err());
        }
        let mut images = vec![String::from("a"), String::from("b")];
        remove_decoy(&mut images, None, Some(7));
        assert_eq!(images.len(), 2);
        remove_decoy(&mut images, Some(i64::MAX), Some(i64::MAX));
        assert_eq!(images.len(), 2);
        remove_decoy(&mut Vec::new(), Some(0), Some(0));
    }
}

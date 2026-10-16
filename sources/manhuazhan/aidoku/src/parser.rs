use alloc::{format, string::String, vec::Vec};
use base64::{Engine, engine::general_purpose::STANDARD};

pub const BASE_URL: &str = "https://m.manhuazhan.com";
const KEYS: [&str; 10] = [
    "4-bXd9iN", "4-RXyjry", "4-oYvwVy", "4-4ZY57U", "4-mbJpU7", "4-6MM2Ei", "4-54TiQr", "4-Ph5xx9",
    "4-bYgePR", "4-Z9A3bW",
];

pub fn absolute(value: &str) -> Option<String> {
    let value = value.trim();
    if value.starts_with("https://") {
        Some(value.into())
    } else if let Some(rest) = value.strip_prefix("http://") {
        Some(format!("https://{rest}"))
    } else if value.starts_with("//") {
        Some(format!("https:{value}"))
    } else if value.starts_with('/') {
        Some(format!("{BASE_URL}{value}"))
    } else {
        None
    }
}

pub fn comic_key(address: &str) -> Option<String> {
    let value = address.split("/comic/").nth(1)?;
    let value: String = value.chars().take_while(char::is_ascii_digit).collect();
    if value.is_empty() { None } else { Some(value) }
}

pub fn chapter_key(address: &str, manga_key: &str) -> Option<String> {
    let value = address.split("/chapter/").nth(1)?.split(".html").next()?;
    let (manga, chapter) = value.split_once('-')?;
    if manga == manga_key
        && !chapter.is_empty()
        && chapter.chars().all(|character| character.is_ascii_digit())
    {
        Some(value.into())
    } else {
        None
    }
}

pub fn quoted_variable(html: &str, name: &str) -> Option<String> {
    let start = html.find(&format!("var {name}"))?;
    let value = html[start..].split_once('=')?.1.trim_start();
    let quote = value.chars().next()?;
    if quote != '\'' && quote != '"' {
        return None;
    }
    Some(value[1..].split(quote).next()?.into())
}

pub fn encrypted_images(html: &str) -> core::result::Result<Vec<String>, &'static str> {
    let payload = quoted_variable(html, "DATA").ok_or("章节缺少 DATA")?;
    let cid_start = html.find("var cid").ok_or("章节缺少 cid")?;
    let cid: u64 = html[cid_start..]
        .split_once('=')
        .ok_or("cid 格式错误")?
        .1
        .trim_start()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .map_err(|_| "cid 格式错误")?;
    let key = KEYS[(cid % 10) as usize].as_bytes();
    let encrypted = STANDARD.decode(payload).map_err(|_| "DATA 格式错误")?;
    let inner: Vec<u8> = encrypted
        .iter()
        .enumerate()
        .map(|(position, byte)| byte ^ key[position % key.len()])
        .collect();
    let decoded = STANDARD.decode(inner).map_err(|_| "图片数据格式错误")?;
    json_images(&decoded)
}

pub fn json_images(payload: &[u8]) -> core::result::Result<Vec<String>, &'static str> {
    let entries: serde_json::Value =
        serde_json::from_slice(payload).map_err(|_| "图片 JSON 格式错误")?;
    let entries = entries.as_array().ok_or("图片列表格式错误")?;
    let mut images = Vec::new();
    for entry in entries {
        if let Some(value) = entry
            .as_str()
            .or_else(|| entry.get("url").and_then(|value| value.as_str()))
        {
            if let Some(url) = absolute(value) {
                if !images.contains(&url) {
                    images.push(url);
                }
            }
        }
    }
    if images.is_empty() {
        Err("图片列表为空")
    } else {
        Ok(images)
    }
}

pub fn urlencode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.~".contains(&byte) {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decodes_all_ten_keys_and_both_entry_shapes() {
        let json = br#"[{"url":"https://example.org/1.jpg"},"//example.org/2.jpg"]"#;
        let encoded = STANDARD.encode(json);
        for (suffix, key) in KEYS.iter().enumerate() {
            let data: Vec<u8> = encoded
                .bytes()
                .enumerate()
                .map(|(position, byte)| byte ^ key.as_bytes()[position % key.len()])
                .collect();
            let html = format!("var cid={suffix};var DATA='{}';", STANDARD.encode(data));
            assert_eq!(
                encrypted_images(&html).unwrap(),
                ["https://example.org/1.jpg", "https://example.org/2.jpg"]
            );
        }
    }
    #[test]
    fn rejects_invalid_payloads_and_cross_comic_chapters() {
        assert_eq!(
            comic_key("https://m.manhuazhan.com/comic/219430").as_deref(),
            Some("219430")
        );
        assert!(encrypted_images("var cid=1;var DATA='invalid!';").is_err());
        assert!(chapter_key("/chapter/2-30.html", "1").is_none());
        assert_eq!(
            chapter_key("/chapter/1-30.html", "1").as_deref(),
            Some("1-30")
        );
        assert_eq!(urlencode("斗 罗&"), "%E6%96%97%20%E7%BD%97%26");
    }
    #[test]
    fn decodes_live_site_regression_fixture() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/manhuazhan/encrypted.json"
        ))
        .unwrap();
        let html = format!(
            "var cid={};var DATA='{}';",
            fixture["cid"],
            fixture["data"].as_str().unwrap()
        );
        let images = encrypted_images(&html).unwrap();
        assert_eq!(images.len(), fixture["count"].as_u64().unwrap() as usize);
        assert_eq!(images[0], fixture["first"].as_str().unwrap());
    }
}

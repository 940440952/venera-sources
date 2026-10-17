use alloc::{format, string::String};

pub const BASE_URL: &str = "https://m.hipmh.com";
pub const API_URL: &str = "https://hipapi1.s3file.top";
pub const READER_URL: &str = "https://reader.hipmh.top";

pub fn urlencode(value: &str) -> String {
    let mut result = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            result.push(byte as char);
        } else {
            result.push_str(&format!("%{byte:02X}"));
        }
    }
    result
}

pub fn valid_key(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
}

pub fn manga_key(url: &str) -> Option<String> {
    let path = if url.starts_with('/') && !url.starts_with("//") {
        url
    } else {
        let tail = url.strip_prefix("https://")?;
        let (host, path) = tail.split_once('/')?;
        if host != "hipmh.com" && !host.ends_with(".hipmh.com") {
            return None;
        }
        path
    };
    let mut segments = path.split('/');
    segments.find(|segment| *segment == "works")?;
    let tail = segments.next()?;
    let key = tail.split(['/', '?', '#']).next()?;
    valid_key(key).then(|| String::from(key))
}

pub fn absolute_image(path: &str, base: &str) -> Option<String> {
    let path = path.trim();
    if path.is_empty() {
        None
    } else if path.starts_with("https://") || path.starts_with("http://") {
        Some(path.into())
    } else if path.starts_with("//") {
        Some(format!("https:{path}"))
    } else if path.contains(':') {
        None
    } else {
        Some(format!(
            "{}/{}",
            base.trim_end_matches('/'),
            path.trim_start_matches('/')
        ))
    }
}

pub fn chapter_api(base: &str) -> Option<String> {
    if !base.starts_with("https://") {
        return None;
    }
    let base = base.trim_end_matches('/');
    Some(if base.ends_with("/v2/chapter") {
        base.into()
    } else if base.ends_with("/v2") {
        format!("{base}/chapter")
    } else {
        format!("{}/v2/chapter", base.strip_suffix("/v1").unwrap_or(base))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_links_and_encodes_queries() {
        assert_eq!(
            manga_key("https://m.hipmh.com/zh/works/bTozMDM4Nw-title").as_deref(),
            Some("bTozMDM4Nw-title")
        );
        assert_eq!(
            manga_key("/zh/works/bTozMDM4Nw-title").as_deref(),
            Some("bTozMDM4Nw-title")
        );
        assert!(manga_key("//evil.test/works/id").is_none());
        assert_eq!(
            manga_key("https://m.hipmh.com/works/bTozMDM4Nw-title?from=search").as_deref(),
            Some("bTozMDM4Nw-title")
        );
        assert!(manga_key("https://hipmh.com.evil.test/works/id").is_none());
        assert!(manga_key("https://evil.test/works/id").is_none());
        assert!(valid_key("bTozMDM4Nw-title"));
        assert!(!valid_key("../id"));
        assert_eq!(urlencode("漫画 &"), "%E6%BC%AB%E7%94%BB%20%26");
    }

    #[test]
    fn normalizes_api_and_image_urls() {
        for base in [
            "https://api.test",
            "https://api.test/",
            "https://api.test/v1",
            "https://api.test/v2",
            "https://api.test/v2/chapter",
        ] {
            assert_eq!(
                chapter_api(base).as_deref(),
                Some("https://api.test/v2/chapter")
            );
        }
        assert!(chapter_api("javascript:alert(1)").is_none());
        assert_eq!(
            absolute_image("//cdn.test/a.webp", BASE_URL).as_deref(),
            Some("https://cdn.test/a.webp")
        );
        assert_eq!(
            absolute_image("/a.webp", "https://cdn.test/").as_deref(),
            Some("https://cdn.test/a.webp")
        );
        assert!(absolute_image("javascript:alert(1)", BASE_URL).is_none());
    }
}

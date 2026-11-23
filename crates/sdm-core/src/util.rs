use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Case-insensitive match where `*` stands for any run of characters.
pub fn wildcard_match(pattern: &str, text: &str) -> bool {
    let pattern: Vec<char> = pattern.to_lowercase().chars().collect();
    let text: Vec<char> = text.to_lowercase().chars().collect();
    let (mut p, mut t) = (0, 0);
    let (mut star, mut mark) = (None, 0);
    while t < text.len() {
        if p < pattern.len() && pattern[p] == '*' {
            star = Some(p);
            mark = t;
            p += 1;
        } else if p < pattern.len() && pattern[p] == text[t] {
            p += 1;
            t += 1;
        } else if let Some(s) = star {
            p = s + 1;
            mark += 1;
            t = mark;
        } else {
            return false;
        }
    }
    while p < pattern.len() && pattern[p] == '*' {
        p += 1;
    }
    p == pattern.len()
}

const RESERVED: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3",
    "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Windows-safe single file name; never empty.
pub fn sanitize_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if c.is_control() || "<>:\"/\\|?*".contains(c) { '_' } else { c })
        .collect();
    let cleaned = cleaned.trim_end_matches(['.', ' ']).trim_start().to_string();
    if cleaned.is_empty() {
        return "download".into();
    }
    let stem = cleaned.split('.').next().unwrap_or("").to_ascii_uppercase();
    if RESERVED.contains(&stem.as_str()) {
        format!("_{cleaned}")
    } else {
        cleaned
    }
}

/// Last non-empty path segment of a URL, percent-decoded and made safe.
pub fn name_from_url(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    let segment = parsed.path_segments()?.rev().find(|s| !s.is_empty())?;
    let decoded = percent_encoding::percent_decode_str(segment).decode_utf8_lossy().into_owned();
    Some(sanitize_name(&decoded))
}

/// How numbers in a batch link are padded with zeros.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatchPadding {
    /// As wide as the wider of start and end.
    Auto,
    /// No padding.
    None,
    /// At least this wide, and never narrower than Auto.
    Width(usize),
}

pub const BATCH_LIMIT: u64 = 1000;

/// Links from a pattern with `*`, one per number from `start` to `end`.
pub fn expand_batch(pattern: &str, start: u64, end: u64, padding: BatchPadding) -> Result<Vec<String>, String> {
    let pattern = pattern.trim();
    if !pattern.contains('*') {
        return Err("Put a * in the link where the number goes.".into());
    }
    if end < start {
        return Err("The last number must not be smaller than the first.".into());
    }
    if end - start + 1 > BATCH_LIMIT {
        return Err(format!("That makes more than {BATCH_LIMIT} links."));
    }
    let auto = start.to_string().len().max(end.to_string().len());
    let width = match padding {
        BatchPadding::Auto => auto,
        BatchPadding::None => 0,
        BatchPadding::Width(n) => n.max(auto),
    };
    let links: Vec<String> = (start..=end).map(|n| pattern.replace('*', &format!("{n:0width$}"))).collect();
    match url::Url::parse(&links[0]) {
        Ok(u) if matches!(u.scheme(), "http" | "https") => Ok(links),
        _ => Err("The first link is not a valid web address.".into()),
    }
}

pub fn truthy(value: &str) -> bool {
    matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_expands_and_pads() {
        let links = expand_batch("https://x/img*.jpg", 8, 10, BatchPadding::Auto).unwrap();
        assert_eq!(links, ["https://x/img08.jpg", "https://x/img09.jpg", "https://x/img10.jpg"]);
        let links = expand_batch("https://x/*.jpg", 8, 10, BatchPadding::None).unwrap();
        assert_eq!(links[0], "https://x/8.jpg");
        let links = expand_batch("https://x/*.jpg", 1, 2, BatchPadding::Width(3)).unwrap();
        assert_eq!(links, ["https://x/001.jpg", "https://x/002.jpg"]);
        let links = expand_batch("https://x/*.jpg", 1, 100, BatchPadding::Width(1)).unwrap();
        assert_eq!(links[0], "https://x/001.jpg");
        assert!(expand_batch("https://x/a.jpg", 1, 2, BatchPadding::Auto).is_err());
        assert!(expand_batch("https://x/*", 5, 4, BatchPadding::Auto).is_err());
        assert!(expand_batch("https://x/*", 0, 1000, BatchPadding::Auto).is_err());
        assert_eq!(expand_batch("https://x/*", 1, 1000, BatchPadding::Auto).unwrap().len(), 1000);
        assert!(expand_batch("ftp://x/*", 1, 2, BatchPadding::Auto).is_err());
    }

    #[test]
    fn wildcard() {
        assert!(wildcard_match("*.example.com/*", "https://a.example.com/x"));
        assert!(wildcard_match("https://a/*", "HTTPS://A/b"));
        assert!(!wildcard_match("https://a/*", "https://b/c"));
        assert!(wildcard_match("*", ""));
    }

    #[test]
    fn names() {
        assert_eq!(sanitize_name("a:b?.txt. "), "a_b_.txt");
        assert_eq!(sanitize_name("CON.txt"), "_CON.txt");
        assert_eq!(sanitize_name("..."), "download");
        assert_eq!(name_from_url("https://h/p/My%20File.zip?x=1").as_deref(), Some("My File.zip"));
        assert_eq!(name_from_url("https://h/"), None);
    }
}

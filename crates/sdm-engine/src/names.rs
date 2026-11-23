use percent_encoding::percent_decode_str;

const RESERVED: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

const MAX_NAME_CHARS: usize = 200;

/// Windows-safe file name: strips `<>:"/\|?*` and control characters, trailing dots
/// and spaces, and reserved device names.
pub fn sanitize_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter(|c| !matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') && !c.is_control())
        .collect();
    let trimmed = cleaned.trim_start_matches(' ').trim_end_matches(['.', ' ']);
    if trimmed.is_empty() {
        return "download".into();
    }
    let mut out = trimmed.to_string();
    let stem = out.split('.').next().unwrap_or("").trim_end();
    if RESERVED.iter().any(|r| r.eq_ignore_ascii_case(stem)) {
        out.insert(0, '_');
    }
    if out.chars().count() > MAX_NAME_CHARS {
        out = shorten(&out);
    }
    out
}

fn shorten(name: &str) -> String {
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 && name.len() - i <= 17 => (&name[..i], &name[i..]),
        _ => (name, ""),
    };
    let keep = MAX_NAME_CHARS.saturating_sub(ext.chars().count());
    let stem: String = stem.chars().take(keep).collect();
    let stem = stem.trim_end_matches(['.', ' ']);
    format!("{stem}{ext}")
}

/// File name from a Content-Disposition header (`filename*` first, then `filename`).
pub fn file_name_from_disposition(value: &str) -> Option<String> {
    let mut extended = None;
    let mut plain = None;
    for (key, val) in split_params(value) {
        match key.as_str() {
            "filename*" => extended = decode_ext_value(&val),
            "filename" => plain = Some(decode_rfc2047(&val)),
            _ => {}
        }
    }
    extended
        .or(plain)
        .map(|n| last_component(&n).trim().to_string())
        .filter(|n| !n.is_empty())
}

/// Last non-empty URL path segment, percent-decoded.
pub fn file_name_from_url(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    let seg = parsed.path_segments()?.rfind(|s| !s.is_empty())?;
    let decoded = percent_decode_str(seg).decode_utf8_lossy().into_owned();
    let decoded = decoded.trim().to_string();
    (!decoded.is_empty()).then_some(decoded)
}

/// Extension (without the dot) for a Content-Type, from a small table.
pub(crate) fn extension_for_type(content_type: &str) -> Option<&'static str> {
    let base = content_type.split(';').next()?.trim().to_ascii_lowercase();
    Some(match base.as_str() {
        "application/zip" | "application/x-zip-compressed" => "zip",
        "application/pdf" => "pdf",
        "video/mp4" => "mp4",
        "application/x-msdownload"
        | "application/vnd.microsoft.portable-executable"
        | "application/x-dosexec" => "exe",
        "application/x-msi" => "msi",
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "audio/mpeg" => "mp3",
        "application/json" => "json",
        "text/plain" => "txt",
        "application/x-7z-compressed" => "7z",
        "application/x-rar-compressed" | "application/vnd.rar" => "rar",
        "application/x-iso9660-image" => "iso",
        "application/vnd.android.package-archive" => "apk",
        _ => return None,
    })
}

/// `name_3.ext` for `name.ext`.
pub(crate) fn numbered(name: &str, n: u32) -> String {
    match name.rfind('.') {
        Some(i) if i > 0 => format!("{}_{}{}", &name[..i], n, &name[i..]),
        _ => format!("{name}_{n}"),
    }
}

fn last_component(name: &str) -> &str {
    name.rsplit(['/', '\\']).next().unwrap_or(name)
}

fn split_params(value: &str) -> Vec<(String, String)> {
    let mut pieces = Vec::new();
    let mut cur = String::new();
    let mut in_quote = false;
    let mut escaped = false;
    for c in value.chars() {
        if in_quote {
            cur.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_quote = false;
            }
        } else if c == '"' {
            in_quote = true;
            cur.push(c);
        } else if c == ';' {
            pieces.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
    }
    pieces.push(cur);
    pieces
        .into_iter()
        .filter_map(|p| {
            let (k, v) = p.split_once('=')?;
            Some((k.trim().to_ascii_lowercase(), unquote(v.trim())))
        })
        .collect()
}

fn unquote(v: &str) -> String {
    let Some(rest) = v.strip_prefix('"') else {
        return v.to_string();
    };
    let mut out = String::new();
    let mut chars = rest.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => break,
            '\\' => match chars.peek() {
                Some('"') | Some('\\') => out.push(chars.next().unwrap_or('\\')),
                _ => out.push('\\'),
            },
            _ => out.push(c),
        }
    }
    out
}

fn decode_ext_value(v: &str) -> Option<String> {
    let (charset, rest) = v.split_once('\'')?;
    let (_lang, encoded) = rest.split_once('\'')?;
    let bytes: Vec<u8> = percent_decode_str(encoded).collect();
    Some(decode_charset(charset, &bytes))
}

fn decode_charset(charset: &str, bytes: &[u8]) -> String {
    let cs = charset.split('*').next().unwrap_or("").trim().to_ascii_lowercase();
    match cs.as_str() {
        "iso-8859-1" | "latin1" | "us-ascii" | "ascii" => bytes.iter().map(|&b| b as char).collect(),
        _ => String::from_utf8_lossy(bytes).into_owned(),
    }
}

fn decode_rfc2047(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    let mut prev_encoded = false;
    while let Some(start) = rest.find("=?") {
        let (before, after) = rest.split_at(start);
        match parse_word(after) {
            Some((decoded, used)) => {
                if !(prev_encoded && before.trim().is_empty()) {
                    out.push_str(before);
                }
                out.push_str(&decoded);
                rest = &after[used..];
                prev_encoded = true;
            }
            None => {
                out.push_str(before);
                out.push_str("=?");
                rest = &after[2..];
                prev_encoded = false;
            }
        }
    }
    out.push_str(rest);
    out
}

fn parse_word(s: &str) -> Option<(String, usize)> {
    let body = s.strip_prefix("=?")?;
    let mut it = body.splitn(3, '?');
    let charset = it.next()?;
    let enc = it.next()?;
    let tail = it.next()?;
    let end = tail.find("?=")?;
    let text = &tail[..end];
    let bytes = match enc {
        "B" | "b" => base64_decode(text)?,
        "Q" | "q" => q_decode(text)?,
        _ => return None,
    };
    let used = 2 + charset.len() + 1 + enc.len() + 1 + text.len() + 2;
    Some((decode_charset(charset, &bytes), used))
}

fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut acc = 0u32;
    let mut bits = 0;
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            _ => return None,
        } as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(out)
}

fn q_decode(s: &str) -> Option<Vec<u8>> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'_' => out.push(b' '),
            b'=' => {
                let hex = s.get(i + 1..i + 3)?;
                out.push(u8::from_str_radix(hex, 16).ok()?);
                i += 2;
            }
            c => out.push(c),
        }
        i += 1;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_strips_bad_characters() {
        assert_eq!(sanitize_file_name("a<b>c:d\"e/f\\g|h?i*j.txt"), "abcdefghij.txt");
        assert_eq!(sanitize_file_name("tab\there\u{7}.bin"), "tabhere.bin");
    }

    #[test]
    fn sanitize_trims_dots_and_spaces() {
        assert_eq!(sanitize_file_name("  name. . "), "name");
        assert_eq!(sanitize_file_name(".."), "download");
        assert_eq!(sanitize_file_name(""), "download");
        assert_eq!(sanitize_file_name(".hidden"), ".hidden");
    }

    #[test]
    fn sanitize_reserved_names() {
        assert_eq!(sanitize_file_name("CON"), "_CON");
        assert_eq!(sanitize_file_name("nul.txt"), "_nul.txt");
        assert_eq!(sanitize_file_name("Com1.tar.gz"), "_Com1.tar.gz");
        assert_eq!(sanitize_file_name("console.txt"), "console.txt");
    }

    #[test]
    fn sanitize_long_names_keep_extension() {
        let long = format!("{}.mkv", "a".repeat(400));
        let out = sanitize_file_name(&long);
        assert_eq!(out.chars().count(), MAX_NAME_CHARS);
        assert!(out.ends_with(".mkv"));
    }

    #[test]
    fn disposition_plain_and_quoted() {
        assert_eq!(file_name_from_disposition("attachment; filename=a.bin").as_deref(), Some("a.bin"));
        assert_eq!(
            file_name_from_disposition("attachment; filename=\"my file name.zip\"").as_deref(),
            Some("my file name.zip")
        );
        assert_eq!(
            file_name_from_disposition("attachment; filename=\"semi;colon.txt\"").as_deref(),
            Some("semi;colon.txt")
        );
        assert_eq!(
            file_name_from_disposition("attachment; filename=\"say \\\"hi\\\".txt\"").as_deref(),
            Some("say \"hi\".txt")
        );
    }

    #[test]
    fn disposition_extended_wins() {
        let v = "attachment; filename=\"fallback.txt\"; filename*=UTF-8''%E2%82%AC%20rates.txt";
        assert_eq!(file_name_from_disposition(v).as_deref(), Some("\u{20ac} rates.txt"));
        let latin = "attachment; filename*=iso-8859-1'en'caf%E9.txt";
        assert_eq!(file_name_from_disposition(latin).as_deref(), Some("caf\u{e9}.txt"));
    }

    #[test]
    fn disposition_rfc2047() {
        assert_eq!(
            file_name_from_disposition("attachment; filename=\"=?UTF-8?B?w6ljb2xlLnR4dA==?=\"").as_deref(),
            Some("\u{e9}cole.txt")
        );
        assert_eq!(
            file_name_from_disposition("attachment; filename=\"=?UTF-8?Q?caf=C3=A9_menu.pdf?=\"").as_deref(),
            Some("caf\u{e9} menu.pdf")
        );
        assert_eq!(
            file_name_from_disposition("attachment; filename=\"=?UTF-8?Q?a_?= =?UTF-8?Q?b.txt?=\"").as_deref(),
            Some("a b.txt")
        );
    }

    #[test]
    fn disposition_strips_paths_and_rejects_empty() {
        assert_eq!(
            file_name_from_disposition("attachment; filename=\"C:\\temp\\x.exe\"").as_deref(),
            Some("x.exe")
        );
        assert_eq!(file_name_from_disposition("attachment; filename=\"../../etc/passwd\"").as_deref(), Some("passwd"));
        assert_eq!(file_name_from_disposition("inline"), None);
        assert_eq!(file_name_from_disposition("attachment; filename=\"\""), None);
    }

    #[test]
    fn url_names() {
        assert_eq!(file_name_from_url("https://x.test/a/b/file%20one.zip?q=1#f").as_deref(), Some("file one.zip"));
        assert_eq!(file_name_from_url("https://x.test/a/b/").as_deref(), Some("b"));
        assert_eq!(file_name_from_url("https://x.test/"), None);
        assert_eq!(file_name_from_url("not a url"), None);
    }

    #[test]
    fn numbering() {
        assert_eq!(numbered("a.tar.gz", 2), "a.tar_2.gz");
        assert_eq!(numbered("plain", 1), "plain_1");
        assert_eq!(numbered(".hidden", 1), ".hidden_1");
    }

    #[test]
    fn extensions() {
        assert_eq!(extension_for_type("application/zip; charset=x"), Some("zip"));
        assert_eq!(extension_for_type("IMAGE/JPEG"), Some("jpg"));
        assert_eq!(extension_for_type("text/html"), None);
    }
}

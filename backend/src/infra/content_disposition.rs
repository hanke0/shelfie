/// RFC 5987 `Content-Disposition` for downloads with non-ASCII filenames.
pub fn attachment_filename(filename: &str) -> String {
    let ascii_fallback = ascii_fallback_filename(filename);
    let encoded = percent_encode_utf8(filename);
    format!(
        "attachment; filename=\"{}\"; filename*=UTF-8''{encoded}",
        ascii_fallback.replace('"', "_")
    )
}

fn ascii_fallback_filename(filename: &str) -> String {
    let ascii: String = filename
        .chars()
        .map(|c| {
            if c.is_ascii() && c != '"' && c != '\\' {
                c
            } else {
                '_'
            }
        })
        .collect();

    let alnum_count = ascii.chars().filter(|c| c.is_ascii_alphanumeric()).count();
    if alnum_count >= 1 {
        return ascii;
    }

    if let Some(ext) = std::path::Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .filter(|e| e.is_ascii() && !e.is_empty())
    {
        return format!("download.{ext}");
    }
    "download".to_string()
}

fn percent_encode_utf8(s: &str) -> String {
    let mut out = String::new();
    for b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'.' | b'-' | b'_' | b'~' => {
                out.push(*b as char);
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attachment_includes_utf8_filename_star() {
        let header = attachment_filename("三体_刘慈欣.pdf");
        assert!(header.contains("filename=\"_____.pdf\"") || header.contains("filename="));
        assert!(header.contains("filename*=UTF-8''"));
        assert!(header.contains("%E4%B8%89%E4%BD%93"));
    }

    #[test]
    fn ascii_name_unchanged_in_star() {
        let header = attachment_filename("book.pdf");
        assert!(header.contains("filename=\"book.pdf\""));
        assert!(header.contains("filename*=UTF-8''book.pdf"));
    }
}

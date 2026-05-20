use crate::domain::auth::AuthUser;
use crate::domain::book::{self, OpdsBookRow};
use crate::domain::category;
use crate::domain::library;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use chrono::{DateTime, Utc};
use std::path::Path;
use uuid::Uuid;

const NAV_TYPE: &str = "application/atom+xml;profile=opds-catalog;kind=navigation";
const ACQ_TYPE: &str = "application/atom+xml;profile=opds-catalog;kind=acquisition";

pub fn resolve_base_url(
    configured: Option<&str>,
    forwarded_proto: Option<&str>,
    host: Option<&str>,
) -> String {
    if let Some(url) = configured.filter(|s| !s.is_empty()) {
        return url.trim_end_matches('/').to_string();
    }
    let proto = forwarded_proto.unwrap_or("http");
    let host = host.unwrap_or("localhost:8080");
    format!("{proto}://{host}")
}

pub fn absolute_url(base: &str, path: &str) -> String {
    if path.starts_with("http://") || path.starts_with("https://") {
        return path.to_string();
    }
    format!("{}{}", base.trim_end_matches('/'), path)
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn now_rfc3339() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn format_opds_updated(raw: &str) -> String {
    if let Ok(dt) = DateTime::parse_from_rfc3339(raw) {
        return dt.with_timezone(&Utc).format("%Y-%m-%dT%H:%M:%SZ").to_string();
    }
    now_rfc3339()
}

fn feed_header(id: &str, title: &str, self_href: &str, kind: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom" xmlns:opds="http://opds-spec.org/2010/catalog" xmlns:dc="http://purl.org/dc/terms/">
  <id>{id}</id>
  <title>{title}</title>
  <updated>{updated}</updated>
  <link rel="self" href="{self_href}" type="{feed_type}"/>
"#,
        id = xml_escape(id),
        title = xml_escape(title),
        updated = now_rfc3339(),
        self_href = xml_escape(self_href),
        feed_type = if kind == "acquisition" {
            ACQ_TYPE
        } else {
            NAV_TYPE
        },
    )
}

/// Link to another navigation catalog (e.g. a library).
fn nav_entry_to_navigation(title: &str, id: &str, subsection: &str, acquisition: Option<&str>) -> String {
    let mut xml = format!(
        r#"  <entry>
    <title>{title}</title>
    <id>{id}</id>
    <updated>{updated}</updated>
    <link rel="subsection" href="{subsection}" type="{nav_type}"/>
"#,
        title = xml_escape(title),
        id = xml_escape(id),
        updated = now_rfc3339(),
        subsection = xml_escape(subsection),
        nav_type = NAV_TYPE,
    );
    if let Some(acq) = acquisition {
        xml.push_str(&format!(
            r#"    <link rel="http://opds-spec.org/acquisition" href="{acq}" type="{acq_type}"/>
"#,
            acq = xml_escape(acq),
            acq_type = ACQ_TYPE,
        ));
    }
    xml.push_str("  </entry>\n");
    xml
}

/// Link directly to an acquisition feed (e.g. 全部图书 / category). Readest and OPDS 1.2
/// expect `subsection` with `kind=acquisition`, not `kind=navigation`.
fn nav_entry_to_acquisition_feed(title: &str, id: &str, acquisition_href: &str) -> String {
    format!(
        r#"  <entry>
    <title>{title}</title>
    <id>{id}</id>
    <updated>{updated}</updated>
    <link rel="subsection" href="{href}" type="{acq_type}"/>
  </entry>
"#,
        title = xml_escape(title),
        id = xml_escape(id),
        updated = now_rfc3339(),
        href = xml_escape(acquisition_href),
        acq_type = ACQ_TYPE,
    )
}

fn mime_for_ext(ext: &str) -> &'static str {
    match ext.to_ascii_lowercase().as_str() {
        "pdf" => "application/pdf",
        "mobi" => "application/x-mobipocket-ebook",
        _ => "application/epub+zip",
    }
}

fn file_ext(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("epub")
        .to_ascii_lowercase()
}

fn acquisition_entry(book: &OpdsBookRow, base: &str) -> String {
    let ext = file_ext(&book.book_file_path);
    let mime = mime_for_ext(&ext);
    let download = absolute_url(base, &format!("/api/v1/books/{}/download", book.id));
    let cover = absolute_url(base, &format!("/api/v1/assets/covers/{}", book.id));
    let cover_thumb = absolute_url(
        base,
        &format!("/api/v1/assets/covers/{}?size=thumb", book.id),
    );
    let author = if book.author.trim().is_empty() {
        "Unknown"
    } else {
        book.author.as_str()
    };
    let updated = format_opds_updated(&book.updated_at);

    let mut xml = format!(
        r#"  <entry>
    <title>{title}</title>
    <id>urn:uuid:{id}</id>
    <updated>{updated}</updated>
    <author><name>{author}</name></author>
    <category term="{category}" label="{category}"/>
    <link rel="http://opds-spec.org/acquisition" href="{download}" type="{mime}"/>
    <link rel="http://opds-spec.org/acquisition/open-access" href="{download}" type="{mime}"/>
    <link rel="http://opds-spec.org/image/thumbnail" href="{cover_thumb}" type="image/jpeg"/>
    <link rel="http://opds-spec.org/image" href="{cover}" type="image/jpeg"/>
"#,
        title = xml_escape(&book.title),
        id = xml_escape(&book.id),
        updated = xml_escape(&updated),
        author = xml_escape(author),
        category = xml_escape(&book.category),
        download = xml_escape(&download),
        cover = xml_escape(&cover),
        cover_thumb = xml_escape(&cover_thumb),
        mime = mime,
    );
    if let Some(lang) = book.language.as_deref().filter(|s| !s.is_empty()) {
        xml.push_str(&format!(
            "    <dc:language>{}</dc:language>\n",
            xml_escape(lang)
        ));
    }
    if let Some(summary) = book.summary.as_deref().filter(|s| !s.is_empty()) {
        xml.push_str(&format!("    <summary>{}</summary>\n", xml_escape(summary)));
    }
    xml.push_str("  </entry>\n");
    xml
}

pub async fn root_navigation_feed(
    state: &AppState,
    user: &AuthUser,
    base: &str,
) -> AppResult<String> {
    let libraries = library::list_libraries(&state.db, user).await?;
    let self_href = absolute_url(base, "/api/v1/opds");
    let mut xml = feed_header("urn:shelfie:opds:root", "Shelfie", &self_href, "navigation");
    for lib in libraries {
        let nav = absolute_url(base, &lib.opds_url);
        let acq = absolute_url(
            base,
            &format!("{}/books", lib.opds_url.trim_end_matches('/')),
        );
        xml.push_str(&nav_entry_to_navigation(
            &lib.name,
            &format!("urn:uuid:{}", lib.id),
            &nav,
            Some(&acq),
        ));
    }
    xml.push_str("</feed>\n");
    Ok(xml)
}

pub async fn library_navigation_feed(
    state: &AppState,
    user: &AuthUser,
    library_id: &Uuid,
    base: &str,
) -> AppResult<String> {
    let perm = library::resolve_permission(&state.db, user, library_id).await?;
    library::require_view(&perm)?;

    let name = library::get_library_name(&state.db, library_id).await?;
    let catalog_path = library::opds_catalog_path(&library_id.to_string());
    let self_href = absolute_url(base, &catalog_path);
    let all_books = absolute_url(base, &format!("{catalog_path}/books"));

    let mut xml = feed_header(
        &format!("urn:uuid:{library_id}:nav"),
        &name,
        &self_href,
        "navigation",
    );
    xml.push_str(&nav_entry_to_acquisition_feed(
        "全部图书",
        &format!("urn:uuid:{library_id}:all"),
        &all_books,
    ));

    let categories = category::list_categories(state, user, library_id).await?;
    for cat in categories {
        let encoded = urlencoding::encode(&cat.name);
        let cat_href = absolute_url(base, &format!("{catalog_path}/books?category={encoded}"));
        xml.push_str(&nav_entry_to_acquisition_feed(
            &cat.name,
            &format!("urn:uuid:{library_id}:cat:{encoded}"),
            &cat_href,
        ));
    }

    xml.push_str("</feed>\n");
    Ok(xml)
}

pub async fn library_acquisition_feed(
    state: &AppState,
    user: &AuthUser,
    library_id: &Uuid,
    category: Option<&str>,
    base: &str,
) -> AppResult<String> {
    let perm = library::resolve_permission(&state.db, user, library_id).await?;
    library::require_view(&perm)?;

    let name = library::get_library_name(&state.db, library_id).await?;
    let catalog_path = library::opds_catalog_path(&library_id.to_string());
    let mut self_path = format!("{catalog_path}/books");
    if let Some(cat) = category {
        self_path.push_str(&format!("?category={}", urlencoding::encode(cat)));
    }
    let self_href = absolute_url(base, &self_path);
    let up_href = absolute_url(base, &catalog_path);

    let title = match category {
        Some(c) => format!("{name} · {c}"),
        None => format!("{name} · 全部图书"),
    };

    let books = book::list_books_for_opds(&state.db, user, *library_id, category, 2000).await?;

    let mut xml = feed_header(
        &format!("urn:uuid:{library_id}:acq:{}", category.unwrap_or("all")),
        &title,
        &self_href,
        "acquisition",
    );
    xml.push_str(&format!(
        r#"  <link rel="up" href="{up_href}" type="{nav_type}"/>
  <link rel="start" href="{up_href}" type="{nav_type}"/>
"#,
        up_href = xml_escape(&up_href),
        nav_type = NAV_TYPE,
    ));
    for book in &books {
        xml.push_str(&acquisition_entry(book, base));
    }
    xml.push_str("</feed>\n");
    Ok(xml)
}

pub fn parse_library_id(id: &str) -> AppResult<Uuid> {
    Uuid::parse_str(id).map_err(|_| AppError::BadRequest("Invalid library id".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acquisition_nav_entry_uses_acquisition_kind() {
        let xml = nav_entry_to_acquisition_feed(
            "全部图书",
            "urn:uuid:test:all",
            "http://example.com/books",
        );
        assert!(xml.contains("kind=acquisition"));
        assert!(!xml.contains("kind=navigation"));
    }
}

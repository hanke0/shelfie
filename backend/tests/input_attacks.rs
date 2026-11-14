mod common;

use axum::http::{header, Method, StatusCode};
use common::{all_files, basic_auth, Part, TestApp};
use serde_json::json;
use shelfie_backend::infra::hash::md5_hex;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Every file the app created must live under the library root (plus the DB itself).
fn assert_confined(app: &TestApp, lib_root: &Path) {
    for file in all_files(&app.root) {
        let name = file.file_name().unwrap().to_string_lossy();
        if name.starts_with("test.db") {
            continue;
        }
        assert!(
            file.starts_with(lib_root),
            "file escaped library root: {}",
            file.display()
        );
    }
}

async fn setup() -> (TestApp, Uuid, PathBuf) {
    let app = TestApp::new().await;
    let lib = app.create_library("馆", "lib").await;
    let root = app.data_root().join("馆");
    (app, lib, root)
}

#[tokio::test]
async fn upload_rejects_traversal_categories() {
    let (app, lib, root) = setup().await;
    for category in [
        "..",
        ".",
        "../../etc",
        "../馆2",
        "a/b",
        "..\\..\\win",
        "C:\\Windows",
        "foo\0bar",
        "line\nbreak",
        "   ",
        "CON",
        "nul",
        "<script>",
        "a|b",
        "不存在的分类",
    ] {
        let res = app
            .upload_book(
                &app.admin.token,
                lib,
                category,
                "x.pdf",
                b"%PDF",
                json!({ "title": "T" }),
            )
            .await;
        assert_eq!(
            res.status,
            StatusCode::BAD_REQUEST,
            "category {category:?}: {}",
            res.text()
        );
    }
    assert_confined(&app, &root);
    assert!(all_files(&root).is_empty());
}

#[tokio::test]
async fn upload_filename_cannot_choose_location_or_dangerous_extension() {
    let (app, lib, root) = setup().await;

    for filename in [
        "shell.php",
        "x.pdf.exe",
        "page.html",
        "noext",
        "evil.svg",
        ".pdf",
    ] {
        let res = app
            .upload_book(
                &app.admin.token,
                lib,
                "未分类",
                filename,
                b"data",
                json!({ "title": filename }),
            )
            .await;
        assert_eq!(
            res.status,
            StatusCode::BAD_REQUEST,
            "{filename}: {}",
            res.text()
        );
    }

    let res = app
        .upload_book(
            &app.admin.token,
            lib,
            "未分类",
            "../../../../tmp/evil.PDF",
            b"%PDF-evil",
            json!({ "title": "Safe", "author": "A" }),
        )
        .await;
    assert_eq!(res.status, StatusCode::CREATED, "{}", res.text());
    let path = PathBuf::from(res.json()["book_file_path"].as_str().unwrap());
    assert_eq!(path, root.join("未分类").join("Safe_A.pdf"));
    assert_eq!(std::fs::read(&path).unwrap(), b"%PDF-evil");
    assert_confined(&app, &root);
}

#[tokio::test]
async fn traversal_in_title_and_author_is_sanitized() {
    let (app, lib, root) = setup().await;
    let res = app
        .upload_book(
            &app.admin.token,
            lib,
            "未分类",
            "a.epub",
            b"epub-bytes",
            json!({ "title": "../../../../etc/passwd", "author": "..\\..\\boot.ini" }),
        )
        .await;
    assert_eq!(res.status, StatusCode::CREATED, "{}", res.text());
    let body = res.json();
    assert_eq!(
        body["title"], "../../../../etc/passwd",
        "metadata keeps original text"
    );
    let path = PathBuf::from(body["book_file_path"].as_str().unwrap());
    assert_eq!(path.parent().unwrap(), root.join("未分类"));
    let name = path.file_name().unwrap().to_string_lossy().to_string();
    assert!(!name.contains('/') && !name.contains('\\'), "{name}");
    assert!(!name.starts_with('.'), "{name}");

    let id = body["id"].as_str().unwrap();
    let res = app
        .json(
            Method::PATCH,
            &format!("/api/v1/books/{id}"),
            &app.admin.token,
            json!({ "metadata": { "title": "/absolute/path", "author": "../x" } }),
        )
        .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.text());
    let path = PathBuf::from(res.json()["book_file_path"].as_str().unwrap());
    assert_eq!(path.parent().unwrap(), root.join("未分类"));
    assert_confined(&app, &root);
}

#[tokio::test]
async fn windows_reserved_base_name_is_replaced() {
    let (app, lib, root) = setup().await;
    let res = app
        .upload_book(
            &app.admin.token,
            lib,
            "未分类",
            "a.pdf",
            b"pdf",
            json!({ "title": "CON", "author": "" }),
        )
        .await;
    assert_eq!(res.status, StatusCode::CREATED);
    assert_eq!(
        PathBuf::from(res.json()["book_file_path"].as_str().unwrap()),
        root.join("未分类").join("untitled.pdf")
    );
}

#[tokio::test]
async fn invalid_metadata_fields_are_rejected() {
    let (app, lib, _) = setup().await;
    for meta in [
        json!({ "title": "line\nbreak" }),
        json!({ "title": "x".repeat(121) }),
        json!({ "title": "T", "author": "tab\tauthor" }),
        json!({ "title": "T", "rating": 9 }),
        json!({ "title": "T", "publish_date": "2024-13" }),
        json!({ "title": "T", "publish_date": "24-01-01" }),
    ] {
        let res = app
            .upload_book(
                &app.admin.token,
                lib,
                "未分类",
                "a.pdf",
                b"pdf",
                meta.clone(),
            )
            .await;
        assert_eq!(
            res.status,
            StatusCode::BAD_REQUEST,
            "{meta}: {}",
            res.text()
        );
    }
}

#[tokio::test]
async fn malformed_multipart_requests() {
    let (app, lib, _) = setup().await;
    let lib_s = lib.to_string();
    let file = Part::File {
        name: "file",
        filename: "a.pdf",
        content_type: "application/pdf",
        data: b"pdf",
    };

    let cases: Vec<(Vec<Part>, &str)> = vec![
        (
            vec![
                Part::Text("library_id", "not-a-uuid"),
                Part::Text("category", "未分类"),
            ],
            "bad uuid",
        ),
        (
            vec![
                Part::Text("category", "未分类"),
                Part::File {
                    name: "file",
                    filename: "a.pdf",
                    content_type: "application/pdf",
                    data: b"pdf",
                },
            ],
            "no library",
        ),
        (
            vec![
                Part::Text("library_id", &lib_s),
                Part::File {
                    name: "file",
                    filename: "a.pdf",
                    content_type: "application/pdf",
                    data: b"pdf",
                },
            ],
            "no category",
        ),
        (
            vec![
                Part::Text("library_id", &lib_s),
                Part::Text("category", "未分类"),
            ],
            "no file",
        ),
        (
            vec![
                Part::Text("library_id", &lib_s),
                Part::Text("category", "未分类"),
                Part::Text("metadata", "{not json"),
                file,
            ],
            "bad metadata",
        ),
        (
            vec![
                Part::Text("library_id", &lib_s),
                Part::Text("category", "未分类"),
                Part::File {
                    name: "file",
                    filename: "a.pdf",
                    content_type: "application/pdf",
                    data: b"pdf",
                },
                Part::File {
                    name: "cover",
                    filename: "c.svg",
                    content_type: "image/svg+xml",
                    data: b"<svg onload=alert(1)>",
                },
            ],
            "svg cover",
        ),
    ];
    for (parts, label) in cases {
        let res = app
            .multipart(Method::POST, "/api/v1/books", &app.admin.token, &parts)
            .await;
        assert_eq!(
            res.status,
            StatusCode::BAD_REQUEST,
            "{label}: {}",
            res.text()
        );
    }

    let res = app
        .send(
            Method::POST,
            "/api/v1/books",
            Some(&common::bearer(&app.admin.token)),
            Some("multipart/form-data; boundary=nope"),
            b"garbage".to_vec(),
            &[],
        )
        .await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn cover_update_rejects_non_image_extensions() {
    let (app, lib, root) = setup().await;
    let book = app.seed_book(lib, "Covered", b"pdf").await;
    for (filename, ct) in [
        ("x.svg", "image/svg+xml"),
        ("x.html", "text/html"),
        ("x", "application/octet-stream"),
        ("x.gif", "image/gif"),
    ] {
        let res = app
            .multipart(
                Method::PUT,
                &format!("/api/v1/books/{book}/cover"),
                &app.admin.token,
                &[Part::File {
                    name: "cover",
                    filename,
                    content_type: ct,
                    data: b"<svg/>",
                }],
            )
            .await;
        assert_eq!(res.status, StatusCode::BAD_REQUEST, "{filename}");
    }
    let res = app
        .multipart(
            Method::PUT,
            &format!("/api/v1/books/{book}/cover"),
            &app.admin.token,
            &[],
        )
        .await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert_confined(&app, &root);
}

#[tokio::test]
async fn category_and_library_names_reject_traversal() {
    let (app, lib, root) = setup().await;
    for name in ["..", "../escape", "a/b", "a\\b", "", "PRN", "x\u{7}"] {
        let res = app
            .json(
                Method::POST,
                &format!("/api/v1/libraries/{lib}/categories"),
                &app.admin.token,
                json!({ "name": name }),
            )
            .await;
        assert_eq!(res.status, StatusCode::BAD_REQUEST, "category {name:?}");
    }
    for encoded in ["..%2F..%2Fetc", "..", "%2E%2E", "a%5Cb"] {
        let res = app
            .delete(
                &format!("/api/v1/libraries/{lib}/categories/{encoded}"),
                &app.admin.token,
            )
            .await;
        assert!(
            res.status.is_client_error(),
            "delete {encoded}: {}",
            res.status
        );
    }

    for (name, slug) in [
        ("../evil", "evil"),
        ("a/b", "ab"),
        ("..", "dots"),
        ("AUX", "aux"),
        ("ok", "Bad Slug"),
        ("ok", "-leading"),
        ("ok", ""),
        ("ok", "../x"),
    ] {
        let res = app
            .json(
                Method::POST,
                "/api/v1/libraries",
                &app.admin.token,
                json!({ "name": name, "slug": slug }),
            )
            .await;
        assert_eq!(res.status, StatusCode::BAD_REQUEST, "{name}/{slug}");
    }
    let entries: Vec<_> = std::fs::read_dir(app.data_root())
        .unwrap()
        .flatten()
        .collect();
    assert_eq!(
        entries.len(),
        1,
        "only the original library directory exists"
    );
    assert_confined(&app, &root);
}

#[tokio::test]
async fn sql_injection_in_query_parameters_is_inert() {
    let (app, lib, _) = setup().await;
    app.seed_book(lib, "Alpha", b"a").await;
    app.seed_book(lib, "Beta", b"b").await;
    let token = &app.admin.token;

    for q in ["' OR 1=1 --", "\") OR 1=1 --", "x'; DROP TABLE books; --"] {
        let res = app
            .get(
                &format!("/api/v1/search?q={}", urlencoding::encode(q)),
                token,
            )
            .await;
        assert_eq!(res.status, StatusCode::OK);
        assert!(res.json().as_array().unwrap().is_empty(), "{q}");
    }
    for sort in [
        "uploaded_at;DROP TABLE books",
        "recent",
        "new",
        "1) UNION SELECT 1--",
    ] {
        let res = app
            .get(
                &format!("/api/v1/books?sort={}", urlencoding::encode(sort)),
                token,
            )
            .await;
        assert_eq!(res.status, StatusCode::OK, "{sort}");
        assert_eq!(res.json().as_array().unwrap().len(), 2, "{sort}");
    }
    let res = app
        .get(
            &format!(
                "/api/v1/books?category={}",
                urlencoding::encode("' OR '1'='1")
            ),
            token,
        )
        .await;
    assert!(res.json().as_array().unwrap().is_empty());

    let res = app
        .send(
            Method::GET,
            &format!(
                "/api/v1/opds/libraries/{lib}/books?category={}",
                urlencoding::encode("' OR '1'='1")
            ),
            Some(&basic_auth("admin", "admin-pass")),
            None,
            vec![],
            &[],
        )
        .await;
    assert_eq!(res.status, StatusCode::OK);
    assert!(!res.text().contains("<entry>"));

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM books")
        .fetch_one(&app.state.db)
        .await
        .unwrap();
    assert_eq!(count.0, 2);
}

#[tokio::test]
async fn result_limits_are_clamped() {
    let (app, lib, _) = setup().await;
    for i in 0..3 {
        app.seed_book(lib, &format!("Book{i}"), format!("data{i}").as_bytes())
            .await;
    }
    let token = &app.admin.token;
    for uri in [
        "/api/v1/search?q=Book&limit=-1",
        "/api/v1/books?limit=-1",
        "/api/v1/home?limit=-1",
    ] {
        let res = app.get(uri, token).await;
        assert_eq!(res.status, StatusCode::OK, "{uri}");
        let body = res.json();
        let list = if body.is_array() {
            body
        } else {
            body["new_arrivals"].clone()
        };
        assert_eq!(list.as_array().unwrap().len(), 1, "{uri}");
    }
}

#[tokio::test]
async fn client_supplied_file_hashes_are_ignored() {
    let (app, lib, _) = setup().await;
    let fake = "0".repeat(32);
    let res = app
        .upload_book(
            &app.admin.token,
            lib,
            "未分类",
            "a.pdf",
            b"real",
            json!({ "title": "H", "file_md5": fake }),
        )
        .await;
    assert_eq!(res.status, StatusCode::CREATED);
    let body = res.json();
    assert_eq!(body["metadata"]["file_md5"], md5_hex(b"real"));

    let res = app
        .json(
            Method::PATCH,
            &format!("/api/v1/books/{}", body["id"].as_str().unwrap()),
            &app.admin.token,
            json!({ "metadata": { "title": "H", "file_md5": fake, "file_sha256": "1".repeat(64) } }),
        )
        .await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.json()["metadata"]["file_md5"], md5_hex(b"real"));
    assert_ne!(res.json()["metadata"]["file_sha256"], "1".repeat(64));
}

#[tokio::test]
async fn duplicate_upload_is_rejected() {
    let (app, lib, _) = setup().await;
    app.seed_book(lib, "Once", b"same-bytes").await;
    let res = app
        .upload_book(
            &app.admin.token,
            lib,
            "未分类",
            "b.pdf",
            b"same-bytes",
            json!({ "title": "Twice" }),
        )
        .await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn opds_output_escapes_xml_and_host_header() {
    let (app, lib, _) = setup().await;
    app.seed_book(lib, "<script>alert(1)</script>&\"'", b"x")
        .await;
    let res = app
        .send(
            Method::GET,
            &format!("/api/v1/opds/libraries/{lib}/books"),
            Some(&basic_auth("admin", "admin-pass")),
            None,
            vec![],
            &[("host", "evil.example\"><injected/>")],
        )
        .await;
    assert_eq!(res.status, StatusCode::OK);
    assert!(res
        .headers
        .get(header::CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap()
        .contains("kind=acquisition"));
    let xml = res.text();
    assert!(!xml.contains("<script>"));
    assert!(xml.contains("&lt;script&gt;alert(1)&lt;/script&gt;&amp;&quot;&apos;"));
    assert!(!xml.contains("<injected/>"));
}

#[tokio::test]
async fn opds_requires_credentials() {
    let (app, lib, _) = setup().await;
    let res = app
        .send(
            Method::GET,
            &format!("/api/v1/opds/libraries/{lib}"),
            None,
            None,
            vec![],
            &[],
        )
        .await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
    assert!(res.headers.contains_key(header::WWW_AUTHENTICATE));
}

#[cfg(unix)]
#[tokio::test]
async fn maintenance_does_not_follow_symlinks_out_of_library() {
    let (app, lib, root) = setup().await;
    let outside = app.root.join("outside");
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("secret.pdf"), b"top-secret").unwrap();
    std::os::unix::fs::symlink(
        outside.join("secret.pdf"),
        root.join("未分类").join("link.pdf"),
    )
    .unwrap();
    std::os::unix::fs::symlink(&outside, root.join("linked-category")).unwrap();

    let params = json!({ "library_id": lib });
    let scan = app
        .run_task(&app.admin.token, "library_scan", params.clone())
        .await;
    assert!(
        scan["result"]["untracked_files"]
            .as_array()
            .unwrap()
            .is_empty(),
        "{scan}"
    );
    for tool in [
        "library_import_untracked",
        "library_delete_untracked",
        "library_clean_dangling",
    ] {
        let task = app.run_task(&app.admin.token, tool, params.clone()).await;
        assert_eq!(task["status"], "completed", "{task}");
    }
    let books = app.get("/api/v1/books", &app.admin.token).await.json();
    assert!(books.as_array().unwrap().is_empty());
    assert!(
        outside.join("secret.pdf").is_file(),
        "cleanup must not delete through symlinks"
    );
}

#[tokio::test]
async fn kosync_rejects_invalid_documents() {
    let app = TestApp::new().await;
    let key = md5_hex(b"admin-pass");
    let headers = [("x-auth-user", "admin"), ("x-auth-key", key.as_str())];
    let res = app
        .send(
            Method::GET,
            "/api/v1/syncs/progress/a:b",
            None,
            None,
            vec![],
            &headers,
        )
        .await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST);

    for body in [
        json!({ "document": "a:b", "progress": "1", "percentage": 0.1, "device": "kindle" }),
        json!({ "document": "", "progress": "1", "percentage": 0.1, "device": "kindle" }),
        json!({ "document": "doc", "progress": "", "percentage": 0.1, "device": "kindle" }),
        json!({ "document": "doc", "progress": "1", "percentage": 0.1, "device": "" }),
    ] {
        let res = app
            .send(
                Method::PUT,
                "/api/v1/syncs/progress",
                None,
                Some("application/json"),
                serde_json::to_vec(&body).unwrap(),
                &headers,
            )
            .await;
        assert_eq!(res.status, StatusCode::BAD_REQUEST, "{body}");
    }
}

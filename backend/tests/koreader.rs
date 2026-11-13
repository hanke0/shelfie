mod common;

use axum::http::{Method, StatusCode};
use common::{TestApp, TestUser};
use serde_json::json;
use shelfie_backend::infra::hash::md5_hex;

async fn kosync_put(
    app: &TestApp,
    user: &TestUser,
    document: &str,
    percentage: f64,
) -> common::TestResponse {
    let key = md5_hex(user.password.as_bytes());
    app.send(
        Method::PUT,
        "/api/v1/syncs/progress",
        None,
        Some("application/json"),
        serde_json::to_vec(&json!({
            "document": document,
            "progress": "12",
            "percentage": percentage,
            "device": "Kindle",
            "device_id": "dev-1",
            "metadata": { "foo": "bar" },
        }))
        .unwrap(),
        &[
            ("x-auth-user", &user.username),
            ("x-auth-key", &key),
            ("accept", "application/vnd.koreader.v1+json"),
        ],
    )
    .await
}

async fn kosync_get(app: &TestApp, user: &TestUser, document: &str) -> serde_json::Value {
    let key = md5_hex(user.password.as_bytes());
    let res = app
        .send(
            Method::GET,
            &format!("/api/v1/syncs/progress/{document}"),
            None,
            None,
            vec![],
            &[("x-auth-user", &user.username), ("x-auth-key", &key)],
        )
        .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.text());
    res.json()
}

#[tokio::test]
async fn progress_roundtrip_and_md5_autolink() {
    let app = TestApp::new().await;
    let lib = app.create_library("馆", "lib").await;
    let reader = app.create_user("reader", "pass-123", "user").await;
    app.add_member(lib, &reader, "member", (true, true, false))
        .await;
    let book = app.seed_book(lib, "Linked", b"ebook-content").await;
    let doc = md5_hex(b"ebook-content");

    let empty = kosync_get(&app, &reader, "unknown-doc").await;
    assert_eq!(empty, json!({ "document": "unknown-doc" }));

    let res = kosync_put(&app, &reader, &doc, 0.5).await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.text());
    assert_eq!(res.json()["document"], doc);

    let got = kosync_get(&app, &reader, &doc).await;
    assert_eq!(got["percentage"], 0.5);
    assert_eq!(got["progress"], "12");
    assert_eq!(got["device"], "Kindle");
    assert_eq!(got["device_id"], "dev-1");

    let detail = app
        .get(&format!("/api/v1/books/{book}"), &reader.token)
        .await
        .json();
    assert_eq!(detail["reading_percent"], 50.0);
    assert_eq!(detail["metadata"]["reading_progress"]["current_page"], 12);
    let history = app
        .get("/api/v1/users/me/reading-history", &reader.token)
        .await
        .json();
    assert_eq!(history[0]["source"], "koreader");

    let links = app
        .get("/api/v1/koreader/links", &reader.token)
        .await
        .json();
    assert_eq!(links[0]["book_id"], book.to_string());
    assert_eq!(links[0]["link_source"], "md5");

    let progress = app
        .get(
            &format!("/api/v1/koreader/progress?library_id={lib}"),
            &app.admin.token,
        )
        .await
        .json();
    assert_eq!(progress[0]["book_title"], "Linked");
    assert_eq!(progress[0]["username"], "reader");
}

#[tokio::test]
async fn md5_autolink_ignores_inaccessible_libraries() {
    let app = TestApp::new().await;
    let lib = app.create_library("馆", "lib").await;
    app.seed_book(lib, "Secret", b"secret-content").await;
    let stranger = app.create_user("stranger", "pass-123", "user").await;

    let res = kosync_put(&app, &stranger, &md5_hex(b"secret-content"), 0.3).await;
    assert_eq!(res.status, StatusCode::OK);
    assert!(app
        .get("/api/v1/koreader/links", &stranger.token)
        .await
        .json()
        .as_array()
        .unwrap()
        .is_empty());
    let rows = app
        .get("/api/v1/koreader/progress", &stranger.token)
        .await
        .json();
    assert!(rows[0]["book_id"].is_null());
}

#[tokio::test]
async fn manual_link_by_other_user_does_not_break_or_leak_to_outsiders() {
    let app = TestApp::new().await;
    let lib = app.create_library("馆", "lib").await;
    let book = app.seed_book(lib, "Private Title", b"content").await;
    let editor = app.create_user("editor", "pass-123", "user").await;
    app.add_member(lib, &editor, "member", (true, true, false))
        .await;
    let outsider = app.create_user("outsider", "pass-123", "user").await;
    let other_lib = app.create_library("别馆", "other").await;
    app.add_member(other_lib, &outsider, "member", (true, true, false))
        .await;

    // Both devices sync the same (unmatched) document before any link exists.
    assert_eq!(
        kosync_put(&app, &outsider, "shared-doc", 0.1).await.status,
        StatusCode::OK
    );
    assert_eq!(
        kosync_put(&app, &editor, "shared-doc", 0.2).await.status,
        StatusCode::OK
    );

    let res = app
        .json(
            Method::PUT,
            "/api/v1/koreader/links",
            &editor.token,
            json!({ "document": "shared-doc", "book_id": book }),
        )
        .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.text());
    assert_eq!(res.json()["link_source"], "manual");
    let detail = app
        .get(&format!("/api/v1/books/{book}"), &editor.token)
        .await
        .json();
    assert_eq!(detail["reading_percent"], 20.0);

    let rows = app
        .get("/api/v1/koreader/progress", &outsider.token)
        .await
        .json();
    assert!(
        rows[0]["book_title"].is_null(),
        "linking must not touch other users' rows: {rows}"
    );

    // The outsider keeps syncing normally and never sees the linked book.
    let res = kosync_put(&app, &outsider, "shared-doc", 0.4).await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.text());
    let rows = app
        .get("/api/v1/koreader/progress", &outsider.token)
        .await
        .json();
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0]["book_id"].is_null(), "{rows:?}");
    assert!(rows[0]["book_title"].is_null(), "{rows:?}");
}

#[tokio::test]
async fn link_management_permissions() {
    let app = TestApp::new().await;
    let lib = app.create_library("馆", "lib").await;
    let book = app.seed_book(lib, "Book", b"b").await;
    let viewer = app.create_user("viewer", "pass-123", "user").await;
    let editor = app.create_user("editor", "pass-123", "user").await;
    app.add_member(lib, &viewer, "member", (true, false, false))
        .await;
    app.add_member(lib, &editor, "member", (true, true, false))
        .await;

    for (body, status) in [
        (
            json!({ "document": "doc", "book_id": book }),
            StatusCode::FORBIDDEN,
        ),
        (
            json!({ "document": "a:b", "book_id": book }),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({ "document": "doc", "book_id": "nope" }),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({ "document": "doc", "book_id": uuid::Uuid::new_v4() }),
            StatusCode::NOT_FOUND,
        ),
    ] {
        let res = app
            .json(
                Method::PUT,
                "/api/v1/koreader/links",
                &viewer.token,
                body.clone(),
            )
            .await;
        assert_eq!(res.status, status, "{body}");
    }

    let res = app
        .json(
            Method::PUT,
            "/api/v1/koreader/links",
            &editor.token,
            json!({ "document": "doc", "book_id": book }),
        )
        .await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(
        app.get("/api/v1/koreader/links", &viewer.token)
            .await
            .json()
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        app.get(
            &format!("/api/v1/koreader/links?library_id={}", uuid::Uuid::new_v4()),
            &viewer.token
        )
        .await
        .json()
        .as_array()
        .unwrap()
        .len(),
        0
    );

    assert_eq!(
        app.delete("/api/v1/koreader/links/doc", &viewer.token)
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.delete("/api/v1/koreader/links/missing", &editor.token)
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.delete("/api/v1/koreader/links/doc", &editor.token)
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    assert!(app
        .get("/api/v1/koreader/links", &editor.token)
        .await
        .json()
        .as_array()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn view_only_member_progress_is_not_written_to_book() {
    let app = TestApp::new().await;
    let lib = app.create_library("馆", "lib").await;
    let book = app.seed_book(lib, "RO", b"ro-content").await;
    let viewer = app.create_user("viewer", "pass-123", "user").await;
    app.add_member(lib, &viewer, "member", (true, false, false))
        .await;

    assert_eq!(
        kosync_put(&app, &viewer, &md5_hex(b"ro-content"), 0.9)
            .await
            .status,
        StatusCode::OK
    );
    let detail = app
        .get(&format!("/api/v1/books/{book}"), &viewer.token)
        .await
        .json();
    assert!(detail["reading_percent"].is_null());
    assert_eq!(
        kosync_get(&app, &viewer, &md5_hex(b"ro-content")).await["percentage"],
        0.9
    );
}

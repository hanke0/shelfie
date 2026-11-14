mod common;

use axum::http::{Method, StatusCode};
use common::{basic_auth, Part, TestApp, TestUser};
use serde_json::json;
use uuid::Uuid;

struct Fixture {
    app: TestApp,
    lib_a: Uuid,
    lib_b: Uuid,
    book_a: Uuid,
    book_b: Uuid,
    viewer: TestUser,
    editor: TestUser,
    deleter: TestUser,
    lib_admin: TestUser,
    hidden: TestUser,
    outsider: TestUser,
}

async fn fixture() -> Fixture {
    let app = TestApp::new().await;
    let lib_a = app.create_library("馆A", "lib-a").await;
    let lib_b = app.create_library("馆B", "lib-b").await;
    let book_a = app.seed_book(lib_a, "BookA", b"%PDF-a").await;
    let book_b = app.seed_book(lib_b, "BookB", b"%PDF-b").await;

    let viewer = app.create_user("viewer", "pass-123", "user").await;
    let editor = app.create_user("editor", "pass-123", "user").await;
    let deleter = app.create_user("deleter", "pass-123", "user").await;
    let lib_admin = app.create_user("libadmin", "pass-123", "user").await;
    let hidden = app.create_user("hidden", "pass-123", "user").await;
    let outsider = app.create_user("outsider", "pass-123", "user").await;
    app.add_member(lib_a, &viewer, "member", (true, false, false))
        .await;
    app.add_member(lib_a, &editor, "member", (true, true, false))
        .await;
    app.add_member(lib_a, &deleter, "member", (true, true, true))
        .await;
    app.add_member(lib_a, &lib_admin, "admin", (false, false, false))
        .await;
    app.add_member(lib_a, &hidden, "member", (false, false, false))
        .await;
    app.add_member(lib_b, &outsider, "member", (true, true, true))
        .await;

    Fixture {
        app,
        lib_a,
        lib_b,
        book_a,
        book_b,
        viewer,
        editor,
        deleter,
        lib_admin,
        hidden,
        outsider,
    }
}

fn ids(v: &serde_json::Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|b| b["id"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn non_member_cannot_touch_foreign_library() {
    let f = fixture().await;
    let (app, a, book) = (&f.app, f.lib_a, f.book_a);
    let token = &f.outsider.token;

    for uri in [
        format!("/api/v1/books/{book}"),
        format!("/api/v1/books/{book}/download"),
        format!("/api/v1/assets/covers/{book}"),
        format!("/api/v1/books?library_id={a}"),
        format!("/api/v1/home?library_id={a}"),
        format!("/api/v1/libraries/{a}/categories"),
        format!("/api/v1/libraries/{a}/members"),
        format!("/api/v1/libraries/{a}/duplicates"),
        format!("/api/v1/opds/libraries/{a}"),
        format!("/api/v1/opds/libraries/{a}/books"),
    ] {
        let res = app.get(&uri, token).await;
        assert_eq!(
            res.status,
            StatusCode::FORBIDDEN,
            "GET {uri}: {}",
            res.text()
        );
    }

    let writes = [
        (
            Method::PATCH,
            format!("/api/v1/books/{book}"),
            json!({ "metadata": { "title": "pwned" } }),
        ),
        (
            Method::PATCH,
            format!("/api/v1/books/{book}/progress"),
            json!({ "reading_progress": { "percent": 50.0 } }),
        ),
        (Method::DELETE, format!("/api/v1/books/{book}"), json!(null)),
        (
            Method::POST,
            format!("/api/v1/libraries/{a}/categories"),
            json!({ "name": "x" }),
        ),
        (
            Method::DELETE,
            format!("/api/v1/libraries/{a}/categories/未分类"),
            json!(null),
        ),
        (
            Method::POST,
            "/api/v1/maintenance/tasks".to_string(),
            json!({ "tool": "library_scan", "params": { "library_id": a } }),
        ),
        (
            Method::POST,
            format!("/api/v1/libraries/{a}/members"),
            json!({ "user_id": f.outsider.id, "role": "member", "can_view": true }),
        ),
        (
            Method::PUT,
            "/api/v1/koreader/links".to_string(),
            json!({ "document": "doc", "book_id": book }),
        ),
    ];
    for (method, uri, body) in writes {
        let res = app.json(method.clone(), &uri, token, body).await;
        assert_eq!(
            res.status,
            StatusCode::FORBIDDEN,
            "{method} {uri}: {}",
            res.text()
        );
    }

    let res = app
        .upload_book(token, a, "未分类", "x.pdf", b"x", json!({ "title": "T" }))
        .await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);

    // Book still intact
    let res = app
        .get(&format!("/api/v1/books/{book}"), &app.admin.token)
        .await;
    assert_eq!(res.json()["title"], "BookA");
}

#[tokio::test]
async fn listings_are_scoped_to_accessible_libraries() {
    let f = fixture().await;
    let app = &f.app;

    let books = app.get("/api/v1/books", &f.outsider.token).await.json();
    assert_eq!(ids(&books), vec![f.book_b.to_string()]);

    let found = app
        .get("/api/v1/search?q=Book", &f.outsider.token)
        .await
        .json();
    assert_eq!(ids(&found), vec![f.book_b.to_string()]);

    let found = app
        .get(
            &format!("/api/v1/search?q=Book&library_id={}", f.lib_a),
            &f.outsider.token,
        )
        .await
        .json();
    assert!(found.as_array().unwrap().is_empty());

    let libs = app.get("/api/v1/libraries", &f.outsider.token).await.json();
    assert_eq!(ids(&libs), vec![f.lib_b.to_string()]);

    let home = app.get("/api/v1/home", &f.outsider.token).await.json();
    assert_eq!(ids(&home["new_arrivals"]), vec![f.book_b.to_string()]);

    let opds = app
        .send(
            Method::GET,
            "/api/v1/opds",
            Some(&basic_auth("outsider", "pass-123")),
            None,
            vec![],
            &[],
        )
        .await;
    assert_eq!(opds.status, StatusCode::OK);
    let xml = opds.text();
    assert!(xml.contains("馆B"));
    assert!(!xml.contains("馆A"));

    let all = app.get("/api/v1/books", &app.admin.token).await.json();
    assert_eq!(all.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn member_without_view_flag_is_treated_as_outsider() {
    let f = fixture().await;
    let app = &f.app;
    let res = app
        .get(&format!("/api/v1/books/{}", f.book_a), &f.hidden.token)
        .await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);
    assert!(app
        .get("/api/v1/libraries", &f.hidden.token)
        .await
        .json()
        .as_array()
        .unwrap()
        .is_empty());
    assert!(app
        .get("/api/v1/books", &f.hidden.token)
        .await
        .json()
        .as_array()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn viewer_is_read_only() {
    let f = fixture().await;
    let (app, a, book) = (&f.app, f.lib_a, f.book_a);
    let token = &f.viewer.token;

    let res = app.get(&format!("/api/v1/books/{book}"), token).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(
        res.json()["permissions"],
        json!({ "can_view": true, "can_edit": false, "can_delete": false })
    );

    let res = app
        .get(&format!("/api/v1/books/{book}/download"), token)
        .await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.body, b"%PDF-a");

    for (method, uri, body) in [
        (
            Method::PATCH,
            format!("/api/v1/books/{book}"),
            json!({ "metadata": { "title": "x" } }),
        ),
        (Method::DELETE, format!("/api/v1/books/{book}"), json!(null)),
        (
            Method::POST,
            format!("/api/v1/libraries/{a}/categories"),
            json!({ "name": "新分类" }),
        ),
        (
            Method::POST,
            "/api/v1/maintenance/tasks".to_string(),
            json!({ "tool": "library_scan", "params": { "library_id": a } }),
        ),
        (
            Method::POST,
            "/api/v1/books/batch-move-category".to_string(),
            json!({ "library_id": a, "book_ids": [book], "category": "未分类" }),
        ),
    ] {
        let res = app.json(method.clone(), &uri, token, body).await;
        assert_eq!(res.status, StatusCode::FORBIDDEN, "{method} {uri}");
    }

    let png = common::tiny_png();
    let res = app
        .multipart(
            Method::PUT,
            &format!("/api/v1/books/{book}/cover"),
            token,
            &[Part::File {
                name: "cover",
                filename: "c.png",
                content_type: "image/png",
                data: &png,
            }],
        )
        .await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);

    let res = app
        .upload_book(token, a, "未分类", "x.pdf", b"x", json!({ "title": "T" }))
        .await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn edit_and_delete_permissions_are_independent() {
    let f = fixture().await;
    let (app, book) = (&f.app, f.book_a);

    let res = app
        .json(
            Method::PATCH,
            &format!("/api/v1/books/{book}"),
            &f.editor.token,
            json!({ "metadata": { "title": "Edited", "author": "Someone" } }),
        )
        .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.text());

    let res = app
        .delete(&format!("/api/v1/books/{book}"), &f.editor.token)
        .await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);

    let res = app
        .delete(&format!("/api/v1/books/{book}"), &f.deleter.token)
        .await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    let res = app
        .get(&format!("/api/v1/books/{book}"), &app.admin.token)
        .await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn batch_move_cannot_pull_books_from_other_library() {
    let f = fixture().await;
    let app = &f.app;
    let res = app
        .json(
            Method::POST,
            "/api/v1/books/batch-move-category",
            &f.editor.token,
            json!({ "library_id": f.lib_a, "book_ids": [f.book_b, f.book_a], "category": "未分类" }),
        )
        .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.text());
    let body = res.json();
    assert_eq!(body["moved"], 1);
    assert_eq!(body["failures"][0]["book_id"], f.book_b.to_string());

    let b = app
        .get(&format!("/api/v1/books/{}", f.book_b), &app.admin.token)
        .await
        .json();
    assert_eq!(b["library_id"], f.lib_b.to_string());
    assert!(b["book_file_path"].as_str().unwrap().contains("馆B"));
}

#[tokio::test]
async fn library_admin_member_management_limits() {
    let f = fixture().await;
    let (app, a) = (&f.app, f.lib_a);
    let token = &f.lib_admin.token;
    let newbie = app.create_user("newbie", "pass-123", "user").await;
    let other_admin = app.create_user("otheradmin", "pass-123", "user").await;
    app.add_member(a, &other_admin, "admin", (true, true, true))
        .await;

    let members = format!("/api/v1/libraries/{a}/members");
    let res = app
        .json(
            Method::POST,
            &members,
            token,
            json!({ "username": "newbie", "role": "member", "can_view": true }),
        )
        .await;
    assert_eq!(res.status, StatusCode::NO_CONTENT, "{}", res.text());

    let res = app
        .json(
            Method::POST,
            &members,
            token,
            json!({ "username": "newbie", "role": "admin" }),
        )
        .await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);

    let res = app
        .json(
            Method::POST,
            &members,
            token,
            json!({ "username": "ghost", "role": "member" }),
        )
        .await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);

    let res = app
        .json(Method::POST, &members, token, json!({ "role": "member" }))
        .await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST);

    let res = app
        .json(
            Method::POST,
            &members,
            token,
            json!({ "username": "newbie", "role": "owner" }),
        )
        .await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST);

    // Re-adding an existing library admin as "member" would silently demote them.
    let res = app
        .json(
            Method::POST,
            &members,
            token,
            json!({ "user_id": other_admin.id, "role": "member", "can_view": false }),
        )
        .await;
    assert_eq!(res.status, StatusCode::FORBIDDEN, "{}", res.text());

    let res = app
        .json(
            Method::PATCH,
            &format!("{members}/{}/permissions", newbie.id),
            token,
            json!({ "role": "member", "can_view": true, "can_edit": true, "can_delete": true }),
        )
        .await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);

    let res = app
        .delete(&format!("{members}/{}", other_admin.id), token)
        .await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);

    let res = app
        .delete(&format!("{members}/{}", f.lib_admin.id), token)
        .await;
    assert_eq!(
        res.status,
        StatusCode::FORBIDDEN,
        "admin removing self hits the admin guard first"
    );

    let res = app.delete(&format!("{members}/{}", newbie.id), token).await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);

    let res = app
        .delete(&format!("{members}/{}", Uuid::new_v4()), token)
        .await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);

    let list = app.get(&members, &app.admin.token).await.json();
    let admin_row = list
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["user_id"] == other_admin.id.to_string())
        .unwrap();
    assert_eq!(admin_row["role"], "admin");
}

#[tokio::test]
async fn regular_member_cannot_manage_members_and_sees_only_self() {
    let f = fixture().await;
    let (app, a) = (&f.app, f.lib_a);
    let members = format!("/api/v1/libraries/{a}/members");

    let list = app.get(&members, &f.viewer.token).await.json();
    let list = list.as_array().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["username"], "viewer");

    let res = app
        .json(
            Method::POST,
            &members,
            &f.deleter.token,
            json!({ "user_id": f.outsider.id, "role": "member" }),
        )
        .await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);

    let res = app
        .delete(&format!("{members}/{}", f.viewer.id), &f.deleter.token)
        .await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);

    let full = app.get(&members, &f.lib_admin.token).await.json();
    assert_eq!(full.as_array().unwrap().len(), 5);
}

#[tokio::test]
async fn system_admin_updates_member_permissions() {
    let f = fixture().await;
    let (app, a) = (&f.app, f.lib_a);
    let uri = format!("/api/v1/libraries/{a}/members/{}/permissions", f.viewer.id);
    let res = app
        .json(
            Method::PATCH,
            &uri,
            &app.admin.token,
            json!({ "role": "member", "can_view": true, "can_edit": true, "can_delete": false }),
        )
        .await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    let res = app
        .json(
            Method::PATCH,
            &format!("/api/v1/books/{}", f.book_a),
            &f.viewer.token,
            json!({ "metadata": { "title": "ok" } }),
        )
        .await;
    assert_eq!(res.status, StatusCode::OK);

    let res = app
        .json(
            Method::PATCH,
            &format!(
                "/api/v1/libraries/{a}/members/{}/permissions",
                Uuid::new_v4()
            ),
            &app.admin.token,
            json!({ "role": "member", "can_view": true, "can_edit": false, "can_delete": false }),
        )
        .await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);

    let res = app
        .json(
            Method::PATCH,
            &uri,
            &app.admin.token,
            json!({ "role": "root", "can_view": true, "can_edit": true, "can_delete": true }),
        )
        .await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn memberships_visible_only_to_self_or_admin() {
    let f = fixture().await;
    let app = &f.app;
    let res = app
        .get(
            &format!("/api/v1/users/{}/memberships", f.viewer.id),
            &f.viewer.token,
        )
        .await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.json()[0]["library_name"], "馆A");

    let res = app
        .get(
            &format!("/api/v1/users/{}/memberships", f.editor.id),
            &f.viewer.token,
        )
        .await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);

    let res = app
        .get(
            &format!("/api/v1/users/{}/memberships", f.editor.id),
            &app.admin.token,
        )
        .await;
    assert_eq!(res.status, StatusCode::OK);
}

#[tokio::test]
async fn maintenance_tasks_are_scoped_by_library_permission() {
    let f = fixture().await;
    let app = &f.app;
    let task = app
        .run_task(
            &f.outsider.token,
            "library_scan",
            json!({ "library_id": f.lib_b }),
        )
        .await;
    assert_eq!(task["status"], "completed", "{task}");
    let id = task["id"].as_str().unwrap();

    // members of another library cannot see or cancel it
    let res = app
        .get(&format!("/api/v1/maintenance/tasks/{id}"), &f.editor.token)
        .await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
    let res = app
        .json(
            Method::POST,
            &format!("/api/v1/maintenance/tasks/{id}/cancel"),
            &f.editor.token,
            json!(null),
        )
        .await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
    let list = app
        .get("/api/v1/maintenance/tasks", &f.editor.token)
        .await
        .json();
    assert!(list.as_array().unwrap().is_empty());
    let list = app
        .get("/api/v1/maintenance/tasks", &app.admin.token)
        .await
        .json();
    assert_eq!(ids(&list), vec![id.to_string()]);

    // finished tasks cannot be cancelled
    let res = app
        .json(
            Method::POST,
            &format!("/api/v1/maintenance/tasks/{id}/cancel"),
            &f.outsider.token,
            json!(null),
        )
        .await;
    assert_eq!(res.status, StatusCode::CONFLICT);

    // destructive tools need delete permission
    let res = app
        .json(
            Method::POST,
            "/api/v1/maintenance/tasks",
            &f.editor.token,
            json!({ "tool": "library_purge_missing", "params": { "library_id": f.lib_a } }),
        )
        .await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);
    let task = app
        .run_task(
            &f.deleter.token,
            "library_purge_missing",
            json!({ "library_id": f.lib_a }),
        )
        .await;
    assert_eq!(task["status"], "completed", "{task}");

    let res = app
        .json(
            Method::POST,
            "/api/v1/maintenance/tasks",
            &f.editor.token,
            json!({ "tool": "library_scan", "params": {} }),
        )
        .await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn malformed_and_unknown_ids() {
    let f = fixture().await;
    let app = &f.app;
    for uri in [
        "/api/v1/books/not-a-uuid",
        "/api/v1/books/1%20OR%201=1",
        "/api/v1/opds/libraries/xyz",
    ] {
        let res = app.get(uri, &app.admin.token).await;
        assert_eq!(res.status, StatusCode::BAD_REQUEST, "{uri}");
    }
    let missing = Uuid::new_v4();
    for uri in [
        format!("/api/v1/books/{missing}"),
        format!("/api/v1/books/{missing}/download"),
        format!("/api/v1/assets/covers/{missing}"),
        format!("/api/v1/libraries/{missing}/categories"),
    ] {
        let res = app.get(&uri, &app.admin.token).await;
        assert_eq!(res.status, StatusCode::NOT_FOUND, "{uri}");
    }
}

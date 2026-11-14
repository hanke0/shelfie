mod common;

use axum::http::{Method, StatusCode};
use common::{TestApp, TestUser};
use serde_json::{json, Value};
use shelfie_backend::infra::hash::{md5_hex, partial_md5_hex};
use uuid::Uuid;

const POS: &str = "/body/DocFragment[3]/body/p[2]/text().0";
const POS_END: &str = "/body/DocFragment[3]/body/p[2]/text().12";
const DT: &str = "2026-01-02 03:04:05";

struct Device<'a> {
    app: &'a TestApp,
    user: &'a TestUser,
    id: &'static str,
}

impl Device<'_> {
    async fn post(&self, path: &str, body: Value) -> Value {
        let key = md5_hex(self.user.password.as_bytes());
        let mut body = body;
        body["deviceId"] = json!(self.id);
        body["deviceModel"] = json!("Readest");
        body["pluginVersion"] = json!("readest-1");
        body["deviceTime"] = json!(device_time(0));
        let res = self
            .app
            .send(
                Method::POST,
                &format!("/api/v1/koreader/plugin/{path}"),
                None,
                Some("application/json"),
                serde_json::to_vec(&body).unwrap(),
                &[("x-auth-user", &self.user.username), ("x-auth-key", &key)],
            )
            .await;
        assert_eq!(res.status, StatusCode::OK, "{path}: {}", res.text());
        res.json()
    }
}

fn device_time(offset_secs: i64) -> String {
    (chrono::Utc::now() + chrono::Duration::seconds(offset_secs))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

fn key(dt: &str, pos: &str) -> String {
    md5_hex(format!("{dt}|{pos}").as_bytes())
}

async fn setup(app: &TestApp) -> (TestUser, Uuid, String) {
    let lib = app.create_library("馆", "lib").await;
    let reader = app.create_user("reader", "pass-123", "user").await;
    app.add_member(lib, &reader, "member", (true, true, false))
        .await;
    let content: Vec<u8> = (0..20_000u32).map(|i| (i % 251) as u8).collect();
    let book = app.seed_book(lib, "Orbit", &content).await;
    (reader, book, partial_md5_hex(&content))
}

fn metadata_file(app: &TestApp) -> Value {
    let path = common::all_files(app.data_root())
        .into_iter()
        .find(|p| p.extension().is_some_and(|e| e == "json"))
        .expect("metadata.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn annotation(note: &str, updated: Option<String>) -> Value {
    json!({
        "datetime": DT,
        "datetimeUpdated": updated,
        "drawer": "lighten",
        "color": "yellow",
        "text": "highlighted words",
        "note": note,
        "chapter": "Chapter 3",
        "pageno": 12,
        "posFormat": "xpointer",
        "pos0": POS,
        "pos1": POS_END,
    })
}

fn to_apply_len(result: &Value) -> usize {
    let t = &result["results"][0]["toApply"];
    ["add", "edit", "delete"]
        .iter()
        .map(|k| t[k].as_array().map_or(0, Vec::len))
        .sum()
}

#[tokio::test]
async fn annotations_sync_between_devices() {
    let app = TestApp::new().await;
    let (reader, book, doc) = setup(&app).await;
    let a = Device {
        app: &app,
        user: &reader,
        id: "device-a",
    };
    let b = Device {
        app: &app,
        user: &reader,
        id: "device-b",
    };
    let k = key(DT, POS);
    let keys = json!([{ "k": k, "dt": DT }]);

    let res = a
        .post(
            "annotations/exchange",
            json!({ "books": [{ "hash": doc, "keys": keys, "keysComplete": true,
                                "changes": [annotation("first", None)] }] }),
        )
        .await;
    assert_eq!(res["results"][0]["bookId"], book.to_string());
    assert_eq!(to_apply_len(&res), 0, "{res}");

    let listed = app
        .get(&format!("/api/v1/books/{book}/annotations"), &reader.token)
        .await
        .json();
    assert_eq!(listed["annotations"][0]["note"], "first");
    assert_eq!(listed["annotations"][0]["text"], "highlighted words");
    assert_eq!(listed["annotations"][0]["pageno"], 12);
    let meta = metadata_file(&app);
    assert_eq!(meta["annotations"][0]["username"], "reader");
    assert_eq!(meta["annotations"][0]["note"], "first");
    let detail = app
        .get(&format!("/api/v1/books/{book}"), &reader.token)
        .await
        .json();
    assert!(detail["metadata"].get("annotations").is_none(), "{detail}");

    // B pulls the new annotation and acks it
    let res = b
        .post(
            "annotations/exchange",
            json!({ "books": [{ "hash": doc, "keys": [], "keysComplete": true, "changes": [] }] }),
        )
        .await;
    let add = &res["results"][0]["toApply"]["add"][0];
    assert_eq!(add["note"], "first");
    assert_eq!(add["pos0"], POS);
    assert_eq!(add["datetime"], DT);
    let server_id = add["serverId"].clone();
    let res = b
        .post(
            "annotations/exchange-ack",
            json!({ "books": [{ "hash": doc, "deleted": [], "applied": [
                { "serverId": server_id, "version": 1, "status": "applied", "verified": true }
            ] }] }),
        )
        .await;
    assert_eq!(res["acked"], 1);
    let res = b
        .post(
            "annotations/exchange",
            json!({ "books": [{ "hash": doc, "keys": keys, "keysComplete": true, "changes": [] }] }),
        )
        .await;
    assert_eq!(to_apply_len(&res), 0, "{res}");

    // A edits the note; B receives it as an edit
    let res = a
        .post(
            "annotations/exchange",
            json!({ "books": [{ "hash": doc, "keys": keys, "keysComplete": true,
                                "changes": [annotation("second", Some(device_time(2)))] }] }),
        )
        .await;
    assert_eq!(to_apply_len(&res), 0, "{res}");
    let res = b
        .post(
            "annotations/exchange",
            json!({ "books": [{ "hash": doc, "keys": keys, "keysComplete": true, "changes": [] }] }),
        )
        .await;
    let edit = &res["results"][0]["toApply"]["edit"][0];
    assert_eq!(edit["note"], "second");
    assert_eq!(edit["key"], k);
    assert_eq!(edit["version"], 2);
    b.post(
        "annotations/exchange-ack",
        json!({ "books": [{ "hash": doc, "deleted": [], "applied": [
            { "serverId": server_id, "version": 2, "status": "applied" }
        ] }] }),
    )
    .await;
    assert_eq!(metadata_file(&app)["annotations"][0]["note"], "second");

    // A deletes it (key disappears from the complete list); B is told to delete
    a.post(
        "annotations/exchange",
        json!({ "books": [{ "hash": doc, "keys": [], "keysComplete": true, "changes": [] }] }),
    )
    .await;
    let listed = app
        .get(&format!("/api/v1/books/{book}/annotations"), &reader.token)
        .await
        .json();
    assert_eq!(listed["annotations"], json!([]));
    assert!(metadata_file(&app).get("annotations").is_none());
    let res = b
        .post(
            "annotations/exchange",
            json!({ "books": [{ "hash": doc, "keys": keys, "keysComplete": true, "changes": [] }] }),
        )
        .await;
    assert_eq!(res["results"][0]["toApply"]["delete"][0]["key"], k);
    let res = b
        .post(
            "annotations/exchange-ack",
            json!({ "books": [{ "hash": doc, "applied": [],
                                "deleted": [{ "serverId": server_id, "status": "applied" }] }] }),
        )
        .await;
    assert_eq!(res["acked"], 1);
    let res = b
        .post(
            "annotations/exchange",
            json!({ "books": [{ "hash": doc, "keys": [], "keysComplete": true, "changes": [] }] }),
        )
        .await;
    assert_eq!(to_apply_len(&res), 0, "{res}");

    // re-pushing the deleted annotation does not resurrect it
    a.post(
        "annotations/exchange",
        json!({ "books": [{ "hash": doc, "keys": keys, "keysComplete": true,
                            "changes": [annotation("second", None)] }] }),
    )
    .await;
    let listed = app
        .get(&format!("/api/v1/books/{book}/annotations"), &reader.token)
        .await
        .json();
    assert_eq!(listed["annotations"], json!([]));
}

#[tokio::test]
async fn bookmarks_sync_between_devices() {
    let app = TestApp::new().await;
    let (reader, book, doc) = setup(&app).await;
    let a = Device {
        app: &app,
        user: &reader,
        id: "device-a",
    };
    let b = Device {
        app: &app,
        user: &reader,
        id: "device-b",
    };

    let key_hdr = md5_hex(reader.password.as_bytes());
    let version = app
        .send(
            Method::GET,
            "/api/v1/koreader/plugin/version",
            None,
            None,
            vec![],
            &[("x-auth-user", &reader.username), ("x-auth-key", &key_hdr)],
        )
        .await
        .json();
    assert_eq!(version["capabilities"], json!(["bookmarkSync"]));

    let res = a
        .post(
            "bookmarks/exchange",
            json!({ "books": [{ "hash": doc, "keys": [{ "k": key(DT, POS), "dt": DT }],
                "keysComplete": true,
                "changes": [{ "datetime": DT, "pos": POS, "pageno": 5, "chapter": "Ch 1" }] }] }),
        )
        .await;
    assert_eq!(res["results"][0]["toApply"]["add"], json!([]));
    let listed = app
        .get(&format!("/api/v1/books/{book}/annotations"), &reader.token)
        .await
        .json();
    assert_eq!(listed["bookmarks"][0]["title"], "Ch 1");
    assert_eq!(listed["bookmarks"][0]["pageno"], 5);
    assert_eq!(metadata_file(&app)["bookmarks"][0]["pos"], POS);

    let res = b
        .post(
            "bookmarks/exchange",
            json!({ "books": [{ "hash": doc, "keys": [], "keysComplete": true, "changes": [] }] }),
        )
        .await;
    let add = &res["results"][0]["toApply"]["add"][0];
    assert_eq!(add["title"], "Ch 1");
    let server_id = add["serverId"].clone();
    let b_dt = "2026-02-03 04:05:06";
    let b_key = key(b_dt, POS);
    let res = b
        .post(
            "bookmarks/exchange-ack",
            json!({ "books": [{ "hash": doc, "deleted": [], "applied": [
                { "serverId": server_id, "status": "applied", "key": b_key, "datetime": b_dt, "pos": POS }
            ] }] }),
        )
        .await;
    assert_eq!(res["acked"], 1);
    let b_keys = json!([{ "k": b_key, "dt": b_dt }]);
    let res = b
        .post(
            "bookmarks/exchange",
            json!({ "books": [{ "hash": doc, "keys": b_keys, "keysComplete": true, "changes": [] }] }),
        )
        .await;
    assert_eq!(res["results"][0]["toApply"]["add"], json!([]));

    a.post(
        "bookmarks/exchange",
        json!({ "books": [{ "hash": doc, "keys": [], "keysComplete": true, "changes": [] }] }),
    )
    .await;
    let res = b
        .post(
            "bookmarks/exchange",
            json!({ "books": [{ "hash": doc, "keys": b_keys, "keysComplete": true, "changes": [] }] }),
        )
        .await;
    assert_eq!(res["results"][0]["toApply"]["delete"][0]["key"], b_key);
    let listed = app
        .get(&format!("/api/v1/books/{book}/annotations"), &reader.token)
        .await
        .json();
    assert_eq!(listed["bookmarks"], json!([]));
}

#[tokio::test]
async fn page_stats_dedup_and_book_states() {
    let app = TestApp::new().await;
    let (reader, book, doc) = setup(&app).await;
    let a = Device {
        app: &app,
        user: &reader,
        id: "device-a",
    };
    let unknown = "0123456789abcdef0123456789abcdef";

    let stats = json!({ "books": [
        { "hash": doc, "events": [
            { "page": 1, "startTime": 1_700_000_000, "durationSeconds": 30, "totalPages": 100 },
            { "page": 2, "startTime": 1_700_000_040, "durationSeconds": 25, "totalPages": 100 },
        ] },
        { "hash": unknown, "events": [] },
    ] });
    let res = a.post("page-stats", stats.clone()).await;
    assert_eq!(res["results"][0]["accepted"], 2);
    assert_eq!(res["results"][0]["watermark"], 1_700_000_040);
    assert_eq!(res["unmatched"], json!([unknown]));
    let res = a.post("page-stats", stats).await;
    assert_eq!(res["results"][0]["accepted"], 0);
    assert_eq!(res["results"][0]["duplicates"], 2);

    let state = |status: &str, modified: &str| json!({ "books": [{ "hash": doc, "status": status, "statusModified": modified }] });
    let res = a.post("book-states", state("complete", "2026-01-02")).await;
    assert_eq!(res["results"][0]["statusApplied"], true);
    let res = a.post("book-states", state("reading", "2026-01-02")).await;
    assert_eq!(res["results"][0]["statusApplied"], false);
    assert_eq!(res["results"][0]["status"], "complete");
    let res = a.post("book-states", state("reading", "2026-01-03")).await;
    assert_eq!(res["results"][0]["statusApplied"], true);
    assert_eq!(res["results"][0]["status"], "reading");

    let summary = app
        .get(
            &format!("/api/v1/books/{book}/reading-summary"),
            &reader.token,
        )
        .await
        .json();
    assert_eq!(summary["status"], "reading");
    assert_eq!(summary["status_modified"], "2026-01-03");
    assert_eq!(summary["total_seconds"], 55);
    assert_eq!(summary["pages_read"], 2);
    assert_eq!(summary["total_pages"], 100);
    assert_eq!(summary["reading_days"], 1);
    assert_eq!(summary["last_read_at"], 1_700_000_040);
}

#[tokio::test]
async fn device_rating_updates_book_rating() {
    let app = TestApp::new().await;
    let (reader, book, doc) = setup(&app).await;
    let a = Device {
        app: &app,
        user: &reader,
        id: "device-a",
    };
    let rating = |r: Value, modified: &str| json!({ "books": [{ "hash": doc, "rating": r, "statusModified": modified }] });

    let res = a.post("book-states", rating(json!(4), "2026-01-02")).await;
    assert_eq!(res["results"][0]["ratingApplied"], true);
    assert_eq!(res["results"][0]["rating"], 4);
    let detail = app
        .get(&format!("/api/v1/books/{book}"), &reader.token)
        .await
        .json();
    assert_eq!(detail["metadata"]["rating"], 4);
    assert_eq!(metadata_file(&app)["rating"], 4);

    // the book changed today, so an older device change does not overwrite it
    let res = a.post("book-states", rating(json!(2), "2026-01-03")).await;
    assert_eq!(res["results"][0]["ratingApplied"], false);
    assert_eq!(res["results"][0]["rating"], 4);
    let res = a.post("book-states", rating(json!(2), "2999-01-01")).await;
    assert_eq!(res["results"][0]["ratingApplied"], true);
    assert_eq!(metadata_file(&app)["rating"], 2);

    let viewer = app.create_user("viewer", "pass-123", "user").await;
    let lib: (String,) = sqlx::query_as("SELECT library_id FROM books WHERE id = ?")
        .bind(book.to_string())
        .fetch_one(&app.state.db)
        .await
        .unwrap();
    app.add_member(
        Uuid::parse_str(&lib.0).unwrap(),
        &viewer,
        "member",
        (true, false, false),
    )
    .await;
    let v = Device {
        app: &app,
        user: &viewer,
        id: "device-v",
    };
    let res = v.post("book-states", rating(json!(5), "2999-01-02")).await;
    assert_eq!(res["results"][0]["ratingApplied"], false);
    assert_eq!(res["results"][0]["rating"], 2);

    let res = a
        .post(
            "book-states",
            json!({ "books": [{ "hash": doc, "ratingCleared": true, "statusModified": "2999-01-03" }] }),
        )
        .await;
    assert_eq!(res["results"][0]["ratingSet"], false);
    assert!(metadata_file(&app).get("rating").is_none_or(Value::is_null));
}

#[tokio::test]
async fn pdf_position_annotations_are_stored_but_not_pushed() {
    let app = TestApp::new().await;
    let (reader, book, doc) = setup(&app).await;
    let a = Device {
        app: &app,
        user: &reader,
        id: "device-a",
    };
    let b = Device {
        app: &app,
        user: &reader,
        id: "device-b",
    };
    let mut pdf = annotation("pdf note", None);
    pdf["posFormat"] = json!("pdf");
    pdf["pos0"] = json!("{\"page\":3,\"x\":10,\"y\":20}");
    a.post(
        "annotations/exchange",
        json!({ "books": [{ "hash": doc, "keys": [], "keysComplete": false, "changes": [pdf] }] }),
    )
    .await;

    let listed = app
        .get(&format!("/api/v1/books/{book}/annotations"), &reader.token)
        .await
        .json();
    assert_eq!(listed["annotations"][0]["note"], "pdf note");
    let res = b
        .post(
            "annotations/exchange",
            json!({ "books": [{ "hash": doc, "keys": [], "keysComplete": true, "changes": [] }] }),
        )
        .await;
    assert_eq!(to_apply_len(&res), 0, "{res}");
    assert_eq!(res["results"][0]["skippedNoPosition"], 1);
}

#[tokio::test]
async fn snapshot_survives_edit_and_restores_on_resync() {
    let app = TestApp::new().await;
    let (reader, book, doc) = setup(&app).await;
    let a = Device {
        app: &app,
        user: &reader,
        id: "device-a",
    };
    a.post(
        "annotations/exchange",
        json!({ "books": [{ "hash": doc, "keys": [{ "k": key(DT, POS), "dt": DT }],
                            "keysComplete": true, "changes": [annotation("kept", None)] }] }),
    )
    .await;

    let res = app
        .json(
            Method::PATCH,
            &format!("/api/v1/books/{book}"),
            &app.admin.token,
            json!({ "metadata": { "title": "Orbit 2", "author": "Author", "category": "未分类" } }),
        )
        .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.text());
    let meta = metadata_file(&app);
    assert_eq!(meta["title"], "Orbit 2");
    assert_eq!(meta["annotations"][0]["note"], "kept");

    sqlx::query("DELETE FROM book_annotations")
        .execute(&app.state.db)
        .await
        .unwrap();
    let lib: (String,) = sqlx::query_as("SELECT library_id FROM books WHERE id = ?")
        .bind(book.to_string())
        .fetch_one(&app.state.db)
        .await
        .unwrap();
    let task = app
        .run_task(
            &app.admin.token,
            "library_resync_metadata",
            json!({ "library_id": lib.0 }),
        )
        .await;
    assert_eq!(task["status"], "completed", "{task}");
    let listed = app
        .get(&format!("/api/v1/books/{book}/annotations"), &reader.token)
        .await
        .json();
    assert_eq!(listed["annotations"][0]["note"], "kept");

    let outsider = app.create_user("outsider", "pass-123", "user").await;
    let res = app
        .get(
            &format!("/api/v1/books/{book}/annotations"),
            &outsider.token,
        )
        .await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn management_lists_devices_books_and_resets_device() {
    let app = TestApp::new().await;
    let (reader, book, doc) = setup(&app).await;
    let a = Device {
        app: &app,
        user: &reader,
        id: "device-a",
    };
    let b = Device {
        app: &app,
        user: &reader,
        id: "device-b",
    };
    let keys = json!([{ "k": key(DT, POS), "dt": DT }]);
    a.post(
        "annotations/exchange",
        json!({ "books": [{ "hash": doc, "keys": keys, "keysComplete": true,
                            "changes": [annotation("n", None)] }] }),
    )
    .await;
    a.post(
        "page-stats",
        json!({ "books": [{ "hash": doc, "events": [
            { "page": 1, "startTime": 1_700_000_000, "durationSeconds": 30, "totalPages": 10 }
        ] }] }),
    )
    .await;
    let res = b
        .post(
            "annotations/exchange",
            json!({ "books": [{ "hash": doc, "keys": [], "keysComplete": true, "changes": [] }] }),
        )
        .await;
    let server_id = res["results"][0]["toApply"]["add"][0]["serverId"].clone();
    b.post(
        "annotations/exchange-ack",
        json!({ "books": [{ "hash": doc, "deleted": [], "applied": [
            { "serverId": server_id, "version": 1, "status": "applied" }
        ] }] }),
    )
    .await;

    let devices = app
        .get("/api/v1/bookorbit/devices", &reader.token)
        .await
        .json();
    let devices = devices.as_array().unwrap();
    assert_eq!(devices.len(), 2, "{devices:?}");
    let dev_a = devices
        .iter()
        .find(|d| d["device_id"] == "device-a")
        .unwrap();
    assert_eq!(dev_a["annotations"], 1);
    assert_eq!(dev_a["page_stats"], 1);
    assert_eq!(dev_a["books"], 1);

    let books = app
        .get("/api/v1/bookorbit/books", &reader.token)
        .await
        .json();
    assert_eq!(books[0]["book_id"], book.to_string());
    assert_eq!(books[0]["title"], "Orbit");
    assert_eq!(books[0]["annotations"], 1);
    assert_eq!(books[0]["total_seconds"], 30);
    let outsider = app.create_user("outsider", "pass-123", "user").await;
    let none = app
        .get("/api/v1/bookorbit/books", &outsider.token)
        .await
        .json();
    assert_eq!(none, json!([]));

    let res = app
        .json(
            Method::POST,
            "/api/v1/bookorbit/devices/reset",
            &reader.token,
            json!({ "device_id": "device-b" }),
        )
        .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.text());
    assert_eq!(res.json()["annotation_states"], 1);
    let res = b
        .post(
            "annotations/exchange",
            json!({ "books": [{ "hash": doc, "keys": keys, "keysComplete": true, "changes": [] }] }),
        )
        .await;
    assert_eq!(
        res["results"][0]["toApply"]["add"][0]["serverId"],
        server_id
    );
    let listed = app
        .get(&format!("/api/v1/books/{book}/annotations"), &reader.token)
        .await
        .json();
    assert_eq!(listed["annotations"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn exchange_rejects_invalid_input() {
    let app = TestApp::new().await;
    let (reader, _book, doc) = setup(&app).await;
    let key_hdr = md5_hex(reader.password.as_bytes());
    for body in [
        json!({ "deviceId": "", "books": [{ "hash": doc }] }),
        json!({ "deviceId": "d", "books": [] }),
        json!({ "deviceId": "d", "books": [{ "hash": "zz" }] }),
        json!({ "deviceId": "d", "books": [{ "hash": doc, "keys": [{ "k": "x", "dt": DT }] }] }),
        json!({ "deviceId": "d", "books": [{ "hash": doc,
            "changes": [{ "datetime": "bad", "drawer": "lighten", "posFormat": "xpointer", "pos0": POS }] }] }),
        json!({ "deviceId": "d", "books": [{ "hash": doc,
            "changes": [{ "datetime": DT, "drawer": "glow", "posFormat": "xpointer", "pos0": POS }] }] }),
    ] {
        let res = app
            .send(
                Method::POST,
                "/api/v1/koreader/plugin/annotations/exchange",
                None,
                Some("application/json"),
                serde_json::to_vec(&body).unwrap(),
                &[("x-auth-user", &reader.username), ("x-auth-key", &key_hdr)],
            )
            .await;
        assert_eq!(res.status, StatusCode::BAD_REQUEST, "{body}");
    }
}

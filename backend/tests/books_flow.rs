mod common;

use axum::http::{header, Method, StatusCode};
use common::{all_files, bearer, Part, TestApp};
use serde_json::json;
use std::path::PathBuf;
use uuid::Uuid;

fn path_of(v: &serde_json::Value) -> PathBuf {
    PathBuf::from(v["book_file_path"].as_str().unwrap())
}

#[tokio::test]
async fn book_lifecycle() {
    let app = TestApp::new().await;
    let lib = app.create_library("馆", "lib").await;
    app.create_category(lib, "科幻").await;
    let root = app.data_root().join("馆");
    let token = app.admin.token.clone();
    let png = common::tiny_png();
    let lib_s = lib.to_string();
    let meta =
        json!({ "title": "三体", "author": "刘慈欣", "isbn": "978-7-5366-9293-0", "rating": 5 })
            .to_string();

    let res = app
        .multipart(
            Method::POST,
            "/api/v1/books",
            &token,
            &[
                Part::Text("library_id", &lib_s),
                Part::Text("category", "科幻"),
                Part::Text("metadata", &meta),
                Part::File {
                    name: "file",
                    filename: "santi.epub",
                    content_type: "application/epub+zip",
                    data: b"epub-bytes",
                },
                Part::File {
                    name: "cover",
                    filename: "c.png",
                    content_type: "image/png",
                    data: &png,
                },
            ],
        )
        .await;
    assert_eq!(res.status, StatusCode::CREATED, "{}", res.text());
    let detail = res.json();
    let id = detail["id"].as_str().unwrap().to_string();
    let dir = root.join("科幻");
    assert_eq!(path_of(&detail), dir.join("三体_刘慈欣.epub"));
    assert!(dir.join("三体_刘慈欣.json").is_file());
    assert!(dir.join("三体_刘慈欣.png").is_file());
    assert_eq!(detail["category"], "科幻");
    assert_eq!(detail["metadata"]["rating"], 5);
    assert_eq!(detail["permissions"]["can_delete"], true);

    // listing & search
    let list = app
        .get(
            &format!("/api/v1/books?library_id={lib}&category=科幻"),
            &token,
        )
        .await
        .json();
    assert_eq!(list.as_array().unwrap().len(), 1);
    for q in ["三体", "刘慈欣", "9293"] {
        let found = app
            .get(
                &format!("/api/v1/search?q={}", urlencoding::encode(q)),
                &token,
            )
            .await
            .json();
        assert_eq!(found.as_array().unwrap().len(), 1, "{q}");
    }

    // download
    let res = app
        .get(&format!("/api/v1/books/{id}/download"), &token)
        .await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.body, b"epub-bytes");
    assert_eq!(res.headers[header::CONTENT_TYPE], "application/epub+zip");
    let disposition = res.headers[header::CONTENT_DISPOSITION].to_str().unwrap();
    assert!(disposition.starts_with("attachment;"));
    assert!(disposition.contains("filename*=UTF-8''%E4%B8%89%E4%BD%93"));

    // cover, etag, thumbnail
    let cover_uri = format!("/api/v1/assets/covers/{id}");
    let res = app.get(&cover_uri, &token).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.headers[header::CONTENT_TYPE], "image/png");
    assert_eq!(res.body, png);
    let etag = res.headers[header::ETAG].to_str().unwrap().to_string();
    let res = app
        .send(
            Method::GET,
            &cover_uri,
            Some(&bearer(&token)),
            None,
            vec![],
            &[("if-none-match", &etag)],
        )
        .await;
    assert_eq!(res.status, StatusCode::NOT_MODIFIED);
    assert!(res.body.is_empty());
    let res = app.get(&format!("{cover_uri}?size=thumb"), &token).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.headers[header::CONTENT_TYPE], "image/jpeg");
    assert!(image::load_from_memory(&res.body).is_ok());
    assert!(dir.join("三体_刘慈欣.thumb.jpg").is_file());

    // rename + move category in one update
    let res = app
        .json(
            Method::PATCH,
            &format!("/api/v1/books/{id}"),
            &token,
            json!({ "metadata": { "title": "三体II", "author": "刘慈欣", "category": "未分类", "publish_date": "2008-05" } }),
        )
        .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.text());
    let moved = root.join("未分类");
    assert_eq!(path_of(&res.json()), moved.join("三体II_刘慈欣.epub"));
    assert!(moved.join("三体II_刘慈欣.png").is_file());
    assert!(moved.join("三体II_刘慈欣.thumb.jpg").is_file());
    assert!(
        all_files(&dir).is_empty(),
        "old category dir must be empty: {:?}",
        all_files(&dir)
    );
    let on_disk: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(moved.join("三体II_刘慈欣.json")).unwrap())
            .unwrap();
    assert_eq!(on_disk["title"], "三体II");
    assert_eq!(on_disk["publish_date"], "2008-05");

    // replace cover with jpg
    let jpg = {
        let img = image::RgbImage::from_pixel(2, 2, image::Rgb([0, 0, 255]));
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Jpeg).unwrap();
        buf.into_inner()
    };
    let res = app
        .multipart(
            Method::PUT,
            &format!("/api/v1/books/{id}/cover"),
            &token,
            &[Part::File {
                name: "cover",
                filename: "new.jpg",
                content_type: "image/jpeg",
                data: &jpg,
            }],
        )
        .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.text());
    assert!(moved.join("三体II_刘慈欣.jpg").is_file());
    assert!(!moved.join("三体II_刘慈欣.png").exists());
    assert!(!moved.join("三体II_刘慈欣.thumb.jpg").exists());

    // reading progress + history + home
    let res = app
        .json(
            Method::PATCH,
            &format!("/api/v1/books/{id}/progress"),
            &token,
            json!({ "reading_progress": { "current_page": 42, "percent": 33.3 } }),
        )
        .await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.json()["reading_percent"], 33.3);
    assert_eq!(
        res.json()["metadata"]["reading_progress"]["current_page"],
        42
    );
    let history = app
        .get("/api/v1/users/me/reading-history", &token)
        .await
        .json();
    assert_eq!(history.as_array().unwrap().len(), 1);
    assert_eq!(history[0]["source"], "web");
    let history = app
        .get(
            &format!("/api/v1/users/me/reading-history?library_id={lib}"),
            &token,
        )
        .await
        .json();
    assert_eq!(history.as_array().unwrap().len(), 1);
    let res = app
        .get("/api/v1/users/me/reading-history?library_id=bad", &token)
        .await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    let home = app
        .get(&format!("/api/v1/home?library_id={lib}"), &token)
        .await
        .json();
    assert_eq!(home["recent"][0]["id"], id);
    assert_eq!(home["aha_moment"].as_array().unwrap().len(), 1);
    let recent = app.get("/api/v1/books?sort=recent", &token).await.json();
    assert_eq!(recent[0]["id"], id);

    // same progress again does not add a history row
    app.json(
        Method::PATCH,
        &format!("/api/v1/books/{id}/progress"),
        &token,
        json!({ "reading_progress": { "current_page": 42, "percent": 33.3 } }),
    )
    .await;
    let history = app
        .get("/api/v1/users/me/reading-history", &token)
        .await
        .json();
    assert_eq!(history.as_array().unwrap().len(), 1);

    // metadata.json on disk never stores per-user progress
    let on_disk = std::fs::read_to_string(moved.join("三体II_刘慈欣.json")).unwrap();
    assert!(!on_disk.contains("reading_progress"));

    // delete
    let res = app.delete(&format!("/api/v1/books/{id}"), &token).await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    assert!(all_files(&root).is_empty(), "{:?}", all_files(&root));
    assert_eq!(
        app.get(&format!("/api/v1/books/{id}"), &token).await.status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn reading_progress_is_per_user() {
    let app = TestApp::new().await;
    let lib = app.create_library("馆", "lib").await;
    let book = app.seed_book(lib, "Shared", b"x").await;
    let alice = app.create_user("alice", "pass-123", "user").await;
    let bob = app.create_user("bob", "pass-123", "user").await;
    app.add_member(lib, &alice, "member", (true, true, false))
        .await;
    app.add_member(lib, &bob, "member", (true, true, false))
        .await;

    let res = app
        .json(
            Method::PATCH,
            &format!("/api/v1/books/{book}/progress"),
            &alice.token,
            json!({ "reading_progress": { "percent": 80.0 } }),
        )
        .await;
    assert_eq!(res.status, StatusCode::OK);

    let bob_view = app
        .get(&format!("/api/v1/books/{book}"), &bob.token)
        .await
        .json();
    assert!(bob_view["reading_percent"].is_null());
    assert!(bob_view["metadata"]["reading_progress"].is_null());
    assert!(app
        .get("/api/v1/users/me/reading-history", &bob.token)
        .await
        .json()
        .as_array()
        .unwrap()
        .is_empty());
    assert!(app.get("/api/v1/home", &bob.token).await.json()["recent"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn batch_move_validation_and_success() {
    let app = TestApp::new().await;
    let lib = app.create_library("馆", "lib").await;
    app.create_category(lib, "历史").await;
    let a = app.seed_book(lib, "A", b"a").await;
    let b = app.seed_book(lib, "B", b"b").await;
    let uri = "/api/v1/books/batch-move-category";
    let token = &app.admin.token;

    let too_many: Vec<Uuid> = (0..101).map(|_| Uuid::new_v4()).collect();
    for (body, status) in [
        (
            json!({ "library_id": lib, "book_ids": [], "category": "历史" }),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({ "library_id": lib, "book_ids": too_many, "category": "历史" }),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({ "library_id": lib, "book_ids": [a], "category": "不存在" }),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({ "library_id": lib, "book_ids": [a], "category": "../x" }),
            StatusCode::BAD_REQUEST,
        ),
    ] {
        assert_eq!(
            app.json(Method::POST, uri, token, body.clone())
                .await
                .status,
            status,
            "{body}"
        );
    }

    let missing = Uuid::new_v4();
    let res = app
        .json(
            Method::POST,
            uri,
            token,
            json!({ "library_id": lib, "book_ids": [a, b, missing], "category": "历史" }),
        )
        .await;
    assert_eq!(res.status, StatusCode::OK);
    let body = res.json();
    assert_eq!(body["moved"], 2);
    assert_eq!(body["failures"][0]["book_id"], missing.to_string());
    let list = app
        .get(
            &format!("/api/v1/books?category={}", urlencoding::encode("历史")),
            token,
        )
        .await
        .json();
    assert_eq!(list.as_array().unwrap().len(), 2);
    assert!(app.data_root().join("馆/历史/A_Author.pdf").is_file());
}

#[tokio::test]
async fn category_rules() {
    let app = TestApp::new().await;
    let lib = app.create_library("馆", "lib").await;
    let token = &app.admin.token;
    let base = format!("/api/v1/libraries/{lib}/categories");

    app.create_category(lib, "小说 文学").await;
    let cats = app.get(&base, token).await.json();
    let names: Vec<&str> = cats
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["小说_文学", "未分类"]);

    // creating again is idempotent
    let res = app
        .json(Method::POST, &base, token, json!({ "name": "小说_文学" }))
        .await;
    assert_eq!(res.status, StatusCode::CREATED);

    app.seed_book(lib, "Novel", b"n").await;
    let cats = app.get(&base, token).await.json();
    let default = cats
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "未分类")
        .unwrap();
    assert_eq!(default["book_count"], 1);

    assert_eq!(
        app.delete(&format!("{base}/未分类"), token).await.status,
        StatusCode::BAD_REQUEST
    );

    let root = app.data_root().join("馆");
    std::fs::write(root.join("小说_文学").join("stray.txt"), b"x").unwrap();
    assert_eq!(
        app.delete(&format!("{base}/小说_文学"), token).await.status,
        StatusCode::CONFLICT
    );
    std::fs::remove_file(root.join("小说_文学").join("stray.txt")).unwrap();
    assert_eq!(
        app.delete(&format!("{base}/小说_文学"), token).await.status,
        StatusCode::NO_CONTENT
    );
    assert!(!root.join("小说_文学").exists());

    // a category that still holds indexed books cannot be deleted
    app.create_category(lib, "满").await;
    let id = app.seed_book(lib, "Full", b"f").await;
    app.json(
        Method::POST,
        "/api/v1/books/batch-move-category",
        token,
        json!({ "library_id": lib, "book_ids": [id], "category": "满" }),
    )
    .await;
    assert_eq!(
        app.delete(&format!("{base}/满"), token).await.status,
        StatusCode::CONFLICT
    );

    // a regular file squatting the category path
    std::fs::write(root.join("squat"), b"x").unwrap();
    let res = app
        .json(Method::POST, &base, token, json!({ "name": "squat" }))
        .await;
    assert_eq!(res.status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn library_create_and_delete_rules() {
    let app = TestApp::new().await;
    let token = &app.admin.token;
    let lib = app.create_library("馆", "lib").await;
    let res = app
        .json(
            Method::POST,
            "/api/v1/libraries",
            token,
            json!({ "name": "另一个", "slug": "lib" }),
        )
        .await;
    assert_eq!(res.status, StatusCode::CONFLICT);

    // same display name gets a unique directory
    let res = app
        .json(
            Method::POST,
            "/api/v1/libraries",
            token,
            json!({ "name": "馆", "slug": "lib-2" }),
        )
        .await;
    assert_eq!(res.status, StatusCode::CREATED);
    assert!(res.json()["root_path"].as_str().unwrap().ends_with("馆_2"));
    assert_eq!(
        res.json()["opds_url"],
        format!(
            "/api/v1/opds/libraries/{}",
            res.json()["id"].as_str().unwrap()
        )
    );

    let book = app.seed_book(lib, "Keep", b"k").await;
    let res = app.delete(&format!("/api/v1/libraries/{lib}"), token).await;
    assert_eq!(res.status, StatusCode::CONFLICT);

    app.delete(&format!("/api/v1/books/{book}"), token).await;
    let res = app.delete(&format!("/api/v1/libraries/{lib}"), token).await;
    assert_eq!(res.status, StatusCode::NO_CONTENT, "{}", res.text());
    assert!(!app.data_root().join("馆").exists());
    assert_eq!(
        app.delete(&format!("/api/v1/libraries/{lib}"), token)
            .await
            .status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn duplicates_are_grouped() {
    let app = TestApp::new().await;
    let lib = app.create_library("馆", "lib").await;
    let a = app.seed_book(lib, "Same Title", b"one").await;
    let b = app.seed_book(lib, "same   title", b"two").await;
    app.seed_book(lib, "Different", b"three").await;

    let res = app
        .get(
            &format!("/api/v1/libraries/{lib}/duplicates"),
            &app.admin.token,
        )
        .await;
    assert_eq!(res.status, StatusCode::OK);
    let groups = res.json()["groups"].clone();
    assert_eq!(groups.as_array().unwrap().len(), 1);
    assert_eq!(groups[0]["kind"], "title");
    let mut ids: Vec<String> = groups[0]["books"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["id"].as_str().unwrap().into())
        .collect();
    ids.sort();
    let mut expected = vec![a.to_string(), b.to_string()];
    expected.sort();
    assert_eq!(ids, expected);
}

#[tokio::test]
async fn maintenance_scan_and_follow_up_tools() {
    let app = TestApp::new().await;
    let lib = app.create_library("馆", "lib").await;
    let token = app.admin.token.clone();
    let cat = app.data_root().join("馆").join("手动");
    std::fs::create_dir_all(&cat).unwrap();
    let dropped = cat.join("dropped_book.pdf");
    std::fs::write(&dropped, b"dropped").unwrap();
    std::fs::write(
        cat.join("dropped_book.json"),
        json!({ "title": "From Disk", "author": "FS" }).to_string(),
    )
    .unwrap();
    std::fs::write(cat.join("bare_file.epub"), b"bare").unwrap();
    std::fs::write(cat.join("ghost.json"), b"{}").unwrap();
    std::fs::write(cat.join("ghost.jpg"), b"jpg").unwrap();
    let params = json!({ "library_id": lib });

    // scan is read-only
    let task = app.run_task(&token, "library_scan", params.clone()).await;
    assert_eq!(task["status"], "completed", "{task}");
    let report = &task["result"];
    assert_eq!(
        report["untracked_files"].as_array().unwrap().len(),
        2,
        "{report}"
    );
    assert_eq!(
        report["dangling_files"].as_array().unwrap().len(),
        2,
        "{report}"
    );
    assert!(report["missing_files"].as_array().unwrap().is_empty());
    let titled = report["untracked_files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["path"] == dropped.to_string_lossy().as_ref())
        .unwrap();
    assert_eq!(titled["title"], "From Disk");
    assert!(cat.join("ghost.json").is_file());
    assert!(app
        .get("/api/v1/books", &token)
        .await
        .json()
        .as_array()
        .unwrap()
        .is_empty());

    // import only the selected file
    let task = app
        .run_task(
            &token,
            "library_import_untracked",
            json!({ "library_id": lib, "items": [dropped.to_string_lossy()] }),
        )
        .await;
    assert_eq!(
        task["result"]["processed"].as_array().unwrap().len(),
        1,
        "{task}"
    );
    let found = app
        .get(
            &format!("/api/v1/search?q={}", urlencoding::encode("From Disk")),
            &token,
        )
        .await
        .json();
    assert_eq!(found[0]["category"], "手动");
    let dropped_id = found[0]["id"].as_str().unwrap().to_string();

    // remaining untracked file is deleted from disk
    let task = app
        .run_task(&token, "library_delete_untracked", params.clone())
        .await;
    assert_eq!(task["status"], "completed", "{task}");
    assert!(!cat.join("bare_file.epub").exists());

    let task = app
        .run_task(&token, "library_clean_dangling", params.clone())
        .await;
    assert_eq!(
        task["result"]["processed"].as_array().unwrap().len(),
        2,
        "{task}"
    );
    assert!(!cat.join("ghost.json").exists() && !cat.join("ghost.jpg").exists());
    assert!(cat.join("dropped_book.json").is_file());

    // disk metadata overrides DB
    std::fs::write(
        cat.join("dropped_book.json"),
        json!({ "title": "Edited On Disk" }).to_string(),
    )
    .unwrap();
    app.run_task(&token, "library_resync_metadata", params.clone())
        .await;
    let detail = app
        .get(&format!("/api/v1/books/{dropped_id}"), &token)
        .await
        .json();
    assert_eq!(detail["title"], "Edited On Disk");

    // missing file -> reported, then purged; unknown ids are skipped
    std::fs::remove_file(&dropped).unwrap();
    let task = app.run_task(&token, "library_scan", params.clone()).await;
    assert_eq!(task["result"]["missing_files"][0]["book_id"], dropped_id);
    let task = app
        .run_task(
            &token,
            "library_purge_missing",
            json!({ "library_id": lib, "items": [dropped_id, "bogus"] }),
        )
        .await;
    assert_eq!(task["result"]["processed"], json!([dropped_id]));
    assert_eq!(task["result"]["skipped"], json!(["bogus"]));
    assert_eq!(
        app.get(&format!("/api/v1/books/{dropped_id}"), &token)
            .await
            .status,
        StatusCode::NOT_FOUND
    );

    let tasks = app.get("/api/v1/maintenance/tasks", &token).await.json();
    assert_eq!(tasks.as_array().unwrap().len(), 7);
    assert!(tasks[0].get("result").is_none(), "list omits results");
    assert_eq!(tasks[0]["tool"], "library_purge_missing");
    assert_eq!(tasks[0]["library_name"], "馆");
    assert_eq!(tasks[0]["created_by"], "admin");
}

#[tokio::test]
async fn maintenance_hash_tools() {
    let app = TestApp::new().await;
    let lib = app.create_library("馆", "lib").await;
    let token = app.admin.token.clone();
    let a = app.seed_book(lib, "A", b"%PDF-a").await;
    let b = app.seed_book(lib, "B", b"%PDF-b").await;
    let params = json!({ "library_id": lib });

    sqlx::query(
        "UPDATE books SET metadata = json_remove(metadata, '$.file_md5', '$.partial_md5', '$.file_sha256') WHERE id = ?",
    )
    .bind(a.to_string())
    .execute(&app.state.db)
    .await
    .unwrap();

    let task = app
        .run_task(&token, "library_fill_missing_hashes", params.clone())
        .await;
    assert_eq!(
        task["result"]["processed"],
        json!([a.to_string()]),
        "{task}"
    );
    let detail = app.get(&format!("/api/v1/books/{a}"), &token).await.json();
    assert_eq!(
        detail["metadata"]["file_md5"],
        shelfie_backend::infra::hash::md5_hex(b"%PDF-a")
    );
    let sidecar: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(path_of(&detail).with_extension("json")).unwrap(),
    )
    .unwrap();
    assert!(sidecar["file_sha256"].is_string(), "{sidecar}");

    let task = app
        .run_task(&token, "library_fill_missing_hashes", params.clone())
        .await;
    assert!(task["result"]["processed"].as_array().unwrap().is_empty());

    let b_path = path_of(&app.get(&format!("/api/v1/books/{b}"), &token).await.json());
    std::fs::write(&b_path, b"%PDF-changed").unwrap();
    let task = app.run_task(&token, "library_refresh_hashes", params).await;
    assert_eq!(
        task["result"]["processed"].as_array().unwrap().len(),
        2,
        "{task}"
    );
    let detail = app.get(&format!("/api/v1/books/{b}"), &token).await.json();
    assert_eq!(
        detail["metadata"]["file_md5"],
        shelfie_backend::infra::hash::md5_hex(b"%PDF-changed")
    );
}

#[tokio::test]
async fn opds_recent_new_and_reading_feeds() {
    let app = TestApp::new().await;
    let token = &app.admin.token;
    let lib = app.create_library("馆", "lib").await;

    let mut ids = Vec::new();
    for i in 0..12 {
        let id = app
            .seed_book(lib, &format!("Book{i:02}"), format!("b{i}").as_bytes())
            .await;
        sqlx::query("UPDATE books SET uploaded_at = ? WHERE id = ?")
            .bind(format!("2026-01-01 00:00:{i:02}"))
            .bind(id.to_string())
            .execute(&app.state.db)
            .await
            .unwrap();
        ids.push(id);
    }

    let nav = app
        .get(&format!("/api/v1/opds/libraries/{lib}"), token)
        .await
        .text();
    assert!(nav.contains("最近新增") && nav.contains("books?sort=new"));
    assert!(nav.contains("最近在读") && nav.contains("books?sort=reading"));

    let xml = app
        .get(
            &format!("/api/v1/opds/libraries/{lib}/books?sort=new"),
            token,
        )
        .await
        .text();
    assert_eq!(xml.matches("<entry>").count(), 10);
    assert!(xml.find("Book11").unwrap() < xml.find("Book02").unwrap());
    assert!(!xml.contains("Book01") && !xml.contains("Book00"));

    let xml = app
        .get(
            &format!("/api/v1/opds/libraries/{lib}/books?sort=reading"),
            token,
        )
        .await
        .text();
    assert_eq!(xml.matches("<entry>").count(), 0);

    for id in [ids[3], ids[7]] {
        let res = app
            .json(
                Method::PATCH,
                &format!("/api/v1/books/{id}/progress"),
                token,
                json!({ "reading_progress": { "percent": 10.0 } }),
            )
            .await;
        assert_eq!(res.status, StatusCode::OK);
    }
    sqlx::query(
        "UPDATE user_reading_progress SET updated_at = '2026-01-02 00:00:00' WHERE book_id = ?",
    )
    .bind(ids[3].to_string())
    .execute(&app.state.db)
    .await
    .unwrap();
    sqlx::query(
        "UPDATE user_reading_progress SET updated_at = '2026-01-01 00:00:00' WHERE book_id = ?",
    )
    .bind(ids[7].to_string())
    .execute(&app.state.db)
    .await
    .unwrap();

    let xml = app
        .get(
            &format!("/api/v1/opds/libraries/{lib}/books?sort=reading"),
            token,
        )
        .await
        .text();
    assert_eq!(xml.matches("<entry>").count(), 2);
    assert!(xml.find("Book03").unwrap() < xml.find("Book07").unwrap());
}

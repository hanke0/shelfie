#![allow(dead_code)]

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use base64::Engine;
use http_body_util::BodyExt;
use serde_json::Value;
use shelfie_backend::api::router::build_router;
use shelfie_backend::config::Config;
use shelfie_backend::domain::auth::{issue_token, AuthUser};
use shelfie_backend::infra::{db, hash::md5_hex};
use shelfie_backend::state::AppState;
use std::path::{Path, PathBuf};
use tower::ServiceExt;
use uuid::Uuid;

pub const JWT_SECRET: &str = "test-secret";
const BOUNDARY: &str = "----shelfie-test-boundary";

pub struct TestApp {
    pub state: AppState,
    pub router: Router,
    pub root: PathBuf,
    pub admin: TestUser,
}

#[derive(Clone)]
pub struct TestUser {
    pub id: Uuid,
    pub username: String,
    pub password: String,
    pub token: String,
}

pub struct TestResponse {
    pub status: StatusCode,
    pub headers: axum::http::HeaderMap,
    pub body: Vec<u8>,
}

impl TestResponse {
    pub fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap_or_else(|_| {
            panic!(
                "body is not JSON (status {}): {}",
                self.status,
                String::from_utf8_lossy(&self.body)
            )
        })
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

pub enum Part<'a> {
    Text(&'a str, &'a str),
    File {
        name: &'a str,
        filename: &'a str,
        content_type: &'a str,
        data: &'a [u8],
    },
}

pub fn multipart_body(parts: &[Part]) -> Vec<u8> {
    let mut out = Vec::new();
    for part in parts {
        out.extend_from_slice(format!("--{BOUNDARY}\r\n").as_bytes());
        match part {
            Part::Text(name, value) => {
                out.extend_from_slice(
                    format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n").as_bytes(),
                );
                out.extend_from_slice(value.as_bytes());
            }
            Part::File {
                name,
                filename,
                content_type,
                data,
            } => {
                out.extend_from_slice(
                    format!(
                        "Content-Disposition: form-data; name=\"{name}\"; filename=\"{filename}\"\r\nContent-Type: {content_type}\r\n\r\n"
                    )
                    .as_bytes(),
                );
                out.extend_from_slice(data);
            }
        }
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());
    out
}

pub fn basic_auth(username: &str, password: &str) -> String {
    let encoded =
        base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"));
    format!("Basic {encoded}")
}

pub fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}

impl TestApp {
    pub async fn new() -> Self {
        let root = std::env::temp_dir().join(format!("shelfie-it-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let config = Config {
            database_url: format!("sqlite:{}?mode=rwc", root.join("test.db").display()),
            data_root: root.join("libraries"),
            jwt_secret: JWT_SECRET.into(),
            host: "127.0.0.1".into(),
            port: 0,
            public_base_url: None,
        };
        let pool = db::connect(&config).await.unwrap();
        let state = AppState::new(pool, config);
        let router = build_router(state.clone());
        let mut app = Self {
            state,
            router,
            root,
            admin: TestUser {
                id: Uuid::nil(),
                username: String::new(),
                password: String::new(),
                token: String::new(),
            },
        };
        app.admin = app.create_user("admin", "admin-pass", "system_admin").await;
        app
    }

    pub fn data_root(&self) -> &Path {
        &self.state.config.data_root
    }

    /// Inserts a user directly with a low bcrypt cost to keep tests fast.
    pub async fn create_user(&self, username: &str, password: &str, role: &str) -> TestUser {
        let id = Uuid::new_v4();
        let hash = bcrypt::hash(password, 4).unwrap();
        sqlx::query(
            "INSERT INTO users (id, username, password_hash, password_md5, role) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(username)
        .bind(hash)
        .bind(md5_hex(password.as_bytes()))
        .bind(role)
        .execute(&self.state.db)
        .await
        .unwrap();
        let token = issue_token(
            JWT_SECRET,
            &AuthUser {
                id,
                username: username.into(),
                role: role.into(),
            },
        )
        .unwrap();
        TestUser {
            id,
            username: username.into(),
            password: password.into(),
            token,
        }
    }

    pub async fn send(
        &self,
        method: Method,
        uri: &str,
        auth: Option<&str>,
        content_type: Option<&str>,
        body: Vec<u8>,
        extra_headers: &[(&str, &str)],
    ) -> TestResponse {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(auth) = auth {
            builder = builder.header(header::AUTHORIZATION, auth);
        }
        if let Some(ct) = content_type {
            builder = builder.header(header::CONTENT_TYPE, ct);
        }
        for (k, v) in extra_headers {
            builder = builder.header(*k, *v);
        }
        let response = self
            .router
            .clone()
            .oneshot(builder.body(Body::from(body)).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let body = response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec();
        TestResponse {
            status,
            headers,
            body,
        }
    }

    pub async fn get(&self, uri: &str, token: &str) -> TestResponse {
        self.send(Method::GET, uri, Some(&bearer(token)), None, vec![], &[])
            .await
    }

    pub async fn delete(&self, uri: &str, token: &str) -> TestResponse {
        self.send(Method::DELETE, uri, Some(&bearer(token)), None, vec![], &[])
            .await
    }

    pub async fn json(&self, method: Method, uri: &str, token: &str, body: Value) -> TestResponse {
        self.send(
            method,
            uri,
            Some(&bearer(token)),
            Some("application/json"),
            serde_json::to_vec(&body).unwrap(),
            &[],
        )
        .await
    }

    pub async fn multipart(
        &self,
        method: Method,
        uri: &str,
        token: &str,
        parts: &[Part<'_>],
    ) -> TestResponse {
        self.send(
            method,
            uri,
            Some(&bearer(token)),
            Some(&format!("multipart/form-data; boundary={BOUNDARY}")),
            multipart_body(parts),
            &[],
        )
        .await
    }

    pub async fn create_library(&self, name: &str, slug: &str) -> Uuid {
        let res = self
            .json(
                Method::POST,
                "/api/v1/libraries",
                &self.admin.token,
                serde_json::json!({ "name": name, "slug": slug }),
            )
            .await;
        assert_eq!(res.status, StatusCode::CREATED, "{}", res.text());
        Uuid::parse_str(res.json()["id"].as_str().unwrap()).unwrap()
    }

    pub async fn add_member(
        &self,
        library_id: Uuid,
        user: &TestUser,
        role: &str,
        perms: (bool, bool, bool),
    ) {
        let res = self
            .json(
                Method::POST,
                &format!("/api/v1/libraries/{library_id}/members"),
                &self.admin.token,
                serde_json::json!({
                    "user_id": user.id.to_string(),
                    "role": role,
                    "can_view": perms.0,
                    "can_edit": perms.1,
                    "can_delete": perms.2,
                }),
            )
            .await;
        assert_eq!(res.status, StatusCode::NO_CONTENT, "{}", res.text());
    }

    pub async fn create_category(&self, library_id: Uuid, name: &str) {
        let res = self
            .json(
                Method::POST,
                &format!("/api/v1/libraries/{library_id}/categories"),
                &self.admin.token,
                serde_json::json!({ "name": name }),
            )
            .await;
        assert_eq!(res.status, StatusCode::CREATED, "{}", res.text());
    }

    pub async fn upload_book(
        &self,
        token: &str,
        library_id: Uuid,
        category: &str,
        filename: &str,
        data: &[u8],
        metadata: Value,
    ) -> TestResponse {
        let lib = library_id.to_string();
        let meta = metadata.to_string();
        self.multipart(
            Method::POST,
            "/api/v1/books",
            token,
            &[
                Part::Text("library_id", &lib),
                Part::Text("category", category),
                Part::Text("metadata", &meta),
                Part::File {
                    name: "file",
                    filename,
                    content_type: "application/octet-stream",
                    data,
                },
            ],
        )
        .await
    }

    /// Uploads a book as admin into the default category and returns its id.
    pub async fn seed_book(&self, library_id: Uuid, title: &str, data: &[u8]) -> Uuid {
        let res = self
            .upload_book(
                &self.admin.token,
                library_id,
                "未分类",
                "book.pdf",
                data,
                serde_json::json!({ "title": title, "author": "Author" }),
            )
            .await;
        assert_eq!(res.status, StatusCode::CREATED, "{}", res.text());
        Uuid::parse_str(res.json()["id"].as_str().unwrap()).unwrap()
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Every file under `dir` (recursive).
pub fn all_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                out.extend(all_files(&path));
            } else {
                out.push(path);
            }
        }
    }
    out
}

pub fn tiny_png() -> Vec<u8> {
    let img = image::RgbImage::from_pixel(4, 4, image::Rgb([200, 10, 10]));
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
    buf.into_inner()
}

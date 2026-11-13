mod common;

use axum::http::{header, Method, StatusCode};
use base64::Engine;
use common::{basic_auth, bearer, TestApp, JWT_SECRET};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde_json::json;
use shelfie_backend::domain::auth::JwtClaims;
use shelfie_backend::infra::hash::md5_hex;

const ME: &str = "/api/v1/auth/me";

fn claims_for(user: &common::TestUser, role: &str, exp_offset_secs: i64) -> JwtClaims {
    JwtClaims {
        sub: user.id.to_string(),
        username: user.username.clone(),
        role: role.into(),
        exp: chrono::Utc::now().timestamp() + exp_offset_secs,
    }
}

async fn me_with(app: &TestApp, auth: &str) -> StatusCode {
    app.send(Method::GET, ME, Some(auth), None, vec![], &[])
        .await
        .status
}

#[tokio::test]
async fn health_and_swagger_are_public() {
    let app = TestApp::new().await;
    let res = app
        .send(Method::GET, "/api/v1/health", None, None, vec![], &[])
        .await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.json()["status"], "ok");

    let res = app
        .send(
            Method::GET,
            "/api-docs/openapi.json",
            None,
            None,
            vec![],
            &[],
        )
        .await;
    assert_eq!(res.status, StatusCode::OK);
    assert!(res.json()["paths"].is_object());
}

#[tokio::test]
async fn missing_authorization_is_401_with_basic_challenge() {
    let app = TestApp::new().await;
    let res = app.send(Method::GET, ME, None, None, vec![], &[]).await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
    assert_eq!(res.json()["code"], "unauthorized");
    assert!(res
        .headers
        .get(header::WWW_AUTHENTICATE)
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("Basic"));
}

#[tokio::test]
async fn valid_bearer_token_is_accepted() {
    let app = TestApp::new().await;
    let res = app.get(ME, &app.admin.token).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.json()["username"], "admin");
    assert_eq!(res.json()["role"], "system_admin");
}

#[tokio::test]
async fn unknown_scheme_and_garbage_tokens_are_rejected() {
    let app = TestApp::new().await;
    for auth in [
        "Token abc",
        "bearer lowercase-scheme",
        "Bearer ",
        "Bearer not-a-jwt",
        "Bearer a.b.c",
        &format!("Bearer {}x", app.admin.token),
    ] {
        assert_eq!(
            me_with(&app, auth).await,
            StatusCode::UNAUTHORIZED,
            "{auth}"
        );
    }
}

#[tokio::test]
async fn alg_none_token_is_rejected() {
    let app = TestApp::new().await;
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let header = b64.encode(r#"{"alg":"none","typ":"JWT"}"#);
    let payload =
        b64.encode(serde_json::to_vec(&claims_for(&app.admin, "system_admin", 3600)).unwrap());
    for token in [
        format!("{header}.{payload}."),
        format!("{header}.{payload}"),
    ] {
        assert_eq!(
            me_with(&app, &bearer(&token)).await,
            StatusCode::UNAUTHORIZED
        );
    }
}

#[tokio::test]
async fn token_signed_with_other_secret_is_rejected() {
    let app = TestApp::new().await;
    let forged = encode(
        &Header::default(),
        &claims_for(&app.admin, "system_admin", 3600),
        &EncodingKey::from_secret(b"attacker-secret"),
    )
    .unwrap();
    assert_eq!(
        me_with(&app, &bearer(&forged)).await,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn token_with_unexpected_algorithm_is_rejected() {
    let app = TestApp::new().await;
    let token = encode(
        &Header::new(Algorithm::HS512),
        &claims_for(&app.admin, "system_admin", 3600),
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
    .unwrap();
    assert_eq!(
        me_with(&app, &bearer(&token)).await,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn tampered_payload_is_rejected() {
    let app = TestApp::new().await;
    let user = app.create_user("alice", "alice-pass", "user").await;
    let parts: Vec<&str> = user.token.split('.').collect();
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let escalated =
        b64.encode(serde_json::to_vec(&claims_for(&user, "system_admin", 3600)).unwrap());
    let tampered = format!("{}.{}.{}", parts[0], escalated, parts[2]);
    assert_eq!(
        me_with(&app, &bearer(&tampered)).await,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn expired_token_is_rejected() {
    let app = TestApp::new().await;
    let token = encode(
        &Header::default(),
        &claims_for(&app.admin, "system_admin", -3600),
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
    .unwrap();
    assert_eq!(
        me_with(&app, &bearer(&token)).await,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn token_of_deleted_user_is_rejected() {
    let app = TestApp::new().await;
    let user = app.create_user("bob", "bob-pass", "user").await;
    assert_eq!(app.get(ME, &user.token).await.status, StatusCode::OK);

    let res = app
        .delete(&format!("/api/v1/users/{}", user.id), &app.admin.token)
        .await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    assert_eq!(
        app.get(ME, &user.token).await.status,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn role_is_loaded_from_db_not_from_stale_claims() {
    let app = TestApp::new().await;
    let user = app.create_user("demoted", "pass-123", "system_admin").await;
    assert_eq!(
        app.get("/api/v1/users", &user.token).await.status,
        StatusCode::OK
    );

    sqlx::query("UPDATE users SET role = 'user' WHERE id = ?")
        .bind(user.id.to_string())
        .execute(&app.state.db)
        .await
        .unwrap();
    let res = app.get("/api/v1/users", &user.token).await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);
    assert_eq!(app.get(ME, &user.token).await.json()["role"], "user");
}

#[tokio::test]
async fn basic_auth_variants() {
    let app = TestApp::new().await;
    let user = app.create_user("colon", "pa:ss:word", "user").await;

    assert_eq!(
        me_with(&app, &basic_auth(&user.username, &user.password)).await,
        StatusCode::OK
    );
    for auth in [
        basic_auth(&user.username, "wrong"),
        basic_auth("nobody", "x"),
        "Basic !!!not-base64!!!".to_string(),
        format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode("no-colon")
        ),
        format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode([0xff, 0xfe, b':', b'x'])
        ),
    ] {
        assert_eq!(
            me_with(&app, &auth).await,
            StatusCode::UNAUTHORIZED,
            "{auth}"
        );
    }
}

#[tokio::test]
async fn login_success_and_failures() {
    let app = TestApp::new().await;
    let login = |username: &str, password: &str| {
        let body = json!({ "username": username, "password": password });
        let app = &app;
        async move {
            app.send(
                Method::POST,
                "/api/v1/auth/login",
                None,
                Some("application/json"),
                serde_json::to_vec(&body).unwrap(),
                &[],
            )
            .await
        }
    };

    let ok = login("admin", "admin-pass").await;
    assert_eq!(ok.status, StatusCode::OK);
    let token = ok.json()["token"].as_str().unwrap().to_string();
    assert_eq!(app.get(ME, &token).await.status, StatusCode::OK);

    for (u, p) in [
        ("admin", "wrong"),
        ("admin", ""),
        ("", ""),
        ("ghost", "admin-pass"),
        ("admin' OR '1'='1", "x"),
        ("admin'--", "anything"),
        ("' OR 1=1; DROP TABLE users; --", "x"),
    ] {
        let res = login(u, p).await;
        assert_eq!(res.status, StatusCode::UNAUTHORIZED, "{u}/{p}");
        assert_eq!(res.json()["message"], "Invalid credentials");
    }

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&app.state.db)
        .await
        .unwrap();
    assert_eq!(count.0, 1, "users table must survive injection attempts");
}

#[tokio::test]
async fn login_rejects_malformed_body() {
    let app = TestApp::new().await;
    let res = app
        .send(
            Method::POST,
            "/api/v1/auth/login",
            None,
            Some("application/json"),
            b"{\"username\": 1}".to_vec(),
            &[],
        )
        .await;
    assert!(res.status.is_client_error());
}

#[tokio::test]
async fn regular_user_cannot_call_admin_endpoints() {
    let app = TestApp::new().await;
    let user = app.create_user("eve", "eve-pass", "user").await;
    let lib = app.create_library("馆", "lib").await;

    let checks = [
        (Method::GET, "/api/v1/users".to_string(), json!(null)),
        (
            Method::POST,
            "/api/v1/auth/register".to_string(),
            json!({ "username": "x", "password": "123456", "role": "system_admin" }),
        ),
        (
            Method::POST,
            "/api/v1/libraries".to_string(),
            json!({ "name": "evil", "slug": "evil" }),
        ),
        (
            Method::DELETE,
            format!("/api/v1/libraries/{lib}"),
            json!(null),
        ),
        (
            Method::DELETE,
            format!("/api/v1/users/{}", app.admin.id),
            json!(null),
        ),
        (
            Method::PATCH,
            format!("/api/v1/users/{}/password", app.admin.id),
            json!({ "new_password": "hijacked" }),
        ),
        (
            Method::GET,
            format!("/api/v1/users/{}/memberships", app.admin.id),
            json!(null),
        ),
    ];
    for (method, uri, body) in checks {
        let res = app.json(method.clone(), &uri, &user.token, body).await;
        assert_eq!(res.status, StatusCode::FORBIDDEN, "{method} {uri}");
    }

    let res = app
        .json(
            Method::PATCH,
            &format!("/api/v1/users/{}/username", app.admin.id),
            &user.token,
            json!({ "username": "pwned" }),
        )
        .await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn change_own_password_requires_current_password() {
    let app = TestApp::new().await;
    let user = app.create_user("carol", "old-pass", "user").await;
    let uri = format!("/api/v1/users/{}/password", user.id);

    let res = app
        .json(
            Method::PATCH,
            &uri,
            &user.token,
            json!({ "new_password": "new-pass" }),
        )
        .await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST);

    let res = app
        .json(
            Method::PATCH,
            &uri,
            &user.token,
            json!({ "current_password": "guess", "new_password": "new-pass" }),
        )
        .await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);

    let res = app
        .json(
            Method::PATCH,
            &uri,
            &user.token,
            json!({ "current_password": "old-pass", "new_password": "123" }),
        )
        .await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST);

    let res = app
        .json(
            Method::PATCH,
            &uri,
            &user.token,
            json!({ "current_password": "old-pass", "new_password": "new-pass" }),
        )
        .await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);

    assert_eq!(
        me_with(&app, &basic_auth("carol", "old-pass")).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        me_with(&app, &basic_auth("carol", "new-pass")).await,
        StatusCode::OK
    );
    let md5: (String,) = sqlx::query_as("SELECT password_md5 FROM users WHERE id = ?")
        .bind(user.id.to_string())
        .fetch_one(&app.state.db)
        .await
        .unwrap();
    assert_eq!(md5.0, md5_hex(b"new-pass"));
}

#[tokio::test]
async fn admin_can_reset_other_password_without_current() {
    let app = TestApp::new().await;
    let user = app.create_user("dave", "old-pass", "user").await;
    let res = app
        .json(
            Method::PATCH,
            &format!("/api/v1/users/{}/password", user.id),
            &app.admin.token,
            json!({ "new_password": "reset-pass" }),
        )
        .await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    assert_eq!(
        me_with(&app, &basic_auth("dave", "reset-pass")).await,
        StatusCode::OK
    );
}

#[tokio::test]
async fn update_username_rules() {
    let app = TestApp::new().await;
    let user = app.create_user("frank", "pass-123", "user").await;
    app.create_user("taken", "pass-123", "user").await;
    let uri = format!("/api/v1/users/{}/username", user.id);

    for (name, status) in [
        ("   ", StatusCode::BAD_REQUEST),
        (&"x".repeat(65) as &str, StatusCode::BAD_REQUEST),
        ("taken", StatusCode::CONFLICT),
    ] {
        let res = app
            .json(
                Method::PATCH,
                &uri,
                &user.token,
                json!({ "username": name }),
            )
            .await;
        assert_eq!(res.status, status, "{name}");
    }

    let res = app
        .json(
            Method::PATCH,
            "/api/v1/users/not-a-uuid/username",
            &user.token,
            json!({ "username": "x" }),
        )
        .await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST);

    let res = app
        .json(
            Method::PATCH,
            &uri,
            &user.token,
            json!({ "username": "  franky  " }),
        )
        .await;
    assert_eq!(res.status, StatusCode::OK);
    let body = res.json();
    assert_eq!(body["user"]["username"], "franky");
    let new_token = body["token"].as_str().unwrap();
    assert_eq!(app.get(ME, new_token).await.json()["username"], "franky");
}

#[tokio::test]
async fn register_validation() {
    let app = TestApp::new().await;
    let lib = app.create_library("馆", "lib").await;
    let register = |body: serde_json::Value| {
        app.json(
            Method::POST,
            "/api/v1/auth/register",
            &app.admin.token,
            body,
        )
    };

    let cases = [
        (
            json!({ "username": "u1", "password": "123456", "role": "root" }),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({ "username": "u1", "password": "123456" }),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({ "username": "u1", "password": "123456", "role": "system_admin", "library_id": lib }),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({ "username": "   ", "password": "123456", "library_id": lib }),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({ "username": "u1", "password": "123", "library_id": lib }),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({ "username": "u1", "password": "123456", "library_id": lib, "library_role": "owner" }),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({ "username": "u1", "password": "123456", "library_id": uuid::Uuid::new_v4() }),
            StatusCode::NOT_FOUND,
        ),
        (
            json!({ "username": "admin", "password": "123456", "role": "system_admin" }),
            StatusCode::CONFLICT,
        ),
    ];
    for (body, status) in cases {
        let res = register(body.clone()).await;
        assert_eq!(res.status, status, "{body}: {}", res.text());
    }

    let res =
        register(json!({ "username": "reader", "password": "123456", "library_id": lib })).await;
    assert_eq!(res.status, StatusCode::CREATED, "{}", res.text());
    assert_eq!(res.json()["role"], "user");

    let users = app.get("/api/v1/users", &app.admin.token).await.json();
    let names: Vec<&str> = users
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["username"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        vec!["admin", "reader"],
        "rejected registrations must not leave users behind"
    );
}

#[tokio::test]
async fn admin_cannot_delete_self_and_unknown_user_is_404() {
    let app = TestApp::new().await;
    let res = app
        .delete(&format!("/api/v1/users/{}", app.admin.id), &app.admin.token)
        .await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST);

    let res = app
        .delete(
            &format!("/api/v1/users/{}", uuid::Uuid::new_v4()),
            &app.admin.token,
        )
        .await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);

    let res = app
        .delete("/api/v1/users/not-a-uuid", &app.admin.token)
        .await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn kosync_authentication() {
    let app = TestApp::new().await;
    let user = app.create_user("koreader", "ko-pass", "user").await;
    let key = md5_hex(b"ko-pass");

    let auth = |u: &str, k: &str| {
        let u = u.to_string();
        let k = k.to_string();
        let app = &app;
        async move {
            app.send(
                Method::GET,
                "/api/v1/users/auth",
                None,
                None,
                vec![],
                &[("x-auth-user", &u), ("x-auth-key", &k)],
            )
            .await
            .status
        }
    };

    assert_eq!(auth(&user.username, &key).await, StatusCode::OK);
    for (u, k) in [
        ("", ""),
        (user.username.as_str(), ""),
        (user.username.as_str(), "ko-pass"),
        (user.username.as_str(), &key.to_uppercase()),
        ("ghost", key.as_str()),
        ("koreader' OR '1'='1", key.as_str()),
    ] {
        assert_eq!(auth(u, k).await, StatusCode::UNAUTHORIZED, "{u}/{k}");
    }

    sqlx::query("UPDATE users SET password_md5 = NULL WHERE id = ?")
        .bind(user.id.to_string())
        .execute(&app.state.db)
        .await
        .unwrap();
    assert_eq!(auth(&user.username, &key).await, StatusCode::UNAUTHORIZED);

    // Protected kosync routes require the headers too
    let res = app
        .send(
            Method::GET,
            "/api/v1/syncs/progress/abc",
            None,
            None,
            vec![],
            &[],
        )
        .await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);

    let res = app
        .send(Method::GET, "/api/v1/healthcheck", None, None, vec![], &[])
        .await;
    assert_eq!(res.status, StatusCode::OK);
}

#[tokio::test]
async fn web_login_enables_kosync_key() {
    let app = TestApp::new().await;
    let user = app.create_user("sync", "sync-pass", "user").await;
    sqlx::query("UPDATE users SET password_md5 = NULL WHERE id = ?")
        .bind(user.id.to_string())
        .execute(&app.state.db)
        .await
        .unwrap();

    let res = app
        .send(
            Method::POST,
            "/api/v1/auth/login",
            None,
            Some("application/json"),
            serde_json::to_vec(&json!({ "username": "sync", "password": "sync-pass" })).unwrap(),
            &[],
        )
        .await;
    assert_eq!(res.status, StatusCode::OK);

    let res = app
        .send(
            Method::GET,
            "/api/v1/users/auth",
            None,
            None,
            vec![],
            &[
                ("x-auth-user", "sync"),
                ("x-auth-key", &md5_hex(b"sync-pass")),
            ],
        )
        .await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.json()["authorized"], "OK");
}

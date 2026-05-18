use crate::api::middleware::auth::AuthContext;
use crate::domain::book::{self, BookCard, BookDetail, UpdateBookRequest, UpdateProgressRequest};
use crate::error::{AppError, AppResult};
use crate::infra::{fs, BookMetadata};
use crate::state::AppState;
use axum::{
    body::Body,
    extract::{multipart::Multipart, Path, Query, State},
    http::{header, StatusCode},
    response::Response,
    Extension, Json,
};
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

fn multipart_error(err: impl std::fmt::Display) -> AppError {
    let msg = err.to_string();
    if msg.contains("length limit") || msg.contains("too large") {
        AppError::BadRequest(
            "Upload too large. Maximum total size is 512MB.".into(),
        )
    } else if msg.contains("multipart") || msg.contains("boundary") {
        AppError::BadRequest(format!("Invalid upload request: {msg}"))
    } else {
        AppError::BadRequest(msg)
    }
}

/// 解析可选封面字段；空 part 视为未上传
fn parse_cover_field(
    data: &[u8],
    filename: Option<&str>,
    content_type: Option<&str>,
) -> AppResult<(Option<Vec<u8>>, Option<String>)> {
    if data.is_empty() {
        return Ok((None, None));
    }
    let ext = fs::resolve_cover_extension(filename, content_type).ok_or_else(|| {
        AppError::BadRequest(
            "Cannot detect cover format. Use .jpg or .png filename.".into(),
        )
    })?;
    Ok((Some(data.to_vec()), Some(ext)))
}

fn parse_metadata_field(data: &[u8]) -> AppResult<BookMetadata> {
    if data.is_empty() {
        return Ok(BookMetadata::default());
    }
    serde_json::from_slice(data)
        .or_else(|_| {
            let text = std::str::from_utf8(data)
                .map_err(|e| AppError::BadRequest(e.to_string()))?;
            serde_json::from_str(text).map_err(|e| {
                AppError::BadRequest(format!("Invalid metadata JSON: {e}"))
            })
        })
        .map_err(Into::into)
}

#[derive(Deserialize, IntoParams)]
pub struct ListBooksQuery {
    pub library_id: Option<Uuid>,
    pub sort: Option<String>,
    #[param(default = 50)]
    pub limit: Option<i64>,
}

#[utoipa::path(get, path = "/books", tag = "Books", params(ListBooksQuery), responses((status = 200, body = [BookCard])), security(("bearer_auth" = [])))]
pub async fn list_books(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Query(q): Query<ListBooksQuery>,
) -> AppResult<Json<Vec<BookCard>>> {
    let limit = q.limit.unwrap_or(50);
    Ok(Json(
        book::list_books(
            &state.db,
            &user,
            q.library_id,
            q.sort.as_deref(),
            limit,
        )
        .await?,
    ))
}

#[utoipa::path(get, path = "/books/{id}", tag = "Books", params(("id" = Uuid, Path)), responses((status = 200, body = BookDetail)), security(("bearer_auth" = [])))]
pub async fn get_book(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<BookDetail>> {
    Ok(Json(book::get_book(&state, &user, &id).await?))
}

#[utoipa::path(
    post,
    path = "/books",
    tag = "Books",
    request_body(content_type = "multipart/form-data", description = "Fields: library_id, category, file (required); metadata, cover (optional)"),
    responses((status = 201, body = BookDetail)),
    security(("bearer_auth" = []))
)]
pub async fn upload_book(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    mut multipart: Multipart,
) -> AppResult<(StatusCode, Json<BookDetail>)> {
    let mut library_id: Option<Uuid> = None;
    let mut category: Option<String> = None;
    let mut metadata: Option<BookMetadata> = None;
    let mut book_bytes: Option<Vec<u8>> = None;
    let mut book_ext: Option<String> = None;
    let mut cover_bytes: Option<Vec<u8>> = None;
    let mut cover_ext: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| multipart_error(e))?
    {
        let name = field.name().unwrap_or("").to_string();
        let filename = field.file_name().map(|s| s.to_string());
        let content_type = field.content_type().map(|s| s.to_string());
        let data = field.bytes().await.map_err(|e| multipart_error(e))?;

        match name.as_str() {
            "library_id" => {
                library_id = Some(
                    Uuid::parse_str(std::str::from_utf8(&data).map_err(|e| AppError::BadRequest(e.to_string()))?)
                        .map_err(|e| AppError::BadRequest(e.to_string()))?,
                );
            }
            "category" => {
                category = Some(String::from_utf8_lossy(&data).to_string());
            }
            "metadata" => {
                metadata = Some(parse_metadata_field(&data)?);
            }
            "file" => {
                book_ext = fs::resolve_book_extension(filename.as_deref(), content_type.as_deref());
                book_bytes = Some(data.to_vec());
            }
            "cover" => {
                let (bytes, ext) = parse_cover_field(
                    &data,
                    filename.as_deref(),
                    content_type.as_deref(),
                )?;
                cover_bytes = bytes;
                cover_ext = ext;
            }
            _ => {}
        }
    }

    let library_id = library_id.ok_or_else(|| AppError::BadRequest("library_id required".into()))?;
    let category = category.ok_or_else(|| AppError::BadRequest("category required".into()))?;
    let book_bytes = book_bytes.ok_or_else(|| AppError::BadRequest("file required".into()))?;
    let book_ext = book_ext.ok_or_else(|| {
        AppError::BadRequest(
            "Cannot detect book format. Use .pdf, .epub, or .mobi filename.".into(),
        )
    })?;
    let metadata = metadata.unwrap_or_default();

    let detail = book::upload_book(
        &state,
        &user,
        library_id,
        category,
        book_bytes,
        book_ext,
        cover_bytes,
        cover_ext,
        metadata,
    )
    .await?;

    Ok((StatusCode::CREATED, Json(detail)))
}

#[utoipa::path(patch, path = "/books/{id}", tag = "Books", params(("id" = Uuid, Path)), request_body = UpdateBookRequest, responses((status = 200, body = BookDetail)), security(("bearer_auth" = [])))]
pub async fn update_book(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateBookRequest>,
) -> AppResult<Json<BookDetail>> {
    Ok(Json(book::update_book(&state, &user, &id, req.metadata).await?))
}

#[utoipa::path(patch, path = "/books/{id}/progress", tag = "Books", params(("id" = Uuid, Path)), request_body = UpdateProgressRequest, responses((status = 200, body = BookDetail)), security(("bearer_auth" = [])))]
pub async fn update_progress(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateProgressRequest>,
) -> AppResult<Json<BookDetail>> {
    Ok(Json(
        book::update_progress(&state, &user, &id, req.reading_progress).await?,
    ))
}

#[utoipa::path(delete, path = "/books/{id}", tag = "Books", params(("id" = Uuid, Path)), responses((status = 204)), security(("bearer_auth" = [])))]
pub async fn delete_book(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    book::delete_book(&state, &user, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    put,
    path = "/books/{id}/cover",
    tag = "Books",
    params(("id" = Uuid, Path)),
    request_body(content_type = "multipart/form-data"),
    responses((status = 200, body = BookDetail)),
    security(("bearer_auth" = []))
)]
pub async fn update_cover(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    mut multipart: Multipart,
) -> AppResult<Json<BookDetail>> {
    let mut cover_bytes: Option<Vec<u8>> = None;
    let mut cover_ext: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| multipart_error(e))?
    {
        if field.name() == Some("cover") {
            let filename = field.file_name().map(|s| s.to_string());
            let content_type = field.content_type().map(|s| s.to_string());
            let data = field.bytes().await.map_err(|e| multipart_error(e))?;
            cover_ext = fs::resolve_cover_extension(filename.as_deref(), content_type.as_deref());
            cover_bytes = Some(data.to_vec());
        }
    }

    let cover_bytes = cover_bytes.ok_or_else(|| AppError::BadRequest("cover required".into()))?;
    let cover_ext = cover_ext.ok_or_else(|| {
        AppError::BadRequest("Cannot detect cover format. Use .jpg or .png filename.".into())
    })?;

    Ok(Json(
        book::update_cover(&state, &user, &id, cover_bytes, cover_ext).await?,
    ))
}

#[utoipa::path(
    get,
    path = "/books/{id}/download",
    tag = "Books",
    params(("id" = Uuid, Path)),
    responses((status = 200, description = "Book file download")),
    security(("bearer_auth" = []))
)]
pub async fn download_book(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> AppResult<Response> {
    let (path, filename) = book::get_download_path(&state, &user, &id).await?;
    let bytes = tokio::fs::read(&path).await?;
    let mime = mime_guess::from_path(&path)
        .first_or_octet_stream()
        .to_string();
    let disposition = crate::infra::content_disposition::attachment_filename(&filename);

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .header(header::CONTENT_DISPOSITION, disposition)
        .body(Body::from(bytes))
        .unwrap())
}

#[utoipa::path(get, path = "/assets/covers/{id}", tag = "Covers", params(("id" = Uuid, Path)), responses((status = 200, content_type = "image/jpeg")), security(("bearer_auth" = [])))]
pub async fn get_cover(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> AppResult<Response> {
    let path = book::get_cover_path(&state, &user, &id).await?;
    let bytes = tokio::fs::read(&path).await?;
    let mime = mime_guess::from_path(&path)
        .first_or_octet_stream()
        .to_string();

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .body(Body::from(bytes))
        .unwrap())
}

#[cfg(test)]
mod upload_cover_tests {
    use super::parse_cover_field;

    #[test]
    fn parse_cover_field_empty_means_optional() {
        let (bytes, ext) = parse_cover_field(&[], None, None).unwrap();
        assert!(bytes.is_none());
        assert!(ext.is_none());
    }

    #[test]
    fn parse_cover_field_with_jpg() {
        let (bytes, ext) = parse_cover_field(b"x", Some("a.jpg"), Some("image/jpeg")).unwrap();
        assert_eq!(bytes, Some(vec![b'x']));
        assert_eq!(ext.as_deref(), Some("jpg"));
    }
}

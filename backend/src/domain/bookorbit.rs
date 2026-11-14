//! BookOrbit KOReader 插件 API（`/koreader/plugin/*`）：书籍匹配、标注/笔记与书签三方交换、
//! 页面阅读统计、阅读状态。Readest 的 BookOrbit 集成即使用这套接口。
//!
//! 设备以 `md5("{datetime}|{pos0}")` 作为条目身份 key；同步状态按「设备 + 文档哈希」记录，
//! 服务端只下发设备尚未确认的变更，设备回执（ack）后推进。

use crate::domain::annotations::{now_ts, persist_metadata, refresh_snapshot};
use crate::domain::auth::AuthUser;
use crate::domain::koreader::resolve_book_for_document;
use crate::domain::{book, library};
use crate::error::{AppError, AppResult};
use crate::infra::BookMetadata;
use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{SqliteConnection, SqlitePool};
use std::collections::{HashMap, HashSet};
use utoipa::ToSchema;
use uuid::Uuid;

const MAX_MATCH_CHECK_HASHES: usize = 500;
const MAX_EXCHANGE_BOOKS: usize = 20;
const MAX_EXCHANGE_CHANGES: usize = 50;
const MAX_KEYS_PER_BOOK: usize = 5000;
const PUSH_DOWN_PAGE: usize = 100;
const MAX_PAGE_STATS_BOOKS: usize = 50;
const MAX_PAGE_STATS_EVENTS: usize = 500;
const MAX_BOOK_STATES: usize = 200;
/// 设备上报的编辑时间超过服务端当前时间该秒数视为时钟异常，退回按设备时间字符串比较
const DEVICE_CLOCK_TOLERANCE_SECS: i64 = 60;
const DEVICE_DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

// ---------- 版本 / 匹配 ----------

/// `GET /koreader/plugin/version`
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginVersionInfo {
    pub plugin_version: String,
    pub server_version: String,
    /// 未声明的能力（如 `bookmarkSync`）客户端不会调用
    pub capabilities: Vec<String>,
}

/// `POST /koreader/plugin/match-check`；设备字段等其余字段忽略
#[derive(Debug, Deserialize, ToSchema)]
pub struct PluginMatchCheckRequest {
    pub hashes: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginMatch {
    pub hash: String,
    pub book_id: String,
    /// Shelfie 一书一文件，与 `book_id` 相同
    pub book_file_id: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PluginMatchCheckResponse {
    pub matches: Vec<PluginMatch>,
}

// ---------- 标注交换 ----------

#[derive(Debug, Deserialize, ToSchema)]
pub struct ExchangeKey {
    pub k: String,
    pub dt: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct KoAnnotation {
    pub datetime: String,
    pub datetime_updated: Option<String>,
    /// lighten / underscore / strikeout / invert
    pub drawer: String,
    pub color: Option<String>,
    pub text: Option<String>,
    pub note: Option<String>,
    pub chapter: Option<String>,
    pub pageno: Option<i64>,
    /// xpointer / pdf
    pub pos_format: String,
    pub pos0: String,
    pub pos1: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationExchangeBook {
    pub hash: String,
    /// 设备上该书全部有效标注的身份 key
    #[serde(default)]
    pub keys: Vec<ExchangeKey>,
    /// `keys` 是否完整；完整时缺失的已同步条目视为设备端删除
    #[serde(default)]
    pub keys_complete: bool,
    #[serde(default)]
    pub changes: Vec<KoAnnotation>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationExchangeRequest {
    pub device_id: String,
    /// 设备当前时间 `YYYY-MM-DD HH:MM:SS`，用于换算设备编辑时间
    pub device_time: Option<String>,
    pub books: Vec<AnnotationExchangeBook>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeAddEntry {
    pub server_id: i64,
    pub version: i64,
    pub datetime: String,
    pub datetime_updated: Option<String>,
    pub drawer: String,
    pub color: String,
    pub text: String,
    pub note: Option<String>,
    pub chapter: Option<String>,
    pub pos_format: String,
    pub pos0: String,
    pub pos1: Option<String>,
    pub pageno: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeEditEntry {
    pub server_id: i64,
    pub version: i64,
    pub key: String,
    pub datetime: Option<String>,
    pub datetime_updated: String,
    pub drawer: String,
    pub color: String,
    pub text: String,
    pub note: Option<String>,
    pub chapter: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeDeleteEntry {
    pub server_id: i64,
    pub key: String,
    pub datetime: Option<String>,
}

#[derive(Debug, Default, Serialize, ToSchema)]
pub struct AnnotationToApply {
    pub add: Vec<ExchangeAddEntry>,
    pub edit: Vec<ExchangeEditEntry>,
    pub delete: Vec<ExchangeDeleteEntry>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationExchangeResult {
    pub hash: String,
    pub book_id: String,
    pub to_apply: AnnotationToApply,
    /// 还有未下发的变更，客户端应再次交换
    pub more: bool,
    pub skipped_no_position: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AnnotationExchangeResponse {
    pub results: Vec<AnnotationExchangeResult>,
    pub unmatched: Vec<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationAckApplied {
    pub server_id: i64,
    pub version: i64,
    /// applied / failed
    pub status: String,
    pub verified: Option<bool>,
    pub corrected: Option<bool>,
    pub pos0: Option<String>,
    pub pos1: Option<String>,
    pub pageno: Option<i64>,
    pub datetime_updated: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AckDeleted {
    pub server_id: i64,
    pub status: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AnnotationAckBook {
    pub hash: String,
    #[serde(default)]
    pub applied: Vec<AnnotationAckApplied>,
    #[serde(default)]
    pub deleted: Vec<AckDeleted>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationAckRequest {
    pub device_id: String,
    pub books: Vec<AnnotationAckBook>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AckResponse {
    pub acked: i64,
}

// ---------- 书签交换 ----------

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct KoBookmark {
    pub datetime: String,
    pub datetime_updated: Option<String>,
    pub pos: String,
    pub pageno: Option<i64>,
    pub chapter: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BookmarkExchangeBook {
    pub hash: String,
    #[serde(default)]
    pub keys: Vec<ExchangeKey>,
    #[serde(default)]
    pub keys_complete: bool,
    #[serde(default)]
    pub changes: Vec<KoBookmark>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BookmarkExchangeRequest {
    pub device_id: String,
    pub books: Vec<BookmarkExchangeBook>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BookmarkAddEntry {
    pub server_id: i64,
    pub pos: String,
    pub pageno: Option<i64>,
    pub title: String,
}

#[derive(Debug, Default, Serialize, ToSchema)]
pub struct BookmarkToApply {
    pub add: Vec<BookmarkAddEntry>,
    pub delete: Vec<ExchangeDeleteEntry>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BookmarkExchangeResult {
    pub hash: String,
    pub book_id: String,
    pub to_apply: BookmarkToApply,
    pub more: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BookmarkExchangeResponse {
    pub results: Vec<BookmarkExchangeResult>,
    pub unmatched: Vec<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BookmarkAckApplied {
    pub server_id: i64,
    pub status: String,
    pub key: Option<String>,
    pub datetime: Option<String>,
    pub pos: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct BookmarkAckBook {
    pub hash: String,
    #[serde(default)]
    pub applied: Vec<BookmarkAckApplied>,
    #[serde(default)]
    pub deleted: Vec<AckDeleted>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BookmarkAckRequest {
    pub device_id: String,
    pub books: Vec<BookmarkAckBook>,
}

// ---------- 统计 / 状态 ----------

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PageStatsEvent {
    pub page: i64,
    /// Unix 秒
    pub start_time: i64,
    pub duration_seconds: i64,
    pub total_pages: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PageStatsBook {
    pub hash: String,
    pub events: Vec<PageStatsEvent>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PageStatsRequest {
    pub device_id: String,
    pub books: Vec<PageStatsBook>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PageStatsResult {
    pub hash: String,
    pub accepted: i64,
    pub duplicates: i64,
    pub watermark: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PageStatsResponse {
    pub results: Vec<PageStatsResult>,
    pub unmatched: Vec<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BookStateEntry {
    pub hash: String,
    /// reading / complete / abandoned
    pub status: Option<String>,
    /// 设备端状态修改日期 `YYYY-MM-DD`；评分冲突时也按此日期判断
    pub status_modified: Option<String>,
    /// 1-5，写入图书元数据评分
    pub rating: Option<i64>,
    /// 设备端清除了评分
    pub rating_cleared: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct BookStatesRequest {
    pub books: Vec<BookStateEntry>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BookStateResult {
    pub hash: String,
    /// 设备状态是否被采纳（同日或更旧的修改不覆盖服务端）
    pub status_applied: bool,
    pub status: Option<String>,
    pub status_modified: Option<String>,
    /// 设备评分是否被采纳（无编辑权限或修改不比图书新时不采纳）
    pub rating_applied: bool,
    pub rating: Option<i64>,
    pub rating_set: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BookStatesResponse {
    pub results: Vec<BookStateResult>,
    pub unmatched: Vec<String>,
}

// ---------- 通用 ----------

fn bad(msg: &str) -> AppError {
    AppError::BadRequest(msg.into())
}

fn is_md5_hex(s: &str) -> bool {
    s.len() == 32 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

fn parse_device_datetime(s: &str) -> Option<NaiveDateTime> {
    if s.len() != 19 {
        return None;
    }
    NaiveDateTime::parse_from_str(s, DEVICE_DATETIME_FORMAT).ok()
}

fn device_key(datetime: &str, pos: &str) -> String {
    format!("{:x}", md5::compute(format!("{datetime}|{pos}")))
}

fn sync_device_id(device_id: &str, hash: &str) -> String {
    format!("{device_id}:{hash}")
}

fn check_device_id(device_id: &str) -> AppResult<()> {
    if device_id.is_empty() || device_id.len() > 100 {
        return Err(bad("deviceId must be 1-100 characters"));
    }
    Ok(())
}

fn check_len(value: Option<&str>, max: usize, field: &str) -> AppResult<()> {
    if value.is_some_and(|v| v.chars().count() > max) {
        return Err(AppError::BadRequest(format!("{field} is too long")));
    }
    Ok(())
}

fn check_hash(hash: &str) -> AppResult<()> {
    if !is_md5_hex(hash) {
        return Err(bad("invalid hash"));
    }
    Ok(())
}

fn check_keys(keys: &[ExchangeKey]) -> AppResult<()> {
    if keys.len() > MAX_KEYS_PER_BOOK {
        return Err(bad("too many keys"));
    }
    if keys
        .iter()
        .any(|k| !is_md5_hex(&k.k) || parse_device_datetime(&k.dt).is_none())
    {
        return Err(bad("invalid key"));
    }
    Ok(())
}

/// 设备时间相对服务端的偏移（秒）
fn clock_offset(device_time: Option<&str>) -> Option<i64> {
    let device = parse_device_datetime(device_time?)?;
    Some(device.and_utc().timestamp() - Utc::now().timestamp())
}

/// 把设备编辑时间换算到服务端时间轴；时钟不可信时返回 None
fn placed_edit_time(datetime: &str, offset: Option<i64>) -> Option<DateTime<Utc>> {
    let placed = parse_device_datetime(datetime)?.and_utc() - Duration::seconds(offset?);
    let now = Utc::now();
    if placed > now + Duration::seconds(DEVICE_CLOCK_TOLERANCE_SECS) {
        return None;
    }
    Some(placed.min(now))
}

fn parse_ts(ts: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(ts)
        .map(|t| t.with_timezone(&Utc))
        .unwrap_or_default()
}

fn format_ts(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn normalize(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

/// 解析请求中的哈希 → 图书；同一哈希只解析一次
async fn resolve_hashes<'a>(
    db: &SqlitePool,
    user: &AuthUser,
    hashes: impl Iterator<Item = &'a str>,
) -> AppResult<HashMap<String, Option<Uuid>>> {
    let mut resolved = HashMap::new();
    for hash in hashes {
        let hash = hash.to_lowercase();
        if resolved.contains_key(&hash) {
            continue;
        }
        let book = resolve_book_for_document(db, user, &hash).await?;
        resolved.insert(hash, book);
    }
    Ok(resolved)
}

// ---------- 版本 / 匹配实现 ----------

pub fn plugin_version() -> PluginVersionInfo {
    PluginVersionInfo {
        plugin_version: "unknown".into(),
        server_version: env!("CARGO_PKG_VERSION").into(),
        capabilities: vec!["bookmarkSync".into()],
    }
}

/// 与进度上报相同的匹配规则（手动链接 → MD5 / partial MD5），命中时同样写入自动链接
pub async fn plugin_match_check(
    db: &SqlitePool,
    user: &AuthUser,
    req: PluginMatchCheckRequest,
) -> AppResult<PluginMatchCheckResponse> {
    if req.hashes.is_empty() || req.hashes.len() > MAX_MATCH_CHECK_HASHES {
        return Err(bad("hashes must contain 1-500 items"));
    }
    if !req.hashes.iter().all(|h| is_md5_hex(h)) {
        return Err(bad("invalid hash"));
    }

    let mut hashes: Vec<String> = req.hashes.iter().map(|h| h.to_lowercase()).collect();
    hashes.sort();
    hashes.dedup();

    let mut matches = Vec::new();
    for hash in hashes {
        if let Some(book_id) = resolve_book_for_document(db, user, &hash).await? {
            matches.push(PluginMatch {
                hash,
                book_id: book_id.to_string(),
                book_file_id: book_id.to_string(),
            });
        }
    }
    Ok(PluginMatchCheckResponse { matches })
}

// ---------- 标注实现 ----------

const ANNOTATION_COLS: &str = "a.id, a.text, a.note, a.chapter, a.drawer, a.color, a.pos_format, \
     a.pos0, a.pos1, a.pageno, a.device_datetime, a.device_updated, a.version, a.updated_at, a.deleted_at";

#[derive(Debug, sqlx::FromRow)]
struct AnnotationRow {
    id: i64,
    text: String,
    note: Option<String>,
    chapter: Option<String>,
    drawer: String,
    color: Option<String>,
    pos_format: String,
    pos0: String,
    pos1: Option<String>,
    pageno: Option<i64>,
    device_datetime: String,
    device_updated: Option<String>,
    version: i64,
    updated_at: String,
    deleted_at: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct AnnotationState {
    external_key: String,
    external_datetime: Option<String>,
    last_applied_version: i64,
    delete_acked_at: Option<String>,
    #[sqlx(flatten)]
    ann: AnnotationRow,
}

fn validate_annotation(a: &KoAnnotation) -> AppResult<()> {
    if parse_device_datetime(&a.datetime).is_none()
        || a.datetime_updated
            .as_deref()
            .is_some_and(|d| parse_device_datetime(d).is_none())
    {
        return Err(bad("invalid annotation datetime"));
    }
    if !["lighten", "underscore", "strikeout", "invert"].contains(&a.drawer.as_str()) {
        return Err(bad("invalid drawer"));
    }
    if !["xpointer", "pdf"].contains(&a.pos_format.as_str()) {
        return Err(bad("invalid posFormat"));
    }
    if a.pos0.is_empty() {
        return Err(bad("pos0 is required"));
    }
    check_len(Some(&a.pos0), 4000, "pos0")?;
    check_len(a.pos1.as_deref(), 4000, "pos1")?;
    check_len(a.text.as_deref(), 10000, "text")?;
    check_len(a.note.as_deref(), 10000, "note")?;
    check_len(a.chapter.as_deref(), 500, "chapter")?;
    check_len(a.color.as_deref(), 30, "color")?;
    Ok(())
}

pub async fn exchange_annotations(
    db: &SqlitePool,
    user: &AuthUser,
    req: AnnotationExchangeRequest,
) -> AppResult<AnnotationExchangeResponse> {
    check_device_id(&req.device_id)?;
    if req.books.is_empty() || req.books.len() > MAX_EXCHANGE_BOOKS {
        return Err(bad("books must contain 1-20 items"));
    }
    if req.books.iter().map(|b| b.changes.len()).sum::<usize>() > MAX_EXCHANGE_CHANGES {
        return Err(bad("too many changes"));
    }
    for book in &req.books {
        check_hash(&book.hash)?;
        check_keys(&book.keys)?;
        book.changes.iter().try_for_each(validate_annotation)?;
    }

    let offset = clock_offset(req.device_time.as_deref());
    let resolved = resolve_hashes(db, user, req.books.iter().map(|b| b.hash.as_str())).await?;
    let user_id = user.id.to_string();
    let mut results = Vec::new();
    let mut unmatched = Vec::new();
    for book in &req.books {
        let hash = book.hash.to_lowercase();
        let Some(book_id) = resolved.get(&hash).copied().flatten() else {
            unmatched.push(hash);
            continue;
        };
        let device = sync_device_id(&req.device_id, &hash);
        let mut tx = db.begin().await?;
        let changed =
            ingest_annotations(&mut tx, &user_id, &book_id, &device, book, offset).await?;
        let (to_apply, more, skipped_no_position) =
            push_down_annotations(&mut tx, &user_id, &book_id, &device, offset).await?;
        tx.commit().await?;
        if changed {
            refresh_snapshot(db, &book_id).await?;
        }
        results.push(AnnotationExchangeResult {
            hash,
            book_id: book_id.to_string(),
            to_apply,
            more,
            skipped_no_position,
        });
    }
    Ok(AnnotationExchangeResponse { results, unmatched })
}

async fn load_annotation_states(
    conn: &mut SqliteConnection,
    user_id: &str,
    book_id: &Uuid,
    device: &str,
) -> AppResult<Vec<AnnotationState>> {
    Ok(sqlx::query_as(sqlx::AssertSqlSafe(format!(
        r#"
        SELECT s.external_key, s.external_datetime, s.last_applied_version, s.delete_acked_at,
               {ANNOTATION_COLS}
        FROM annotation_sync_states s JOIN book_annotations a ON a.id = s.annotation_id
        WHERE s.device_id = ? AND a.user_id = ? AND a.book_id = ?
        ORDER BY a.id
        "#
    )))
    .bind(device)
    .bind(user_id)
    .bind(book_id.to_string())
    .fetch_all(&mut *conn)
    .await?)
}

async fn upsert_annotation_state(
    conn: &mut SqliteConnection,
    annotation_id: i64,
    device: &str,
    key: &str,
    datetime: &str,
    last_applied_version: i64,
) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO annotation_sync_states
            (annotation_id, device_id, external_key, external_datetime, last_applied_version,
             delete_acked_at, updated_at)
        VALUES (?, ?, ?, ?, ?, NULL, ?)
        ON CONFLICT(annotation_id, device_id) DO UPDATE SET
            external_key = excluded.external_key,
            external_datetime = excluded.external_datetime,
            last_applied_version = excluded.last_applied_version,
            delete_acked_at = NULL,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(annotation_id)
    .bind(device)
    .bind(key)
    .bind(datetime)
    .bind(last_applied_version)
    .bind(now_ts())
    .execute(&mut *conn)
    .await?;
    Ok(())
}

async fn set_last_applied(
    conn: &mut SqliteConnection,
    annotation_id: i64,
    device: &str,
    version: i64,
) -> AppResult<()> {
    sqlx::query(
        "UPDATE annotation_sync_states SET last_applied_version = ?, updated_at = ? \
         WHERE annotation_id = ? AND device_id = ?",
    )
    .bind(version)
    .bind(now_ts())
    .bind(annotation_id)
    .bind(device)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// 处理设备上报：新增、编辑（后写者胜）与设备端删除。返回是否有内容变化
async fn ingest_annotations(
    conn: &mut SqliteConnection,
    user_id: &str,
    book_id: &Uuid,
    device: &str,
    book: &AnnotationExchangeBook,
    offset: Option<i64>,
) -> AppResult<bool> {
    let present_keys: HashSet<&str> = book.keys.iter().map(|k| k.k.as_str()).collect();
    let present_dts: HashSet<&str> = book.keys.iter().map(|k| k.dt.as_str()).collect();

    // 设备本地数据丢失（重装等）：此前确认过的条目一个都不在 keys 中 → 全部重新下发，而不是当作删除
    let states = load_annotation_states(conn, user_id, book_id, device).await?;
    let applied_live: Vec<&AnnotationState> = states
        .iter()
        .filter(|s| s.ann.deleted_at.is_none() && s.last_applied_version > 0)
        .collect();
    let lost_copy = book.keys_complete
        && applied_live.len() >= 2
        && applied_live
            .iter()
            .all(|s| !present_keys.contains(s.external_key.as_str()));
    if lost_copy {
        sqlx::query(
            r#"
            UPDATE annotation_sync_states SET last_applied_version = 0
            WHERE device_id = ? AND annotation_id IN
                (SELECT id FROM book_annotations WHERE user_id = ? AND book_id = ? AND deleted_at IS NULL)
            "#,
        )
        .bind(device)
        .bind(user_id)
        .bind(book_id.to_string())
        .execute(&mut *conn)
        .await?;
    }

    let mut changed = false;
    let mut seen = HashSet::new();
    for change in book.changes.iter().rev() {
        let key = device_key(&change.datetime, &change.pos0);
        if seen.insert(key.clone()) {
            changed |=
                ingest_annotation(conn, user_id, book_id, device, &key, change, offset).await?;
        }
    }

    if book.keys_complete && !lost_copy {
        let now = now_ts();
        for s in load_annotation_states(conn, user_id, book_id, device).await? {
            let gone = s.ann.deleted_at.is_none()
                && s.last_applied_version > 0
                && !present_keys.contains(s.external_key.as_str())
                && !s
                    .external_datetime
                    .as_deref()
                    .is_some_and(|dt| present_dts.contains(dt));
            if !gone {
                continue;
            }
            sqlx::query("UPDATE book_annotations SET deleted_at = ? WHERE id = ?")
                .bind(&now)
                .bind(s.ann.id)
                .execute(&mut *conn)
                .await?;
            sqlx::query(
                "UPDATE annotation_sync_states SET delete_acked_at = ? \
                 WHERE annotation_id = ? AND device_id = ?",
            )
            .bind(&now)
            .bind(s.ann.id)
            .bind(device)
            .execute(&mut *conn)
            .await?;
            changed = true;
        }
    }
    Ok(changed)
}

async fn ingest_annotation(
    conn: &mut SqliteConnection,
    user_id: &str,
    book_id: &Uuid,
    device: &str,
    key: &str,
    change: &KoAnnotation,
    offset: Option<i64>,
) -> AppResult<bool> {
    let known: Option<AnnotationState> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        r#"
        SELECT s.external_key, s.external_datetime, s.last_applied_version, s.delete_acked_at,
               {ANNOTATION_COLS}
        FROM annotation_sync_states s JOIN book_annotations a ON a.id = s.annotation_id
        WHERE s.device_id = ? AND s.external_key = ? AND a.user_id = ? AND a.book_id = ?
        LIMIT 1
        "#
    )))
    .bind(device)
    .bind(key)
    .bind(user_id)
    .bind(book_id.to_string())
    .fetch_optional(&mut *conn)
    .await?;
    if let Some(s) = known {
        if s.ann.deleted_at.is_some() {
            return Ok(false);
        }
        return match apply_annotation_edit(conn, &s.ann, change, offset).await? {
            Some(version) => {
                if s.last_applied_version > 0 {
                    set_last_applied(conn, s.ann.id, device, version).await?;
                }
                Ok(true)
            }
            None => Ok(false),
        };
    }

    // 其它设备已同步过的同一条标注（相同身份时间 + 位置）
    let same_identity: Option<AnnotationRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        r#"
        SELECT {ANNOTATION_COLS} FROM book_annotations a
        WHERE a.user_id = ? AND a.book_id = ? AND a.device_datetime = ? AND a.pos0 = ?
        ORDER BY a.deleted_at IS NOT NULL, a.id
        LIMIT 1
        "#
    )))
    .bind(user_id)
    .bind(book_id.to_string())
    .bind(&change.datetime)
    .bind(&change.pos0)
    .fetch_optional(&mut *conn)
    .await?;
    if let Some(ann) = same_identity {
        // 已删除的不复活：状态记为已确认，随后的下发会让设备删除它
        upsert_annotation_state(conn, ann.id, device, key, &change.datetime, ann.version).await?;
        if ann.deleted_at.is_some() {
            return Ok(false);
        }
        if let Some(version) = apply_annotation_edit(conn, &ann, change, offset).await? {
            set_last_applied(conn, ann.id, device, version).await?;
            return Ok(true);
        }
        return Ok(false);
    }

    let now = now_ts();
    let (id,): (i64,) = sqlx::query_as(
        r#"
        INSERT INTO book_annotations (user_id, book_id, text, note, chapter, drawer, color,
            pos_format, pos0, pos1, pageno, device_datetime, device_updated, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(book_id.to_string())
    .bind(change.text.as_deref().unwrap_or(""))
    .bind(normalize(change.note.as_deref()))
    .bind(normalize(change.chapter.as_deref()))
    .bind(&change.drawer)
    .bind(&change.color)
    .bind(&change.pos_format)
    .bind(&change.pos0)
    .bind(&change.pos1)
    .bind(change.pageno)
    .bind(&change.datetime)
    .bind(&change.datetime_updated)
    .bind(&now)
    .bind(&now)
    .fetch_one(&mut *conn)
    .await?;
    upsert_annotation_state(conn, id, device, key, &change.datetime, 1).await?;
    Ok(true)
}

/// 设备编辑比服务端内容新时写入；内容有变化返回新版本号
async fn apply_annotation_edit(
    conn: &mut SqliteConnection,
    ann: &AnnotationRow,
    change: &KoAnnotation,
    offset: Option<i64>,
) -> AppResult<Option<i64>> {
    let edited = change
        .datetime_updated
        .as_deref()
        .unwrap_or(&change.datetime);
    let placed = placed_edit_time(edited, offset);
    let newer = match placed {
        Some(t) => t > parse_ts(&ann.updated_at),
        None => {
            edited
                > ann
                    .device_updated
                    .as_deref()
                    .unwrap_or(&ann.device_datetime)
        }
    };
    if !newer {
        return Ok(None);
    }

    let note = normalize(change.note.as_deref());
    let text = change.text.clone().unwrap_or_else(|| ann.text.clone());
    let chapter = normalize(change.chapter.as_deref()).or_else(|| ann.chapter.clone());
    let color = change.color.clone().or_else(|| ann.color.clone());
    let pos1 = change.pos1.clone().or_else(|| ann.pos1.clone());
    let pageno = change.pageno.or(ann.pageno);
    let same = note == ann.note
        && text == ann.text
        && chapter == ann.chapter
        && color == ann.color
        && change.drawer == ann.drawer
        && pos1 == ann.pos1;
    if same {
        sqlx::query("UPDATE book_annotations SET device_updated = ?, pageno = ? WHERE id = ?")
            .bind(&change.datetime_updated)
            .bind(pageno)
            .bind(ann.id)
            .execute(&mut *conn)
            .await?;
        return Ok(None);
    }

    sqlx::query(
        r#"
        UPDATE book_annotations SET text = ?, note = ?, chapter = ?, drawer = ?, color = ?,
            pos1 = ?, pageno = ?, device_updated = ?, version = version + 1, updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(&text)
    .bind(&note)
    .bind(&chapter)
    .bind(&change.drawer)
    .bind(&color)
    .bind(&pos1)
    .bind(pageno)
    .bind(&change.datetime_updated)
    .bind(format_ts(placed.unwrap_or_else(Utc::now)))
    .bind(ann.id)
    .execute(&mut *conn)
    .await?;
    Ok(Some(ann.version + 1))
}

/// 下发时的编辑时间：设备时间轴上的「现在」，且严格晚于已记录的设备编辑时间
fn mint_edit_datetime(ann: &AnnotationRow, offset: Option<i64>) -> String {
    let now = (Utc::now() + Duration::seconds(offset.unwrap_or(0))).naive_utc();
    let stored = ann
        .device_updated
        .as_deref()
        .and_then(parse_device_datetime);
    let minted = match stored {
        Some(s) if s >= now => s + Duration::seconds(1),
        _ => now,
    };
    minted.format(DEVICE_DATETIME_FORMAT).to_string()
}

async fn push_down_annotations(
    conn: &mut SqliteConnection,
    user_id: &str,
    book_id: &Uuid,
    device: &str,
    offset: Option<i64>,
) -> AppResult<(AnnotationToApply, bool, i64)> {
    let states = load_annotation_states(conn, user_id, book_id, device).await?;
    let unsent: Vec<AnnotationRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        r#"
        SELECT {ANNOTATION_COLS} FROM book_annotations a
        WHERE a.user_id = ? AND a.book_id = ? AND a.deleted_at IS NULL AND a.pos_format = 'xpointer'
          AND NOT EXISTS (SELECT 1 FROM annotation_sync_states s
                          WHERE s.annotation_id = a.id AND s.device_id = ?)
        ORDER BY a.id
        "#
    )))
    .bind(user_id)
    .bind(book_id.to_string())
    .bind(device)
    .fetch_all(&mut *conn)
    .await?;

    let mut out = AnnotationToApply::default();
    let mut budget = PUSH_DOWN_PAGE;
    let mut more = false;
    let mut take = |budget: &mut usize| {
        if *budget == 0 {
            more = true;
            false
        } else {
            *budget -= 1;
            true
        }
    };

    for s in &states {
        if s.ann.deleted_at.is_some() && s.delete_acked_at.is_none() && take(&mut budget) {
            out.delete.push(ExchangeDeleteEntry {
                server_id: s.ann.id,
                key: s.external_key.clone(),
                datetime: s.external_datetime.clone(),
            });
        }
    }
    for s in &states {
        let a = &s.ann;
        if a.deleted_at.is_none()
            && s.last_applied_version > 0
            && a.version > s.last_applied_version
            && take(&mut budget)
        {
            out.edit.push(ExchangeEditEntry {
                server_id: a.id,
                version: a.version,
                key: s.external_key.clone(),
                datetime: Some(a.device_datetime.clone()),
                datetime_updated: mint_edit_datetime(a, offset),
                drawer: a.drawer.clone(),
                color: a.color.clone().unwrap_or_else(|| "yellow".into()),
                text: a.text.clone(),
                note: a.note.clone(),
                chapter: a.chapter.clone(),
            });
        }
    }
    let resend = states
        .iter()
        .filter(|s| {
            s.ann.deleted_at.is_none()
                && s.last_applied_version == 0
                && s.ann.pos_format == "xpointer"
        })
        .map(|s| &s.ann);
    for a in resend.chain(unsent.iter()) {
        if !take(&mut budget) {
            break;
        }
        out.add.push(ExchangeAddEntry {
            server_id: a.id,
            version: a.version,
            datetime: a.device_datetime.clone(),
            datetime_updated: a.device_updated.clone(),
            drawer: a.drawer.clone(),
            color: a.color.clone().unwrap_or_else(|| "yellow".into()),
            text: a.text.clone(),
            note: a.note.clone(),
            chapter: a.chapter.clone(),
            pos_format: a.pos_format.clone(),
            pos0: a.pos0.clone(),
            pos1: a.pos1.clone(),
            pageno: a.pageno,
        });
    }

    // KOReader / Readest 只能应用 xpointer 位置，PDF 页面坐标标注不下发
    let (skipped,): (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM book_annotations a
        WHERE a.user_id = ? AND a.book_id = ? AND a.deleted_at IS NULL AND a.pos_format = 'pdf'
          AND NOT EXISTS (SELECT 1 FROM annotation_sync_states s
                          WHERE s.annotation_id = a.id AND s.device_id = ?
                            AND s.last_applied_version > 0)
        "#,
    )
    .bind(user_id)
    .bind(book_id.to_string())
    .bind(device)
    .fetch_one(&mut *conn)
    .await?;
    Ok((out, more, skipped))
}

pub async fn ack_annotations(
    db: &SqlitePool,
    user: &AuthUser,
    req: AnnotationAckRequest,
) -> AppResult<AckResponse> {
    check_device_id(&req.device_id)?;
    if req.books.len() > MAX_EXCHANGE_BOOKS {
        return Err(bad("too many books"));
    }
    for book in &req.books {
        check_hash(&book.hash)?;
        for a in &book.applied {
            check_len(a.pos0.as_deref(), 4000, "pos0")?;
            if a.datetime_updated
                .as_deref()
                .is_some_and(|d| parse_device_datetime(d).is_none())
            {
                return Err(bad("invalid datetimeUpdated"));
            }
        }
    }

    let resolved = resolve_hashes(db, user, req.books.iter().map(|b| b.hash.as_str())).await?;
    let user_id = user.id.to_string();
    let mut acked = 0;
    for book in &req.books {
        let hash = book.hash.to_lowercase();
        let Some(book_id) = resolved.get(&hash).copied().flatten() else {
            continue;
        };
        let device = sync_device_id(&req.device_id, &hash);
        let mut tx = db.begin().await?;
        for entry in &book.applied {
            let ann: Option<AnnotationRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
                "SELECT {ANNOTATION_COLS} FROM book_annotations a \
                 WHERE a.id = ? AND a.user_id = ? AND a.book_id = ?"
            )))
            .bind(entry.server_id)
            .bind(&user_id)
            .bind(book_id.to_string())
            .fetch_optional(&mut *tx)
            .await?;
            let Some(ann) = ann.filter(|a| a.version == entry.version) else {
                continue;
            };
            if entry.status == "applied" {
                let verified = entry.verified != Some(false);
                let pos0 = entry.pos0.as_deref().unwrap_or(&ann.pos0);
                let key = device_key(&ann.device_datetime, pos0);
                let version = if verified { ann.version } else { 0 };
                upsert_annotation_state(
                    &mut tx,
                    ann.id,
                    &device,
                    &key,
                    &ann.device_datetime,
                    version,
                )
                .await?;
                if let (true, Some(du)) = (verified, entry.datetime_updated.as_deref()) {
                    sqlx::query("UPDATE book_annotations SET device_updated = ? WHERE id = ?")
                        .bind(du)
                        .bind(ann.id)
                        .execute(&mut *tx)
                        .await?;
                }
            }
            acked += 1;
        }
        for entry in book.deleted.iter().filter(|d| d.status == "applied") {
            let result = sqlx::query(
                r#"
                UPDATE annotation_sync_states SET delete_acked_at = ?
                WHERE annotation_id = ? AND device_id = ? AND annotation_id IN
                    (SELECT id FROM book_annotations WHERE user_id = ? AND book_id = ?)
                "#,
            )
            .bind(now_ts())
            .bind(entry.server_id)
            .bind(&device)
            .bind(&user_id)
            .bind(book_id.to_string())
            .execute(&mut *tx)
            .await?;
            acked += result.rows_affected() as i64;
        }
        tx.commit().await?;
    }
    Ok(AckResponse { acked })
}

// ---------- 书签实现 ----------

const BOOKMARK_COLS: &str = "b.id, b.pos, b.pageno, b.chapter, b.title, b.updated_at, b.deleted_at";

#[derive(Debug, sqlx::FromRow)]
struct BookmarkRow {
    id: i64,
    pos: String,
    pageno: Option<i64>,
    chapter: Option<String>,
    title: String,
    updated_at: String,
    deleted_at: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct BookmarkLink {
    external_key: String,
    link_datetime: Option<String>,
    applied_at: String,
    #[sqlx(flatten)]
    bm: BookmarkRow,
}

/// 书签标题：备注 → 章节 → 页码
fn bookmark_title(b: &KoBookmark) -> String {
    normalize(b.note.as_deref())
        .or_else(|| normalize(b.chapter.as_deref()))
        .or_else(|| b.pageno.map(|p| format!("Page {p}")))
        .unwrap_or_else(|| "Bookmark".into())
}

fn validate_bookmark(b: &KoBookmark) -> AppResult<()> {
    if parse_device_datetime(&b.datetime).is_none() {
        return Err(bad("invalid bookmark datetime"));
    }
    if b.pos.is_empty() {
        return Err(bad("pos is required"));
    }
    check_len(Some(&b.pos), 4000, "pos")?;
    check_len(b.chapter.as_deref(), 500, "chapter")?;
    check_len(b.note.as_deref(), 500, "note")?;
    Ok(())
}

pub async fn exchange_bookmarks(
    db: &SqlitePool,
    user: &AuthUser,
    req: BookmarkExchangeRequest,
) -> AppResult<BookmarkExchangeResponse> {
    check_device_id(&req.device_id)?;
    if req.books.is_empty() || req.books.len() > MAX_EXCHANGE_BOOKS {
        return Err(bad("books must contain 1-20 items"));
    }
    if req.books.iter().map(|b| b.changes.len()).sum::<usize>() > MAX_EXCHANGE_CHANGES {
        return Err(bad("too many changes"));
    }
    for book in &req.books {
        check_hash(&book.hash)?;
        check_keys(&book.keys)?;
        book.changes.iter().try_for_each(validate_bookmark)?;
    }

    let resolved = resolve_hashes(db, user, req.books.iter().map(|b| b.hash.as_str())).await?;
    let user_id = user.id.to_string();
    let mut results = Vec::new();
    let mut unmatched = Vec::new();
    for book in &req.books {
        let hash = book.hash.to_lowercase();
        let Some(book_id) = resolved.get(&hash).copied().flatten() else {
            unmatched.push(hash);
            continue;
        };
        let device = sync_device_id(&req.device_id, &hash);
        let mut tx = db.begin().await?;
        let changed = ingest_bookmarks(&mut tx, &user_id, &book_id, &device, book).await?;
        let (to_apply, more) = push_down_bookmarks(&mut tx, &user_id, &book_id, &device).await?;
        tx.commit().await?;
        if changed {
            refresh_snapshot(db, &book_id).await?;
        }
        results.push(BookmarkExchangeResult {
            hash,
            book_id: book_id.to_string(),
            to_apply,
            more,
        });
    }
    Ok(BookmarkExchangeResponse { results, unmatched })
}

async fn load_bookmark_links(
    conn: &mut SqliteConnection,
    user_id: &str,
    book_id: &Uuid,
    device: &str,
) -> AppResult<Vec<BookmarkLink>> {
    Ok(sqlx::query_as(sqlx::AssertSqlSafe(format!(
        r#"
        SELECT l.external_key, l.device_datetime AS link_datetime, l.applied_at, {BOOKMARK_COLS}
        FROM bookmark_sync_links l JOIN book_bookmarks b ON b.id = l.bookmark_id
        WHERE l.device_id = ? AND b.user_id = ? AND b.book_id = ?
        ORDER BY b.id
        "#
    )))
    .bind(device)
    .bind(user_id)
    .bind(book_id.to_string())
    .fetch_all(&mut *conn)
    .await?)
}

async fn upsert_bookmark_link(
    conn: &mut SqliteConnection,
    bookmark_id: i64,
    device: &str,
    key: &str,
    datetime: Option<&str>,
) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO bookmark_sync_links (bookmark_id, device_id, external_key, device_datetime, applied_at)
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT(bookmark_id, device_id) DO UPDATE SET
            external_key = excluded.external_key,
            device_datetime = excluded.device_datetime,
            applied_at = excluded.applied_at
        "#,
    )
    .bind(bookmark_id)
    .bind(device)
    .bind(key)
    .bind(datetime)
    .bind(now_ts())
    .execute(&mut *conn)
    .await?;
    Ok(())
}

async fn ingest_bookmarks(
    conn: &mut SqliteConnection,
    user_id: &str,
    book_id: &Uuid,
    device: &str,
    book: &BookmarkExchangeBook,
) -> AppResult<bool> {
    let present: HashSet<&str> = book.keys.iter().map(|k| k.k.as_str()).collect();

    let links = load_bookmark_links(conn, user_id, book_id, device).await?;
    let live_links: Vec<&BookmarkLink> =
        links.iter().filter(|l| l.bm.deleted_at.is_none()).collect();
    let lost_copy = book.keys_complete
        && live_links.len() >= 2
        && live_links
            .iter()
            .all(|l| !present.contains(l.external_key.as_str()));
    if lost_copy {
        sqlx::query(
            r#"
            DELETE FROM bookmark_sync_links WHERE device_id = ? AND bookmark_id IN
                (SELECT id FROM book_bookmarks WHERE user_id = ? AND book_id = ? AND deleted_at IS NULL)
            "#,
        )
        .bind(device)
        .bind(user_id)
        .bind(book_id.to_string())
        .execute(&mut *conn)
        .await?;
    }
    let links_by_key: HashMap<&str, &BookmarkLink> = if lost_copy {
        HashMap::new()
    } else {
        links.iter().map(|l| (l.external_key.as_str(), l)).collect()
    };

    let mut changed = false;
    let mut seen = HashSet::new();
    for change in &book.changes {
        let key = device_key(&change.datetime, &change.pos);
        if !seen.insert(key.clone()) {
            continue;
        }
        let title = bookmark_title(change);
        if let Some(link) = links_by_key.get(key.as_str()) {
            // 设备端改名：仅在服务端自上次下发后未改动时采纳
            let bm = &link.bm;
            if bm.deleted_at.is_none() && bm.title != title && bm.updated_at <= link.applied_at {
                let now = now_ts();
                sqlx::query(
                    "UPDATE book_bookmarks SET title = ?, note = ?, chapter = ?, pageno = ?, \
                     updated_at = ? WHERE id = ?",
                )
                .bind(&title)
                .bind(normalize(change.note.as_deref()))
                .bind(normalize(change.chapter.as_deref()).or_else(|| bm.chapter.clone()))
                .bind(change.pageno.or(bm.pageno))
                .bind(&now)
                .bind(bm.id)
                .execute(&mut *conn)
                .await?;
                upsert_bookmark_link(conn, bm.id, device, &key, Some(&change.datetime)).await?;
                changed = true;
            }
            continue;
        }

        let existing: Option<(i64,)> = sqlx::query_as(
            "SELECT id FROM book_bookmarks WHERE user_id = ? AND book_id = ? AND pos = ? \
             AND deleted_at IS NULL ORDER BY id LIMIT 1",
        )
        .bind(user_id)
        .bind(book_id.to_string())
        .bind(&change.pos)
        .fetch_optional(&mut *conn)
        .await?;
        if let Some((id,)) = existing {
            upsert_bookmark_link(conn, id, device, &key, Some(&change.datetime)).await?;
            continue;
        }

        let now = now_ts();
        let (id,): (i64,) = sqlx::query_as(
            r#"
            INSERT INTO book_bookmarks (user_id, book_id, pos, pageno, chapter, note, title,
                device_datetime, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(user_id)
        .bind(book_id.to_string())
        .bind(&change.pos)
        .bind(change.pageno)
        .bind(normalize(change.chapter.as_deref()))
        .bind(normalize(change.note.as_deref()))
        .bind(&title)
        .bind(&change.datetime)
        .bind(&now)
        .bind(&now)
        .fetch_one(&mut *conn)
        .await?;
        upsert_bookmark_link(conn, id, device, &key, Some(&change.datetime)).await?;
        changed = true;
    }

    if book.keys_complete && !lost_copy {
        let now = now_ts();
        for l in load_bookmark_links(conn, user_id, book_id, device).await? {
            if l.bm.deleted_at.is_some() || present.contains(l.external_key.as_str()) {
                continue;
            }
            sqlx::query("UPDATE book_bookmarks SET deleted_at = ?, updated_at = ? WHERE id = ?")
                .bind(&now)
                .bind(&now)
                .bind(l.bm.id)
                .execute(&mut *conn)
                .await?;
            sqlx::query("DELETE FROM bookmark_sync_links WHERE bookmark_id = ? AND device_id = ?")
                .bind(l.bm.id)
                .bind(device)
                .execute(&mut *conn)
                .await?;
            changed = true;
        }
    }
    Ok(changed)
}

async fn push_down_bookmarks(
    conn: &mut SqliteConnection,
    user_id: &str,
    book_id: &Uuid,
    device: &str,
) -> AppResult<(BookmarkToApply, bool)> {
    let links = load_bookmark_links(conn, user_id, book_id, device).await?;
    let applied: HashMap<i64, &str> = links
        .iter()
        .map(|l| (l.bm.id, l.applied_at.as_str()))
        .collect();
    let live: Vec<BookmarkRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {BOOKMARK_COLS} FROM book_bookmarks b \
         WHERE b.user_id = ? AND b.book_id = ? AND b.deleted_at IS NULL ORDER BY b.id"
    )))
    .bind(user_id)
    .bind(book_id.to_string())
    .fetch_all(&mut *conn)
    .await?;

    let mut out = BookmarkToApply {
        delete: links
            .iter()
            .filter(|l| l.bm.deleted_at.is_some())
            .map(|l| ExchangeDeleteEntry {
                server_id: l.bm.id,
                key: l.external_key.clone(),
                datetime: l.link_datetime.clone(),
            })
            .collect(),
        add: Vec::new(),
    };
    let pending: Vec<&BookmarkRow> = live
        .iter()
        .filter(|b| {
            applied
                .get(&b.id)
                .is_none_or(|at| b.updated_at.as_str() > *at)
        })
        .collect();
    let more = pending.len() > PUSH_DOWN_PAGE;
    out.add = pending
        .into_iter()
        .take(PUSH_DOWN_PAGE)
        .map(|b| BookmarkAddEntry {
            server_id: b.id,
            pos: b.pos.clone(),
            pageno: b.pageno,
            title: b.title.clone(),
        })
        .collect();
    Ok((out, more))
}

pub async fn ack_bookmarks(
    db: &SqlitePool,
    user: &AuthUser,
    req: BookmarkAckRequest,
) -> AppResult<AckResponse> {
    check_device_id(&req.device_id)?;
    if req.books.len() > MAX_EXCHANGE_BOOKS {
        return Err(bad("too many books"));
    }
    for book in &req.books {
        check_hash(&book.hash)?;
        for a in &book.applied {
            if a.key.as_deref().is_some_and(|k| !is_md5_hex(k))
                || a.datetime
                    .as_deref()
                    .is_some_and(|d| parse_device_datetime(d).is_none())
            {
                return Err(bad("invalid bookmark ack"));
            }
        }
    }

    let resolved = resolve_hashes(db, user, req.books.iter().map(|b| b.hash.as_str())).await?;
    let user_id = user.id.to_string();
    let mut acked = 0;
    for book in &req.books {
        let hash = book.hash.to_lowercase();
        let Some(book_id) = resolved.get(&hash).copied().flatten() else {
            continue;
        };
        let device = sync_device_id(&req.device_id, &hash);
        let mut tx = db.begin().await?;
        for entry in book.applied.iter().filter(|a| a.status == "applied") {
            let bm: Option<(String,)> = sqlx::query_as(
                "SELECT pos FROM book_bookmarks WHERE id = ? AND user_id = ? AND book_id = ? \
                 AND deleted_at IS NULL",
            )
            .bind(entry.server_id)
            .bind(&user_id)
            .bind(book_id.to_string())
            .fetch_optional(&mut *tx)
            .await?;
            let Some((pos,)) = bm else {
                continue;
            };
            let key = match (&entry.key, &entry.datetime) {
                (Some(k), _) => k.to_lowercase(),
                (None, Some(dt)) => device_key(dt, entry.pos.as_deref().unwrap_or(&pos)),
                (None, None) => continue,
            };
            upsert_bookmark_link(
                &mut tx,
                entry.server_id,
                &device,
                &key,
                entry.datetime.as_deref(),
            )
            .await?;
            acked += 1;
        }
        for entry in book.deleted.iter().filter(|d| d.status == "applied") {
            let result = sqlx::query(
                r#"
                DELETE FROM bookmark_sync_links WHERE bookmark_id = ? AND device_id = ? AND bookmark_id IN
                    (SELECT id FROM book_bookmarks WHERE user_id = ? AND book_id = ?)
                "#,
            )
            .bind(entry.server_id)
            .bind(&device)
            .bind(&user_id)
            .bind(book_id.to_string())
            .execute(&mut *tx)
            .await?;
            acked += result.rows_affected() as i64;
        }
        tx.commit().await?;
    }
    Ok(AckResponse { acked })
}

// ---------- 统计 / 状态实现 ----------

pub async fn upload_page_stats(
    db: &SqlitePool,
    user: &AuthUser,
    req: PageStatsRequest,
) -> AppResult<PageStatsResponse> {
    check_device_id(&req.device_id)?;
    if req.books.is_empty() || req.books.len() > MAX_PAGE_STATS_BOOKS {
        return Err(bad("books must contain 1-50 items"));
    }
    if req.books.iter().map(|b| b.events.len()).sum::<usize>() > MAX_PAGE_STATS_EVENTS {
        return Err(bad("too many events"));
    }
    for book in &req.books {
        check_hash(&book.hash)?;
        if book
            .events
            .iter()
            .any(|e| e.page < 0 || e.start_time < 1 || e.duration_seconds < 0 || e.total_pages < 1)
        {
            return Err(bad("invalid page stats event"));
        }
    }

    let resolved = resolve_hashes(db, user, req.books.iter().map(|b| b.hash.as_str())).await?;
    let user_id = user.id.to_string();
    let mut results = Vec::new();
    let mut unmatched = Vec::new();
    for book in &req.books {
        let hash = book.hash.to_lowercase();
        let Some(book_id) = resolved.get(&hash).copied().flatten() else {
            unmatched.push(hash);
            continue;
        };
        let mut tx = db.begin().await?;
        let mut accepted = 0;
        for e in &book.events {
            let result = sqlx::query(
                r#"
                INSERT OR IGNORE INTO reading_page_stats
                    (user_id, book_id, device_id, page, start_time, duration_seconds, total_pages)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&user_id)
            .bind(book_id.to_string())
            .bind(&req.device_id)
            .bind(e.page)
            .bind(e.start_time)
            .bind(e.duration_seconds)
            .bind(e.total_pages)
            .execute(&mut *tx)
            .await?;
            accepted += result.rows_affected() as i64;
        }
        tx.commit().await?;
        results.push(PageStatsResult {
            hash,
            accepted,
            duplicates: book.events.len() as i64 - accepted,
            watermark: book.events.iter().map(|e| e.start_time).max().unwrap_or(0),
        });
    }
    Ok(PageStatsResponse { results, unmatched })
}

pub async fn upload_book_states(
    db: &SqlitePool,
    user: &AuthUser,
    req: BookStatesRequest,
) -> AppResult<BookStatesResponse> {
    if req.books.is_empty() || req.books.len() > MAX_BOOK_STATES {
        return Err(bad("books must contain 1-200 items"));
    }
    for book in &req.books {
        check_hash(&book.hash)?;
        if book
            .status
            .as_deref()
            .is_some_and(|s| !["reading", "complete", "abandoned"].contains(&s))
        {
            return Err(bad("invalid status"));
        }
        if book
            .status_modified
            .as_deref()
            .is_some_and(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").is_err())
        {
            return Err(bad("invalid statusModified"));
        }
        if book.rating.is_some_and(|r| !(1..=5).contains(&r)) {
            return Err(bad("rating must be 1-5"));
        }
    }

    let resolved = resolve_hashes(db, user, req.books.iter().map(|b| b.hash.as_str())).await?;
    let user_id = user.id.to_string();
    let mut results = Vec::new();
    let mut unmatched = Vec::new();
    for book in &req.books {
        let hash = book.hash.to_lowercase();
        let Some(book_id) = resolved.get(&hash).copied().flatten() else {
            unmatched.push(hash);
            continue;
        };
        let current: Option<(String, String)> = sqlx::query_as(
            "SELECT status, status_modified FROM user_book_status WHERE user_id = ? AND book_id = ?",
        )
        .bind(&user_id)
        .bind(book_id.to_string())
        .fetch_optional(db)
        .await?;

        let mut status_applied = false;
        let mut stored = current.clone();
        if let Some(status) = book.status.as_deref() {
            let accept = match (&current, book.status_modified.as_deref()) {
                (Some((s, _)), _) if s == status => {
                    status_applied = true;
                    false
                }
                (None, _) => true,
                // 同日或更旧的设备修改不覆盖服务端
                (Some((_, modified)), Some(device)) => device > modified.as_str(),
                (Some(_), None) => false,
            };
            if accept {
                let modified = book
                    .status_modified
                    .clone()
                    .unwrap_or_else(|| Utc::now().format("%Y-%m-%d").to_string());
                sqlx::query(
                    r#"
                    INSERT INTO user_book_status (user_id, book_id, status, status_modified, updated_at)
                    VALUES (?, ?, ?, ?, ?)
                    ON CONFLICT(user_id, book_id) DO UPDATE SET
                        status = excluded.status,
                        status_modified = excluded.status_modified,
                        updated_at = excluded.updated_at
                    "#,
                )
                .bind(&user_id)
                .bind(book_id.to_string())
                .bind(status)
                .bind(&modified)
                .bind(now_ts())
                .execute(db)
                .await?;
                stored = Some((status.to_string(), modified));
                status_applied = true;
            }
        }
        let (rating_applied, rating) = apply_rating(db, user, &book_id, book).await?;
        let (status, status_modified) = stored.unzip();
        results.push(BookStateResult {
            hash,
            status_applied,
            status,
            status_modified,
            rating_applied,
            rating_set: rating.is_some(),
            rating,
        });
    }
    Ok(BookStatesResponse { results, unmatched })
}

/// 设备评分写入图书（共享）元数据的 `rating`，需图书馆编辑权限。
/// 图书已有不同评分时，仅当设备修改日期晚于图书最近更新日期才覆盖。返回（是否采纳, 当前评分）
async fn apply_rating(
    db: &SqlitePool,
    user: &AuthUser,
    book_id: &Uuid,
    entry: &BookStateEntry,
) -> AppResult<(bool, Option<i64>)> {
    let row = book::fetch_row_internal(db, book_id).await?;
    let mut meta = BookMetadata::from_json(&row.metadata)?;
    let current = meta.rating.map(i64::from);
    let target = match (entry.rating, entry.rating_cleared) {
        (Some(r), _) => Some(r),
        (None, Some(true)) => None,
        _ => return Ok((false, current)),
    };
    if target == current {
        return Ok((true, current));
    }
    let library_id =
        Uuid::parse_str(&row.library_id).map_err(|e| AppError::Internal(e.to_string()))?;
    let perm = library::resolve_permission(db, user, &library_id).await?;
    if !perm.can_edit {
        return Ok((false, current));
    }
    if current.is_some() {
        let book_date = row.updated_at.get(..10).unwrap_or("");
        if entry
            .status_modified
            .as_deref()
            .is_none_or(|device| device <= book_date)
        {
            return Ok((false, current));
        }
    }

    meta.rating = target.map(|r| r as u8);
    persist_metadata(db, book_id, &row.metadata_file_path, &meta).await?;
    sqlx::query("UPDATE books SET updated_at = ? WHERE id = ?")
        .bind(Utc::now().to_rfc3339())
        .bind(book_id.to_string())
        .execute(db)
        .await?;
    Ok((true, target))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_datetime_requires_koreader_format() {
        assert!(parse_device_datetime("2024-01-02 03:04:05").is_some());
        assert!(parse_device_datetime("2024-01-02T03:04:05").is_none());
    }

    #[test]
    fn bookmark_title_prefers_note_then_chapter() {
        let mut b = KoBookmark {
            datetime: "2024-01-02 03:04:05".into(),
            datetime_updated: None,
            pos: "/body/p[1]".into(),
            pageno: Some(7),
            chapter: Some("Ch 1".into()),
            note: Some("  ".into()),
        };
        assert_eq!(bookmark_title(&b), "Ch 1");
        b.chapter = None;
        assert_eq!(bookmark_title(&b), "Page 7");
        b.note = Some("mine".into());
        assert_eq!(bookmark_title(&b), "mine");
    }
}

//! 图书馆目录与数据库的对比扫描及后续修复动作（作为系统维护工具在后台任务中执行）。

use crate::domain::book::upsert_from_fs;
use crate::domain::library;
use crate::domain::tasks::TaskContext;
use crate::error::AppResult;
use crate::infra::fs::{self, ScannedBookFile};
use crate::infra::BookMetadata;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use utoipa::ToSchema;
use uuid::Uuid;

/// 数据库有记录、磁盘上图书文件已不存在
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MissingFileBook {
    pub book_id: String,
    pub title: String,
    pub author: String,
    pub category: String,
    pub book_file_path: String,
}

/// 磁盘上有图书文件、数据库中没有记录
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UntrackedBookFile {
    pub path: String,
    pub category: String,
    /// 同名 metadata 中的书名（若有）
    pub title: Option<String>,
    pub size: u64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LibraryScanReport {
    pub library_id: String,
    pub scanned_files: usize,
    pub db_books: usize,
    /// 文件与记录一一对应的图书数
    pub in_sync: usize,
    pub missing_files: Vec<MissingFileBook>,
    pub untracked_files: Vec<UntrackedBookFile>,
    /// 无对应图书文件的 metadata / 封面 / 缩略图
    pub dangling_files: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct MaintenanceActionReport {
    /// 成功处理的项（图书 ID 或文件路径）
    pub processed: Vec<String>,
    /// 指定了但当前已不符合条件、被跳过的项
    pub skipped: Vec<String>,
    pub errors: Vec<String>,
}

struct DbBookRow {
    id: String,
    category: String,
    book_file_path: String,
    metadata_file_path: String,
    metadata: String,
}

struct Snapshot {
    lib_root: PathBuf,
    scanned: Vec<ScannedBookFile>,
    db_rows: Vec<DbBookRow>,
}

impl Snapshot {
    async fn load(db: &SqlitePool, library_id: &Uuid) -> AppResult<Self> {
        let lib_root = library::get_library_root(db, library_id).await?;
        let scanned = fs::scan_library_books(&lib_root).await?;
        let rows: Vec<(String, String, String, String, String)> = sqlx::query_as(
            "SELECT id, category, book_file_path, metadata_file_path, metadata FROM books WHERE library_id = ?",
        )
        .bind(library_id.to_string())
        .fetch_all(db)
        .await?;
        let db_rows = rows
            .into_iter()
            .map(
                |(id, category, book_file_path, metadata_file_path, metadata)| DbBookRow {
                    id,
                    category,
                    book_file_path,
                    metadata_file_path,
                    metadata,
                },
            )
            .collect();
        Ok(Self {
            lib_root,
            scanned,
            db_rows,
        })
    }

    fn untracked(&self) -> Vec<&ScannedBookFile> {
        let db_paths: HashSet<&str> = self
            .db_rows
            .iter()
            .map(|r| r.book_file_path.as_str())
            .collect();
        self.scanned
            .iter()
            .filter(|s| !db_paths.contains(s.book_file.to_string_lossy().as_ref()))
            .collect()
    }

    fn missing(&self) -> Vec<&DbBookRow> {
        let fs_paths: HashSet<String> = self
            .scanned
            .iter()
            .map(|s| s.book_file.to_string_lossy().into_owned())
            .collect();
        self.db_rows
            .iter()
            .filter(|r| {
                !fs_paths.contains(&r.book_file_path) && !Path::new(&r.book_file_path).is_file()
            })
            .collect()
    }

    /// 文件与记录对应的图书：(记录, 扫描到的文件)
    fn tracked(&self) -> Vec<(&DbBookRow, &ScannedBookFile)> {
        let by_path: HashMap<String, &ScannedBookFile> = self
            .scanned
            .iter()
            .map(|s| (s.book_file.to_string_lossy().into_owned(), s))
            .collect();
        self.db_rows
            .iter()
            .filter_map(|r| by_path.get(&r.book_file_path).map(|s| (r, *s)))
            .collect()
    }
}

/// `items` 为空时处理全部候选；否则只处理同时在候选中的项，其余记为 skipped
fn select<T>(
    candidates: Vec<T>,
    key: impl Fn(&T) -> String,
    items: Option<&[String]>,
    report: &mut MaintenanceActionReport,
) -> Vec<T> {
    let Some(items) = items else {
        return candidates;
    };
    let wanted: HashSet<&str> = items.iter().map(String::as_str).collect();
    let available: HashSet<String> = candidates.iter().map(&key).collect();
    report.skipped.extend(
        items
            .iter()
            .filter(|i| !available.contains(i.as_str()))
            .cloned(),
    );
    candidates
        .into_iter()
        .filter(|c| wanted.contains(key(c).as_str()))
        .collect()
}

fn path_key(s: &&ScannedBookFile) -> String {
    s.book_file.to_string_lossy().into_owned()
}

pub async fn scan_library(
    db: &SqlitePool,
    library_id: &Uuid,
    ctx: &TaskContext,
) -> AppResult<LibraryScanReport> {
    ctx.set_message("扫描磁盘与数据库");
    let snap = Snapshot::load(db, library_id).await?;
    let untracked = snap.untracked();
    let missing = snap.missing();
    ctx.set_total((untracked.len() + missing.len()) as u64);

    let mut report = LibraryScanReport {
        library_id: library_id.to_string(),
        scanned_files: snap.scanned.len(),
        db_books: snap.db_rows.len(),
        in_sync: snap.tracked().len(),
        missing_files: vec![],
        untracked_files: vec![],
        dangling_files: vec![],
        errors: vec![],
    };

    let mut done = 0u64;
    for entry in untracked {
        if ctx.is_cancelled() {
            return Ok(report);
        }
        let path = entry.book_file.to_string_lossy().into_owned();
        done += 1;
        ctx.set_progress(done, path.clone());
        let title = match entry.book_file.parent() {
            Some(dir) => {
                match fs::find_metadata_in_dir(dir, fs::file_stem(&entry.book_file).as_deref()) {
                    Some(meta) => fs::read_metadata_file(&meta).await.ok().map(|m| m.title),
                    None => None,
                }
            }
            None => None,
        };
        let size = tokio::fs::metadata(&entry.book_file)
            .await
            .map(|m| m.len())
            .unwrap_or(0);
        report.untracked_files.push(UntrackedBookFile {
            path,
            category: entry.category.clone(),
            title,
            size,
        });
    }

    for row in missing {
        done += 1;
        ctx.set_progress(done, row.book_file_path.clone());
        let meta: BookMetadata = serde_json::from_str(&row.metadata).unwrap_or_default();
        report.missing_files.push(MissingFileBook {
            book_id: row.id.clone(),
            title: meta.title,
            author: meta.author,
            category: row.category.clone(),
            book_file_path: row.book_file_path.clone(),
        });
    }

    ctx.set_message("查找多余附属文件");
    match fs::find_dangling_files(&snap.lib_root, &snap.scanned).await {
        Ok(paths) => {
            report.dangling_files = paths
                .into_iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect();
        }
        Err(e) => report.errors.push(format!("dangling: {e}")),
    }
    ctx.set_message("完成");
    Ok(report)
}

/// 删除数据库中图书文件已缺失的记录
pub async fn purge_missing(
    db: &SqlitePool,
    library_id: &Uuid,
    items: Option<&[String]>,
    ctx: &TaskContext,
) -> AppResult<MaintenanceActionReport> {
    let snap = Snapshot::load(db, library_id).await?;
    let mut report = MaintenanceActionReport::default();
    let targets = select(snap.missing(), |r| r.id.clone(), items, &mut report);
    ctx.set_total(targets.len() as u64);
    for (i, row) in targets.into_iter().enumerate() {
        if ctx.is_cancelled() {
            break;
        }
        ctx.set_progress(i as u64 + 1, row.book_file_path.clone());
        match sqlx::query("DELETE FROM books WHERE id = ? AND library_id = ?")
            .bind(&row.id)
            .bind(library_id.to_string())
            .execute(db)
            .await
        {
            Ok(_) => report.processed.push(row.id.clone()),
            Err(e) => report.errors.push(format!("{}: {e}", row.id)),
        }
    }
    Ok(report)
}

/// 把磁盘上未入库的图书导入数据库（读取同名 metadata，无则按文件名生成）
pub async fn import_untracked(
    db: &SqlitePool,
    library_id: &Uuid,
    items: Option<&[String]>,
    ctx: &TaskContext,
) -> AppResult<MaintenanceActionReport> {
    let snap = Snapshot::load(db, library_id).await?;
    let mut report = MaintenanceActionReport::default();
    let targets = select(snap.untracked(), path_key, items, &mut report);
    ctx.set_total(targets.len() as u64);
    for (i, entry) in targets.into_iter().enumerate() {
        if ctx.is_cancelled() {
            break;
        }
        let path = entry.book_file.to_string_lossy().into_owned();
        ctx.set_progress(i as u64 + 1, path.clone());
        let book_id = Uuid::new_v4();
        match upsert_from_fs(db, library_id, &entry.category, &book_id, &entry.book_file).await {
            Ok(()) => report.processed.push(path),
            Err(e) => report.errors.push(format!("{path}: {e}")),
        }
    }
    Ok(report)
}

/// 从磁盘删除未入库的图书文件及其同名附属文件
pub async fn delete_untracked(
    db: &SqlitePool,
    library_id: &Uuid,
    items: Option<&[String]>,
    ctx: &TaskContext,
) -> AppResult<MaintenanceActionReport> {
    let snap = Snapshot::load(db, library_id).await?;
    let mut report = MaintenanceActionReport::default();
    let targets = select(snap.untracked(), path_key, items, &mut report);
    ctx.set_total(targets.len() as u64);
    for (i, entry) in targets.into_iter().enumerate() {
        if ctx.is_cancelled() {
            break;
        }
        let path = entry.book_file.to_string_lossy().into_owned();
        ctx.set_progress(i as u64 + 1, path.clone());
        match fs::remove_untracked_book(&entry.book_file).await {
            Ok(removed) => report.processed.extend(
                removed
                    .into_iter()
                    .map(|p| p.to_string_lossy().into_owned()),
            ),
            Err(e) => report.errors.push(format!("{path}: {e}")),
        }
    }
    Ok(report)
}

/// 删除无对应图书文件的 metadata / 封面 / 缩略图
pub async fn clean_dangling(
    db: &SqlitePool,
    library_id: &Uuid,
    items: Option<&[String]>,
    ctx: &TaskContext,
) -> AppResult<MaintenanceActionReport> {
    let snap = Snapshot::load(db, library_id).await?;
    let dangling = fs::find_dangling_files(&snap.lib_root, &snap.scanned).await?;
    let mut report = MaintenanceActionReport::default();
    let targets = select(
        dangling,
        |p| p.to_string_lossy().into_owned(),
        items,
        &mut report,
    );
    ctx.set_total(targets.len() as u64);
    for (i, path) in targets.into_iter().enumerate() {
        if ctx.is_cancelled() {
            break;
        }
        let path_str = path.to_string_lossy().into_owned();
        ctx.set_progress(i as u64 + 1, path_str.clone());
        match fs::remove_dangling_file(&path).await {
            Ok(()) => report.processed.push(path_str),
            Err(e) => report.errors.push(format!("{path_str}: {e}")),
        }
    }
    Ok(report)
}

/// 以磁盘 metadata.json 覆盖数据库中对应图书的元数据与分类
pub async fn resync_metadata(
    db: &SqlitePool,
    library_id: &Uuid,
    items: Option<&[String]>,
    ctx: &TaskContext,
) -> AppResult<MaintenanceActionReport> {
    let snap = Snapshot::load(db, library_id).await?;
    let mut report = MaintenanceActionReport::default();
    let targets = select(snap.tracked(), |(r, _)| r.id.clone(), items, &mut report);
    ctx.set_total(targets.len() as u64);
    for (i, (row, entry)) in targets.into_iter().enumerate() {
        if ctx.is_cancelled() {
            break;
        }
        ctx.set_progress(i as u64 + 1, row.book_file_path.clone());
        let Ok(book_id) = Uuid::parse_str(&row.id) else {
            report.errors.push(format!("{}: invalid id", row.id));
            continue;
        };
        match upsert_from_fs(db, library_id, &entry.category, &book_id, &entry.book_file).await {
            Ok(()) => report.processed.push(row.id.clone()),
            Err(e) => report.errors.push(format!("{}: {e}", row.id)),
        }
    }
    Ok(report)
}

fn lacks_hashes(meta: &BookMetadata) -> bool {
    meta.file_md5.is_none() || meta.partial_md5.is_none() || meta.file_sha256.is_none()
}

/// 重新计算图书文件的 MD5 / partial MD5 / SHA-256，写回数据库与 metadata.json。
/// `only_missing` 时只处理缺少任一哈希的图书；文件缺失的图书不在候选内。
pub async fn refresh_hashes(
    db: &SqlitePool,
    library_id: &Uuid,
    items: Option<&[String]>,
    only_missing: bool,
    ctx: &TaskContext,
) -> AppResult<MaintenanceActionReport> {
    let snap = Snapshot::load(db, library_id).await?;
    let mut report = MaintenanceActionReport::default();
    let candidates: Vec<(&DbBookRow, BookMetadata)> = snap
        .db_rows
        .iter()
        .filter(|r| Path::new(&r.book_file_path).is_file())
        .map(|r| (r, serde_json::from_str(&r.metadata).unwrap_or_default()))
        .filter(|(_, meta)| !only_missing || lacks_hashes(meta))
        .collect();
    let targets = select(candidates, |(r, _)| r.id.clone(), items, &mut report);
    ctx.set_total(targets.len() as u64);
    for (i, (row, mut meta)) in targets.into_iter().enumerate() {
        if ctx.is_cancelled() {
            break;
        }
        ctx.set_progress(i as u64 + 1, row.book_file_path.clone());
        match rehash_book(db, row, &mut meta).await {
            Ok(()) => report.processed.push(row.id.clone()),
            Err(e) => report.errors.push(format!("{}: {e}", row.id)),
        }
    }
    Ok(report)
}

async fn rehash_book(db: &SqlitePool, row: &DbBookRow, meta: &mut BookMetadata) -> AppResult<()> {
    meta.refresh_book_file_hashes(Path::new(&row.book_file_path))
        .await?;
    sqlx::query("UPDATE books SET metadata = ?, updated_at = ? WHERE id = ?")
        .bind(meta.to_json()?)
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(&row.id)
        .execute(db)
        .await?;
    let meta_path = Path::new(&row.metadata_file_path);
    if let (Some(dir), Some(base)) = (meta_path.parent(), fs::file_stem(meta_path)) {
        fs::write_metadata_file(dir, &base, meta).await?;
    }
    Ok(())
}

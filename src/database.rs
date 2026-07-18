use anyhow::{anyhow, Result};
use rusqlite::{params, Connection};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::models::{File, Folder, PendingContribution};

/// نوع الاتصال بقاعدة البيانات (آمن للاستخدام بين الـ threads)
pub type DbPool = Arc<Mutex<Connection>>;

// ============================================================
//  تهيئة قاعدة البيانات
// ============================================================

pub fn init_db(database_path: &str) -> Result<DbPool> {
    // تنظيف البادئة لـ rusqlite التقليدية
    let path = database_path
        .strip_prefix("sqlite://")
        .unwrap_or(database_path);

    let conn = Connection::open(path)?;

    conn.execute_batch(
        "PRAGMA journal_mode=WAL;

        CREATE TABLE IF NOT EXISTS folders (
            id        TEXT PRIMARY KEY,
            name      TEXT NOT NULL,
            parent_id TEXT
        );

        CREATE TABLE IF NOT EXISTS files (
            id        TEXT PRIMARY KEY,
            name      TEXT NOT NULL,
            folder_id TEXT NOT NULL,
            file_id   TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS pending_contributions (
            id          TEXT PRIMARY KEY,
            user_id     INTEGER NOT NULL,
            username    TEXT,
            description TEXT NOT NULL,
            file_id     TEXT NOT NULL,
            file_name   TEXT NOT NULL
        );",
    )?;

    Ok(Arc::new(Mutex::new(conn)))
}

// ============================================================
//  دوال المجلدات
// ============================================================

pub async fn get_subfolders(pool: &DbPool, folder_id: Option<&str>) -> Result<Vec<Folder>> {
    let conn = pool.lock().await;
    
    // إصلاح مشكلة انتقال الملكية عبر فصل الحالات بوضوح
    let mut stmt = match folder_id {
        Some(_) => conn.prepare("SELECT id, name, parent_id FROM folders WHERE parent_id = ? ORDER BY name")?,
        None => conn.prepare("SELECT id, name, parent_id FROM folders WHERE parent_id IS NULL ORDER BY name")?,
    };

    let rows = match folder_id {
        Some(id) => stmt.query_map(params![id], map_folder)?,
        None => stmt.query_map([], map_folder)?,
    };

    let mut results = Vec::new();
    for row in rows {
        results.push(row.map_err(|e| anyhow!(e))?);
    }
    Ok(results)
}

pub async fn get_folder(pool: &DbPool, folder_id: &str) -> Result<Option<Folder>> {
    let conn = pool.lock().await;
    let mut stmt = conn.prepare("SELECT id, name, parent_id FROM folders WHERE id = ?")?;

    let mut rows = stmt.query_map(params![folder_id], map_folder)?;
    if let Some(row) = rows.next() {
        Ok(Some(row.map_err(|e| anyhow!(e))?))
    } else {
        Ok(None)
    }
}

pub async fn insert_folder(
    pool: &DbPool,
    id: &str,
    name: &str,
    parent_id: Option<&str>,
) -> Result<()> {
    let conn = pool.lock().await;
    conn.execute(
        "INSERT INTO folders (id, name, parent_id) VALUES (?1, ?2, ?3)",
        params![id, name, parent_id],
    )?;
    Ok(())
}

// ============================================================
//  دوال الملفات
// ============================================================

pub async fn get_files(pool: &DbPool, folder_id: &str) -> Result<Vec<File>> {
    let conn = pool.lock().await;
    let mut stmt = conn.prepare(
        "SELECT id, name, folder_id, file_id FROM files WHERE folder_id = ? ORDER BY name",
    )?;
    let rows = stmt.query_map(params![folder_id], map_file)?;
    
    let mut results = Vec::new();
    for row in rows {
        results.push(row.map_err(|e| anyhow!(e))?);
    }
    Ok(results)
}

pub async fn get_file(pool: &DbPool, file_pk: &str) -> Result<Option<File>> {
    let conn = pool.lock().await;
    let mut stmt = conn.prepare("SELECT id, name, folder_id, file_id FROM files WHERE id = ?")?;
    let mut rows = stmt.query_map(params![file_pk], map_file)?;
    
    if let Some(row) = rows.next() {
        Ok(Some(row.map_err(|e| anyhow!(e))?))
    } else {
        Ok(None)
    }
}

pub async fn insert_file(
    pool: &DbPool,
    id: &str,
    name: &str,
    folder_id: &str,
    file_id: &str,
) -> Result<()> {
    let conn = pool.lock().await;
    conn.execute(
        "INSERT INTO files (id, name, folder_id, file_id) VALUES (?1, ?2, ?3, ?4)",
        params![id, name, folder_id, file_id],
    )?;
    Ok(())
}

// ============================================================
//  البحث
// ============================================================

pub async fn search_files(pool: &DbPool, keyword: &str) -> Result<Vec<File>> {
    let pattern = format!("%{}%", keyword);
    let conn = pool.lock().await;
    let mut stmt = conn.prepare(
        "SELECT id, name, folder_id, file_id FROM files WHERE name LIKE ?1 ORDER BY name LIMIT 500",
    )?;
    let rows = stmt.query_map(params![pattern], map_file)?;
    
    let mut results = Vec::new();
    for row in rows {
        results.push(row.map_err(|e| anyhow!(e))?);
    }
    Ok(results)
}

// ============================================================
//  دوال المساهمات
// ============================================================

pub async fn insert_pending_contribution(pool: &DbPool, c: &PendingContribution) -> Result<()> {
    let conn = pool.lock().await;
    conn.execute(
        "INSERT INTO pending_contributions (id, user_id, username, description, file_id, file_name)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![c.id, c.user_id, c.username, c.description, c.file_id, c.file_name],
    )?;
    Ok(())
}

pub async fn delete_pending_contribution(pool: &DbPool, id: &str) -> Result<()> {
    let conn = pool.lock().await;
    conn.execute("DELETE FROM pending_contributions WHERE id = ?", params![id])?;
    Ok(())
}

pub async fn get_all_pending_contributions(pool: &DbPool) -> Result<Vec<PendingContribution>> {
    let conn = pool.lock().await;
    let mut stmt = conn.prepare(
        "SELECT id, user_id, username, description, file_id, file_name
         FROM pending_contributions ORDER BY rowid",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(PendingContribution {
            id: r.get(0)?,
            user_id: r.get(1)?,
            username: r.get(2)?,
            description: r.get(3)?,
            file_id: r.get(4)?,
            file_name: r.get(5)?,
        })
    })?;
    
    let mut results = Vec::new();
    for row in rows {
        results.push(row.map_err(|e| anyhow!(e))?);
    }
    Ok(results)
}

// ============================================================
//  مساعدات تحويل الصفوف
// ============================================================

fn map_folder(r: &rusqlite::Row<'_>) -> rusqlite::Result<Folder> {
    Ok(Folder {
        id: r.get(0)?,
        name: r.get(1)?,
        parent_id: r.get(2)?,
    })
}

fn map_file(r: &rusqlite::Row<'_>) -> rusqlite::Result<File> {
    Ok(File {
        id: r.get(0)?,
        name: r.get(1)?,
        folder_id: r.get(2)?,
        file_id: r.get(3)?,
    })
}


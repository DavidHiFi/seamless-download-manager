use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use crate::dpapi;
use crate::model::{Category, DownloadItem, PowerAction, Queue, ResumeState, Schedule, SourceKind, Status};

const SCHEMA_VERSION: i32 = 2;

pub struct Store {
    conn: Mutex<Connection>,
}

#[derive(Serialize, Deserialize, Default)]
struct Secrets {
    headers: Vec<(String, String)>,
    basic_auth: Option<(String, String)>,
}

const DOWNLOAD_COLUMNS: &str = "id, url, kind, referrer, user_agent, secrets, folder, name, name_locked, size, downloaded, status, error, \
     date_added, date_started, date_finished, queue_id, category_id, threads, speed_limit, checksum, resume";

fn seal(item: &DownloadItem) -> Result<Option<Vec<u8>>> {
    if item.headers.is_empty() && item.basic_auth.is_none() {
        return Ok(None);
    }
    let json = serde_json::to_vec(&Secrets { headers: item.headers.clone(), basic_auth: item.basic_auth.clone() })?;
    Ok(Some(dpapi::protect(&json)?))
}

fn unseal(blob: Option<Vec<u8>>) -> Secrets {
    let Some(blob) = blob else { return Secrets::default() };
    match dpapi::unprotect(&blob).ok().and_then(|plain| serde_json::from_slice(&plain).ok()) {
        Some(secrets) => secrets,
        None => {
            tracing::warn!("could not decrypt stored headers; they are dropped");
            Secrets::default()
        }
    }
}

fn kind_str(kind: SourceKind) -> &'static str {
    match kind {
        SourceKind::Http => "http",
        SourceKind::Hls => "hls",
    }
}

fn row_to_item(row: &Row<'_>) -> rusqlite::Result<DownloadItem> {
    let secrets = unseal(row.get(5)?);
    let kind: String = row.get(2)?;
    let status: String = row.get(11)?;
    let resume: Option<String> = row.get(21)?;
    Ok(DownloadItem {
        id: row.get(0)?,
        url: row.get(1)?,
        kind: if kind == "hls" { SourceKind::Hls } else { SourceKind::Http },
        referrer: row.get(3)?,
        user_agent: row.get(4)?,
        headers: secrets.headers,
        basic_auth: secrets.basic_auth,
        folder: row.get(6)?,
        name: row.get(7)?,
        name_locked: row.get::<_, i64>(8)? != 0,
        size: row.get::<_, Option<i64>>(9)?.map(|v| v as u64),
        downloaded: row.get::<_, i64>(10)? as u64,
        status: Status::parse(&status).unwrap_or(Status::Paused),
        error: row.get(12)?,
        date_added: row.get(13)?,
        date_started: row.get(14)?,
        date_finished: row.get(15)?,
        queue_id: row.get(16)?,
        category_id: row.get(17)?,
        threads: row.get::<_, Option<i64>>(18)?.map(|v| v as u32),
        speed_limit: row.get::<_, i64>(19)? as u64,
        checksum: row.get(20)?,
        resume: resume.and_then(|text| serde_json::from_str::<ResumeState>(&text).ok()),
    })
}

impl Store {
    pub fn open(path: &Path) -> Result<Store> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let conn = Connection::open(path).with_context(|| "could not open sdm.db")?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        Self::init(conn)
    }

    pub fn open_memory() -> Result<Store> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Store> {
        let version: i32 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version < 1 {
            conn.execute_batch(
                "CREATE TABLE downloads (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    url TEXT NOT NULL,
                    kind TEXT NOT NULL,
                    referrer TEXT,
                    user_agent TEXT,
                    secrets BLOB,
                    folder TEXT NOT NULL,
                    name TEXT NOT NULL,
                    name_locked INTEGER NOT NULL DEFAULT 0,
                    size INTEGER,
                    downloaded INTEGER NOT NULL DEFAULT 0,
                    status TEXT NOT NULL,
                    error TEXT,
                    date_added INTEGER NOT NULL,
                    date_started INTEGER,
                    date_finished INTEGER,
                    queue_id INTEGER,
                    category_id INTEGER,
                    threads INTEGER,
                    speed_limit INTEGER NOT NULL DEFAULT 0,
                    checksum TEXT,
                    resume TEXT
                );
                CREATE TABLE queues (
                    id INTEGER PRIMARY KEY,
                    name TEXT NOT NULL,
                    max_concurrent INTEGER NOT NULL,
                    items TEXT NOT NULL,
                    schedule TEXT NOT NULL,
                    stop_on_empty INTEGER NOT NULL
                );
                CREATE TABLE categories (
                    id INTEGER PRIMARY KEY,
                    name TEXT NOT NULL,
                    icon TEXT NOT NULL,
                    folder TEXT NOT NULL,
                    use_folder INTEGER NOT NULL,
                    extensions TEXT NOT NULL,
                    url_patterns TEXT NOT NULL
                );",
            )?;
        }
        if version < 2 {
            conn.execute_batch("ALTER TABLE queues ADD COLUMN power_action TEXT NOT NULL DEFAULT 'none';")?;
        }
        conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        Ok(Store { conn: Mutex::new(conn) })
    }

    fn conn(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Inserts a download. Id 0 takes the next free id, any other id is kept.
    pub fn insert_download(&self, item: &DownloadItem) -> Result<i64> {
        let secrets = seal(item)?;
        let resume = item.resume.as_ref().map(serde_json::to_string).transpose()?;
        let conn = self.conn();
        conn.execute(
            &format!("INSERT INTO downloads ({DOWNLOAD_COLUMNS}) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22)"),
            params![
                (item.id != 0).then_some(item.id),
                item.url,
                kind_str(item.kind),
                item.referrer,
                item.user_agent,
                secrets,
                item.folder,
                item.name,
                item.name_locked,
                item.size.map(|v| v as i64),
                item.downloaded as i64,
                item.status.as_str(),
                item.error,
                item.date_added,
                item.date_started,
                item.date_finished,
                item.queue_id,
                item.category_id,
                item.threads.map(|v| v as i64),
                item.speed_limit as i64,
                item.checksum,
                resume,
            ],
        )?;
        Ok(if item.id != 0 { item.id } else { conn.last_insert_rowid() })
    }

    pub fn update_download(&self, item: &DownloadItem) -> Result<()> {
        let secrets = seal(item)?;
        let resume = item.resume.as_ref().map(serde_json::to_string).transpose()?;
        self.conn().execute(
            "UPDATE downloads SET url=?2, kind=?3, referrer=?4, user_agent=?5, secrets=?6, folder=?7, name=?8, name_locked=?9, size=?10, \
             downloaded=?11, status=?12, error=?13, date_added=?14, date_started=?15, date_finished=?16, queue_id=?17, category_id=?18, \
             threads=?19, speed_limit=?20, checksum=?21, resume=?22 WHERE id=?1",
            params![
                item.id,
                item.url,
                kind_str(item.kind),
                item.referrer,
                item.user_agent,
                secrets,
                item.folder,
                item.name,
                item.name_locked,
                item.size.map(|v| v as i64),
                item.downloaded as i64,
                item.status.as_str(),
                item.error,
                item.date_added,
                item.date_started,
                item.date_finished,
                item.queue_id,
                item.category_id,
                item.threads.map(|v| v as i64),
                item.speed_limit as i64,
                item.checksum,
                resume,
            ],
        )?;
        Ok(())
    }

    pub fn delete_download(&self, id: i64) -> Result<()> {
        self.conn().execute("DELETE FROM downloads WHERE id=?1", params![id])?;
        Ok(())
    }

    pub fn download_exists(&self, id: i64) -> Result<bool> {
        Ok(self.conn().query_row("SELECT 1 FROM downloads WHERE id=?1", params![id], |_| Ok(())).optional()?.is_some())
    }

    pub fn get_download(&self, id: i64) -> Result<Option<DownloadItem>> {
        Ok(self
            .conn()
            .query_row(&format!("SELECT {DOWNLOAD_COLUMNS} FROM downloads WHERE id=?1"), params![id], row_to_item)
            .optional()?)
    }

    pub fn load_downloads(&self) -> Result<Vec<DownloadItem>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(&format!("SELECT {DOWNLOAD_COLUMNS} FROM downloads ORDER BY id"))?;
        let rows = stmt.query_map([], row_to_item)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn save_queue(&self, queue: &Queue) -> Result<()> {
        self.conn().execute(
            "INSERT INTO queues (id, name, max_concurrent, items, schedule, stop_on_empty, power_action) VALUES (?1,?2,?3,?4,?5,?6,?7)
             ON CONFLICT(id) DO UPDATE SET name=?2, max_concurrent=?3, items=?4, schedule=?5, stop_on_empty=?6, power_action=?7",
            params![
                queue.id,
                queue.name,
                queue.max_concurrent,
                serde_json::to_string(&queue.items)?,
                serde_json::to_string(&queue.schedule)?,
                queue.stop_on_empty,
                queue.power_action.as_str()
            ],
        )?;
        Ok(())
    }

    pub fn delete_queue(&self, id: i64) -> Result<()> {
        self.conn().execute("DELETE FROM queues WHERE id=?1", params![id])?;
        Ok(())
    }

    pub fn load_queues(&self) -> Result<Vec<Queue>> {
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT id, name, max_concurrent, items, schedule, stop_on_empty, power_action FROM queues ORDER BY id")?;
        let rows = stmt.query_map([], |row| {
            let items: String = row.get(3)?;
            let schedule: String = row.get(4)?;
            Ok(Queue {
                id: row.get(0)?,
                name: row.get(1)?,
                max_concurrent: row.get(2)?,
                items: serde_json::from_str(&items).unwrap_or_default(),
                schedule: serde_json::from_str::<Schedule>(&schedule).unwrap_or_default(),
                stop_on_empty: row.get::<_, i64>(5)? != 0,
                power_action: PowerAction::parse(&row.get::<_, String>(6)?),
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn save_category(&self, category: &Category) -> Result<()> {
        self.conn().execute(
            "INSERT INTO categories (id, name, icon, folder, use_folder, extensions, url_patterns) VALUES (?1,?2,?3,?4,?5,?6,?7)
             ON CONFLICT(id) DO UPDATE SET name=?2, icon=?3, folder=?4, use_folder=?5, extensions=?6, url_patterns=?7",
            params![
                category.id,
                category.name,
                category.icon,
                category.folder,
                category.use_folder,
                serde_json::to_string(&category.extensions)?,
                serde_json::to_string(&category.url_patterns)?
            ],
        )?;
        Ok(())
    }

    pub fn delete_category(&self, id: i64) -> Result<()> {
        self.conn().execute("DELETE FROM categories WHERE id=?1", params![id])?;
        Ok(())
    }

    pub fn load_categories(&self) -> Result<Vec<Category>> {
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT id, name, icon, folder, use_folder, extensions, url_patterns FROM categories ORDER BY id")?;
        let rows = stmt.query_map([], |row| {
            let extensions: String = row.get(5)?;
            let patterns: String = row.get(6)?;
            Ok(Category {
                id: row.get(0)?,
                name: row.get(1)?,
                icon: row.get(2)?,
                folder: row.get(3)?,
                use_folder: row.get::<_, i64>(4)? != 0,
                extensions: serde_json::from_str(&extensions).unwrap_or_default(),
                url_patterns: serde_json::from_str(&patterns).unwrap_or_default(),
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Next id for a new queue or category: above the highest and above 10, like ABDM.
    pub fn next_queue_id(&self) -> Result<i64> {
        let max: Option<i64> = self.conn().query_row("SELECT MAX(id) FROM queues", [], |r| r.get(0))?;
        Ok(max.unwrap_or(0).max(10) + 1)
    }

    pub fn next_category_id(&self) -> Result<i64> {
        let max: Option<i64> = self.conn().query_row("SELECT MAX(id) FROM categories", [], |r| r.get(0))?;
        Ok(max.unwrap_or(-1).max(5) + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdm_engine::Part;

    fn sample() -> DownloadItem {
        let mut item = DownloadItem::new("https://example.com/a.zip", "D:\\Dl", "a.zip");
        item.headers = vec![("Cookie".into(), "fake=1".into())];
        item.basic_auth = Some(("user".into(), "pw".into()));
        item.size = Some(100);
        item.resume = Some(ResumeState {
            file_name: "a.zip".into(),
            total: Some(100),
            parts: vec![Part { from: 0, to: 99, current: 10 }],
            ..Default::default()
        });
        item
    }

    #[test]
    fn download_round_trip_keeps_secrets() {
        let store = Store::open_memory().unwrap();
        let id = store.insert_download(&sample()).unwrap();
        let loaded = store.get_download(id).unwrap().unwrap();
        assert_eq!(loaded.headers, vec![("Cookie".to_string(), "fake=1".to_string())]);
        assert_eq!(loaded.basic_auth, Some(("user".into(), "pw".into())));
        assert_eq!(loaded.resume.unwrap().parts[0].current, 10);
    }

    #[test]
    fn secrets_are_encrypted_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sdm.db");
        {
            let store = Store::open(&path).unwrap();
            store.insert_download(&sample()).unwrap();
        }
        let raw = std::fs::read(&path).unwrap();
        let wal = std::fs::read(dir.path().join("sdm.db-wal")).unwrap_or_default();
        for bytes in [raw, wal] {
            assert!(!bytes.windows(6).any(|w| w == b"fake=1"));
        }
    }

    #[test]
    fn explicit_ids_are_kept_and_next_id_follows() {
        let store = Store::open_memory().unwrap();
        let mut item = sample();
        item.id = 502;
        assert_eq!(store.insert_download(&item).unwrap(), 502);
        item.id = 0;
        assert_eq!(store.insert_download(&item).unwrap(), 503);
        assert!(store.download_exists(502).unwrap());
    }

    #[test]
    fn update_and_delete() {
        let store = Store::open_memory().unwrap();
        let id = store.insert_download(&sample()).unwrap();
        let mut item = store.get_download(id).unwrap().unwrap();
        item.status = Status::Completed;
        item.headers.clear();
        store.update_download(&item).unwrap();
        let again = store.get_download(id).unwrap().unwrap();
        assert_eq!(again.status, Status::Completed);
        assert!(again.headers.is_empty());
        store.delete_download(id).unwrap();
        assert!(store.get_download(id).unwrap().is_none());
    }

    #[test]
    fn queues_and_categories_upsert() {
        let store = Store::open_memory().unwrap();
        let mut queue = Queue::main();
        queue.items = vec![3, 1, 2];
        store.save_queue(&queue).unwrap();
        queue.name = "Night".into();
        queue.power_action = PowerAction::Sleep;
        store.save_queue(&queue).unwrap();
        let queues = store.load_queues().unwrap();
        assert_eq!(queues.len(), 1);
        assert_eq!(queues[0].items, vec![3, 1, 2]);
        assert_eq!(queues[0].power_action, PowerAction::Sleep);
        assert_eq!(store.next_queue_id().unwrap(), 11);

        let cat = crate::model::default_categories(Path::new("D:\\Dl")).remove(0);
        store.save_category(&cat).unwrap();
        assert_eq!(store.load_categories().unwrap(), vec![cat]);
        assert_eq!(store.next_category_id().unwrap(), 6);
    }

    #[test]
    fn migration_is_stable_on_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sdm.db");
        Store::open(&path).unwrap().insert_download(&sample()).unwrap();
        let store = Store::open(&path).unwrap();
        assert_eq!(store.load_downloads().unwrap().len(), 1);
    }

    #[test]
    fn version_one_database_gains_power_action() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sdm.db");
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE downloads (id INTEGER PRIMARY KEY AUTOINCREMENT, url TEXT NOT NULL, kind TEXT NOT NULL, referrer TEXT,
                    user_agent TEXT, secrets BLOB, folder TEXT NOT NULL, name TEXT NOT NULL, name_locked INTEGER NOT NULL DEFAULT 0,
                    size INTEGER, downloaded INTEGER NOT NULL DEFAULT 0, status TEXT NOT NULL, error TEXT, date_added INTEGER NOT NULL,
                    date_started INTEGER, date_finished INTEGER, queue_id INTEGER, category_id INTEGER, threads INTEGER,
                    speed_limit INTEGER NOT NULL DEFAULT 0, checksum TEXT, resume TEXT);
                 CREATE TABLE queues (id INTEGER PRIMARY KEY, name TEXT NOT NULL, max_concurrent INTEGER NOT NULL, items TEXT NOT NULL,
                    schedule TEXT NOT NULL, stop_on_empty INTEGER NOT NULL);
                 CREATE TABLE categories (id INTEGER PRIMARY KEY, name TEXT NOT NULL, icon TEXT NOT NULL, folder TEXT NOT NULL,
                    use_folder INTEGER NOT NULL, extensions TEXT NOT NULL, url_patterns TEXT NOT NULL);
                 INSERT INTO queues VALUES (0, 'Main', 2, '[]', '{}', 0);
                 PRAGMA user_version = 1;",
            )
            .unwrap();
        }
        let store = Store::open(&path).unwrap();
        let queues = store.load_queues().unwrap();
        assert_eq!(queues[0].name, "Main");
        assert_eq!(queues[0].power_action, PowerAction::None);
    }
}

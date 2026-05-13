// Dictionary lookup backed by a bundled, read-only SQLite db.
//
// The schema mirrors scripts/build-dict.mjs:
//   entries(word PK, phonetic, translation, lang_src, lang_tgt)
//
// A single r2d2 pool is shared via AppState. lookup_conn is the pure
// function; lookup is the thin adapter that pulls a connection.

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde::Serialize;
use std::path::Path;

use crate::error::{AppError, AppResult};

pub type DictPool = Pool<SqliteConnectionManager>;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DictEntry {
    pub word: String,
    pub phonetic: String,
    pub translation: String,
    pub lang_pair: String,
}

/// Open a read-only pool over the bundled db.
pub fn open(db_path: &Path) -> AppResult<DictPool> {
    let manager = SqliteConnectionManager::file(db_path)
        .with_flags(OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX);
    Pool::builder()
        .max_size(4)
        .build(manager)
        .map_err(|e| AppError::Dict(format!("open pool: {e}")))
}

/// Look up a word via the pool. Returns Ok(None) if not found.
pub fn lookup(pool: &DictPool, word: &str) -> AppResult<Option<DictEntry>> {
    let conn = pool
        .get()
        .map_err(|e| AppError::Dict(format!("get conn: {e}")))?;
    lookup_conn(&conn, word)
}

/// Pure lookup against a connection; words are matched lowercased.
pub fn lookup_conn(conn: &Connection, word: &str) -> AppResult<Option<DictEntry>> {
    let key = word.trim().to_lowercase();
    if key.is_empty() {
        return Ok(None);
    }
    let mut stmt = conn
        .prepare_cached(
            "SELECT phonetic, translation, lang_src, lang_tgt
             FROM entries WHERE word = ?1",
        )
        .map_err(|e| AppError::Dict(format!("prepare: {e}")))?;
    let row = stmt
        .query_row([&key], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })
        .optional()
        .map_err(|e| AppError::Dict(format!("query: {e}")))?;
    Ok(row.map(|(phonetic, translation, src, tgt)| DictEntry {
        word: key,
        phonetic,
        translation: unescape_translation(&translation),
        lang_pair: format!("{src}-{tgt}"),
    }))
}

/// ECDICT stores newlines as the literal two-character sequence `\n`.
/// Convert them to real newlines so the UI can render multi-line entries.
fn unescape_translation(raw: &str) -> String {
    raw.replace("\\n", "\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Connection {
        let conn = Connection::open_in_memory().expect("open mem");
        conn.execute_batch(
            "CREATE TABLE entries (
                word TEXT PRIMARY KEY,
                phonetic TEXT NOT NULL DEFAULT '',
                translation TEXT NOT NULL DEFAULT '',
                lang_src TEXT NOT NULL,
                lang_tgt TEXT NOT NULL
            ) WITHOUT ROWID;
            INSERT INTO entries VALUES ('hello', 'həˈləʊ', '你好', 'en', 'zh');
            INSERT INTO entries VALUES ('world', 'wɜːld', '世界', 'en', 'zh');",
        )
        .expect("seed");
        conn
    }

    #[test]
    fn known_word_hits() {
        let conn = fixture();
        let entry = lookup_conn(&conn, "hello").unwrap().expect("found");
        assert_eq!(entry.word, "hello");
        assert_eq!(entry.translation, "你好");
        assert_eq!(entry.lang_pair, "en-zh");
    }

    #[test]
    fn lookup_is_case_and_whitespace_insensitive() {
        let conn = fixture();
        let entry = lookup_conn(&conn, "  Hello  ").unwrap().expect("found");
        assert_eq!(entry.word, "hello");
    }

    #[test]
    fn unknown_word_returns_none() {
        let conn = fixture();
        assert!(lookup_conn(&conn, "qzx").unwrap().is_none());
    }

    #[test]
    fn empty_query_returns_none() {
        let conn = fixture();
        assert!(lookup_conn(&conn, "   ").unwrap().is_none());
    }

    #[test]
    fn translation_newlines_are_unescaped() {
        let conn = Connection::open_in_memory().expect("open mem");
        conn.execute_batch(
            "CREATE TABLE entries (
                word TEXT PRIMARY KEY,
                phonetic TEXT NOT NULL DEFAULT '',
                translation TEXT NOT NULL DEFAULT '',
                lang_src TEXT NOT NULL,
                lang_tgt TEXT NOT NULL
            ) WITHOUT ROWID;
            INSERT INTO entries VALUES ('run', '', 'n. 跑\\nvt. 经营', 'en', 'zh');",
        )
        .expect("seed");
        let entry = lookup_conn(&conn, "run").unwrap().expect("found");
        assert_eq!(entry.translation, "n. 跑\nvt. 经营");
    }
}

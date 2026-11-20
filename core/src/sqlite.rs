//! A small, safe way into the system's SQLite: just what the library needs.
//!
//! Sofa links the SQLite its distribution ships (`libsqlite3`), which the distribution keeps patched, and talks to
//! it through the dozen C functions below. A connection opens a file (or memory), runs statements with `?` values,
//! and reads rows one at a time; a transaction rolls back unless it is committed. Everything else SQLite can do is
//! left out on purpose.
//!
//! The rules that keep the C side sound live here, in one place:
//! - a [`Statement`] borrows its [`Connection`], a [`Rows`] borrows its statement, and a [`Row`] borrows its
//!   [`Rows`], so no handle outlives what it points into and a row's text can't outlive the next step;
//! - text values are bound as copies (`SQLITE_TRANSIENT`), so SQLite never reads Rust memory after a call returns;
//! - a column is read as text only when SQLite holds it as text, so reading never converts a value in place.

#![warn(clippy::undocumented_unsafe_blocks)]

use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::fmt;
use std::marker::PhantomData;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::ptr;

#[repr(C)]
struct Sqlite3 {
    _private: [u8; 0],
}

#[repr(C)]
struct Sqlite3Stmt {
    _private: [u8; 0],
}

const SQLITE_OK: c_int = 0;
const SQLITE_MISUSE: c_int = 21;
const SQLITE_RANGE: c_int = 25;
const SQLITE_ROW: c_int = 100;
const SQLITE_DONE: c_int = 101;
const SQLITE_OPEN_READWRITE: c_int = 0x0000_0002;
const SQLITE_OPEN_CREATE: c_int = 0x0000_0004;
// Sofa uses a connection from one thread only, so SQLite's own locking around it isn't needed.
const SQLITE_OPEN_NOMUTEX: c_int = 0x0000_8000;
const SQLITE_INTEGER: c_int = 1;
const SQLITE_TEXT: c_int = 3;
const SQLITE_NULL: c_int = 5;
// `SQLITE_TRANSIENT`, the destructor value that tells SQLite to copy a bound value at once. In C it is
// `(sqlite3_destructor_type)-1`; it is declared below as a pointer-sized integer, which is passed the same way.
const SQLITE_TRANSIENT: isize = -1;

#[link(name = "sqlite3")]
unsafe extern "C" {
    fn sqlite3_open_v2(filename: *const c_char, db: *mut *mut Sqlite3, flags: c_int, vfs: *const c_char) -> c_int;
    fn sqlite3_close_v2(db: *mut Sqlite3) -> c_int;
    fn sqlite3_errmsg(db: *mut Sqlite3) -> *const c_char;
    fn sqlite3_extended_errcode(db: *mut Sqlite3) -> c_int;
    fn sqlite3_exec(
        db: *mut Sqlite3,
        sql: *const c_char,
        callback: *const c_void,
        arg: *mut c_void,
        errmsg: *mut *mut c_char,
    ) -> c_int;
    fn sqlite3_prepare_v2(
        db: *mut Sqlite3,
        sql: *const c_char,
        bytes: c_int,
        stmt: *mut *mut Sqlite3Stmt,
        tail: *mut *const c_char,
    ) -> c_int;
    fn sqlite3_finalize(stmt: *mut Sqlite3Stmt) -> c_int;
    fn sqlite3_reset(stmt: *mut Sqlite3Stmt) -> c_int;
    fn sqlite3_bind_parameter_count(stmt: *mut Sqlite3Stmt) -> c_int;
    fn sqlite3_bind_null(stmt: *mut Sqlite3Stmt, index: c_int) -> c_int;
    fn sqlite3_bind_int64(stmt: *mut Sqlite3Stmt, index: c_int, value: i64) -> c_int;
    fn sqlite3_bind_text(stmt: *mut Sqlite3Stmt, index: c_int, text: *const c_char, bytes: c_int, d: isize) -> c_int;
    fn sqlite3_step(stmt: *mut Sqlite3Stmt) -> c_int;
    fn sqlite3_column_count(stmt: *mut Sqlite3Stmt) -> c_int;
    fn sqlite3_column_type(stmt: *mut Sqlite3Stmt, column: c_int) -> c_int;
    fn sqlite3_column_int64(stmt: *mut Sqlite3Stmt, column: c_int) -> i64;
    fn sqlite3_column_text(stmt: *mut Sqlite3Stmt, column: c_int) -> *const u8;
    fn sqlite3_column_bytes(stmt: *mut Sqlite3Stmt, column: c_int) -> c_int;
    fn sqlite3_changes64(db: *mut Sqlite3) -> i64;
    fn sqlite3_last_insert_rowid(db: *mut Sqlite3) -> i64;
}

/// What went wrong: SQLite's (extended) result code and its message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    /// SQLite's extended result code, or [`SQLITE_MISUSE`]/[`SQLITE_RANGE`] for a mistake caught on the Rust side.
    pub code: i32,
    /// What SQLite (or this module) said about it.
    pub message: String,
}

impl Error {
    fn new(code: c_int, message: impl Into<String>) -> Self {
        Self { code, message: message.into() }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (SQLite code {})", self.message, self.code)
    }
}

impl std::error::Error for Error {}

/// A result from this module.
pub type Result<T> = std::result::Result<T, Error>;

/// A value bound to a statement's `?`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Value<'a> {
    /// SQL `NULL`.
    Null,
    /// A whole number.
    Int(i64),
    /// Text, copied by SQLite when bound.
    Text(&'a str),
}

impl From<i64> for Value<'_> {
    fn from(n: i64) -> Self {
        Self::Int(n)
    }
}

impl From<usize> for Value<'_> {
    /// Counts and positions: a value past `i64::MAX` (which can't happen on a real list) stays at the maximum.
    fn from(n: usize) -> Self {
        Self::Int(i64::try_from(n).unwrap_or(i64::MAX))
    }
}

impl<'a> From<&'a str> for Value<'a> {
    fn from(s: &'a str) -> Self {
        Self::Text(s)
    }
}

impl<'a> From<&'a String> for Value<'a> {
    fn from(s: &'a String) -> Self {
        Self::Text(s)
    }
}

impl<'a, T: Into<Value<'a>>> From<Option<T>> for Value<'a> {
    fn from(v: Option<T>) -> Self {
        v.map_or(Self::Null, Into::into)
    }
}

/// An open database.
#[derive(Debug)]
pub struct Connection {
    db: *mut Sqlite3,
}

impl Connection {
    /// Opens (or creates) the database file at `path`.
    ///
    /// # Errors
    ///
    /// The path holds a NUL byte, or SQLite can't open or create the file.
    pub fn open(path: &Path) -> Result<Self> {
        let name = CString::new(path.as_os_str().as_bytes())
            .map_err(|_| Error::new(SQLITE_MISUSE, "the database path holds a NUL byte"))?;
        Self::open_c(&name)
    }

    /// A database in memory, gone when the connection closes.
    ///
    /// # Errors
    ///
    /// SQLite can't start.
    pub fn open_in_memory() -> Result<Self> {
        Self::open_c(c":memory:")
    }

    fn open_c(name: &CStr) -> Result<Self> {
        let mut db = ptr::null_mut();
        let flags = SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE | SQLITE_OPEN_NOMUTEX;
        // SAFETY: `name` is NUL-terminated and lives across the call; `db` is a valid place for the handle; a null
        // VFS name means the default one.
        let rc = unsafe { sqlite3_open_v2(name.as_ptr(), &raw mut db, flags, ptr::null()) };
        // Even a failed open usually hands back a handle (to read the message from), which must be closed.
        let conn = Self { db };
        if rc != SQLITE_OK {
            return Err(if db.is_null() {
                Error::new(rc, "SQLite couldn't allocate a connection")
            } else {
                conn.error()
            });
        }
        Ok(conn)
    }

    /// The connection's last error.
    fn error(&self) -> Error {
        // SAFETY: `self.db` is an open handle; the message is NUL-terminated, owned by SQLite and valid until the
        // next call on this connection, so it is copied out at once.
        let (code, message) = unsafe {
            (sqlite3_extended_errcode(self.db), CStr::from_ptr(sqlite3_errmsg(self.db)).to_string_lossy().into_owned())
        };
        Error::new(code, message)
    }

    /// Runs `sql`, which may hold several statements separated by `;` and takes no values. Rows they return are
    /// dropped.
    ///
    /// # Errors
    ///
    /// `sql` holds a NUL byte, or a statement fails (the ones before it stay done).
    pub fn execute_batch(&self, sql: &str) -> Result<()> {
        let sql = CString::new(sql).map_err(|_| Error::new(SQLITE_MISUSE, "the SQL holds a NUL byte"))?;
        // SAFETY: `self.db` is open; `sql` is NUL-terminated and lives across the call; no callback, so its argument
        // is unused; a null `errmsg` place asks SQLite not to allocate a message (it is read from the handle).
        let rc = unsafe { sqlite3_exec(self.db, sql.as_ptr(), ptr::null(), ptr::null_mut(), ptr::null_mut()) };
        if rc == SQLITE_OK { Ok(()) } else { Err(self.error()) }
    }

    /// Compiles one statement.
    ///
    /// # Errors
    ///
    /// The SQL doesn't compile, is empty, or is longer than SQLite takes.
    pub fn prepare(&self, sql: &str) -> Result<Statement<'_>> {
        let len = c_int::try_from(sql.len()).map_err(|_| Error::new(SQLITE_MISUSE, "the SQL is too long"))?;
        let mut stmt = ptr::null_mut();
        // SAFETY: `self.db` is open; `sql` points at `len` readable bytes (SQLite stops there, no NUL needed);
        // `stmt` is a valid place for the handle; a null tail place is allowed.
        let rc = unsafe { sqlite3_prepare_v2(self.db, sql.as_ptr().cast(), len, &raw mut stmt, ptr::null_mut()) };
        if rc != SQLITE_OK {
            return Err(self.error());
        }
        if stmt.is_null() {
            return Err(Error::new(SQLITE_MISUSE, "the SQL holds no statement"));
        }
        Ok(Statement { conn: self, stmt })
    }

    /// Runs one statement with `values` and says how many rows it changed.
    ///
    /// # Errors
    ///
    /// The statement doesn't compile, the values don't fit it, or it fails.
    pub fn execute(&self, sql: &str, values: &[Value<'_>]) -> Result<usize> {
        let mut stmt = self.prepare(sql)?;
        let mut rows = stmt.query(values)?;
        while rows.next()?.is_some() {}
        // SAFETY: `self.db` is open.
        let changed = unsafe { sqlite3_changes64(self.db) };
        Ok(usize::try_from(changed).unwrap_or(0))
    }

    /// The first row of one statement, read with `read`; `None` when there is none.
    ///
    /// # Errors
    ///
    /// The statement fails, or `read` does.
    pub fn query_row<T>(
        &self,
        sql: &str,
        values: &[Value<'_>],
        read: impl FnOnce(&Row<'_>) -> Result<T>,
    ) -> Result<Option<T>> {
        let mut stmt = self.prepare(sql)?;
        let mut rows = stmt.query(values)?;
        rows.next()?.map(|row| read(&row)).transpose()
    }

    /// The first row of one statement, read with `read`, which must be there.
    ///
    /// # Errors
    ///
    /// The statement fails or returns no row, or `read` fails.
    pub fn query_one<T>(&self, sql: &str, values: &[Value<'_>], read: impl FnOnce(&Row<'_>) -> Result<T>) -> Result<T> {
        self.query_row(sql, values, read)?.ok_or_else(|| Error::new(SQLITE_DONE, "the query returned no row"))
    }

    /// Every row of one statement, each read with `read`.
    ///
    /// # Errors
    ///
    /// The statement fails, or `read` does.
    pub fn query_all<T>(
        &self,
        sql: &str,
        values: &[Value<'_>],
        mut read: impl FnMut(&Row<'_>) -> Result<T>,
    ) -> Result<Vec<T>> {
        let mut stmt = self.prepare(sql)?;
        let mut rows = stmt.query(values)?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(read(&row)?);
        }
        Ok(out)
    }

    /// The row id of the last row inserted on this connection.
    #[must_use]
    pub fn last_insert_rowid(&self) -> i64 {
        // SAFETY: `self.db` is open.
        unsafe { sqlite3_last_insert_rowid(self.db) }
    }

    /// Begins a transaction; it rolls back when dropped without [`Transaction::commit`].
    ///
    /// # Errors
    ///
    /// SQLite can't begin one (for example, one is already open).
    pub fn transaction(&mut self) -> Result<Transaction<'_>> {
        self.execute_batch("BEGIN")?;
        Ok(Transaction { conn: self, open: true })
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        // SAFETY: `self.db` came from `sqlite3_open_v2` and is closed only here. Every `Statement` borrows the
        // connection, so none is left; `close_v2` would wait for one anyway. Null is allowed.
        unsafe { sqlite3_close_v2(self.db) };
    }
}

/// A transaction on a connection: reads and writes go through it like through the connection (it derefs to one).
#[derive(Debug)]
pub struct Transaction<'c> {
    conn: &'c Connection,
    open: bool,
}

impl Transaction<'_> {
    /// Makes the transaction's changes stay.
    ///
    /// # Errors
    ///
    /// SQLite can't commit; the changes are then rolled back.
    pub fn commit(mut self) -> Result<()> {
        self.conn.execute_batch("COMMIT")?;
        self.open = false;
        Ok(())
    }
}

impl std::ops::Deref for Transaction<'_> {
    type Target = Connection;

    fn deref(&self) -> &Connection {
        self.conn
    }
}

impl Drop for Transaction<'_> {
    fn drop(&mut self) {
        if self.open {
            // Nothing to tell anyone if this fails: SQLite has then already rolled the transaction back itself.
            let _ = self.conn.execute_batch("ROLLBACK");
        }
    }
}

/// A compiled statement.
#[derive(Debug)]
pub struct Statement<'c> {
    conn: &'c Connection,
    stmt: *mut Sqlite3Stmt,
}

impl Statement<'_> {
    /// Binds `values` to the statement's `?`s, in order, and starts it. The statement is reset first, so it can be
    /// run again.
    ///
    /// # Errors
    ///
    /// The number of values isn't the number of `?`s, or a value can't be bound.
    pub fn query(&mut self, values: &[Value<'_>]) -> Result<Rows<'_>> {
        // SAFETY: `self.stmt` is a live statement; resetting one that hasn't run is allowed. Its result repeats the
        // last step's error, which was reported then.
        unsafe { sqlite3_reset(self.stmt) };
        // SAFETY: `self.stmt` is a live statement.
        let wanted = unsafe { sqlite3_bind_parameter_count(self.stmt) };
        if usize::try_from(wanted).ok() != Some(values.len()) {
            return Err(Error::new(SQLITE_RANGE, format!("the statement takes {wanted} values, not {}", values.len())));
        }
        for (index, value) in (1..).zip(values) {
            let rc = match *value {
                // SAFETY: a live statement, an index within its parameter count (checked above).
                Value::Null => unsafe { sqlite3_bind_null(self.stmt, index) },
                // SAFETY: as above.
                Value::Int(n) => unsafe { sqlite3_bind_int64(self.stmt, index, n) },
                Value::Text(s) => {
                    let len = c_int::try_from(s.len()).map_err(|_| Error::new(SQLITE_MISUSE, "a text is too long"))?;
                    // SAFETY: as above; `s` points at `len` readable bytes of UTF-8, and `SQLITE_TRANSIENT` makes
                    // SQLite copy them before the call returns, so it never reads `s` later.
                    unsafe { sqlite3_bind_text(self.stmt, index, s.as_ptr().cast(), len, SQLITE_TRANSIENT) }
                }
            };
            if rc != SQLITE_OK {
                return Err(self.conn.error());
            }
        }
        Ok(Rows { stmt: self, done: false })
    }
}

impl Drop for Statement<'_> {
    fn drop(&mut self) {
        // SAFETY: `self.stmt` came from `sqlite3_prepare_v2` and is finalized only here; every `Rows` and `Row`
        // borrows the statement, so none is left. The result repeats the last step's error, already reported.
        unsafe { sqlite3_finalize(self.stmt) };
    }
}

/// The rows a running statement returns, read one at a time with [`Rows::next`].
#[derive(Debug)]
pub struct Rows<'s> {
    stmt: &'s Statement<'s>,
    done: bool,
}

impl Rows<'_> {
    /// Steps to the next row; `None` once the statement is done.
    ///
    /// # Errors
    ///
    /// The statement fails.
    #[allow(clippy::should_implement_trait)] // A row borrows the `Rows`, which `Iterator` can't express.
    pub fn next(&mut self) -> Result<Option<Row<'_>>> {
        if self.done {
            return Ok(None);
        }
        // SAFETY: a live statement with all its values bound. No `Row` of an earlier step is alive: each borrows
        // `self` mutably.
        match unsafe { sqlite3_step(self.stmt.stmt) } {
            SQLITE_ROW => Ok(Some(Row { stmt: self.stmt.stmt, columns: self.columns(), _rows: PhantomData })),
            SQLITE_DONE => {
                self.done = true;
                Ok(None)
            }
            _ => {
                self.done = true;
                Err(self.stmt.conn.error())
            }
        }
    }

    fn columns(&self) -> c_int {
        // SAFETY: a live statement.
        unsafe { sqlite3_column_count(self.stmt.stmt) }
    }
}

/// One row: its columns are read by index, from 0.
#[derive(Debug)]
pub struct Row<'r> {
    stmt: *mut Sqlite3Stmt,
    columns: c_int,
    _rows: PhantomData<&'r mut ()>,
}

impl Row<'_> {
    /// Column `i` as `T`.
    ///
    /// # Errors
    ///
    /// There is no column `i`, or it holds another kind of value.
    pub fn get<T: FromColumn>(&self, i: usize) -> Result<T> {
        T::from_column(self, i)
    }

    /// Column `i` as text, read in place; `None` when it isn't text (or isn't valid UTF-8).
    ///
    /// # Errors
    ///
    /// There is no column `i`.
    pub fn text(&self, i: usize) -> Result<Option<&str>> {
        let column = self.column(i)?;
        if self.kind(column) != SQLITE_TEXT {
            return Ok(None);
        }
        // SAFETY: the statement is on this row (a `Row` lives only until the next step), `column` is in range and
        // holds text, so `column_text` converts nothing and returns its bytes, then `column_bytes` their length
        // (called second, as SQLite asks). The bytes stay put until the statement steps, resets or is finalized,
        // all of which need the `Rows` this row borrows, so the slice can't outlive them.
        let bytes = unsafe {
            let text = sqlite3_column_text(self.stmt, column);
            let len = usize::try_from(sqlite3_column_bytes(self.stmt, column)).unwrap_or(0);
            if text.is_null() { &[][..] } else { std::slice::from_raw_parts(text, len) }
        };
        Ok(std::str::from_utf8(bytes).ok())
    }

    fn column(&self, i: usize) -> Result<c_int> {
        c_int::try_from(i)
            .ok()
            .filter(|c| *c < self.columns)
            .ok_or_else(|| Error::new(SQLITE_RANGE, format!("there is no column {i}")))
    }

    fn kind(&self, column: c_int) -> c_int {
        // SAFETY: the statement is on this row and `column` is in range.
        unsafe { sqlite3_column_type(self.stmt, column) }
    }

    fn int(&self, i: usize) -> Result<Option<i64>> {
        let column = self.column(i)?;
        match self.kind(column) {
            SQLITE_NULL => Ok(None),
            // SAFETY: the statement is on this row, `column` is in range and holds an integer (no conversion).
            SQLITE_INTEGER => Ok(Some(unsafe { sqlite3_column_int64(self.stmt, column) })),
            _ => Err(Error::new(SQLITE_MISUSE, format!("column {i} isn't a whole number"))),
        }
    }
}

/// A kind of value a column can be read as.
pub trait FromColumn: Sized {
    /// Column `i` of `row` as this kind.
    ///
    /// # Errors
    ///
    /// There is no such column, or it holds another kind of value.
    fn from_column(row: &Row<'_>, i: usize) -> Result<Self>;
}

impl FromColumn for i64 {
    fn from_column(row: &Row<'_>, i: usize) -> Result<Self> {
        row.int(i)?.ok_or_else(|| Error::new(SQLITE_MISUSE, format!("column {i} is NULL")))
    }
}

impl FromColumn for Option<i64> {
    fn from_column(row: &Row<'_>, i: usize) -> Result<Self> {
        row.int(i)
    }
}

impl FromColumn for String {
    fn from_column(row: &Row<'_>, i: usize) -> Result<Self> {
        row.text(i)?.map(str::to_owned).ok_or_else(|| Error::new(SQLITE_MISUSE, format!("column {i} isn't text")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let db = Connection::open_in_memory().expect("an in-memory database");
        db.execute_batch("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT, n INTEGER)").expect("a table");
        db
    }

    #[test]
    fn values_go_in_and_come_back_out() -> Result<()> {
        let db = db();
        let name = String::from("caf\u{e9} \u{1F6CB}\0and after a NUL");
        assert_eq!(db.execute("INSERT INTO t (name, n) VALUES (?1, ?2)", &[(&name).into(), 7_i64.into()])?, 1);
        let id = db.last_insert_rowid();
        db.execute("INSERT INTO t (name, n) VALUES (?1, ?2)", &[Value::Null, None::<i64>.into()])?;
        let rows = db.query_all("SELECT name, n FROM t ORDER BY id", &[], |r| {
            Ok((r.text(0)?.map(str::to_owned), r.get::<Option<i64>>(1)?))
        })?;
        assert_eq!(rows, [(Some(name.clone()), Some(7)), (None, None)]);
        let back = db.query_row("SELECT name FROM t WHERE id = ?1", &[id.into()], |r| r.get::<String>(0))?;
        assert_eq!(back.as_deref(), Some(name.as_str()));
        assert_eq!(db.query_row("SELECT 1 FROM t WHERE id = 99", &[], |r| r.get::<i64>(0))?, None);
        Ok(())
    }

    #[test]
    fn wrong_kinds_and_counts_are_errors_not_conversions() -> Result<()> {
        let db = db();
        db.execute("INSERT INTO t (name, n) VALUES ('x', NULL)", &[])?;
        let row = |sql: &str| db.query_row(sql, &[], |r| r.get::<i64>(0));
        assert!(row("SELECT name FROM t").is_err());
        assert!(row("SELECT n FROM t").is_err());
        assert!(db.query_row("SELECT n FROM t", &[], |r| r.get::<String>(0)).is_err());
        assert!(db.query_row("SELECT n FROM t", &[], |r| r.get::<i64>(5)).is_err());
        assert!(db.execute("INSERT INTO t (name) VALUES (?1)", &[]).is_err());
        assert!(db.execute("INSERT INTO t (name) VALUES (?1)", &["a".into(), "b".into()]).is_err());
        assert!(db.execute("NOT SQL", &[]).is_err());
        assert!(db.prepare("  ").is_err());
        Ok(())
    }

    #[test]
    fn a_transaction_rolls_back_unless_committed() -> Result<()> {
        let mut db = db();
        {
            let tx = db.transaction()?;
            tx.execute("INSERT INTO t (name) VALUES ('gone')", &[])?;
        }
        {
            let tx = db.transaction()?;
            tx.execute("INSERT INTO t (name) VALUES ('kept')", &[])?;
            tx.commit()?;
        }
        let names = db.query_all("SELECT name FROM t", &[], |r| r.get::<String>(0))?;
        assert_eq!(names, ["kept"]);
        Ok(())
    }

    #[test]
    fn a_statement_runs_again_with_new_values() -> Result<()> {
        let db = db();
        let mut insert = db.prepare("INSERT INTO t (n) VALUES (?1)")?;
        for n in 0..3_i64 {
            while insert.query(&[n.into()])?.next()?.is_some() {}
        }
        drop(insert);
        let sum = db.query_row("SELECT SUM(n) FROM t", &[], |r| r.get::<i64>(0))?;
        assert_eq!(sum, Some(3));
        Ok(())
    }

    #[test]
    fn a_failed_open_is_an_error() {
        let err = Connection::open(Path::new("/nonexistent-dir-for-sofa/x.db")).expect_err("no such folder");
        assert_ne!(err.code, SQLITE_OK);
        assert!(Connection::open(Path::new("a\0b")).is_err());
    }
}

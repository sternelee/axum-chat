use libsql::Builder;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct DatabaseError(pub String);

impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Database error: {}", self.0)
    }
}

impl std::error::Error for DatabaseError {}

impl From<String> for DatabaseError {
    fn from(s: String) -> Self {
        DatabaseError(s)
    }
}

// Add conversions from common error types
impl From<libsql::Error> for DatabaseError {
    fn from(err: libsql::Error) -> Self {
        DatabaseError(format!("LibSQL error: {}", err))
    }
}

impl From<std::io::Error> for DatabaseError {
    fn from(err: std::io::Error) -> Self {
        DatabaseError(format!("IO error: {}", err))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryResult {
    pub rows: Vec<serde_json::Value>,
    pub rows_affected: u64,
}

pub struct Database {
    conn: Arc<Mutex<Option<libsql::Connection>>>,
    db_path: String,
}

impl Database {
    pub fn new(db_path: String) -> Self {
        Self {
            conn: Arc::new(Mutex::new(None)),
            db_path,
        }
    }

    pub async fn connect(&self) -> Result<(), DatabaseError> {
        // Ensure the parent directory exists before attempting to open the database
        let db_path = Path::new(&self.db_path);
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create database directory '{}': {}. Please check directory permissions.", parent.display(), e))?;
        }

        let db = Builder::new_local(&self.db_path)
            .build()
            .await
            .map_err(|e| format!("Failed to build database: {}", e))?;

        let conn = db
            .connect()
            .map_err(|e| format!("Failed to connect to database: {}", e))?;

        let mut lock = self.conn.lock().await;
        *lock = Some(conn);

        // Enable WAL mode for better concurrent access
        drop(lock);
        self.execute("PRAGMA journal_mode=WAL", vec![]).await?;

        // Set busy timeout to 5 seconds (5000 milliseconds)
        self.execute("PRAGMA busy_timeout=5000", vec![]).await?;

        Ok(())
    }

    pub async fn execute(
        &self,
        sql: &str,
        params: Vec<serde_json::Value>,
    ) -> Result<QueryResult, String> {
        self.execute_with_retry(sql, params, 3).await
    }

    async fn execute_with_retry(
        &self,
        sql: &str,
        params: Vec<serde_json::Value>,
        max_retries: u32,
    ) -> Result<QueryResult, String> {
        let mut attempt = 0;

        loop {
            let lock = self.conn.lock().await;
            let conn = lock.as_ref().ok_or("Database not connected")?;

            // Convert JSON values to libsql Values
            let libsql_params: Vec<libsql::Value> =
                params.iter().map(|v| json_to_libsql_value(v)).collect();

            // Check if this is a SELECT query - if so, use query() instead
            let sql_trimmed = sql.trim_start().to_uppercase();
            let result = if sql_trimmed.starts_with("SELECT") || sql_trimmed.starts_with("PRAGMA") {
                // This is a query that returns rows, use query() instead
                let mut stmt = match conn.prepare(sql).await {
                    Ok(stmt) => stmt,
                    Err(e) => {
                        let error_msg = format!("Prepare error: {}", e);
                        if Self::is_busy_error(&error_msg) && attempt < max_retries {
                            drop(lock);
                            attempt += 1;
                            tokio::time::sleep(tokio::time::Duration::from_millis(
                                10 * attempt as u64,
                            ))
                            .await;
                            continue;
                        }
                        return Err(error_msg);
                    }
                };

                let mut rows_result = match stmt.query(libsql_params).await {
                    Ok(rows) => rows,
                    Err(e) => {
                        let error_msg = format!("Query error: {}", e);
                        if Self::is_busy_error(&error_msg) && attempt < max_retries {
                            drop(lock);
                            attempt += 1;
                            tokio::time::sleep(tokio::time::Duration::from_millis(
                                10 * attempt as u64,
                            ))
                            .await;
                            continue;
                        }
                        return Err(error_msg);
                    }
                };

                let mut rows = Vec::new();

                while let Some(row) = rows_result
                    .next()
                    .await
                    .map_err(|e| format!("Row fetch error: {}", e))?
                {
                    let mut row_obj = serde_json::Map::new();
                    let column_count = row.column_count();

                    for i in 0..column_count {
                        let value = row
                            .get_value(i)
                            .map_err(|e| format!("Get value error: {}", e))?;
                        let column_name = row
                            .column_name(i)
                            .unwrap_or(&format!("column_{}", i))
                            .to_string();
                        row_obj.insert(column_name, libsql_value_to_json(&value));
                    }

                    rows.push(serde_json::Value::Object(row_obj));
                }

                Ok(QueryResult {
                    rows,
                    rows_affected: 0,
                })
            } else {
                // This is an INSERT/UPDATE/DELETE/CREATE, use execute()
                match conn.execute(sql, libsql_params).await {
                    Ok(rows_affected) => Ok(QueryResult {
                        rows: vec![],
                        rows_affected,
                    }),
                    Err(e) => {
                        let error_msg = format!("Execute error: {}", e);
                        if Self::is_busy_error(&error_msg) && attempt < max_retries {
                            drop(lock);
                            attempt += 1;
                            tokio::time::sleep(tokio::time::Duration::from_millis(
                                10 * attempt as u64,
                            ))
                            .await;
                            continue;
                        }
                        Err(error_msg)
                    }
                }
            };

            return result;
        }
    }

    fn is_busy_error(error_msg: &str) -> bool {
        error_msg.contains("database is locked") || error_msg.contains("SQLITE_BUSY")
    }

    pub async fn query(
        &self,
        sql: &str,
        params: Vec<serde_json::Value>,
    ) -> Result<QueryResult, String> {
        let lock = self.conn.lock().await;
        let conn = lock.as_ref().ok_or("Database not connected")?;

        // Convert JSON values to libsql Values
        let libsql_params: Vec<libsql::Value> =
            params.iter().map(|v| json_to_libsql_value(v)).collect();

        let mut stmt = conn
            .prepare(sql)
            .await
            .map_err(|e| format!("Prepare error: {}", e))?;

        let mut rows_result = stmt
            .query(libsql_params)
            .await
            .map_err(|e| format!("Query error: {}", e))?;

        let mut rows = Vec::new();

        while let Some(row) = rows_result
            .next()
            .await
            .map_err(|e| format!("Row fetch error: {}", e))?
        {
            let mut row_obj = serde_json::Map::new();

            // Get column count
            let column_count = row.column_count();

            for i in 0..column_count {
                let value = row
                    .get_value(i)
                    .map_err(|e| format!("Get value error: {}", e))?;
                let column_name = row
                    .column_name(i)
                    .unwrap_or(&format!("column_{}", i))
                    .to_string();

                row_obj.insert(column_name, libsql_value_to_json(&value));
            }

            rows.push(serde_json::Value::Object(row_obj));
        }

        Ok(QueryResult {
            rows,
            rows_affected: 0,
        })
    }

    pub async fn batch(
        &self,
        statements: Vec<(String, Vec<serde_json::Value>)>,
    ) -> Result<Vec<QueryResult>, String> {
        let mut results = Vec::new();

        for (sql, params) in statements {
            let result = self.execute(&sql, params).await?;
            results.push(result);
        }

        Ok(results)
    }

    /// Close the database connection gracefully
    /// This should be called when the application exits to release file handles
    #[allow(dead_code)]
    pub async fn close(&self) -> Result<(), String> {
        let lock = self.conn.lock().await;
        if lock.is_some() {
            // Run PRAGMA optimize before closing (SQLite best practice)
            drop(lock);
            let _ = self.execute("PRAGMA optimize", vec![]).await;

            // Now set connection to None to release it
            let mut lock = self.conn.lock().await;
            *lock = None;
            log::info!("Database connection closed successfully");
        }
        Ok(())
    }

    /// Synchronous close for use in Drop or sync contexts
    pub fn close_sync(&self) {
        // Try to acquire lock and clear connection
        // This is a best-effort cleanup in sync context
        if let Ok(rt) = tokio::runtime::Runtime::new() {
            let conn = self.conn.clone();
            rt.block_on(async move {
                let mut lock = conn.lock().await;
                *lock = None;
                log::info!("Database connection closed (sync)");
            });
        }
    }
}

// Convert serde_json::Value to libsql::Value
fn json_to_libsql_value(v: &serde_json::Value) -> libsql::Value {
    match v {
        serde_json::Value::Null => libsql::Value::Null,
        serde_json::Value::Bool(b) => libsql::Value::Integer(if *b { 1 } else { 0 }),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                libsql::Value::Integer(i)
            } else if let Some(f) = n.as_f64() {
                libsql::Value::Real(f)
            } else {
                libsql::Value::Null
            }
        }
        serde_json::Value::String(s) => libsql::Value::Text(s.clone()),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            // Convert complex types to JSON string
            libsql::Value::Text(v.to_string())
        }
    }
}

// Convert libsql::Value to serde_json::Value
fn libsql_value_to_json(v: &libsql::Value) -> serde_json::Value {
    match v {
        libsql::Value::Null => serde_json::Value::Null,
        libsql::Value::Integer(i) => serde_json::Value::Number((*i).into()),
        libsql::Value::Real(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        libsql::Value::Text(s) => serde_json::Value::String(s.clone()),
        libsql::Value::Blob(b) => serde_json::Value::String(base64_encode(b)),
    }
}

fn base64_encode(data: &[u8]) -> String {
    use std::io::Write;
    let mut buf = Vec::new();
    {
        let mut encoder =
            base64::write::EncoderWriter::new(&mut buf, &base64::engine::general_purpose::STANDARD);
        encoder.write_all(data).unwrap();
    }
    String::from_utf8(buf).unwrap()
}

// Type alias for easier migration
pub type Connection = Database;

use sqlx::{SqlitePool, Row};
use chrono::{DateTime, Utc};
use crate::error::Result;
use crate::config::Config;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CityReport {
    pub id: Option<i64>,
    pub xml_file: String,
    pub lmo: String,
    pub city: String,
    pub warning_kind: String,
    pub status: String,
    pub created_at: Option<DateTime<Utc>>,
    pub is_delete: bool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct VPWW54Xml {
    pub id: Option<i64>,
    pub xml_file: String,
    pub lmo: String,
    pub created_at: Option<DateTime<Utc>>,
    pub is_delete: bool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Extra {
    pub id: Option<i64>,
    pub last_modified: String,
    pub created_at: Option<DateTime<Utc>>,
}

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new() -> Result<Self> {
        let config = Config::from_env()?;

        // Create data directory if it doesn't exist
        std::fs::create_dir_all(std::path::Path::new(&config.db_path).parent().unwrap())?;

        let pool = SqlitePool::connect(&format!("sqlite:{}", config.db_path)).await?;
        Ok(Self { pool })
    }

    pub async fn init_schema(&self) -> Result<()> {
        // Create tables if they don't exist
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS extra (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                last_modified TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS vpww54xml (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                xml_file TEXT NOT NULL,
                lmo TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                is_delete BOOLEAN DEFAULT 0
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS city_report (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                xml_file TEXT NOT NULL,
                lmo TEXT NOT NULL,
                city TEXT NOT NULL,
                warning_kind TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                is_delete BOOLEAN DEFAULT 0
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        tracing::info!("Database schema initialized");
        Ok(())
    }

    // Extra table operations
    pub async fn get_extra_last_modified(&self) -> Result<Option<String>> {
        let row = sqlx::query("SELECT last_modified FROM extra ORDER BY id DESC LIMIT 1")
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| r.get("last_modified")))
    }

    pub async fn update_extra(&self, last_modified: &str) -> Result<()> {
        sqlx::query("INSERT INTO extra (last_modified) VALUES (?)")
            .bind(last_modified)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // VPWW54xml table operations
    pub async fn get_vpww54_by_file(&self, xml_file: &str) -> Result<Option<VPWW54Xml>> {
        let record = sqlx::query_as::<_, VPWW54Xml>(
            "SELECT * FROM vpww54xml WHERE xml_file = ? AND is_delete = 0"
        )
        .bind(xml_file)
        .fetch_optional(&self.pool)
        .await?;
        Ok(record)
    }

    pub async fn create_vpww54(&self, xml_file: &str, lmo: &str) -> Result<()> {
        sqlx::query("INSERT INTO vpww54xml (xml_file, lmo) VALUES (?, ?)")
            .bind(xml_file)
            .bind(lmo)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // CityReport table operations
    pub async fn get_city_report(
        &self,
        city: &str,
        warning_kind: &str,
    ) -> Result<Option<CityReport>> {
        let record = sqlx::query_as::<_, CityReport>(
            "SELECT * FROM city_report WHERE city = ? AND warning_kind = ? AND is_delete = 0"
        )
        .bind(city)
        .bind(warning_kind)
        .fetch_optional(&self.pool)
        .await?;
        Ok(record)
    }

    pub async fn create_city_report(&self, report: &CityReport) -> Result<()> {
        sqlx::query(
            "INSERT INTO city_report (xml_file, lmo, city, warning_kind, status) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&report.xml_file)
        .bind(&report.lmo)
        .bind(&report.city)
        .bind(&report.warning_kind)
        .bind(&report.status)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_city_report(&self, id: i64, xml_file: &str, status: &str) -> Result<()> {
        sqlx::query("UPDATE city_report SET xml_file = ?, status = ? WHERE id = ?")
            .bind(xml_file)
            .bind(status)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn soft_delete_city_report(&self, id: i64) -> Result<()> {
        sqlx::query("UPDATE city_report SET is_delete = 1 WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_old_records(&self, days: i64) -> Result<()> {
        // Delete records older than specified days
        sqlx::query(
            "DELETE FROM city_report WHERE is_delete = 1 AND created_at < datetime('now', '-' || ? || ' days')"
        )
        .bind(days)
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "DELETE FROM vpww54xml WHERE is_delete = 1 AND created_at < datetime('now', '-' || ? || ' days')"
        )
        .bind(days)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

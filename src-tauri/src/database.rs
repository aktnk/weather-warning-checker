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

        tracing::debug!("Database path: {}", config.db_path);

        // Create data directory if it doesn't exist
        let db_parent = std::path::Path::new(&config.db_path).parent().unwrap();
        tracing::debug!("Creating parent directory: {:?}", db_parent);
        std::fs::create_dir_all(db_parent)?;

        let db_url = format!("sqlite://{}?mode=rwc", config.db_path);
        tracing::debug!("Connecting to database: {}", db_url);
        let pool = SqlitePool::connect(&db_url).await?;
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
    /// Get city report by lmo, city, and warning_kind
    /// Corresponds to Python's checkCityAndKindDataSameInCityReport()
    pub async fn get_city_report(
        &self,
        lmo: &str,
        city: &str,
        warning_kind: &str,
    ) -> Result<Option<CityReport>> {
        let record = sqlx::query_as::<_, CityReport>(
            "SELECT * FROM city_report WHERE lmo = ? AND city = ? AND warning_kind = ? AND is_delete = 0"
        )
        .bind(lmo)
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

    // ========================================================================
    // New methods to match Python implementation
    // ========================================================================

    /// Add VPWW54xml record if not exists
    /// Corresponds to Python's addVPWW54xml()
    pub async fn add_vpww54_xml(&self, lmo: &str, xml_file: &str) -> Result<()> {
        // Check if already exists
        let exists = sqlx::query(
            "SELECT id FROM vpww54xml WHERE xml_file = ? AND lmo = ? AND is_delete = 0"
        )
        .bind(xml_file)
        .bind(lmo)
        .fetch_optional(&self.pool)
        .await?;

        if exists.is_none() {
            tracing::debug!("Creating VPWW54xml record: {}", xml_file);
            self.create_vpww54(xml_file, lmo).await?;
        }

        Ok(())
    }

    /// Update city report xml_file only (status unchanged)
    /// Corresponds to Python's updateCityReportByXmlfile()
    pub async fn update_city_report_xmlfile(
        &self,
        lmo: &str,
        city: &str,
        warning_kind: &str,
        xml_file: &str,
    ) -> Result<()> {
        tracing::debug!(
            "Updating xmlfile for {} - {} to {}",
            city,
            warning_kind,
            xml_file
        );

        sqlx::query(
            "UPDATE city_report SET xml_file = ? WHERE lmo = ? AND city = ? AND warning_kind = ? AND is_delete = 0"
        )
        .bind(xml_file)
        .bind(lmo)
        .bind(city)
        .bind(warning_kind)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete city reports by status (soft delete all reports for a city)
    /// Corresponds to Python's deleteCityReportByStatus()
    pub async fn delete_city_reports_by_city(&self, lmo: &str, city: &str) -> Result<()> {
        tracing::info!("Deleting all reports for {} - {}", lmo, city);

        sqlx::query(
            "UPDATE city_report SET is_delete = 1 WHERE lmo = ? AND city = ? AND is_delete = 0"
        )
        .bind(lmo)
        .bind(city)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete city reports by LMO (only status='解除')
    /// Corresponds to Python's deleteCityReportByLMO()
    pub async fn delete_city_reports_by_lmo(&self, lmo: &str) -> Result<()> {
        tracing::info!("Deleting cancelled reports for LMO: {}", lmo);

        let rows = sqlx::query(
            "UPDATE city_report SET is_delete = 1 WHERE lmo = ? AND status = '解除' AND is_delete = 0"
        )
        .bind(lmo)
        .execute(&self.pool)
        .await?;

        tracing::debug!("Deleted {} city report records", rows.rows_affected());
        Ok(())
    }

    /// Delete VPWW54xml records by LMO and move XML files
    /// Corresponds to Python's deleteVPWW54xmlByLMO()
    pub async fn delete_vpww54_by_lmo(&self, lmo: &str) -> Result<()> {
        tracing::info!("Deleting VPWW54xml records for LMO: {}", lmo);

        // Get all XML files for this LMO
        let records = sqlx::query_as::<_, VPWW54Xml>(
            "SELECT * FROM vpww54xml WHERE lmo = ? AND is_delete = 0"
        )
        .bind(lmo)
        .fetch_all(&self.pool)
        .await?;

        let config = Config::from_env()?;
        let record_count = records.len();

        for record in records {
            tracing::debug!("Marking XML as deleted: {}", record.xml_file);

            // Soft delete in DB
            sqlx::query("UPDATE vpww54xml SET is_delete = 1 WHERE id = ?")
                .bind(record.id)
                .execute(&self.pool)
                .await?;

            // Move XML file to deleted directory
            let src_path = std::path::Path::new(&config.data_dir).join(&record.xml_file);
            let dst_path = std::path::Path::new(&config.deleted_dir).join(&record.xml_file);

            if src_path.exists() {
                // Create deleted directory if it doesn't exist
                std::fs::create_dir_all(&config.deleted_dir)?;

                if let Err(e) = std::fs::rename(&src_path, &dst_path) {
                    tracing::warn!("Failed to move XML file {}: {}", record.xml_file, e);
                }
            }
        }

        tracing::debug!("Deleted {} VPWW54xml records", record_count);
        Ok(())
    }

    /// Get XML file from the latest city report
    /// Used to check if XML file has changed
    pub async fn get_city_report_xmlfile(
        &self,
        lmo: &str,
        city: &str,
        warning_kind: &str,
    ) -> Result<Option<String>> {
        let row = sqlx::query(
            "SELECT xml_file FROM city_report WHERE lmo = ? AND city = ? AND warning_kind = ? AND is_delete = 0"
        )
        .bind(lmo)
        .bind(city)
        .bind(warning_kind)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.get("xml_file")))
    }
}

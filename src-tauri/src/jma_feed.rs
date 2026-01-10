use reqwest::Client;
use serde::Deserialize;
use crate::error::Result;
use crate::config::Config;
use crate::database::Database;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct JMAFeed {
    client: Client,
    config: Config,
}

#[derive(Debug, Deserialize)]
pub struct WarningData {
    pub city: String,
    pub warning_kind: String,
    pub status: String,
}

impl JMAFeed {
    pub fn new(config: Config) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    /// Fetch the extra.xml feed with If-Modified-Since header
    pub async fn fetch_extra_xml(&self, db: &Database) -> Result<Option<Vec<u8>>> {
        let url = "https://www.data.jma.go.jp/developer/xml/feed/extra.xml";

        let mut request = self.client.get(url);

        // Add If-Modified-Since header if we have a previous Last-Modified value
        if let Some(last_modified) = db.get_extra_last_modified().await? {
            request = request.header("If-Modified-Since", last_modified);
        }

        let response = request.send().await?;

        // If 304 Not Modified, return None
        if response.status() == 304 {
            tracing::debug!("Extra.xml not modified");
            return Ok(None);
        }

        // Store Last-Modified header for next request
        if let Some(last_modified) = response.headers().get("Last-Modified") {
            if let Ok(last_modified_str) = last_modified.to_str() {
                db.update_extra(last_modified_str).await?;
            }
        }

        let content = response.bytes().await?.to_vec();
        Ok(Some(content))
    }

    /// Parse extra.xml to get VPWW54 entries
    pub async fn parse_extra_xml(&self, _xml_content: &[u8]) -> Result<Vec<VPWWEntry>> {
        // TODO: Implement XML parsing to extract VPWW54 entries
        // This will parse the feed and find entries with title containing "気象警報・注意報"
        // and extract the entry link and LMO name

        tracing::warn!("parse_extra_xml not yet implemented");
        Ok(vec![])
    }

    /// Download and parse a VPWW54 XML file
    pub async fn fetch_vpww54(&self, url: &str, filename: &str) -> Result<Vec<WarningData>> {
        // Check if file already exists in cache
        let file_path = PathBuf::from(&self.config.data_dir).join(filename);

        if file_path.exists() {
            tracing::debug!("Using cached VPWW54 file: {}", filename);
            let content = std::fs::read_to_string(&file_path)?;
            return self.parse_vpww54(&content);
        }

        // Download the file
        let response = self.client.get(url).send().await?;
        let content = response.text().await?;

        // Save to cache
        std::fs::create_dir_all(&self.config.data_dir)?;
        std::fs::write(&file_path, &content)?;

        self.parse_vpww54(&content)
    }

    /// Parse VPWW54 XML format
    fn parse_vpww54(&self, _xml_content: &str) -> Result<Vec<WarningData>> {
        // TODO: Implement VPWW54 XML parsing
        // This format contains weather warning information for different cities
        // Need to extract city names, warning types, and status (発表/継続/解除)

        tracing::warn!("parse_vpww54 not yet implemented");
        Ok(vec![])
    }

    /// Get latest VPWW54 entry for a specific LMO (Local Meteorological Observatory)
    pub async fn get_latest_vpww54_for_lmo(
        &self,
        _lmo: &str,
    ) -> Result<Option<Vec<WarningData>>> {
        // TODO: Implement
        // 1. Fetch extra.xml
        // 2. Parse and find entries for the specified LMO
        // 3. Get the latest entry
        // 4. Download and parse the VPWW54 XML

        tracing::warn!("get_latest_vpww54_for_lmo not yet implemented");
        Ok(None)
    }
}

#[derive(Debug)]
pub struct VPWWEntry {
    pub lmo: String,
    pub url: String,
    pub filename: String,
}

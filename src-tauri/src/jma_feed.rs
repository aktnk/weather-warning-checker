use reqwest::Client;
use serde::Deserialize;
use crate::error::Result;
use crate::config::Config;
use crate::database::Database;
use std::path::PathBuf;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct JMAFeed {
    client: Client,
    config: Config,
}

// ============================================================================
// Data structures for extra.xml feed parsing
// ============================================================================

/// Represents a single entry in the extra.xml feed
/// Corresponds to Python's JMAFeedEntryData class
#[derive(Debug, Clone)]
pub struct FeedEntry {
    pub title: String,
    pub id: String,
    pub updated: DateTime<Utc>,
    pub author_name: String,
    pub link: String,
    pub content: String,
}

/// Represents a VPWW54 entry extracted from extra.xml
#[derive(Debug, Clone)]
pub struct VPWWEntry {
    pub lmo: String,
    pub url: String,
    pub filename: String,
    pub updated: DateTime<Utc>,
}

// ============================================================================
// Data structures for VPWW54 XML parsing
// ============================================================================

/// Complete VPWW54 XML data
/// Corresponds to Python's VPWW54XMLData class
#[derive(Debug, Clone)]
pub struct VPWW54Data {
    pub control: VPWW54Control,
    pub head: VPWW54Head,
    pub warnings: Vec<CityWarning>,
}

/// Control section of VPWW54 XML
/// Corresponds to Python's VPWW54Control class
#[derive(Debug, Clone)]
pub struct VPWW54Control {
    pub title: String,
    pub datetime: DateTime<Utc>,
    pub status: String,
    pub publishing_office: String,
}

/// Head section of VPWW54 XML
/// Corresponds to Python's VPWW54Head class
#[derive(Debug, Clone)]
pub struct VPWW54Head {
    pub title: String,
    pub report_datetime: DateTime<Utc>,
    pub info_type: String,
    pub info_kind: String,
}

/// Warning data for a specific city
/// Corresponds to Python's VPWW54BodyWarningTypeCity class
#[derive(Debug, Clone)]
pub struct CityWarning {
    pub area_name: String,
    pub change_status: Option<String>,
    pub kinds: Vec<WarningKind>,
}

/// Individual warning kind and status
#[derive(Debug, Clone)]
pub struct WarningKind {
    pub kind_name: Option<String>,
    pub status: String,
}

// Legacy structure for backward compatibility
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
    /// Saves the file to cache directory for future use
    pub async fn fetch_extra_xml(&self, db: &Database) -> Result<Option<Vec<u8>>> {
        let url = "https://www.data.jma.go.jp/developer/xml/feed/extra.xml";
        let cache_path = PathBuf::from(&self.config.data_dir).join("extra.xml");

        let mut request = self.client.get(url);

        // Add If-Modified-Since header if we have a previous Last-Modified value
        if let Some(last_modified) = db.get_extra_last_modified().await? {
            tracing::debug!("Requesting extra.xml with If-Modified-Since: {}", last_modified);
            request = request.header("If-Modified-Since", last_modified);
        }

        let response = request.send().await?;

        // If 304 Not Modified, return None
        if response.status() == 304 {
            tracing::debug!("Extra.xml not modified (304)");
            return Ok(None);
        }

        // Store Last-Modified header for next request
        if let Some(last_modified) = response.headers().get("Last-Modified") {
            if let Ok(last_modified_str) = last_modified.to_str() {
                db.update_extra(last_modified_str).await?;
                tracing::debug!("Updated Last-Modified: {}", last_modified_str);
            }
        }

        let content = response.bytes().await?.to_vec();

        // Save to cache
        std::fs::create_dir_all(&self.config.data_dir)?;
        std::fs::write(&cache_path, &content)?;
        tracing::debug!("Saved extra.xml to cache");

        Ok(Some(content))
    }

    /// Parse extra.xml to get VPWW54 entries
    /// Filters entries by title "気象警報・注意報(H27)" and extracts LMO information
    pub async fn parse_extra_xml(&self, xml_content: &[u8]) -> Result<Vec<VPWWEntry>> {
        use quick_xml::events::Event;
        use quick_xml::Reader;

        const VPWW54_TITLE: &str = "気象警報・注意報（Ｈ２７）";

        let mut reader = Reader::from_reader(xml_content);
        reader.config_mut().trim_text(true);

        let mut entries = Vec::new();
        let mut current_entry: Option<FeedEntry> = None;
        let mut current_tag = String::new();
        let mut current_text = String::new();
        let mut in_author = false;

        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    current_tag = tag_name.clone();

                    if tag_name == "entry" {
                        current_entry = Some(FeedEntry {
                            title: String::new(),
                            id: String::new(),
                            updated: Utc::now(),
                            author_name: String::new(),
                            link: String::new(),
                            content: String::new(),
                        });
                    } else if tag_name == "author" {
                        in_author = true;
                    } else if tag_name == "link" {
                        // Extract href attribute from <link> tag (can be empty element)
                        if let Some(entry) = current_entry.as_mut() {
                            for attr in e.attributes() {
                                if let Ok(attr) = attr {
                                    if attr.key.as_ref() == b"href" {
                                        entry.link = String::from_utf8_lossy(&attr.value).to_string();
                                    }
                                }
                            }
                        }
                    }
                    current_text.clear();
                }
                Ok(Event::End(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    if tag_name == "entry" {
                        // Entry completed, check if it's VPWW54 and add to list
                        if let Some(entry) = current_entry.take() {
                            if entry.title.contains(VPWW54_TITLE) {
                                // Extract filename from URL
                                let filename = entry.link.split('/').last()
                                    .unwrap_or("unknown.xml")
                                    .to_string();

                                tracing::debug!(
                                    "Found VPWW54 entry: LMO={}, URL={}, filename={}",
                                    entry.author_name,
                                    entry.link,
                                    filename
                                );

                                let vpww_entry = VPWWEntry {
                                    lmo: entry.author_name.clone(),
                                    url: entry.link.clone(),
                                    filename,
                                    updated: entry.updated,
                                };
                                entries.push(vpww_entry);
                            }
                        }
                    } else if tag_name == "author" {
                        in_author = false;
                    }
                    current_tag.clear();
                }
                Ok(Event::Text(e)) => {
                    current_text = e.unescape().unwrap_or_default().to_string();

                    if let Some(entry) = current_entry.as_mut() {
                        match current_tag.as_str() {
                            "title" => entry.title = current_text.clone(),
                            "id" => entry.id = current_text.clone(),
                            "updated" => {
                                // Parse ISO 8601 datetime
                                if let Ok(dt) = DateTime::parse_from_rfc3339(&current_text) {
                                    entry.updated = dt.with_timezone(&Utc);
                                }
                            }
                            "name" if in_author => entry.author_name = current_text.clone(),
                            _ => {}
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    tracing::error!("Error parsing extra.xml at position {}: {:?}", reader.buffer_position(), e);
                    return Err(crate::error::WeatherCheckerError::XmlParse(
                        format!("XML parse error: {}", e)
                    ));
                }
                _ => {}
            }
            buf.clear();
        }

        // Sort by updated time (newest first)
        entries.sort_by(|a, b| b.updated.cmp(&a.updated));

        tracing::debug!("Parsed {} VPWW54 entries from extra.xml", entries.len());
        Ok(entries)
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
    /// Extracts warning information from the JMA VPWW54 format
    fn parse_vpww54(&self, xml_content: &str) -> Result<Vec<WarningData>> {
        use quick_xml::events::Event;
        use quick_xml::Reader;

        let mut reader = Reader::from_str(xml_content);
        reader.config_mut().trim_text(true);

        let mut vpww54_data: Option<VPWW54Data> = None;
        let mut control: Option<VPWW54Control> = None;
        let mut head: Option<VPWW54Head> = None;
        let mut warnings: Vec<CityWarning> = Vec::new();

        let mut current_city_warning: Option<CityWarning> = None;
        let mut current_path = Vec::new();
        let mut current_text = String::new();

        // Track current context
        let mut in_control = false;
        let mut in_head = false;
        let mut in_warning_type_city = false;
        let mut in_item = false;
        let mut in_kind = false;

        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    current_path.push(tag_name.clone());

                    match tag_name.as_str() {
                        "Control" => {
                            in_control = true;
                            control = Some(VPWW54Control {
                                title: String::new(),
                                datetime: Utc::now(),
                                status: String::new(),
                                publishing_office: String::new(),
                            });
                        }
                        "Head" => in_head = true,
                        "Warning" | "Information" => {
                            // Check if it's the city-level warning type
                            for attr in e.attributes() {
                                if let Ok(attr) = attr {
                                    if attr.key.as_ref() == b"type" {
                                        let type_val = String::from_utf8_lossy(&attr.value);
                                        if type_val == "気象警報・注意報（市町村等）" {
                                            in_warning_type_city = true;
                                        }
                                    }
                                }
                            }
                        }
                        "Item" if in_warning_type_city => {
                            in_item = true;
                            current_city_warning = Some(CityWarning {
                                area_name: String::new(),
                                change_status: None,
                                kinds: Vec::new(),
                            });
                        }
                        "Kind" if in_item => {
                            in_kind = true;
                        }
                        _ => {}
                    }
                    current_text.clear();
                }
                Ok(Event::End(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    match tag_name.as_str() {
                        "Control" => in_control = false,
                        "Head" => in_head = false,
                        "Warning" | "Information" => in_warning_type_city = false,
                        "Item" if in_item => {
                            in_item = false;
                            if let Some(cw) = current_city_warning.take() {
                                warnings.push(cw);
                            }
                        }
                        "Kind" => in_kind = false,
                        _ => {}
                    }

                    current_path.pop();
                }
                Ok(Event::Text(e)) => {
                    current_text = e.unescape().unwrap_or_default().to_string();

                    // Parse based on current context
                    if in_control {
                        if let Some(ref mut ctrl) = control {
                            let parent = current_path.get(current_path.len() - 1).map(|s| s.as_str());
                            match parent {
                                Some("Title") => ctrl.title = current_text.clone(),
                                Some("DateTime") => {
                                    if let Ok(dt) = DateTime::parse_from_rfc3339(&current_text) {
                                        ctrl.datetime = dt.with_timezone(&Utc);
                                    }
                                }
                                Some("Status") => ctrl.status = current_text.clone(),
                                Some("PublishingOffice") => ctrl.publishing_office = current_text.clone(),
                                _ => {}
                            }
                        }
                    } else if in_head {
                        if head.is_none() {
                            head = Some(VPWW54Head {
                                title: String::new(),
                                report_datetime: Utc::now(),
                                info_type: String::new(),
                                info_kind: String::new(),
                            });
                        }
                        if let Some(ref mut h) = head {
                            let parent = current_path.get(current_path.len() - 1).map(|s| s.as_str());
                            match parent {
                                Some("Title") => h.title = current_text.clone(),
                                Some("ReportDateTime") => {
                                    // Handle both formats: with +09:00 or Z
                                    let normalized = current_text.replace("+09:00", "+0900");
                                    if let Ok(dt) = DateTime::parse_from_rfc3339(&normalized) {
                                        h.report_datetime = dt.with_timezone(&Utc);
                                    } else if let Ok(dt) = DateTime::parse_from_rfc3339(&current_text) {
                                        h.report_datetime = dt.with_timezone(&Utc);
                                    }
                                }
                                Some("InfoType") => h.info_type = current_text.clone(),
                                Some("InfoKind") => h.info_kind = current_text.clone(),
                                _ => {}
                            }
                        }
                    } else if in_item {
                        if let Some(ref mut cw) = current_city_warning {
                            let parent = current_path.get(current_path.len() - 1).map(|s| s.as_str());
                            match parent {
                                Some("Name") if current_path.contains(&"Area".to_string()) => {
                                    cw.area_name = current_text.clone();
                                }
                                Some("ChangeStatus") => {
                                    cw.change_status = Some(current_text.clone());
                                }
                                Some("Name") if in_kind => {
                                    // Add kind with name
                                    cw.kinds.push(WarningKind {
                                        kind_name: Some(current_text.clone()),
                                        status: String::new(),
                                    });
                                }
                                Some("Status") if in_kind => {
                                    // Update status of last kind
                                    if let Some(last_kind) = cw.kinds.last_mut() {
                                        last_kind.status = current_text.clone();
                                    } else {
                                        // Status without name (解除 case)
                                        cw.kinds.push(WarningKind {
                                            kind_name: None,
                                            status: current_text.clone(),
                                        });
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    tracing::error!("Error parsing VPWW54 XML: {:?}", e);
                    return Err(crate::error::WeatherCheckerError::XmlParse(
                        format!("VPWW54 parse error: {}", e)
                    ));
                }
                _ => {}
            }
            buf.clear();
        }

        // Build the complete VPWW54Data structure
        if let (Some(ctrl), Some(hd)) = (control, head) {
            vpww54_data = Some(VPWW54Data {
                control: ctrl,
                head: hd,
                warnings,
            });
        }

        // Convert to legacy WarningData format for backward compatibility
        let mut result = Vec::new();
        if let Some(data) = vpww54_data {
            for warning in data.warnings {
                if warning.kinds.is_empty() {
                    // No kinds means "発表警報・注意報はなし"
                    result.push(WarningData {
                        city: warning.area_name.clone(),
                        warning_kind: String::new(),
                        status: "発表警報・注意報はなし".to_string(),
                    });
                } else {
                    for kind in warning.kinds {
                        if let Some(kind_name) = kind.kind_name {
                            result.push(WarningData {
                                city: warning.area_name.clone(),
                                warning_kind: kind_name,
                                status: kind.status,
                            });
                        } else if kind.status == "発表警報・注意報はなし" {
                            // Handle explicit "no warnings" status
                            result.push(WarningData {
                                city: warning.area_name.clone(),
                                warning_kind: String::new(),
                                status: kind.status,
                            });
                        }
                    }
                }
            }
        }

        tracing::debug!("Parsed {} warnings from VPWW54 XML", result.len());
        Ok(result)
    }

    /// Get latest VPWW54 entry for a specific LMO (Local Meteorological Observatory)
    /// This is the main entry point that orchestrates the entire workflow:
    /// 1. Fetch extra.xml with conditional request (If-Modified-Since)
    /// 2. Parse extra.xml and filter entries by LMO
    /// 3. Get the latest entry for the specified LMO
    /// 4. Download and parse the VPWW54 XML
    /// Returns: Option<(warnings, xml_filename)>
    pub async fn get_latest_vpww54_for_lmo(
        &self,
        lmo: &str,
        db: &Database,
    ) -> Result<Option<(Vec<WarningData>, String)>> {
        tracing::info!("Fetching latest VPWW54 for LMO: {}", lmo);

        // Step 1: Fetch extra.xml with conditional request
        let xml_content = match self.fetch_extra_xml(db).await? {
            Some(content) => content,
            None => {
                // 304 Not Modified - read from cache
                let cache_path = PathBuf::from(&self.config.data_dir).join("extra.xml");
                if cache_path.exists() {
                    std::fs::read(&cache_path)?
                } else {
                    tracing::warn!("No extra.xml available (not modified and no cache)");
                    return Ok(None);
                }
            }
        };

        // Step 2: Parse extra.xml
        let vpww_entries = self.parse_extra_xml(&xml_content).await?;

        // Step 3: Filter by LMO and get the latest entry
        let lmo_entries: Vec<_> = vpww_entries
            .into_iter()
            .filter(|entry| entry.lmo == lmo)
            .collect();

        if lmo_entries.is_empty() {
            tracing::info!("No VPWW54 entries found for LMO: {}", lmo);
            return Ok(None);
        }

        // Entries are already sorted by updated time (newest first)
        let latest_entry = &lmo_entries[0];
        tracing::info!(
            "Found latest VPWW54 for {}: {} (updated: {})",
            lmo,
            latest_entry.filename,
            latest_entry.updated
        );

        // Step 4: Download and parse VPWW54 XML
        let warnings = self.fetch_vpww54(&latest_entry.url, &latest_entry.filename).await?;

        tracing::info!("Successfully retrieved {} warnings for {}", warnings.len(), lmo);

        // Return warnings and XML filename (filename will be recorded in DB by caller)
        Ok(Some((warnings, latest_entry.filename.clone())))
    }
}

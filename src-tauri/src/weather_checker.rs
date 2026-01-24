use crate::config::Config;
use crate::database::{Database, CityReport};
use crate::jma_feed::JMAFeed;
use crate::notification::EmailNotifier;
use crate::error::Result;

pub struct WeatherChecker {
    config: Config,
    db: Database,
    jma_feed: JMAFeed,
    notifier: EmailNotifier,
}

impl WeatherChecker {
    pub async fn new() -> Result<Self> {
        let config = Config::from_env()?;
        let db = Database::new().await?;
        let jma_feed = JMAFeed::new(config.clone());
        let notifier = EmailNotifier::new(config.clone());

        Ok(Self {
            config,
            db,
            jma_feed,
            notifier,
        })
    }

    pub async fn run_check(&self) -> Result<()> {
        tracing::info!("Starting weather check...");

        // Monitored regions (matching Python version)
        self.check_warnings("静岡地方気象台", &["裾野市", "御殿場市"])
            .await?;

        tracing::info!("Weather check completed");
        Ok(())
    }

    async fn check_warnings(&self, lmo: &str, cities: &[&str]) -> Result<()> {
        tracing::debug!("Checking warnings for {} - {:?}", lmo, cities);

        // Get latest VPWW54 data for this LMO
        let warnings_opt = self.jma_feed.get_latest_vpww54_for_lmo(lmo, &self.db).await?;

        let Some((warnings, xml_filename)) = warnings_opt else {
            // No entry in extra.xml for this LMO
            // Delete cancelled warnings and associated XML records
            tracing::info!("No entry in extra.xml for {}, cleaning up old data", lmo);
            self.db.delete_city_reports_by_lmo(lmo).await?;
            self.db.delete_vpww54_by_lmo(lmo).await?;
            return Ok(());
        };

        // Check if there are any warnings at all
        if warnings.is_empty() {
            tracing::debug!("No warnings in XML for {}", lmo);
            return Ok(());
        }

        // Process each warning
        for warning in warnings {
            // Filter for specified cities
            if !cities.contains(&warning.city.as_str()) {
                continue;
            }

            // Check for "no warnings" status
            if warning.warning_kind.is_empty() && warning.status == "発表警報・注意報はなし" {
                tracing::info!(
                    "No active warnings for {} - {}, deleting old reports",
                    lmo,
                    warning.city
                );
                self.db.delete_city_reports_by_city(lmo, &warning.city).await?;
                continue;
            }

            // Skip entries with empty warning_kind (but not the special "no warnings" case)
            if warning.warning_kind.is_empty() {
                continue;
            }

            self.process_warning(lmo, &warning.city, &warning.warning_kind, &warning.status, &xml_filename)
                .await?;
        }

        Ok(())
    }

    async fn process_warning(
        &self,
        lmo: &str,
        city: &str,
        warning_kind: &str,
        new_status: &str,
        xml_filename: &str,
    ) -> Result<()> {
        // Check if we already have a record for this lmo+city+warning combination
        let existing = self.db.get_city_report(lmo, city, warning_kind).await?;

        match existing {
            Some(record) => {
                // Compare status
                if record.status != new_status {
                    // Status changed - send notification and update DB
                    tracing::info!(
                        "Warning status changed for {} - {}: {} -> {}",
                        city,
                        warning_kind,
                        record.status,
                        new_status
                    );

                    self.notifier
                        .send_warning_notification(city, warning_kind, new_status, lmo)
                        .await?;

                    // Update record with new status and xml_file
                    self.db
                        .update_city_report(record.id.unwrap(), xml_filename, new_status)
                        .await?;

                    // Add to VPWW54xml table if XML file changed
                    if record.xml_file != xml_filename {
                        self.db.add_vpww54_xml(lmo, xml_filename).await?;
                    }
                } else if record.xml_file != xml_filename {
                    // Status same but XML file changed - update DB without notification
                    tracing::debug!(
                        "XML file changed for {} - {} (status unchanged: {})",
                        city,
                        warning_kind,
                        new_status
                    );

                    self.db
                        .update_city_report_xmlfile(lmo, city, warning_kind, xml_filename)
                        .await?;

                    self.db.add_vpww54_xml(lmo, xml_filename).await?;
                } else {
                    // Everything is the same - already published
                    tracing::debug!(
                        "No changes for {} - {}: {} (already published)",
                        city,
                        warning_kind,
                        new_status
                    );
                }
            }
            None => {
                // New warning - send notification and create record
                tracing::info!("New warning for {} - {}: {}", city, warning_kind, new_status);

                self.notifier
                    .send_warning_notification(city, warning_kind, new_status, lmo)
                    .await?;

                let report = CityReport {
                    id: None,
                    xml_file: xml_filename.to_string(),
                    lmo: lmo.to_string(),
                    city: city.to_string(),
                    warning_kind: warning_kind.to_string(),
                    status: new_status.to_string(),
                    created_at: None,
                    is_delete: false,
                };

                self.db.create_city_report(&report).await?;
                self.db.add_vpww54_xml(lmo, xml_filename).await?;
            }
        }

        Ok(())
    }
}

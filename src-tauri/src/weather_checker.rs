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

        // TODO: Configure monitored regions
        // For now, these are hardcoded to match the Python version
        // Temporarily testing with Fukushima which has active warnings
        self.check_warnings("福島地方気象台", &["会津若松市", "郡山市"])
            .await?;
        self.check_warnings("静岡地方気象台", &["裾野市", "御殿場市"])
            .await?;
        self.check_warnings("東京管区気象台", &["千代田区"])
            .await?;

        tracing::info!("Weather check completed");
        Ok(())
    }

    async fn check_warnings(&self, lmo: &str, cities: &[&str]) -> Result<()> {
        tracing::debug!("Checking warnings for {} - {:?}", lmo, cities);

        // Get latest VPWW54 data for this LMO
        let warnings_opt = self.jma_feed.get_latest_vpww54_for_lmo(lmo, &self.db).await?;

        let Some(warnings) = warnings_opt else {
            tracing::debug!("No new warnings for {}", lmo);
            return Ok(());
        };

        // Filter warnings for specified cities
        for warning in warnings {
            if !cities.contains(&warning.city.as_str()) {
                continue;
            }

            self.process_warning(lmo, &warning.city, &warning.warning_kind, &warning.status)
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
    ) -> Result<()> {
        // Check if we already have a record for this city+warning combination
        let existing = self.db.get_city_report(city, warning_kind).await?;

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

                    // Update record
                    self.db
                        .update_city_report(record.id.unwrap(), "current.xml", new_status)
                        .await?;
                } else {
                    tracing::debug!(
                        "No status change for {} - {}: {}",
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
                    xml_file: "current.xml".to_string(),
                    lmo: lmo.to_string(),
                    city: city.to_string(),
                    warning_kind: warning_kind.to_string(),
                    status: new_status.to_string(),
                    created_at: None,
                    is_delete: false,
                };

                self.db.create_city_report(&report).await?;
            }
        }

        Ok(())
    }
}

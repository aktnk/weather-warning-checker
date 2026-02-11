use crate::config::Config;
use crate::error::Result;
use chrono::{DateTime, FixedOffset, Utc};
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use std::env;

pub struct EmailNotifier {
    config: Config,
}

const DEFAULT_URL: &str = "https://www.jma.go.jp/bosai/warning/#lang=ja";

impl EmailNotifier {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub async fn send_warning_notification(
        &self,
        city: &str,
        warning_kind: &str,
        status: &str,
        lmo: &str,
        jma_url: Option<&str>,
        control_datetime: &DateTime<Utc>,
    ) -> Result<()> {
        // Subject format: {city}:{warning}:{status}
        // Add "test:" prefix when RUST_LOG contains "debug"
        let base_subject = format!("{}:{}:{}", city, warning_kind, status);
        let subject = if env::var("RUST_LOG")
            .map(|v| v.contains("debug"))
            .unwrap_or(false)
        {
            format!("test:{}", base_subject)
        } else {
            base_subject
        };

        // Convert control datetime (UTC) to JST for display, matching Python implementation
        let jst = FixedOffset::east_opt(9 * 3600).unwrap();
        let jst_datetime = control_datetime.with_timezone(&jst);
        let timestamp = jst_datetime.format("%Y/%m/%d %H:%M:%S").to_string();

        // Get JMA URL for the city (use config URL or fall back to default)
        let resolved_url = jma_url.unwrap_or(DEFAULT_URL);
        let city_name = if jma_url.is_some() { city } else { "全国" };

        // Body format matching Python implementation:
        // LWO:{obs}
        // DATE:{dts}
        // CITY:{city}
        // WARN:{warning}
        // STAT:{status}
        // LINK:気象庁｜{city名}の警報・注意報
        // URL:{url}
        // END
        let body = format!(
            "LWO:{}\nDATE:{}\nCITY:{}\nWARN:{}\nSTAT:{}\nLINK:気象庁｜{}の警報・注意報\nURL:{}\nEND",
            lmo, timestamp, city, warning_kind, status, city_name, resolved_url
        );

        let mut email_builder = Message::builder()
            .from(self.config.gmail_from.parse()?)
            .to(self.config.email_to.parse()?)
            .subject(subject);

        if let Some(bcc) = &self.config.email_bcc {
            email_builder = email_builder.bcc(bcc.parse()?);
        }

        let email = email_builder.header(ContentType::TEXT_PLAIN).body(body)?;

        let creds = Credentials::new(
            self.config.gmail_from.clone(),
            self.config.gmail_app_pass.clone(),
        );

        let mailer = SmtpTransport::relay("smtp.gmail.com")?
            .credentials(creds)
            .build();

        mailer.send(&email)?;

        tracing::info!(
            "Sent notification for {} - {} ({})",
            city,
            warning_kind,
            status
        );

        Ok(())
    }

    pub async fn send_system_notification(&self, event: &str, details: &str) -> Result<()> {
        let base_subject = format!("weather-checker: {}", event);
        let subject = if env::var("RUST_LOG")
            .map(|v| v.contains("debug"))
            .unwrap_or(false)
        {
            format!("test:{}", base_subject)
        } else {
            base_subject
        };

        let jst = FixedOffset::east_opt(9 * 3600).unwrap();
        let jst_now = Utc::now().with_timezone(&jst);
        let timestamp = jst_now.format("%Y/%m/%d %H:%M:%S").to_string();

        let body = format!(
            "EVENT:{}\nDATE:{}\nDETAILS:{}\nEND",
            event, timestamp, details
        );

        let mut email_builder = Message::builder()
            .from(self.config.gmail_from.parse()?)
            .to(self.config.email_to.parse()?)
            .subject(subject);

        if let Some(bcc) = &self.config.email_bcc {
            email_builder = email_builder.bcc(bcc.parse()?);
        }

        let email = email_builder.header(ContentType::TEXT_PLAIN).body(body)?;

        let creds = Credentials::new(
            self.config.gmail_from.clone(),
            self.config.gmail_app_pass.clone(),
        );

        let mailer = SmtpTransport::relay("smtp.gmail.com")?
            .credentials(creds)
            .build();

        mailer.send(&email)?;

        tracing::info!("Sent system notification: {}", event);

        Ok(())
    }

}

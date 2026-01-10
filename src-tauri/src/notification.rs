use lettre::{Message, SmtpTransport, Transport};
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use crate::config::Config;
use crate::error::Result;

pub struct EmailNotifier {
    config: Config,
}

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
    ) -> Result<()> {
        let subject = format!("[気象警報] {} - {}", city, warning_kind);

        // Create JMA URL for the city
        let jma_url = self.create_jma_url(lmo, city);

        let body = format!(
            "【{}】\n\n種別: {}\n状態: {}\n\n詳細: {}",
            city, warning_kind, status, jma_url
        );

        let mut email_builder = Message::builder()
            .from(self.config.gmail_from.parse()?)
            .to(self.config.email_to.parse()?)
            .subject(subject);

        if let Some(bcc) = &self.config.email_bcc {
            email_builder = email_builder.bcc(bcc.parse()?);
        }

        let email = email_builder
            .header(ContentType::TEXT_PLAIN)
            .body(body)?;

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

    fn create_jma_url(&self, lmo: &str, city: &str) -> String {
        // TODO: Implement proper URL construction based on LMO and city
        // For now, return general JMA warnings page
        format!(
            "https://www.jma.go.jp/bosai/warning/#area_type=class20s&area_code={}&lang=ja",
            self.get_area_code(lmo, city)
        )
    }

    fn get_area_code(&self, _lmo: &str, _city: &str) -> &str {
        // TODO: Implement area code mapping
        "unknown"
    }
}

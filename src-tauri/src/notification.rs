use std::env;
use lettre::{Message, SmtpTransport, Transport};
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use chrono::{DateTime, Local};
use crate::config::Config;
use crate::error::Result;

pub struct EmailNotifier {
    config: Config,
}

/// City to JMA URL mapping
/// Corresponds to Python's JMAWeb.py
struct CityUrlMapping {
    city: &'static str,
    url: &'static str,
}

const CITY_URL_MAPPINGS: &[CityUrlMapping] = &[
    CityUrlMapping {
        city: "裾野市",
        url: "https://www.jma.go.jp/bosai/warning/#lang=ja&area_type=class20s&area_code=2222000",
    },
    CityUrlMapping {
        city: "御殿場市",
        url: "https://www.jma.go.jp/bosai/warning/#lang=ja&area_type=class20s&area_code=2221500",
    },
    CityUrlMapping {
        city: "三島市",
        url: "https://www.jma.go.jp/bosai/warning/#lang=ja&area_type=class20s&area_code=2220600",
    },
    CityUrlMapping {
        city: "熱海市",
        url: "https://www.jma.go.jp/bosai/warning/#lang=ja&area_type=class20s&area_code=2220500",
    },
    CityUrlMapping {
        city: "都城市",
        url: "https://www.jma.go.jp/bosai/warning/#lang=ja&area_type=class20s&area_code=4520200",
    },
    CityUrlMapping {
        city: "つがる市",
        url: "https://www.jma.go.jp/bosai/warning/#lang=ja&area_type=class20s&area_code=0220900",
    },
];

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

        // Get current timestamp
        let now: DateTime<Local> = Local::now();
        let timestamp = now.format("%Y/%m/%d %H:%M:%S").to_string();

        // Get JMA URL for the city
        let (city_name, jma_url) = self.get_jma_link(city);

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
            lmo, timestamp, city, warning_kind, status, city_name, jma_url
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

    /// Get JMA URL link for a city
    /// Returns (city_name, url) tuple
    /// Corresponds to Python's JMAWebURLs.getLink()
    fn get_jma_link(&self, city: &str) -> (&str, &str) {
        for mapping in CITY_URL_MAPPINGS {
            if mapping.city == city {
                return (mapping.city, mapping.url);
            }
        }
        // Default to national page if city not found
        ("全国", DEFAULT_URL)
    }
}

use std::net::IpAddr;

use reqwest::{Client, redirect::Policy};
use serde::Serialize;
use url::{Host, Url};

use crate::{config::Config, error::VelesError, rate_limit::RateLimiter};

#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct FetchedPage {
    pub url: String,
    pub final_url: String,
    pub status: u16,
    pub content_type: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct Fetcher {
    client: Client,
    max_page_bytes: u64,
}

impl Fetcher {
    pub fn new(config: &Config) -> Result<Self, VelesError> {
        let client = Client::builder()
            .user_agent(config.user_agent.clone())
            .timeout(config.request_timeout)
            .redirect(Policy::limited(5))
            .build()?;

        Ok(Self {
            client,
            max_page_bytes: config.max_page_bytes,
        })
    }

    pub async fn fetch(
        &self,
        url: &str,
        rate_limiter: &RateLimiter,
    ) -> Result<FetchedPage, VelesError> {
        let parsed = validate_public_http_url(url)?;
        rate_limiter.wait().await;

        let response = self.client.get(parsed.clone()).send().await?;
        let status = response.status();
        let final_url = response.url().to_string();
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);

        if !status.is_success() {
            return Err(VelesError::HttpStatus {
                url: final_url,
                status: status.as_u16(),
            });
        }

        if let Some(size) = response.content_length()
            && size > self.max_page_bytes
        {
            return Err(VelesError::ResponseTooLarge {
                size,
                limit: self.max_page_bytes,
            });
        }

        let bytes = response.bytes().await?;
        if bytes.len() as u64 > self.max_page_bytes {
            return Err(VelesError::ResponseTooLarge {
                size: bytes.len() as u64,
                limit: self.max_page_bytes,
            });
        }

        let text = String::from_utf8_lossy(&bytes).into_owned();

        Ok(FetchedPage {
            url: parsed.to_string(),
            final_url,
            status: status.as_u16(),
            content_type,
            text,
        })
    }
}

pub fn validate_public_http_url(value: &str) -> Result<Url, VelesError> {
    let url = Url::parse(value).map_err(|err| VelesError::InvalidUrl(err.to_string()))?;

    match url.scheme() {
        "http" | "https" => {}
        scheme => {
            return Err(VelesError::BlockedUrl(format!(
                "unsupported URL scheme: {scheme}"
            )));
        }
    }

    let host = url
        .host()
        .ok_or_else(|| VelesError::InvalidUrl("URL must include a host".into()))?;

    match host {
        Host::Domain(domain) => {
            let domain = domain.to_ascii_lowercase();
            if domain == "localhost" || domain.ends_with(".localhost") {
                return Err(VelesError::BlockedUrl(
                    "localhost domains are blocked by default".into(),
                ));
            }
        }
        Host::Ipv4(addr) => block_private_ip(IpAddr::V4(addr))?,
        Host::Ipv6(addr) => block_private_ip(IpAddr::V6(addr))?,
    }

    Ok(url)
}

fn block_private_ip(addr: IpAddr) -> Result<(), VelesError> {
    let blocked = match addr {
        IpAddr::V4(addr) => {
            addr.is_private()
                || addr.is_loopback()
                || addr.is_link_local()
                || addr.is_broadcast()
                || addr.is_documentation()
                || addr.is_unspecified()
        }
        IpAddr::V6(addr) => {
            addr.is_loopback()
                || addr.is_unspecified()
                || addr.is_unique_local()
                || addr.is_unicast_link_local()
        }
    };

    if blocked {
        return Err(VelesError::BlockedUrl(format!(
            "private or local IP address is blocked: {addr}"
        )));
    }

    Ok(())
}

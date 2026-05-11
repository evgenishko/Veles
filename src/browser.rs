use std::{
    net::TcpListener,
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use fantoccini::{Client, ClientBuilder, wd::Capabilities};
use serde_json::{Map, Value, json};
use tokio::{process::Command, sync::Mutex, time::sleep};

use crate::{
    config::BrowserConfig,
    error::VelesError,
    fetch::{FetchedPage, validate_public_http_url},
    rate_limit::RateLimiter,
};

#[derive(Debug, Clone)]
pub struct BrowserRenderer {
    config: BrowserConfig,
    permission_granted: Arc<AtomicBool>,
    session_lock: Arc<Mutex<()>>,
}

#[derive(Debug, Clone, Copy)]
pub struct BrowserRenderOptions {
    pub allow_browser: bool,
    pub headless: Option<bool>,
    pub settle: Option<Duration>,
}

impl BrowserRenderer {
    pub fn new(config: BrowserConfig) -> Self {
        Self {
            config,
            permission_granted: Arc::new(AtomicBool::new(false)),
            session_lock: Arc::new(Mutex::new(())),
        }
    }

    pub async fn render(
        &self,
        url: &str,
        rate_limiter: &RateLimiter,
        options: BrowserRenderOptions,
    ) -> Result<FetchedPage, VelesError> {
        if !self.config.enabled {
            return Err(VelesError::BrowserDisabled);
        }

        if options.allow_browser {
            self.permission_granted.store(true, Ordering::SeqCst);
        } else if !self.permission_granted.load(Ordering::SeqCst) {
            return Err(VelesError::BrowserPermissionRequired);
        }

        let parsed = validate_public_http_url(url)?;
        rate_limiter.wait().await;

        let _guard = self.session_lock.lock().await;
        let headless = options.headless.unwrap_or(self.config.headless);
        let settle = options.settle.unwrap_or(self.config.settle);

        self.render_once(parsed.as_str(), headless, settle).await
    }

    async fn render_once(
        &self,
        url: &str,
        headless: bool,
        settle: Duration,
    ) -> Result<FetchedPage, VelesError> {
        let port = reserve_local_port()?;
        let webdriver_url = format!("http://127.0.0.1:{port}");
        let mut child = Command::new(&self.config.driver)
            .arg("--port")
            .arg(port.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|err| {
                VelesError::Browser(format!("failed to start {}: {err}", self.config.driver))
            })?;

        let mut client = None;
        let result = async {
            wait_for_webdriver(&webdriver_url, self.config.page_timeout).await?;

            let connected = connect_firefox(
                &webdriver_url,
                self.config.firefox_binary.as_deref(),
                headless,
                self.config.page_timeout,
            )
            .await?;
            client = Some(connected);
            let active = client.as_ref().expect("client was just set");

            tokio::time::timeout(self.config.page_timeout, active.goto(url))
                .await
                .map_err(|_| VelesError::Browser("page load timed out".into()))?
                .map_err(|err| VelesError::Browser(err.to_string()))?;

            sleep(settle).await;

            let final_url = active
                .current_url()
                .await
                .map_err(|err| VelesError::Browser(err.to_string()))?
                .to_string();
            let html = rendered_outer_html(active).await?;

            Ok(FetchedPage {
                url: url.to_owned(),
                final_url,
                status: 200,
                content_type: Some("text/html; rendered=firefox".into()),
                text: html,
            })
        }
        .await;

        if let Some(active) = client.take() {
            let _ = active.close().await;
        }
        let _ = child.kill().await;
        let _ = child.wait().await;

        result
    }
}

async fn rendered_outer_html(client: &Client) -> Result<String, VelesError> {
    let value = client
        .execute(
            r#"
            document.querySelectorAll('script,style,noscript,template').forEach((node) => node.remove());
            return document.documentElement.outerHTML;
            "#,
            Vec::new(),
        )
        .await
        .map_err(|err| VelesError::Browser(err.to_string()))?;

    value
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| VelesError::Browser("browser did not return rendered HTML".into()))
}

fn reserve_local_port() -> Result<u16, VelesError> {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .map_err(|err| VelesError::Browser(format!("failed to reserve local port: {err}")))?;
    let port = listener
        .local_addr()
        .map_err(|err| VelesError::Browser(err.to_string()))?
        .port();
    drop(listener);
    Ok(port)
}

async fn wait_for_webdriver(url: &str, timeout: Duration) -> Result<(), VelesError> {
    let status_url = format!("{url}/status");
    let client = reqwest::Client::new();
    let started = std::time::Instant::now();

    while started.elapsed() < timeout {
        if client
            .get(&status_url)
            .send()
            .await
            .is_ok_and(|response| response.status().is_success())
        {
            return Ok(());
        }
        sleep(Duration::from_millis(100)).await;
    }

    Err(VelesError::Browser(
        "geckodriver did not become ready".into(),
    ))
}

async fn connect_firefox(
    webdriver_url: &str,
    firefox_binary: Option<&str>,
    headless: bool,
    page_timeout: Duration,
) -> Result<Client, VelesError> {
    let mut caps = Capabilities::new();
    caps.insert("browserName".into(), Value::String("firefox".into()));
    caps.insert("acceptInsecureCerts".into(), Value::Bool(false));
    caps.insert(
        "timeouts".into(),
        json!({
            "pageLoad": page_timeout.as_millis(),
            "script": page_timeout.as_millis(),
            "implicit": 0
        }),
    );

    // geckodriver creates a temporary Firefox profile per WebDriver session.
    let mut args = vec!["-private-window".to_owned()];
    if headless {
        args.push("-headless".into());
    }

    let mut firefox_options = Map::new();
    firefox_options.insert(
        "args".into(),
        Value::Array(args.into_iter().map(Value::String).collect()),
    );
    if let Some(binary) = firefox_binary {
        firefox_options.insert("binary".into(), Value::String(binary.to_owned()));
    }
    caps.insert("moz:firefoxOptions".into(), Value::Object(firefox_options));

    let mut builder =
        ClientBuilder::rustls().map_err(|err| VelesError::Browser(err.to_string()))?;
    builder.capabilities(caps);
    builder
        .connect(webdriver_url)
        .await
        .map_err(|err| VelesError::Browser(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::reserve_local_port;

    #[test]
    fn reserves_local_port() {
        let port = reserve_local_port().expect("reserve port");
        assert!(port > 0);
    }
}

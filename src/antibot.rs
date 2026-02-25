use crate::auth::add_bypass_headers;
use crate::config::Config;
use log::warn;
use serde::{Deserialize, Serialize};
use wreq::header::{HeaderMap, HeaderValue, USER_AGENT};
use wreq::{Client, Url};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectiveAntiBotProvider {
    Flaresolverr,
    Native,
}

impl EffectiveAntiBotProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            EffectiveAntiBotProvider::Flaresolverr => "flaresolverr",
            EffectiveAntiBotProvider::Native => "native",
        }
    }
}

#[derive(Debug)]
pub struct AntiBotBootstrap {
    pub headers: HeaderMap,
    pub provider: EffectiveAntiBotProvider,
}

pub async fn bootstrap_login(
    app_config: &Config,
    domain: &str,
    client: &Client,
) -> Result<AntiBotBootstrap, Box<dyn std::error::Error>> {
    let provider = app_config.anti_bot_provider.to_lowercase();

    match provider.as_str() {
        "native" => native_bootstrap(domain, client).await,
        "flaresolverr" => match flaresolverr_bootstrap(app_config, domain, client).await {
            Ok(bootstrap) => Ok(bootstrap),
            Err(e) => {
                warn!(
                    "[antibot] strategy=flaresolverr event=fallback_to_native reason=\"{}\"",
                    e
                );
                native_bootstrap(domain, client).await
            }
        },
        unknown => {
            warn!(
                "[antibot] strategy={} event=unknown_provider fallback=native",
                unknown
            );
            native_bootstrap(domain, client).await
        }
    }
}

async fn native_bootstrap(
    domain: &str,
    client: &Client,
) -> Result<AntiBotBootstrap, Box<dyn std::error::Error>> {
    let mut headers = HeaderMap::new();
    add_bypass_headers(&mut headers);

    let cookie = wreq::cookie::CookieBuilder::new("account_created", "true")
        .domain(domain)
        .path("/")
        .http_only(true)
        .secure(true)
        .build();

    let url = Url::parse(format!("https://{domain}/").as_str())?;
    client.set_cookie(&url, cookie);

    let response = client
        .get(format!("https://{domain}{}", crate::LOGIN_PAGE))
        .headers(headers.clone())
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("Failed to fetch login page: {}", response.status()).into());
    }

    let mut has_ygg_cookie = false;
    for cookie in response.cookies() {
        if cookie.name() == "ygg_" {
            has_ygg_cookie = true;
            break;
        }
    }

    if !has_ygg_cookie {
        return Err("No ygg_ cookie found".into());
    }

    Ok(AntiBotBootstrap {
        headers,
        provider: EffectiveAntiBotProvider::Native,
    })
}

#[derive(Debug, Serialize)]
struct FlaresolverrRequest<'a> {
    cmd: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    session: Option<&'a str>,
    #[serde(rename = "maxTimeout", skip_serializing_if = "Option::is_none")]
    max_timeout: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct FlaresolverrResponse {
    status: String,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    session: Option<String>,
    #[serde(default)]
    solution: Option<FlaresolverrSolution>,
}

#[derive(Debug, Deserialize)]
struct FlaresolverrSolution {
    #[serde(rename = "userAgent")]
    user_agent: String,
    #[serde(default)]
    cookies: Vec<FlaresolverrCookie>,
}

#[derive(Debug, Deserialize)]
struct FlaresolverrCookie {
    name: String,
    value: String,
    #[serde(default)]
    domain: Option<String>,
    #[serde(default)]
    path: Option<String>,
}

async fn flaresolverr_bootstrap(
    app_config: &Config,
    domain: &str,
    client: &Client,
) -> Result<AntiBotBootstrap, Box<dyn std::error::Error>> {
    let endpoint = format!("{}/v1", app_config.flaresolverr_url.trim_end_matches('/'));
    let fs_client = Client::new();

    let create_session = fs_client
        .post(&endpoint)
        .json(&FlaresolverrRequest {
            cmd: "sessions.create",
            url: None,
            session: None,
            max_timeout: Some(app_config.flaresolverr_timeout_ms),
        })
        .send()
        .await?;
    let create_session: FlaresolverrResponse = create_session.json().await?;
    if create_session.status != "ok" {
        return Err(format!(
            "Flaresolverr session creation failed: {}",
            create_session
                .message
                .unwrap_or_else(|| "unknown error".to_string())
        )
        .into());
    }

    let session_id = create_session
        .session
        .ok_or("Flaresolverr returned no session id")?;

    let login_url = format!("https://{domain}{}", crate::LOGIN_PAGE);
    let get_login = fs_client
        .post(&endpoint)
        .json(&FlaresolverrRequest {
            cmd: "request.get",
            url: Some(&login_url),
            session: Some(&session_id),
            max_timeout: Some(app_config.flaresolverr_timeout_ms),
        })
        .send()
        .await?;

    let get_login: FlaresolverrResponse = get_login.json().await?;

    let destroy_session = fs_client
        .post(&endpoint)
        .json(&FlaresolverrRequest {
            cmd: "sessions.destroy",
            url: None,
            session: Some(&session_id),
            max_timeout: Some(app_config.flaresolverr_timeout_ms),
        })
        .send()
        .await;

    if let Err(e) = destroy_session {
        warn!(
            "[antibot] strategy=flaresolverr event=sessions_destroy_failed error=\"{}\"",
            e
        );
    }

    if get_login.status != "ok" {
        return Err(format!(
            "Flaresolverr request.get failed: {}",
            get_login
                .message
                .unwrap_or_else(|| "unknown error".to_string())
        )
        .into());
    }

    let solution = get_login
        .solution
        .ok_or("Flaresolverr response has no solution")?;

    let base_url = Url::parse(format!("https://{domain}/").as_str())?;
    for c in solution.cookies {
        let cookie = wreq::cookie::CookieBuilder::new(c.name, c.value)
            .domain(c.domain.unwrap_or_else(|| domain.to_string()))
            .path(c.path.unwrap_or_else(|| "/".to_string()))
            .http_only(true)
            .secure(true)
            .build();
        client.set_cookie(&base_url, cookie);
    }

    let mut headers = HeaderMap::new();
    add_bypass_headers(&mut headers);
    if !solution.user_agent.is_empty() {
        headers.insert(USER_AGENT, HeaderValue::from_str(&solution.user_agent)?);
    }

    Ok(AntiBotBootstrap {
        headers,
        provider: EffectiveAntiBotProvider::Flaresolverr,
    })
}

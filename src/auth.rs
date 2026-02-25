use crate::domain::get_leaked_ip;
use crate::resolver::AsyncDNSResolverAdapter;
use crate::{DOMAIN, LOGIN_PAGE, LOGIN_PROCESS_PAGE};
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::Write;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::sync::{Arc, OnceLock};
use wreq::header::HeaderMap;
use wreq::{Client, Url};
use wreq_util::{Emulation, EmulationOS, EmulationOption};

pub static KEY: OnceLock<String> = OnceLock::new();

#[derive(Debug)]
pub enum LoginError {
    AntibotUnavailable,
    ChallengeNotSolved,
    InvalidCredentials,
}

impl fmt::Display for LoginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoginError::AntibotUnavailable => write!(f, "Anti-bot unavailable"),
            LoginError::ChallengeNotSolved => {
                write!(f, "Challenge not solved: missing ygg_ cookie")
            }
            LoginError::InvalidCredentials => write!(f, "Invalid username or password"),
        }
    }
}

impl Error for LoginError {}

#[derive(Debug, Clone, Copy)]
pub enum AuthProvider {
    Native,
}

#[derive(Clone)]
pub struct AntibotContext {
    headers: HeaderMap,
}

pub async fn login(
    username: &str,
    password: &str,
    use_sessions: bool,
) -> Result<Client, Box<dyn std::error::Error>> {
    debug!("Logging in with username: {}", username);

    let emu = EmulationOption::builder()
        .emulation(Emulation::Chrome132) // no H3 check on CF before 133
        .emulation_os(EmulationOS::Windows)
        .build();

    let domain_lock = DOMAIN.lock()?;
    let cloned_guard = domain_lock.clone();
    let domain = cloned_guard.as_str();
    drop(domain_lock);

    let leaked_ip = get_leaked_ip().await?;

    let client = Client::builder()
        .emulation(emu)
        .gzip(true)
        .deflate(true)
        .brotli(true)
        .zstd(true)
        .cookie_store(true)
        .dns_resolver(Arc::new(AsyncDNSResolverAdapter::new()?))
        .cert_verification(false)
        .verify_hostname(false)
        .resolve(
            &domain,
            SocketAddr::new(IpAddr::from_str(leaked_ip.as_str())?, 443),
        )
        .build()?;

    let provider = AuthProvider::Native;

    let start = std::time::Instant::now();

    if use_sessions {
        // check if the session file exists
        let session_file = format!("sessions/{}.cookies", username);
        if std::path::Path::new(&session_file.clone()).exists() {
            debug!("Session file found: {}", session_file);
            // load the session from the file
            let cookies = std::fs::read_to_string(&session_file)?;
            let cookies = cookies.split(";").collect::<Vec<&str>>();
            let cookies_len = cookies.len();
            for cookie in cookies {
                let cookie = cookie.trim();
                if cookie.is_empty() {
                    continue;
                }
                let parts: Vec<&str> = cookie.split('=').collect();
                if parts.len() != 2 {
                    continue;
                }
                let name = parts[0].trim();
                let value = parts[1].trim();
                let cookie = wreq::cookie::CookieBuilder::new(name, value)
                    .domain(domain)
                    .path("/")
                    .http_only(true)
                    .secure(true)
                    .build();
                let url = Url::parse(format!("https://{domain}/").as_str())?;
                client.set_cookie(&url, cookie);
            }
            debug!("Restored {} cookies from session file", cookies_len);
        }

        // check if the session is still valid
        let response = client
            .get(format!("https://{domain}/"))
            .headers(HeaderMap::new())
            .send()
            .await?;
        if response.status().is_success() {
            let stop = std::time::Instant::now();
            debug!(
                "Successfully resumed session in {:?}",
                stop.duration_since(start)
            );
            return Ok(client);
        } else {
            debug!(
                "Session is not valid, deleting session file (code {})",
                response.status()
            );
            // session is not valid, delete the file
            let _ = std::fs::remove_file(&session_file);
            debug!("Session file deleted");
        }
    }

    client.clear_cookies();

    let context = prepare_antibot_context(&client, domain, provider).await?;
    perform_login_with_context(&client, domain, username, password, &context).await?;

    let stop = std::time::Instant::now();
    debug!("Logged in successfully in {:?}", stop.duration_since(start));

    if use_sessions {
        save_session(username, &client).await?;
    }

    Ok(client)
}

pub async fn prepare_antibot_context(
    client: &Client,
    domain: &str,
    provider: AuthProvider,
) -> Result<AntibotContext, Box<dyn std::error::Error>> {
    let mut headers = HeaderMap::new();

    match provider {
        AuthProvider::Native => {
            add_native_bypass_headers(&mut headers);

            let url = Url::parse(format!("https://{domain}/").as_str())?;
            let cookie = wreq::cookie::CookieBuilder::new("account_created", "true")
                .domain(domain)
                .path("/")
                .http_only(true)
                .secure(true)
                .build();
            client.set_cookie(&url, cookie);

            let response = client
                .get(format!("https://{domain}{LOGIN_PAGE}"))
                .headers(headers.clone())
                .send()
                .await?;

            if !response.status().is_success() {
                return Err(Box::new(LoginError::AntibotUnavailable));
            }
            let _headers = response.headers();
        }
    }

    Ok(AntibotContext { headers })
}

pub async fn perform_login_with_context(
    client: &Client,
    domain: &str,
    username: &str,
    password: &str,
    context: &AntibotContext,
) -> Result<(), Box<dyn std::error::Error>> {
    let url = Url::parse(format!("https://{domain}/").as_str())?;
    let cookie_header = client
        .get_cookies(&url)
        .and_then(|h| h.to_str().ok().map(|value| value.to_string()));
    let has_ygg_cookie = cookie_header
        .as_deref()
        .is_some_and(|cookies| cookies.contains("ygg_="));
    if !has_ygg_cookie {
        return Err(Box::new(LoginError::ChallengeNotSolved));
    }

    let payload = [("id", username), ("pass", password)];

    let response = client
        .post(format!("https://{domain}{LOGIN_PROCESS_PAGE}"))
        .headers(context.headers.clone())
        .form(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        if response.status() == 401 {
            error!("Invalid username or password");
            return Err(Box::new(LoginError::InvalidCredentials));
        }
        return Err(format!("Failed to login: {}", response.status()).into());
    }

    let _headers = response.headers();

    let response = client
        .get(format!("https://{domain}/"))
        .headers(context.headers.clone())
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(format!("Failed to fetch site root page: {}", response.status()).into());
    }

    let _headers = response.cookies();
    Ok(())
}

async fn save_session(username: &str, client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    // save the session in a file
    let mut file = File::create(format!("sessions/{}.cookies", username))?;
    let cookies_header = client
        .get_cookies(&Url::parse(
            format!("https://{}/", DOMAIN.lock()?.as_str()).as_str(),
        )?)
        .unwrap();
    let cookies_header_value = cookies_header.to_str()?;
    debug!("Cookies: {}", cookies_header_value);
    file.write_all(cookies_header_value.as_bytes())?;
    file.flush()?;

    Ok(())
}

fn add_native_bypass_headers(headers: &mut HeaderMap) {
    let own_ip_lock = crate::domain::OWN_IP.get();
    if let Some(own_ip) = own_ip_lock {
        headers.insert("CF-Connecting-IP", own_ip.parse().unwrap());
        headers.insert("X-Forwarded-For", own_ip.parse().unwrap());
    }
}

use crate::antibot::bootstrap_login;
use crate::config::Config;
use crate::domain::get_leaked_ip;
use crate::resolver::AsyncDNSResolverAdapter;
use crate::{DOMAIN, LOGIN_PROCESS_PAGE};
use std::fs::File;
use std::io::Write;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::sync::{Arc, OnceLock};
use wreq::header::HeaderMap;
use wreq::{Client, Url};
use wreq_util::{Emulation, EmulationOS, EmulationOption};

pub static KEY: OnceLock<String> = OnceLock::new();

pub async fn login(
    username: &str,
    password: &str,
    use_sessions: bool,
    config: &Config,
) -> Result<Client, Box<dyn std::error::Error>> {
    debug!("Logging in with username: {}", username);

    let emu = EmulationOption::builder()
        .emulation(Emulation::Chrome132)
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

    let start = std::time::Instant::now();

    if use_sessions {
        let session_file = format!("sessions/{}.cookies", username);
        if std::path::Path::new(&session_file.clone()).exists() {
            debug!("Session file found: {}", session_file);
            let cookies = std::fs::read_to_string(&session_file)?;
            let cookies = cookies.split(';').collect::<Vec<&str>>();
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

        let mut resume_headers = HeaderMap::new();
        add_bypass_headers(&mut resume_headers);
        let response = client
            .get(format!("https://{domain}/"))
            .headers(resume_headers)
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
            let _ = std::fs::remove_file(&session_file);
            debug!("Session file deleted");
        }
    }

    client.clear_cookies();

    let antibot_bootstrap = bootstrap_login(config, domain, &client).await?;
    info!(
        "[antibot] event=login_bootstrap_completed strategy={} username={}",
        antibot_bootstrap.provider.as_str(),
        username
    );

    let payload = [("id", username), ("pass", password)];

    let response = client
        .post(format!("https://{domain}{LOGIN_PROCESS_PAGE}"))
        .headers(antibot_bootstrap.headers.clone())
        .form(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        if response.status() == 401 {
            error!("Invalid username or password");
            return Err("Invalid username or password".into());
        }
        return Err(format!("Failed to login: {}", response.status()).into());
    }

    let response = client
        .get(format!("https://{domain}/"))
        .headers(antibot_bootstrap.headers)
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(format!("Failed to fetch site root page: {}", response.status()).into());
    }

    let stop = std::time::Instant::now();
    debug!("Logged in successfully in {:?}", stop.duration_since(start));

    if use_sessions {
        save_session(username, &client).await?;
    }

    Ok(client)
}

async fn save_session(username: &str, client: &Client) -> Result<(), Box<dyn std::error::Error>> {
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

pub fn add_bypass_headers(headers: &mut HeaderMap) {
    let own_ip_lock = crate::domain::OWN_IP.get();
    if let Some(own_ip) = own_ip_lock {
        headers.insert("CF-Connecting-IP", own_ip.parse().unwrap());
        headers.insert("X-Forwarded-For", own_ip.parse().unwrap());
    }
}

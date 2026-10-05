// The real Fetch: one HTTP client for the process, with reused
// connections, one User-Agent (the NWS refuses anonymous requests and the
// others ask for identification) and one timeout, as Http.cs had. TLS
// certificates are checked against the system's own store, and Windows'
// proxy setting is followed, as .NET's client did.

use std::time::Duration;

use ureq::Agent;
use ureq::tls::{RootCerts, TlsConfig};

use crate::fetch::{Fetch, FetchError};

pub const TIMEOUT: Duration = Duration::from_secs(20);

pub struct UreqFetch {
    agent: Agent,
    // Developer-only, like WEATHERSPELL_FONT_POINTS was: every request
    // fails as if the network were down, so the cached view and its wording
    // can be rehearsed without disconnecting the PC.
    offline: bool,
}

impl UreqFetch {
    // "Weatherspell/0.2.0", say; the contact is added here.
    pub fn new(product: &str) -> UreqFetch {
        let config = Agent::config_builder()
            .timeout_global(Some(TIMEOUT))
            .user_agent(format!(
                "{product} (+https://github.com/PlanetLinux98/weatherspell)"
            ))
            .tls_config(
                TlsConfig::builder()
                    .root_certs(RootCerts::PlatformVerifier)
                    .build(),
            )
            .build();
        UreqFetch {
            agent: config.into(),
            offline: std::env::var_os("WEATHERSPELL_OFFLINE").is_some_and(|v| !v.is_empty()),
        }
    }
}

fn host(url: &str) -> String {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    rest.split(['/', '?', '#'])
        .next()
        .unwrap_or(rest)
        .to_string()
}

impl Fetch for UreqFetch {
    fn get(&self, url: &str, timeout: Option<Duration>) -> Result<String, FetchError> {
        let host = host(url);
        if self.offline {
            return Err(FetchError::Offline { host });
        }
        let mut request = self.agent.get(url).header("Accept", "application/json");
        if let Some(t) = timeout {
            request = request.config().timeout_global(Some(t)).build();
        }
        let failed = |e: ureq::Error, host: String| match e {
            ureq::Error::StatusCode(code) => FetchError::Status {
                code,
                reason: ureq::http::StatusCode::from_u16(code)
                    .ok()
                    .and_then(|s| s.canonical_reason())
                    .unwrap_or_default()
                    .to_string(),
                host,
            },
            ureq::Error::Timeout(_) => FetchError::TimedOut { host },
            other => FetchError::Unreachable {
                host,
                detail: other.to_string(),
            },
        };
        let mut response = request.call().map_err(|e| failed(e, host.clone()))?;
        response
            .body_mut()
            .read_to_string()
            .map_err(|e| failed(e, host))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn hosts_are_named_from_the_address() {
        assert_eq!(
            super::host("https://dd.weather.gc.ca/today/citypage_weather/ON/03/"),
            "dd.weather.gc.ca"
        );
        assert_eq!(
            super::host("https://api.weather.gov/points/42.65,-73.75"),
            "api.weather.gov"
        );
        assert_eq!(
            super::host("https://forecast.weather.gov/MapClick.php?lat=1"),
            "forecast.weather.gov"
        );
    }
}

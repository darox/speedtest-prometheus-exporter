use std::env;

const DEFAULT_PORT: u16 = 9798;
const DEFAULT_INTERVAL_SECS: u64 = 300;
const MIN_INTERVAL_SECS: u64 = 30;

#[derive(Debug)]
pub enum ConfigError {
    InvalidPort(String),
    InvalidInterval(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPort(v) => write!(f, "invalid PORT value: {v}"),
            Self::InvalidInterval(v) => {
                write!(
                    f,
                    "invalid SPEEDTEST_INTERVAL_SECS value: {v} (minimum: {MIN_INTERVAL_SECS}s)"
                )
            }
        }
    }
}

impl std::error::Error for ConfigError {}

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub interval_secs: u64,
    pub server_id: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let port = env::var("PORT")
            .unwrap_or_else(|_| DEFAULT_PORT.to_string())
            .parse::<u16>()
            .map_err(|_| ConfigError::InvalidPort("not a valid u16".into()))?;

        let interval_secs = match env::var("SPEEDTEST_INTERVAL_SECS") {
            Ok(v) => v
                .parse::<u64>()
                .map_err(|_| ConfigError::InvalidInterval(v.clone()))?,
            Err(_) => DEFAULT_INTERVAL_SECS,
        };

        if interval_secs < MIN_INTERVAL_SECS {
            return Err(ConfigError::InvalidInterval(interval_secs.to_string()));
        }

        let server_id = env::var("SPEEDTEST_SERVER_ID").ok();

        Ok(Self {
            port,
            interval_secs,
            server_id,
        })
    }

    #[allow(dead_code)]
    pub fn new() -> Result<Self, ConfigError> {
        Self::from_env()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_env(vars: &[(&str, Option<&str>)], f: impl FnOnce() -> Result<Config, ConfigError>) -> Result<Config, ConfigError> {
        // Save current values
        let saved: Vec<(&str, Option<String>)> = vars
            .iter()
            .map(|(k, _)| (*k, env::var(*k).ok()))
            .collect();

        // Set test values
        for (key, value) in vars {
            if let Some(v) = value {
                unsafe { env::set_var(key, v) };
            } else {
                unsafe { env::remove_var(key) };
            }
        }

        let result = f();

        // Restore
        for (key, original) in &saved {
            if let Some(v) = original {
                unsafe { env::set_var(key, v) };
            } else {
                unsafe { env::remove_var(key) };
            }
        }

        result
    }

    #[test]
    fn defaults_when_no_env() {
        let config = with_env(
            &[
                ("PORT", None),
                ("SPEEDTEST_INTERVAL_SECS", None),
                ("SPEEDTEST_SERVER_ID", None),
            ],
            Config::from_env,
        )
        .unwrap();
        assert_eq!(config.port, DEFAULT_PORT);
        assert_eq!(config.interval_secs, DEFAULT_INTERVAL_SECS);
        assert!(config.server_id.is_none());
    }

    #[test]
    fn custom_port() {
        let config = with_env(
            &[
                ("PORT", Some("8080")),
                ("SPEEDTEST_INTERVAL_SECS", None),
                ("SPEEDTEST_SERVER_ID", None),
            ],
            Config::from_env,
        )
        .unwrap();
        assert_eq!(config.port, 8080);
    }

    #[test]
    fn custom_server_id() {
        let config = with_env(
            &[
                ("PORT", None),
                ("SPEEDTEST_INTERVAL_SECS", None),
                ("SPEEDTEST_SERVER_ID", Some("1234")),
            ],
            Config::from_env,
        )
        .unwrap();
        assert_eq!(config.server_id, Some("1234".to_string()));
    }

    #[test]
    fn rejects_invalid_port() {
        let err = with_env(
            &[("PORT", Some("not-a-number")), ("SPEEDTEST_INTERVAL_SECS", None)],
            Config::from_env,
        )
        .unwrap_err();
        assert!(matches!(err, ConfigError::InvalidPort(_)));
    }

    #[test]
    fn rejects_interval_below_minimum() {
        let err = with_env(
            &[("PORT", None), ("SPEEDTEST_INTERVAL_SECS", Some("10"))],
            Config::from_env,
        )
        .unwrap_err();
        assert!(matches!(err, ConfigError::InvalidInterval(_)));
    }

    #[test]
    fn accepts_minimum_interval() {
        let config = with_env(
            &[("PORT", None), ("SPEEDTEST_INTERVAL_SECS", Some("30"))],
            Config::from_env,
        )
        .unwrap();
        assert_eq!(config.interval_secs, 30);
    }

    #[test]
    fn rejects_non_numeric_interval() {
        let err = with_env(
            &[("PORT", None), ("SPEEDTEST_INTERVAL_SECS", Some("abc"))],
            Config::from_env,
        )
        .unwrap_err();
        assert!(matches!(err, ConfigError::InvalidInterval(_)));
    }
}

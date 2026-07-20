// Copyright (c) 2023-2026 Tim Oliver Rabl
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Deserialize;

// Reject unknown keys rather than ignoring them. A misplaced or typo'd key
// (e.g. `database` nested under `default:`, where it does not belong) was
// silently dropped, so the CLI fell back to the default database path and
// operated on the wrong data with no indication anything was wrong.
#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ConfigFile {
    pub database: Option<String>,
    #[serde(default)]
    pub default: ProfileConfig,
    #[serde(default)]
    pub profiles: HashMap<String, ProfileConfig>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ProfileConfig {
    pub provider: Option<i64>,
    pub plan: Option<String>,
    pub peer: Option<String>,
    pub format: Option<String>,
    pub streams: Option<u16>,
    pub duration: Option<u64>,
    pub speed_unit: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ResolvedConfig {
    pub database: Option<String>,
    pub provider: Option<i64>,
    pub plan: Option<String>,
    pub peer: Option<String>,
    pub format: Option<String>,
    pub streams: Option<u16>,
    pub duration: Option<u64>,
    pub speed_unit: Option<String>,
}

impl ResolvedConfig {
    /// Fold in the `active_provider` stored by `provider switch`.
    ///
    /// Precedence is CLI flag > config file > stored setting: the stored value
    /// is the least explicit signal, so it only fills a gap. Without this the
    /// setting was written and read back by `provider show` but consulted by
    /// nothing else, making `provider switch` a no-op.
    pub fn apply_stored_provider(&mut self, stored: Option<i64>) {
        if self.provider.is_none() {
            self.provider = stored;
        }
    }
}

impl ConfigFile {
    pub fn load(path: Option<&str>) -> Result<Self> {
        let config_path = if let Some(p) = path {
            let path = PathBuf::from(p);
            if !path.exists() {
                anyhow::bail!("config file not found: {p}");
            }
            Some(path)
        } else {
            Self::find_default()
        };

        match config_path {
            Some(p) => {
                let contents = std::fs::read_to_string(&p)
                    .with_context(|| format!("failed to read config: {}", p.display()))?;
                let config: ConfigFile = serde_yaml_ng::from_str(&contents)
                    .with_context(|| format!("failed to parse config: {}", p.display()))?;
                log::debug!("loaded config from {}", p.display());
                Ok(config)
            }
            None => Ok(ConfigFile::default()),
        }
    }

    fn find_default() -> Option<PathBuf> {
        if let Some(config_dir) = dirs::config_dir() {
            let xdg = config_dir.join("bbmctl").join("config.yaml");
            if xdg.exists() {
                return Some(xdg);
            }
        }
        if let Some(home) = dirs::home_dir() {
            let legacy = home.join(".bbmctl").join("config.yaml");
            if legacy.exists() {
                return Some(legacy);
            }
        }
        None
    }

    pub fn resolve(&self, profile: Option<&str>) -> Result<ResolvedConfig> {
        let mut resolved = ResolvedConfig {
            database: self.database.clone(),
            provider: self.default.provider,
            plan: self.default.plan.clone(),
            peer: self.default.peer.clone(),
            format: self.default.format.clone(),
            streams: self.default.streams,
            duration: self.default.duration,
            speed_unit: self.default.speed_unit.clone(),
        };

        if let Some(name) = profile {
            let profile = self
                .profiles
                .get(name)
                .ok_or_else(|| anyhow::anyhow!("profile '{name}' not found in config"))?;
            if let Some(v) = profile.provider {
                resolved.provider = Some(v);
            }
            if let Some(ref v) = profile.plan {
                resolved.plan = Some(v.clone());
            }
            if let Some(ref v) = profile.peer {
                resolved.peer = Some(v.clone());
            }
            if let Some(ref v) = profile.format {
                resolved.format = Some(v.clone());
            }
            if let Some(v) = profile.streams {
                resolved.streams = Some(v);
            }
            if let Some(v) = profile.duration {
                resolved.duration = Some(v);
            }
            if let Some(ref v) = profile.speed_unit {
                resolved.speed_unit = Some(v.clone());
            }
        }

        Ok(resolved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_config() {
        let config: ConfigFile = serde_yaml_ng::from_str("").unwrap();
        let resolved = config.resolve(None).unwrap();
        assert!(resolved.provider.is_none());
        assert!(resolved.database.is_none());
    }

    #[test]
    fn default_profile() {
        let yaml = r#"
database: "/tmp/test.db"
default:
  provider: 437
  plan: "8515"
  streams: 4
"#;
        let config: ConfigFile = serde_yaml_ng::from_str(yaml).unwrap();
        let resolved = config.resolve(None).unwrap();
        assert_eq!(resolved.provider, Some(437));
        assert_eq!(resolved.plan.as_deref(), Some("8515"));
        assert_eq!(resolved.streams, Some(4));
        assert_eq!(resolved.database.as_deref(), Some("/tmp/test.db"));
    }

    #[test]
    fn profile_overrides_default() {
        let yaml = r#"
default:
  provider: 437
  plan: "8515"
  streams: 8
profiles:
  office:
    provider: 251
    plan: "9001"
"#;
        let config: ConfigFile = serde_yaml_ng::from_str(yaml).unwrap();
        let resolved = config.resolve(Some("office")).unwrap();
        assert_eq!(resolved.provider, Some(251));
        assert_eq!(resolved.plan.as_deref(), Some("9001"));
        assert_eq!(resolved.streams, Some(8));
    }

    #[test]
    fn unknown_profile_errors() {
        let config = ConfigFile::default();
        let result = config.resolve(Some("nonexistent"));
        assert!(result.is_err());
    }

    #[test]
    fn missing_file_returns_default() {
        let config = ConfigFile::load(None).unwrap();
        assert!(config.database.is_none());
    }
}

#[cfg(test)]
mod active_provider_tests {
    use super::*;

    /// `provider switch` writes `active_provider` to the database and
    /// `provider show` reads it back, but nothing else ever consulted it --
    /// `test`, `history add`, `compare` and `campaign` all resolved the
    /// provider from the CLI flag or config file only. The user switched
    /// providers, saw it confirmed, and nothing changed.
    #[test]
    fn stored_provider_is_used_when_nothing_else_supplies_one() {
        let mut resolved = ResolvedConfig::default();
        assert_eq!(resolved.provider, None);

        resolved.apply_stored_provider(Some(437));

        assert_eq!(
            resolved.provider,
            Some(437),
            "a stored active provider must be used when no flag or config supplies one"
        );
    }

    /// The config file is more explicit than a stored setting, so it wins.
    #[test]
    fn config_file_provider_beats_stored_provider() {
        let mut resolved = ResolvedConfig {
            provider: Some(100),
            ..ResolvedConfig::default()
        };

        resolved.apply_stored_provider(Some(437));

        assert_eq!(resolved.provider, Some(100));
    }

    /// No stored value must leave the resolution untouched.
    #[test]
    fn absent_stored_provider_changes_nothing() {
        let mut resolved = ResolvedConfig {
            provider: Some(100),
            ..ResolvedConfig::default()
        };

        resolved.apply_stored_provider(None);

        assert_eq!(resolved.provider, Some(100));
    }
}

#[cfg(test)]
mod strict_config_tests {
    use super::*;

    /// A misplaced key must be an error, not silently dropped.
    ///
    /// `database` is top-level, but nesting it under `default:` is a natural
    /// mistake. Serde ignored it, the CLI fell back to the default database
    /// path, and the user got no indication -- so commands silently operated
    /// on the wrong database.
    #[test]
    fn misplaced_key_is_rejected() {
        let yaml = "default:\n  database: /tmp/somewhere.db\n";

        let result: Result<ConfigFile, _> = serde_yaml_ng::from_str(yaml);

        let err = result.expect_err("a key in the wrong section must be rejected, not ignored");
        assert!(
            err.to_string().contains("database"),
            "the error should name the offending key, got: {err}"
        );
    }

    /// An outright typo must also be caught.
    #[test]
    fn unknown_top_level_key_is_rejected() {
        let yaml = "databse: /tmp/typo.db\n";

        let result: Result<ConfigFile, _> = serde_yaml_ng::from_str(yaml);

        assert!(
            result.is_err(),
            "a typo'd key must be rejected rather than silently ignored"
        );
    }

    /// A correct config must still parse.
    #[test]
    fn valid_config_still_parses() {
        let yaml = "database: /tmp/ok.db\ndefault:\n  provider: 437\n";

        let config: ConfigFile =
            serde_yaml_ng::from_str(yaml).expect("a well-formed config must parse");

        assert_eq!(config.database.as_deref(), Some("/tmp/ok.db"));
        assert_eq!(config.default.provider, Some(437));
    }
}

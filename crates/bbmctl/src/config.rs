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

#[derive(Debug, Deserialize, Default)]
pub struct ConfigFile {
    pub database: Option<String>,
    #[serde(default)]
    pub default: ProfileConfig,
    #[serde(default)]
    pub profiles: HashMap<String, ProfileConfig>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ProfileConfig {
    pub provider: Option<i64>,
    pub plan: Option<String>,
    pub peer: Option<String>,
    pub format: Option<String>,
    pub streams: Option<u16>,
    pub duration: Option<u64>,
    pub speed_unit: Option<String>,
}

#[derive(Debug, Clone)]
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

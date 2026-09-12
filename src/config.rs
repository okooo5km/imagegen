//! Portable configuration and secrets. Author: okooo5km(十里).
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
};

pub const DEFAULT_MODEL: &str = "gpt-image-2.5-flare";
pub const SIGNUP: &str = "https://apimart.ai";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Defaults {
    pub model: Option<String>,
    pub aspect_ratio: Option<String>,
    pub size: Option<String>,
    pub resolution: Option<String>,
    pub quality: Option<String>,
    pub output_format: Option<String>,
    pub background: Option<String>,
    pub n: Option<u8>,
    pub timeout: Option<u64>,
    pub poll_interval: Option<u64>,
    pub concurrency: Option<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default = "version")]
    pub version: u8,
    #[serde(default)]
    pub defaults: Defaults,
}
fn version() -> u8 {
    1
}
impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            defaults: Defaults::default(),
        }
    }
}

pub fn home() -> Result<PathBuf> {
    let var = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    env::var_os(var)
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .context("Cannot locate home directory; set IMAGEGEN_CONFIG_DIR.")
}
pub fn config_dir() -> Result<PathBuf> {
    if let Some(path) = env::var_os("IMAGEGEN_CONFIG_DIR").filter(|s| !s.is_empty()) {
        return Ok(PathBuf::from(path));
    }
    if cfg!(windows) {
        return Ok(env::var_os("APPDATA")
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .unwrap_or(home()?.join("AppData/Roaming"))
            .join("imagegen"));
    }
    Ok(env::var_os("XDG_CONFIG_HOME")
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or(home()?.join(".config"))
        .join("imagegen"))
}
pub fn expand(path: &Path) -> Result<PathBuf> {
    if path == Path::new("~") {
        return home();
    }
    if let Ok(tail) = path.strip_prefix("~") {
        return Ok(home()?.join(tail));
    }
    Ok(path.to_path_buf())
}
pub fn read_text(path: &Path) -> Result<String> {
    Ok(fs::read_to_string(path)
        .with_context(|| format!("Cannot read {}", path.display()))?
        .trim_start_matches('\u{feff}')
        .to_owned())
}
pub fn private_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().context("File needs a parent directory")?;
    fs::create_dir_all(parent)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        temp.as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))?;
    }
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    temp.persist(path)
        .map_err(|e| e.error)
        .with_context(|| format!("Cannot save {}", path.display()))?;
    Ok(())
}
impl Config {
    pub fn load(dir: &Path) -> Result<Self> {
        let path = dir.join("config.toml");
        let config = if path.exists() {
            toml::from_str::<Self>(&read_text(&path)?)
                .context("Invalid config.toml; use imagegen config path to locate it")?
        } else if dir.join("config.json").exists() {
            // Read old Python settings without mutating them. Explicit config writes migrate them.
            let value: serde_json::Value =
                serde_json::from_str(&read_text(&dir.join("config.json"))?)?;
            let model = value
                .get("default_model")
                .and_then(|s| s.as_str())
                .map(str::to_owned);
            Self {
                defaults: Defaults {
                    model,
                    ..Defaults::default()
                },
                ..Self::default()
            }
        } else {
            Self::default()
        };
        config.validate()?;
        Ok(config)
    }
    pub fn validate(&self) -> Result<()> {
        if self.version != 1 {
            bail!("Unsupported config version {}", self.version);
        }
        let d = &self.defaults;
        if let Some(model) = &d.model {
            crate::models::validate_model(model)?;
        }
        if let Some(ratio) = &d.aspect_ratio {
            crate::models::validate_ratio(ratio)?;
        }
        if let Some(size) = &d.size {
            crate::models::validate_size(size)?;
        }
        for (value, allowed, name) in [
            (&d.resolution, &["1k", "2k", "4k"][..], "resolution"),
            (
                &d.quality,
                &["low", "medium", "high", "xhigh", "max", "auto"][..],
                "quality",
            ),
            (
                &d.output_format,
                &["png", "jpeg", "webp"][..],
                "output_format",
            ),
            (
                &d.background,
                &["auto", "opaque", "transparent"][..],
                "background",
            ),
        ] {
            if let Some(v) = value
                && !allowed.contains(&v.as_str())
            {
                bail!("Invalid {name}: {v}");
            }
        }
        if d.n.is_some_and(|n| !(1..=4).contains(&n)) {
            bail!("n must be 1..4");
        }
        if d.concurrency.is_some_and(|n| !(1..=8).contains(&n)) {
            bail!("concurrency must be 1..8");
        }
        if d.timeout.is_some_and(|n| !(1..=86400).contains(&n)) {
            bail!("timeout must be 1..86400 seconds");
        }
        if d.poll_interval.is_some_and(|n| !(1..=60).contains(&n)) {
            bail!("poll_interval must be 1..60 seconds");
        }
        Ok(())
    }
    pub fn save(&self, dir: &Path) -> Result<()> {
        self.validate()?;
        private_write(
            &dir.join("config.toml"),
            toml::to_string_pretty(self)?.as_bytes(),
        )
    }
    pub fn set(&mut self, key: &str, value: Option<&str>) -> Result<()> {
        let mut object = serde_json::to_value(&self.defaults)?;
        let map = object.as_object_mut().context("Invalid defaults")?;
        if !map.contains_key(key) {
            bail!("Unknown default: {key}. Use imagegen config show.");
        }
        let parsed = match value {
            None => serde_json::Value::Null,
            Some(v) if ["n", "timeout", "poll_interval", "concurrency"].contains(&key) => {
                serde_json::json!(
                    v.parse::<u64>()
                        .context("Expected a non-negative integer")?
                )
            }
            Some(v) => serde_json::json!(v),
        };
        map.insert(key.to_owned(), parsed);
        self.defaults = serde_json::from_value(object)?;
        self.validate()
    }
}

pub fn resolve_key(dir: &Path) -> Result<Option<(String, String)>> {
    if let Ok(key) = env::var("APIMART_API_KEY")
        && !key.trim().is_empty()
    {
        return Ok(Some((key.trim().to_owned(), "APIMART_API_KEY".into())));
    }
    let mut paths = vec![dir.join("apimart.key")];
    if env::var_os("IMAGEGEN_CONFIG_DIR").is_none() {
        let legacy = home()?.join(".config/imagegen/apimart.key");
        if !paths.contains(&legacy) {
            paths.push(legacy);
        }
    }
    for path in paths {
        if path.is_file()
            && let Some(key) = read_text(&path)?
                .lines()
                .map(str::trim)
                .find(|l| !l.is_empty())
        {
            return Ok(Some((key.to_owned(), path.display().to_string())));
        }
    }
    Ok(None)
}
pub fn require_key(dir: &Path) -> Result<String> {
    let (key, _) = resolve_key(dir)?.with_context(|| format!("API key missing. Register: {SIGNUP}; create a key at https://apimart.ai/keys; run imagegen auth set in your terminal."))?;
    if key.contains(['\r', '\n']) {
        bail!("API key must be a single line");
    }
    Ok(key)
}

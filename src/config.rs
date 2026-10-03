use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{env, fs, path::PathBuf};

use crate::process_pdf_to_text::PdfBackend;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrobidConfig {
    pub source: String,
    pub url: String,
    pub server_binary: PathBuf,
    pub server_workdir: PathBuf,
    pub server_args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MineruConfig {
    pub source: String,
    pub binary: PathBuf,
    pub server_binary: PathBuf,
    pub server_workdir: PathBuf,
    pub server_args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub backend: PdfBackend,
    pub grobid: GrobidConfig,
    pub mineru: MineruConfig,
}

impl Default for Config {
    fn default() -> Self {
        let services_dir = env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("~"))
            .join("opt/services");

        Self {
            backend: PdfBackend::Grobid,
            grobid: GrobidConfig {
                source: "https://github.com/grobidOrg/grobid".into(),
                url: "http://localhost:8070/api/processFulltextDocument".into(),
                server_binary: services_dir.join("grobid/gradlew"),
                server_workdir: services_dir.join("grobid"),
                server_args: vec![":grobid-service:run".into()],
            },
            mineru: MineruConfig {
                source: "https://github.com/opendatalab/MinerU".into(),
                binary: services_dir.join("mineru/.venv/bin/mineru-kit"),
                server_binary: services_dir.join("mineru/.venv/bin/mineru-kit"),
                server_workdir: services_dir.join("mineru"),
                server_args: vec![
                    "api-server".into(),
                    "--host".into(),
                    "127.0.0.1".into(),
                    "--port".into(),
                    "8000".into(),
                    "--tier".into(),
                    "standard".into(),
                ],
            },
        }
    }
}

impl Config {
    pub fn load(path: &std::path::Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read config {}", path.display()))?;
        serde_json::from_str(&content)
            .with_context(|| format!("failed to parse config {}", path.display()))
    }

    pub fn save(&self, path: &std::path::Path) -> Result<()> {
        let content =
            serde_json::to_string_pretty(self).context("failed to serialize config")?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create config directory {}", parent.display())
            })?;
        }

        fs::write(path, content)
            .with_context(|| format!("failed to write config {}", path.display()))
    }

    /// Returns `true` if the configuration file was created.
    pub fn ensure_exist(path: &std::path::Path) -> Result<bool> {
        if path.exists() {
            return Ok(false);
        }

        Self::default().save(path)?;
        Ok(true)
    }

    pub fn load_or_create() -> Result<Self> {
        let path = Self::default_path();

        if Self::ensure_exist(&path)? {
            println!("Created default config at {:?}", path);
            Ok(Self::default())
        } else {
            Self::load(&path)
        }
    }

    pub fn default_path() -> PathBuf {
        if let Some(config_home) = env::var_os("XDG_CONFIG_HOME") {
            return PathBuf::from(config_home).join("pdf_to_text/config.json");
        }

        if let Some(home) = env::var_os("HOME") {
            return PathBuf::from(home).join(".config/pdf_to_text/config.json");
        }

        PathBuf::from("config.json")
    }
}

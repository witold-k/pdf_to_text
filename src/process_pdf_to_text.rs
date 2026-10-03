use anyhow::{bail, Context, Result};
use crate::config::Config;
use crate::pdf2json::GrobidConverter;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::str::FromStr;

/// Backend used to turn a PDF into a text file suitable for tokenization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PdfBackend {
    Grobid,
    Mineru,
}

impl fmt::Display for PdfBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Grobid => write!(f, "grobid"),
            Self::Mineru => write!(f, "mineru"),
        }
    }
}

impl FromStr for PdfBackend {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "grobid" => Ok(Self::Grobid),
            "mineru" => Ok(Self::Mineru),
            _ => bail!("unknown PDF backend '{value}'; expected 'grobid' or 'mineru'"),
        }
    }
}

/// Processes one PDF and writes the backend's primary text representation.
///
/// Both backends expose the same boundary to the rest of the pipeline:
/// PDF in, UTF-8 text file out. Backend-specific auxiliary output is not part
/// of the tokenization interface.
pub fn process_pdf_to_text(
    config: &Config,
    backend: PdfBackend,
    input: &Path,
    output: &Path,
) -> Result<()> {
    match backend {
        PdfBackend::Grobid => process_grobid(config, input, output),
        PdfBackend::Mineru => process_mineru(config, input, output),
    }
}

fn process_grobid(config: &Config, input: &Path, output: &Path) -> Result<()> {
    println!("PDF -> text [grobid]: {} -> {}", input.display(), output.display());

    let converter = GrobidConverter::new(&config.grobid.url);
    let pdf_path = input.to_string_lossy();
    let tei = converter.extract_tei(&pdf_path)?;
    let paper = converter.tei_to_json(&tei)?;

    let mut parts = Vec::new();
    if let Some(title) = paper.get("title").and_then(|value| value.as_str())
        && !title.is_empty()
    {
        parts.push(title.to_owned());
    }
    if let Some(abstract_) = paper.get("abstract_").and_then(|value| value.as_str())
        && !abstract_.is_empty()
    {
        parts.push(abstract_.to_owned());
    }
    if let Some(sections) = paper.get("sections").and_then(|value| value.as_array()) {
        for section in sections {
            if let Some(heading) = section.get("heading").and_then(|value| value.as_str())
                && !heading.is_empty()
            {
                parts.push(heading.to_owned());
            }
            if let Some(text) = section.get("text").and_then(|value| value.as_str())
                && !text.is_empty()
            {
                parts.push(text.to_owned());
            }
        }
    }

    fs::write(output, parts.join("\n\n"))
        .with_context(|| format!("failed to write {}", output.display()))?;
    Ok(())
}

fn process_mineru(config: &Config, input: &Path, output: &Path) -> Result<()> {
    println!("PDF -> text [mineru]: {} -> {}", input.display(), output.display());

    let status = Command::new(&config.mineru.binary)
        .arg("parse")
        .arg(input)
        .arg("-o")
        .arg(output)
        .arg("--tier")
        .arg("standard")
        .status()
        .with_context(|| {
            format!(
                "failed to start MinerU binary {}",
                config.mineru.binary.display()
            )
        })?;

    if !status.success() {
        bail!("MinerU failed for {} with status {status}", input.display());
    }

    Ok(())
}

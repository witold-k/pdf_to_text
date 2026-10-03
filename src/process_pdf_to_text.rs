use anyhow::{bail, Context, Result};
use crate::config::Config;
use crate::pdf2json::GrobidConverter;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
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

/// Processes one PDF into a backend-specific output directory.
///
/// `output` is the canonical path seen by the rest of the pipeline. The
/// backend writes its files into a sibling directory named after the PDF stem,
/// and `output` becomes a symbolic link to the backend's primary artifact.
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
    let output_dir = output_directory(output)?;
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("failed to create {}", output_dir.display()))?;
    let primary = output_dir.join(
        output
            .file_name()
            .context("output path has no file name")?,
    );

    println!(
        "PDF -> text [grobid]: {} -> {}",
        input.display(),
        output_dir.display()
    );

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

    fs::write(&primary, parts.join("\n\n"))
        .with_context(|| format!("failed to write {}", primary.display()))?;
    create_primary_link(output, &primary)?;

    Ok(())
}

fn output_directory(output: &Path) -> Result<PathBuf> {
    let parent = output.parent().context("output path has no parent")?;
    let stem = output.file_stem().context("output path has no file stem")?;
    Ok(parent.join(stem))
}

fn find_primary_artifact(output_dir: &Path, extension: &str) -> Result<PathBuf> {
    let mut matches = Vec::new();
    for entry in fs::read_dir(output_dir)
        .with_context(|| format!("failed to read {}", output_dir.display()))?
    {
        let path = entry?.path();
        if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some(extension) {
            matches.push(path);
        }
    }

    match matches.as_slice() {
        [primary] => Ok(primary.clone()),
        [] => bail!(
            "backend produced no .{extension} primary artifact in {}",
            output_dir.display()
        ),
        _ => bail!(
            "backend produced multiple .{extension} files in {}; primary artifact is ambiguous",
            output_dir.display()
        ),
    }
}

#[cfg(unix)]
fn create_primary_link(link: &Path, target: &Path) -> Result<()> {
    use std::os::unix::fs::symlink;

    if link.exists() || link.symlink_metadata().is_ok() {
        fs::remove_file(link)
            .with_context(|| format!("failed to remove old link {}", link.display()))?;
    }

    let parent = link.parent().context("link path has no parent")?;
    let relative_target = target
        .strip_prefix(parent)
        .with_context(|| format!("{} is not below {}", target.display(), parent.display()))?;

    symlink(relative_target, link).with_context(|| {
        format!(
            "failed to create link {} -> {}",
            link.display(),
            relative_target.display()
        )
    })
}

#[cfg(not(unix))]
fn create_primary_link(_link: &Path, _target: &Path) -> Result<()> {
    bail!("canonical output links are currently supported only on Unix systems")
}

fn process_mineru(config: &Config, input: &Path, output: &Path) -> Result<()> {
    let output_dir = output_directory(output)?;
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("failed to create {}", output_dir.display()))?;

    println!(
        "PDF -> text [mineru]: {} -> {}",
        input.display(),
        output_dir.display()
    );

    let status = Command::new(&config.mineru.binary)
        .arg("parse")
        .arg(input)
        .arg("-o")
        .arg(&output_dir)
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

    let primary = find_primary_artifact(&output_dir, "md")?;
    create_primary_link(output, &primary)?;

    Ok(())
}

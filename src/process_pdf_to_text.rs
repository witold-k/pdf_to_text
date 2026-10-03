use crate::error::{Error, Result};
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
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        if value.eq_ignore_ascii_case("grobid") {
            Ok(Self::Grobid)
        } else if value.eq_ignore_ascii_case("mineru") {
            Ok(Self::Mineru)
        } else {
            Err(Error::InvalidBackend)
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
    fs::create_dir_all(&output_dir)?;
    let primary = output_dir.join(
        output
            .file_name()
            .ok_or(Error::InvalidPath("output path has no file name"))?,
    );

    println!(
        "PDF -> text [grobid]: {} -> {}",
        input.display(),
        output_dir.display()
    );

    let converter = GrobidConverter::new(&config.grobid.url);
    let pdf_path = input.to_string_lossy();
    let tei = converter.extract_tei(&pdf_path)?;
    let paper = converter.tei_to_paper(&tei)?;

    let stem = output
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or(Error::InvalidPath("output path has no UTF-8 file stem"))?;
    let tei_output = output_dir.join(format!("{stem}.tei.xml"));
    let json_output = output_dir.join(format!("{stem}.json"));

    fs::write(tei_output, &tei)?;
    fs::write(json_output, paper.to_pretty_string()?)?;
    fs::write(&primary, paper.to_text())?;
    create_primary_link(output, &primary)?;

    Ok(())
}

fn output_directory(output: &Path) -> Result<PathBuf> {
    let parent = output.parent().ok_or(Error::InvalidPath("output path has no parent"))?;
    let stem = output.file_stem().ok_or(Error::InvalidPath("output path has no file stem"))?;
    Ok(parent.join(stem))
}

fn find_primary_artifact(output_dir: &Path, extension: &str) -> Result<PathBuf> {
    let mut matches = Vec::new();
    for entry in fs::read_dir(output_dir)? {
        let path = entry?.path();
        if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some(extension) {
            matches.push(path);
        }
    }

    match matches.as_slice() {
        [primary] => Ok(primary.clone()),
        [] => Err(Error::NoPrimaryArtifact),
        _ => Err(Error::MultiplePrimaryArtifacts),
    }
}

#[cfg(unix)]
fn create_primary_link(link: &Path, target: &Path) -> Result<()> {
    use std::os::unix::fs::symlink;

    if link.exists() || link.symlink_metadata().is_ok() {
        fs::remove_file(link)?;
    }

    let parent = link.parent().ok_or(Error::InvalidPath("link path has no parent"))?;
    let relative_target = target
        .strip_prefix(parent)
        .map_err(|_| Error::InvalidPath("link target is outside link parent"))?;

    Ok(symlink(relative_target, link)?)
}

#[cfg(not(unix))]
fn create_primary_link(_link: &Path, _target: &Path) -> Result<()> {
    Err(Error::UnsupportedPlatform)
}

fn process_mineru(config: &Config, input: &Path, output: &Path) -> Result<()> {
    let output_dir = output_directory(output)?;
    fs::create_dir_all(&output_dir)?;

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
        .status()?;

    if !status.success() {
        return Err(Error::MineruFailed(status));
    }

    let primary = find_primary_artifact(&output_dir, "md")?;
    create_primary_link(output, &primary)?;

    Ok(())
}

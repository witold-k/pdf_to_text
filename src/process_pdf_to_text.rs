use anyhow::{Context, Result};
use crate::pdf2json::GrobidConverter;
use std::fs;
use std::path::Path;

/// Processes one PDF through GROBID and writes structured JSON.
///
/// The output contains title, authors, abstract, body sections, and references.
pub fn process_pdf_to_text(input: &Path, output: &Path) -> Result<()> {
    println!("PDF -> JSON: {} -> {}", input.display(), output.display());

    let converter = GrobidConverter::new();
    let pdf_path = input.to_string_lossy();
    let tei = converter.extract_tei(&pdf_path)?;
    let paper = converter.tei_to_json(&tei)?;
    let json = serde_json::to_string_pretty(&paper)?;

    fs::write(output, json)
        .with_context(|| format!("failed to write {}", output.display()))?;

    println!("Saved JSON -> {}", output.display());
    Ok(())
}

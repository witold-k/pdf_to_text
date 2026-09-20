use crate::pdf2json::GrobidConverter;
use std::fs;
use std::path::Path;
use anyhow::Result;
use serde_json::Value;

/// Processes a PDF file and extracts text to a specified output file.
///
/// # Arguments
///
/// * `input` - A reference to the input PDF file path.
/// * `output` - A reference to the output text file path.
///
/// # Returns
///
/// * `Result<()>` - Returns `Ok(())` if the operation is successful, otherwise returns an error.
pub fn process_pdf_to_text(input: &Path, output: &Path) -> Result<()> {
    println!("PDF -> text: {} -> {}", input.display(), output.display());

    let converter = GrobidConverter::new();
    let pdf_path = input.display().to_string();
    let tei: String = converter.extract_tei(&pdf_path)?;
    let paper: Value = converter.tei_to_json(&tei)?;

    let json = serde_json::to_string_pretty(&paper)?;

    fs::write(output, json)?;

    println!("Saved text -> {}", output.display());
    Ok(())
}


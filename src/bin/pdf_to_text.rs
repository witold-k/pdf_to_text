use anyhow::Result;

use fsscanner::fsscanner_mt;
use pdf_to_text::process_pdf_to_text::process_pdf_to_text;
use pdf_to_text::process_text_to_token::process_text_to_token;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: pdf_to_text <pdf_input_dir> <output_dir>");
        std::process::exit(1);
    }

    let pdf_input   = &args[1];
    let output_root = &args[2];

    //
    // Stage 1: PDF -> text
    //

    let text_output = format!("{}/text", output_root);

    fsscanner_mt::process_dir_map(
        pdf_input,
        &text_output,
        "pdf",   // input extension
        "json",  // structured output suffix
        |input, output| process_pdf_to_text(input, output).map_err(Into::into),
    ).map_err(|error| anyhow::anyhow!(error.to_string()))?;

    //
    // Stage 2: text -> token_db
    //

    fsscanner_mt::process_dir_map_multi(
        &text_output,
        &text_output,
        "json",
        &["tok", "tdb"],
        |input, outputs| {
            process_text_to_token(input, &outputs[0], &outputs[1]).map_err(Into::into)
        },
    )
    .map_err(|error| anyhow::anyhow!(error.to_string()))?;

    Ok(())
}

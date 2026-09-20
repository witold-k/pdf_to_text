use anyhow::Result;

use fsscanner::fsscanner_mt;
use pdf_to_text::process_pdf_to_text::process_pdf_to_text;
use pdf_to_text::process_text_to_token::process_text_to_token;
use std::path::Path;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: pipeline <pdf_input_dir> <output_dir>");
        std::process::exit(1);
    }

    let pdf_input   = &args[1];
    let output_root = &args[2];

    //
    // Stage 1: PDF -> text
    //

    let text_output = format!("{}/text", output_root);

    let _ = fsscanner_mt::process_dir_map(
        pdf_input,
        &text_output,
        "pdf",   // input extension
        "txt",   // output suffix
        |input, output| process_pdf_to_text(input, output).map_err(|e| e.into()),
    );

    //
    // Stage 2: text -> token_db
    //

    let _ = fsscanner_mt::process_dir_map(
        &text_output,
        &text_output,
        "txt",
        "tok",
        |input, output| {
            let tokendb_path = output.to_path_buf().join("db");
            let output_paths: Vec<&Path> = vec![output, &tokendb_path];
            process_text_to_token(input, &output_paths).map_err(|e| e.into())
        },
    );

    Ok(())
}

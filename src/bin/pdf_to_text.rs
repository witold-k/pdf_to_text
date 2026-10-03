use anyhow::Result;

use fsscanner::fsscanner_mt;
use pdf_to_text::config::Config;
use pdf_to_text::process_pdf_to_text::{process_pdf_to_text, PdfBackend};
use pdf_to_text::process_text_to_token::process_text_to_token;
use pdf_to_text::service::start_service;

fn usage() {
    eprintln!("Usage:");
    eprintln!("  pdf_to_text [grobid|mineru] <pdf_input_dir> <output_dir>");
    eprintln!("  pdf_to_text start [grobid|mineru]");
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let config = Config::load_or_create()?;

    if args.get(1).map(String::as_str) == Some("start") {
        let backend = match args.get(2) {
            Some(value) => value.parse::<PdfBackend>()?,
            None => config.backend,
        };

        if args.len() > 3 {
            usage();
            std::process::exit(1);
        }

        let mut child = start_service(&config, backend)?;
        let status = child.wait()?;
        if !status.success() {
            anyhow::bail!("{} service exited with status {status}", backend);
        }
        return Ok(());
    }

    if args.len() != 3 && args.len() != 4 {
        usage();
        std::process::exit(1);
    }

    let (backend, pdf_input, output_root) = if args.len() == 4 {
        (args[1].parse::<PdfBackend>()?, &args[2], &args[3])
    } else {
        (config.backend, &args[1], &args[2])
    };

    //
    // Stage 1: PDF -> text
    //

    let text_output = format!("{}/text", output_root);

    let worker_config = config.clone();
    fsscanner_mt::process_dir_map(
        pdf_input,
        &text_output,
        "pdf",
        "md",
        move |input, output| {
            process_pdf_to_text(&worker_config, backend, input, output).map_err(Into::into)
        },
    )
    .map_err(|error| anyhow::anyhow!(error.to_string()))?;

    //
    // Stage 2: text -> token_db
    //

    fsscanner_mt::process_dir_map_multi(
        &text_output,
        &text_output,
        "md",
        &["tok", "tdb"],
        |input, outputs| {
            process_text_to_token(input, &outputs[0], &outputs[1]).map_err(Into::into)
        },
    )
    .map_err(|error| anyhow::anyhow!(error.to_string()))?;

    Ok(())
}

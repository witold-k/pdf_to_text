// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use pdf_to_text::error::{Error, Result};

use fsscanner::fsscanner_mt;
use pdf_to_text::config::Config;
use pdf_to_text::process_pdf_to_text::{process_pdf_to_text, PdfBackend};
use pdf_to_text::process_text_to_token::{
    process_join_token, process_text_to_token, process_token_to_global,
};
use pdf_to_text::service::start_service;
use std::path::PathBuf;
use token_db::TokenDb;

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
            return Err(Error::ServiceFailed(status));
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

    let workers = match backend {
        PdfBackend::Grobid => config.grobid.threads,
        PdfBackend::Mineru => config.mineru.threads,
    };

    let worker_config = config.clone();
    fsscanner_mt::process_dir_map_outdated_with_workers(
        pdf_input,
        output_root,
        "pdf",
        "md",
        workers,
        move |input, output| {
            process_pdf_to_text(&worker_config, backend, input, output).map_err(|_| fsscanner::Error::Callback("PDF conversion failed"))
        },
    )
    .map_err(|_| Error::FsScanner)?;

    //
    // Stage 2: text -> per-document token stream + lookup database
    //

    fsscanner_mt::process_dir_map_with_workers(
        pdf_input,
        output_root,
        "pdf",
        "tok",
        1,
        move |_pdf, token_output| {
            let text_input = token_output.with_extension("md");
            let db_output = token_output.with_extension("tdb");
            if fsscanner_mt::needs_update(&text_input, token_output)?
                || fsscanner_mt::needs_update(&text_input, &db_output)?
            {
                process_text_to_token(&text_input, token_output, &db_output)
                    .map_err(|_| fsscanner::Error::Callback("token conversion failed"))?;
            }
            Ok(())
        },
    )
    .map_err(|_| Error::FsScanner)?;

    //
    // Stage 3: per-document lookup databases -> merged corpus database
    //

    let token_db_output = PathBuf::from(output_root).join("token_db.tdb");
    if fsscanner_mt::dir_needs_update(output_root, "tdb", &token_db_output)
        .map_err(|_| Error::FsScanner)?
    {
        println!("token merge -> {}", token_db_output.display());

        let token_db = fsscanner_mt::process_dir_state_and_map(
            TokenDb::new(),
            output_root,
            output_root,
            "tdb",
            "unused",
            {
                let token_db_output = token_db_output.clone();
                move |db, input, _| {
                    if input == token_db_output {
                        return Ok(());
                    }
                    process_join_token(db, input)
                        .map_err(|_| fsscanner::Error::Callback("token merge failed"))
                }
            },
        )
        .map_err(|_| Error::FsScanner)?;

        token_db.save(&token_db_output)?;
    }

    //
    // Stage 4: document-local token IDs -> corpus-global token IDs
    //

    let global_db = TokenDb::load(&token_db_output)?;
    fsscanner_mt::process_dir_map_with_workers(
        output_root,
        output_root,
        "tdb",
        "unused",
        1,
        {
            let token_db_output = token_db_output.clone();
            move |local_db_input, _| {
                if local_db_input == token_db_output {
                    return Ok(());
                }

                let token_input = local_db_input.with_extension("tok");
                let stem = local_db_input
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .ok_or(fsscanner::Error::Callback("invalid token database path"))?;
                let global_token_output = local_db_input.with_file_name(format!("{stem}_glob.tok"));

                if fsscanner_mt::needs_update(&token_db_output, &global_token_output)? {
                    process_token_to_global(
                        &token_input,
                        local_db_input,
                        &global_db,
                        &global_token_output,
                    )
                    .map_err(|_| fsscanner::Error::Callback("global token remap failed"))?;
                }
                Ok(())
            }
        },
    )
    .map_err(|_| Error::FsScanner)?;

    Ok(())
}

// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use fsscanner::fsscanner_mt;
use std::path::PathBuf;
use token_db::TokenDb;
use pdf_to_text_wrapper::error::{Error, Result};
use pdf_to_text_wrapper::process_text_to_token::process_join_token;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 4 {
        eprintln!("Usage: join_token_db <db_input_dir> <output_db_file> <input_extension>");
        std::process::exit(1);
    }

    let db_input_dir = &args[1];
    let output_db_file = &args[2];
    let input_extension = &args[3];
    let output_path = PathBuf::from(output_db_file);

    let db = fsscanner_mt::process_dir_state_and_map(
        TokenDb::default(),
        db_input_dir,
        db_input_dir,
        input_extension,
        "unused",
        {
            let output_path = output_path.clone();
            move |db, input, _| {
                if input == output_path {
                    return Ok(());
                }
                process_join_token(db, input)
                    .map_err(|_| fsscanner::Error::Callback("token merge failed"))
            }
        },
    ).map_err(|_| Error::FsScanner)?;

    db.save(output_db_file)?;
    Ok(())
}

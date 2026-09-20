use anyhow::Result;

use fsscanner::fsscanner_mt;
use pdf_to_text::process_text_to_token::process_join_token;
use token_db::TokenDb;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 4 {
        eprintln!("Usage: join_token_db <db_input_dir> <output_db_file> <input_extension>");
        std::process::exit(1);
    }

    let db_input_dir = &args[1];
    let output_db_file = &args[2];
    let input_extension = &args[3];

    let db = fsscanner_mt::process_dir_state_and_map(
        TokenDb::default(),
        db_input_dir,
        db_input_dir,
        input_extension,
        "unused",
        |db, input, _| process_join_token(db, input).map_err(Into::into),
    ).map_err(|error| anyhow::anyhow!(error.to_string()))?;

    db.save(output_db_file)?;
    Ok(())
}

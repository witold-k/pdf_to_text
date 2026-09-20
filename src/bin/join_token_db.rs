use anyhow::Result;

use fsscanner::fsscanner_mt;
use pdf_to_text::process_text_to_token::process_join_token;
use token_db::token_db::TokenDB;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 4 {
        eprintln!("Usage: pipeline <db_input_dir> <output_db_file>");
        std::process::exit(1);
    }

    let db_input_dir = &args[1];
    let _output_db_file = &args[2];
    let input_extension = &args[3];

    let _ = fsscanner_mt::process_dir_state_and_map(
        TokenDB::default(),
        db_input_dir,
        db_input_dir, // missing &str argument
        db_input_dir, // missing &str argument
        input_extension.as_str(),   // input extension
        |db, input, _| process_join_token(db, input).map_err(|e| e.into()),
    );

    Ok(())
}

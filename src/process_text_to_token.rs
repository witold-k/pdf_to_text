use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use token_db::TokenDb;
use simplelexer::chunkkind::ChunkKind;
use simplelexer::quotelexer::QuoteLexer;

pub fn tokenize(db: &mut TokenDb, text: &str) -> Result<Vec<u32>> {
    let lexer = QuoteLexer::new(text);

    lexer
        .lex()?
        .into_iter()
        .filter(|c| c.kind != ChunkKind::Whitespace)
        .map(|c| db.insert(c.text).map(|id| id.get()).map_err(Into::into))
        .collect()
}

/// Tokenizes one UTF-8 text file and writes both the token-ID stream and
/// the per-file token database.
pub fn process_text_to_token(input: &Path, token_output: &Path, db_output: &Path) -> Result<()> {
    println!("text -> token: {} -> {}", input.display(), token_output.display());

    let data = fs::read_to_string(input)
        .with_context(|| format!("failed to read {}", input.display()))?;
    let mut db = TokenDb::default();
    let tokenized = tokenize(&mut db, &data)?;
    let encoded = postcard::to_allocvec(&tokenized)
        .context("failed to serialize token IDs")?;

    fs::write(token_output, encoded)
        .with_context(|| format!("failed to write {}", token_output.display()))?;
    db.save(db_output)?;

    Ok(())
}

pub fn process_join_token(db: &mut TokenDb, input: &Path) -> Result<()> {
    let local_token = TokenDb::load(input)?;
    db.merge(&local_token)?;

    Ok(())
}

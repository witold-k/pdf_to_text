use anyhow::{Context, Result};
use simplelexer::chunkkind::ChunkKind;
use simplelexer::quotelexer::QuoteLexer;
use std::fs;
use std::path::Path;
use token_db::TokenDb;

pub fn tokenize(db: &mut TokenDb, text: &str) -> Result<Vec<u32>> {
    let lexer = QuoteLexer::new(text);

    lexer
        .lex()?
        .into_iter()
        .filter(|c| c.kind != ChunkKind::Whitespace)
        .map(|c| db.insert(c.text).map(|id| id.get()).map_err(Into::into))
        .collect()
}

/// Tokenizes one UTF-8 text file into a document-local token database.
///
/// The resulting artifact contains only this document's token vocabulary and
/// occurrence counts. Corpus-wide identity is established later by merging
/// all document artifacts.
pub fn process_text_to_token(
    input: &Path,
    token_output: &Path,
) -> Result<()> {
    println!("text -> token: {} -> {}", input.display(), token_output.display());

    let data = fs::read_to_string(input)
        .with_context(|| format!("failed to read {}", input.display()))?;
    let mut db = TokenDb::new();
    let _tokenized = tokenize(&mut db, &data)?;
    db.save(token_output)
        .with_context(|| format!("failed to write {}", token_output.display()))?;

    Ok(())
}


pub fn process_join_token(db: &mut TokenDb, input: &Path) -> Result<()> {
    let local_token = TokenDb::load(input)?;
    db.merge(&local_token)?;

    Ok(())
}

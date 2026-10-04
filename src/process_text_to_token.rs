use crate::error::{Error, Result};
use simplelexer::chunkkind::ChunkKind;
use simplelexer::quotelexer::QuoteLexer;
use std::fs;
use std::path::Path;
use token_db::{TokenDb, TokenId};

pub fn save_token_stream(path: &Path, tokens: &[TokenId]) -> Result<()> {
    let data = postcard::to_allocvec(tokens)?;
    fs::write(path, data)?;
    Ok(())
}

pub fn load_token_stream(path: &Path) -> Result<Vec<TokenId>> {
    let data = fs::read(path)?;
    Ok(postcard::from_bytes(&data)?)
}

pub fn tokenize(db: &mut TokenDb, text: &str) -> Result<Vec<TokenId>> {
    let lexer = QuoteLexer::new(text);

    lexer
        .lex()?
        .into_iter()
        .filter(|c| c.kind != ChunkKind::Whitespace)
        .map(|c| db.insert(c.text).map_err(Into::into))
        .collect()
}

/// Tokenizes one UTF-8 text file into a token stream and its document-local database.
pub fn process_text_to_token(
    input: &Path,
    token_output: &Path,
    db_output: &Path,
) -> Result<()> {
    println!(
        "text -> token: {} -> {}, {}",
        input.display(),
        token_output.display(),
        db_output.display()
    );

    let data = fs::read_to_string(input)?;
    let mut db = TokenDb::new();
    let tokens = tokenize(&mut db, &data)?;

    save_token_stream(token_output, &tokens)?;
    db.save(db_output)?;

    Ok(())
}

pub fn process_join_token(db: &mut TokenDb, input: &Path) -> Result<()> {
    let local_token = TokenDb::load(input)?;
    db.merge(&local_token)?;

    Ok(())
}

/// Rewrites a document-local token stream to corpus-global token IDs.
pub fn process_token_to_global(
    token_input: &Path,
    local_db_input: &Path,
    global_db: &TokenDb,
    global_token_output: &Path,
) -> Result<()> {
    let local_db = TokenDb::load(local_db_input)?;
    let local_to_global = local_db
        .iter()
        .map(|(_, entry)| {
            global_db
                .id(entry.text())
                .ok_or_else(|| Error::MissingGlobalToken(entry.text().to_owned()))
        })
        .collect::<Result<Vec<_>>>()?;

    let local_tokens = load_token_stream(token_input)?;
    let global_tokens = local_tokens
        .into_iter()
        .map(|id| {
            local_to_global
                .get(id.get() as usize)
                .copied()
                .ok_or(Error::InvalidTokenId(id))
        })
        .collect::<Result<Vec<_>>>()?;

    save_token_stream(global_token_output, &global_tokens)
}

use anyhow::Result;
use std::fs;
use std::path::Path;
use token_db::TokenDb;
use simplelexer::chunkkind::ChunkKind;
use simplelexer::quotelexer::QuoteLexer;

pub fn tokenize(db: &mut TokenDb, text: &str) -> Vec<usize> {
    let mut lexer = QuoteLexer::new(text);

    lexer
        .lex()
        .into_iter()
        .filter(|c| c.kind != ChunkKind::Whitespace)
        .map(|c| db.push(c.text))
        .collect()
}

///
/// output[0]:
/// output[1]:
pub fn process_text_to_token(input: &Path, output: &[&Path]) -> Result<()> {
    println!("text -> token: {} -> {}", input.display(), output[0].display());

    let data = fs::read_to_string(input)?;
    let mut db = TokenDb::default();
    let tokenized = tokenize(&mut db, data.as_str());
    let encoded = postcard::to_allocvec(&tokenized).unwrap();
    let _ = fs::write(output[0], encoded);

    db.save(output[1])?;
    Ok(())
}

pub fn process_join_token(db: &mut TokenDb, input: &Path) -> Result<()> {
    let local_token = TokenDb::load(input)?;
    db.join(&local_token);

    Ok(())
}

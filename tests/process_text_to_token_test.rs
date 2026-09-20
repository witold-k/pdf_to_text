use pdf_to_text::process_text_to_token::tokenize;
use token_db::TokenDb;

#[test]
fn tokenizes_valid_text() {
    let mut db = TokenDb::new();
    let tokens = tokenize(&mut db, "NAME = \"value\"").unwrap();

    assert!(!tokens.is_empty());
    assert!(!db.is_empty());
}

#[test]
fn rejects_malformed_quoted_input() {
    let mut db = TokenDb::new();
    let error = tokenize(&mut db, "NAME = \"unterminated").unwrap_err();

    assert!(error.to_string().contains("unterminated quoted string"));
}

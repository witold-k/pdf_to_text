use pdf_to_text::process_text_to_token::{
    load_token_stream, process_join_token, process_text_to_token, process_token_to_global,
    save_token_stream, tokenize,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use token_db::TokenDb;

fn test_dir(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("pdf_to_text_{name}_{}_{unique}", std::process::id()));
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn tokenizes_valid_text() {
    let mut db = TokenDb::new();
    let tokens = tokenize(&mut db, "NAME = \"value\"").unwrap();

    assert!(!tokens.is_empty());
    assert!(!db.is_empty());
}

#[test]
fn tokenization_preserves_document_order_and_repetition() {
    let mut db = TokenDb::new();
    let tokens = tokenize(&mut db, "a a b a").unwrap();

    let a = db.id("a").unwrap();
    let b = db.id("b").unwrap();
    assert_eq!(tokens, vec![a, a, b, a]);
    assert_eq!(db.get_by_token("a").unwrap().count(), 3);
    assert_eq!(db.get_by_token("b").unwrap().count(), 1);
}

#[test]
fn process_writes_separate_token_stream_and_lookup_database() {
    let dir = test_dir("artifacts");
    let input = dir.join("document.md");
    let token_output = dir.join("document.tok");
    let db_output = dir.join("document.tdb");
    fs::write(&input, "a a b a").unwrap();

    process_text_to_token(&input, &token_output, &db_output).unwrap();

    let tokens = load_token_stream(&token_output).unwrap();

    let db = TokenDb::load(&db_output).unwrap();
    let a = db.id("a").unwrap();
    let b = db.id("b").unwrap();
    assert_eq!(tokens, vec![a, a, b, a]);

    assert_eq!(db.get_by_token("a").unwrap().count(), 3);
    assert_eq!(db.get_by_token("b").unwrap().count(), 1);

    assert!(TokenDb::load(&token_output).is_err());

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn rejects_malformed_quoted_input() {
    let mut db = TokenDb::new();
    let error = tokenize(&mut db, "NAME = \"unterminated").unwrap_err();

    assert!(error.to_string().contains("unterminated quoted string"));
}

#[test]
fn join_merges_document_databases_not_token_streams() {
    let dir = test_dir("merge");
    let first_path = dir.join("first.tdb");
    let second_path = dir.join("second.tdb");

    let mut first = TokenDb::new();
    first.insert("a").unwrap();
    first.insert("a").unwrap();
    first.save(&first_path).unwrap();

    let mut second = TokenDb::new();
    second.insert("a").unwrap();
    second.insert("b").unwrap();
    second.save(&second_path).unwrap();

    let mut merged = TokenDb::new();
    process_join_token(&mut merged, &first_path).unwrap();
    process_join_token(&mut merged, &second_path).unwrap();

    assert_eq!(merged.get_by_token("a").unwrap().count(), 3);
    assert_eq!(merged.get_by_token("b").unwrap().count(), 1);

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn global_stream_maps_local_ids_through_token_text() {
    let dir = test_dir("global");
    let local_db_path = dir.join("document.tdb");
    let local_tok_path = dir.join("document.tok");
    let global_tok_path = dir.join("document_glob.tok");

    let mut local_db = TokenDb::new();
    let b = local_db.insert("b").unwrap();
    let a = local_db.insert("a").unwrap();
    local_db.save(&local_db_path).unwrap();
    save_token_stream(&local_tok_path, &[b, a, b]).unwrap();

    let mut global_db = TokenDb::new();
    let a_global = global_db.insert("a").unwrap();
    let b_global = global_db.insert("b").unwrap();

    process_token_to_global(
        &local_tok_path,
        &local_db_path,
        &global_db,
        &global_tok_path,
    )
    .unwrap();

    assert_eq!(
        load_token_stream(&global_tok_path).unwrap(),
        vec![b_global, a_global, b_global]
    );

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn global_stream_rejects_unknown_local_id() {
    let dir = test_dir("invalid_global");
    let local_db_path = dir.join("document.tdb");
    let local_tok_path = dir.join("document.tok");
    let global_tok_path = dir.join("document_glob.tok");

    let mut local_db = TokenDb::new();
    local_db.insert("a").unwrap();
    local_db.save(&local_db_path).unwrap();
    // TokenId is transparently encoded as its u32 index. Write an invalid
    // index directly to verify that the stream is checked against local_db.
    fs::write(&local_tok_path, postcard::to_allocvec(&[1_u32]).unwrap()).unwrap();

    let mut global_db = TokenDb::new();
    global_db.insert("a").unwrap();

    assert!(
        process_token_to_global(
            &local_tok_path,
            &local_db_path,
            &global_db,
            &global_tok_path,
        )
        .is_err()
    );

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn global_stream_rejects_token_missing_from_global_database() {
    let dir = test_dir("missing_global_token");
    let local_db_path = dir.join("document.tdb");
    let local_tok_path = dir.join("document.tok");
    let global_tok_path = dir.join("document_glob.tok");

    let mut local_db = TokenDb::new();
    let local_id = local_db.insert("local-only").unwrap();
    local_db.save(&local_db_path).unwrap();
    save_token_stream(&local_tok_path, &[local_id]).unwrap();

    let global_db = TokenDb::new();

    let error = process_token_to_global(
        &local_tok_path,
        &local_db_path,
        &global_db,
        &global_tok_path,
    )
    .unwrap_err();

    assert!(
        error
            .to_string()
            .contains("merged token database is missing token")
    );
    assert!(!global_tok_path.exists());

    fs::remove_dir_all(dir).unwrap();
}

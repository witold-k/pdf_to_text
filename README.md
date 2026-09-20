# pdf_to_text

Batch pipeline for converting PDF documents into structured text and token data.

The repository is intentionally small. It connects three existing building blocks:

- [GROBID](https://github.com/kermitt2/grobid) for PDF -> TEI extraction,
- `fsscanner` for parallel directory traversal and output mapping,
- `token_db` for stable token IDs and token-frequency databases.

The current pipeline is aimed at corpus preparation rather than end-user document conversion.

## Pipeline

```text
PDF directory
    |
    v
GROBID
    |
    v
TEI XML
    |
    v
structured JSON
    |
    v
simplelexer
    |
    +--> token ID stream (.tok)
    |
    +--> per-document token database (.tdb)

per-document .tdb files
    |
    v
join_token_db
    |
    v
merged token database
```

## Requirements

- Rust toolchain compatible with edition 2024.
- A running GROBID service at:
  `http://localhost:8070/api/processFulltextDocument`
- `jq` when using the coverage helpers from the `Justfile`.
- The local `simplelexer` dependency currently expected at `../simplelexer`.

## Build and test

```bash
just build
```

This runs:

```bash
cargo build
cargo test
cargo clippy
```

## PDF pipeline

```bash
cargo run --bin pdf_to_text -- <pdf_input_dir> <output_dir>
```

For every PDF, stage 1 writes structured GROBID-derived JSON below:

```text
<output_dir>/text/
```

while preserving the input directory layout.

Stage 2 reads those JSON files and creates:

- `.tok`: a Postcard-encoded `Vec<u32>` of token IDs,
- `.tdb`: the per-document `token_db::TokenDb` used to assign those IDs.

The token IDs in a `.tok` file are local to the matching `.tdb` file.

## Joining token databases

```bash
cargo run --bin join_token_db -- <db_input_dir> <output_db_file> <input_extension>
```

Example:

```bash
cargo run --bin join_token_db -- corpus/text corpus/token_db.tdb tdb
```

The command scans all files with the requested extension, merges their token counts into one `TokenDb`, and saves the result to `<output_db_file>`.

Note that merging token databases can change token IDs. The current command only creates the merged database; it does not rewrite existing `.tok` streams to the merged ID space.

## Structured PDF output

The JSON extracted from GROBID currently contains:

- title,
- authors,
- abstract,
- body sections with headings and text,
- bibliography entries.

Despite the repository name, this stage intentionally keeps document structure instead of flattening everything into plain text.

## Design notes

- Directory-processing errors are propagated instead of silently ignored.
- Token serialization and filesystem errors are returned to the caller.
- `token_db` owns token identity and persistence; this repository only prepares data for it.
- Matrix construction, weighting, SVD, embeddings, and other downstream analysis do not belong here.

## Status

This is a corpus-preparation utility under active development. The data formats and CLI may still change as the downstream text/SVD experiments take shape.

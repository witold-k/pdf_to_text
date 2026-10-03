# pdf_to_text

`pdf_to_text` is a small orchestration tool for preparing a local PDF corpus.

Its job is deliberately limited: connect existing components, map files between
pipeline stages, and keep their outputs in a predictable layout. PDF extraction,
filesystem scanning, lexing, and token-database behavior belong to the
specialized components that implement them. The small `.tok` stream is a
pipeline artifact owned by this orchestrator and uses the existing serializer.

## Components and responsibilities

- **GROBID / MinerU** extract usable document content from PDFs. Their
  backend-specific artifacts stay together in a per-document directory.
  GROBID's native TEI response is retained there; `pdf2json` projects it into
  structured JSON and the canonical text consumed by the token pipeline.
- **fsscanner** discovers files, preserves directory layout, maps input paths to
  output paths, and controls parallel processing. `pdf_to_text` does not
  implement its own directory walker or worker pool.
- **simplelexer** splits extracted UTF-8 text into lexical chunks. Whitespace
  chunks are discarded; the remaining chunk text is passed to `token_db`.
  `simplelexer` is intentionally a small lexer, not an NLP tokenizer. The
  choice of lexer/tokenizer is a pipeline component rather than PDF extraction
  logic.
- **token_db** owns token identity, token-database persistence, and merging.
  Tokenization uses `token_db` to build a document-local `.tdb` lookup database
  and records the returned local IDs, in document order, as the per-document
  `.tok` stream. The corpus-wide `token_db.tdb` is built by merging all
  document-local `.tdb` databases.

The repository should avoid reimplementing functionality that belongs in one of
these components. In particular, matrix construction, TF-IDF/BM25 weighting,
SVD/LSA, embeddings, and search belong downstream.

## Pipeline

```text
PDF corpus
    |
    +--> GROBID or MinerU
            |
            +--> backend artifacts per PDF
            |
            +--> canonical extracted text
                        |
                        +--> simplelexer / token_db
                                |
                                +--> per-document .tok  (ordered token IDs)
                                +--> per-document .tdb  (local ID lookup + counts)

all per-document .tdb files
    |
    +--> token_db merge
            |
            +--> token_db.tdb
                    |
                    +--> each document .tok + .tdb
                            |
                            +--> per-document *_glob.tok (corpus-global IDs)
```

The stages form a build-style dependency chain:

```text
.pdf  ->  .md  ->  (.tok + .tdb)
                       |
all document .tdb -----+--> token_db.tdb
                              |
(document.tok + document.tdb)-+--> document_glob.tok
```

A `.tok` file is the ordered stream of document-local token IDs. It preserves
token order and repetition and is interpreted together with the matching document
`.tdb`.

A document `.tdb` is the lookup database for that stream. It maps local token IDs
to token text and stores occurrence counts. It can be produced independently for
each document.

The corpus `token_db.tdb` is rebuilt by loading and merging all document-local
`.tdb` files with `TokenDb::merge`. The `.tok` streams are not merged.

After the corpus database exists, a final pipeline stage translates every
`document.tok` from document-local IDs to the IDs of `token_db.tdb`. For each
local ID, the matching `document.tdb` supplies the token text and `token_db.tdb`
supplies that token's corpus-global ID. The translated stream is written next
to the original as `document_glob.tok`. The local `.tok` is retained unchanged.

Consequently the incremental rules are:

- rebuild extracted text when its PDF is newer;
- rebuild both a document's `.tok` and `.tdb` when its extracted text is newer,
  or when either artifact is missing;
- rebuild `token_db.tdb` when it is missing or any document-local `.tdb` is newer;
- rebuild `document_glob.tok` only when `token_db.tdb` is newer (or the global
  stream is missing). In the normal pipeline, `.tok` and `.tdb` are regenerated
  together; the updated document `.tdb` then rebuilds `token_db.tdb`. The merged
  database is therefore the single freshness boundary for this final stage.

For GROBID, a corpus containing `bitcoin.pdf` is intended to contain:

```text
<output>/
├── bitcoin/
│   ├── bitcoin.tei.xml
│   ├── bitcoin.json
│   └── bitcoin.md
├── bitcoin.md -> bitcoin/bitcoin.md
├── bitcoin.tok
├── bitcoin.tdb
├── bitcoin_glob.tok
└── token_db.tdb
```

The outer `bitcoin.md` path remains the stable interface between extraction and
tokenization. For GROBID, Markdown is not a native backend artifact: GROBID's
TEI XML is retained unchanged, `pdf2json` creates the simplified document model
stored as JSON, and the same model supplies the canonical text written as `.md`.
This keeps the rich extraction available without exposing backend-specific data
to the token pipeline.

## Token artifacts and database

`token_db` has one responsibility here: token identity, counts, merging, and
persistence of `TokenDb` data (`.tdb`). It does not know about `.tok` files.

The ordered `.tok` stream is a `pdf_to_text` pipeline artifact. It is only the
serialized sequence of document-local token IDs returned while filling the
matching `TokenDb`; its persistence deliberately stays small and uses the
already present `postcard` serializer rather than introducing another storage
format or pushing a second responsibility into `token_db`.

This ownership is intentional even though `pdf_to_text` should otherwise contain
as little functionality as possible: the stream describes document order in this
pipeline, not token-database state. Moving `.tok` persistence into `token_db`
would give that crate a second data model and responsibility merely because the
stream happens to contain `TokenId` values.

For each processed document the two outputs therefore form a pair with separate
responsibilities:

- `document.tok`: owned by `pdf_to_text`; ordered, repeated document-local IDs;
- `document.tdb`: owned by `token_db`; local ID-to-token lookup and counts.

The IDs in `document.tok` are interpreted against that document's
`document.tdb`. They are not corpus-global IDs. The corpus database
`token_db.tdb` is produced only from the document-local `.tdb` files; `.tok`
files are neither inputs to `TokenDb::merge` nor rewritten when databases are
merged.

Once the merge is complete, `pdf_to_text` derives `document_glob.tok` from the
triple `document.tok + document.tdb + token_db.tdb`. The `_glob.tok` stream has
the same length, order, and repetitions as the local `.tok`; only its numeric
IDs are replaced by their corpus-global IDs. These generated files live on the
same directory level as their corresponding `.tok` files and can therefore be
consumed downstream without the document-local lookup database when only the
corpus-global token space is needed.

## Backend concurrency

PDF extraction concurrency is configured independently for each backend in
`~/.config/pdf_to_text/config.json`:

```json
{
  "grobid": {
    "threads": 1
  },
  "mineru": {
    "threads": 1
  }
}
```

Both default to one worker. This is particularly important for MinerU because
multiple simultaneous local VLM/vLLM instances can consume substantial GPU
memory. The worker limit is passed to `fsscanner`; scheduling itself remains a
`fsscanner` responsibility.

## Running

Use the configured default backend:

```sh
pdf_to_text <pdf_input_dir> <output_dir>
```

or select one explicitly:

```sh
pdf_to_text mineru <pdf_input_dir> <output_dir>
pdf_to_text grobid <pdf_input_dir> <output_dir>
```

Backend services can be started with:

```sh
pdf_to_text start mineru
pdf_to_text start grobid
```

## Build and test

```sh
just build
```

The `build` recipe runs `cargo build`, `cargo test`, and `cargo clippy`, so it is
the normal pre-merge check for this repository.

Tests live under `tests/`, mirror the relevant `src/` hierarchy, and use the
`_test.rs` suffix. For example, tests for `src/process_text_to_token.rs` belong
in `tests/process_text_to_token_test.rs`.

## Design direction

`pdf_to_text` is an orchestrator, not a framework.

A useful rule for future changes is: if functionality can naturally live in
GROBID, MinerU, `fsscanner`, `simplelexer`, `token_db`, or a downstream
search/matrix crate, it should live there rather than grow a second
implementation here. This repository should mainly express the pipeline and
the contracts between its stages.

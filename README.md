# pdf_to_text

`pdf_to_text` is a small orchestration tool for preparing a local PDF corpus.

Its job is deliberately limited: connect existing components, map files between
pipeline stages, and keep their outputs in a predictable layout. PDF extraction,
filesystem scanning, lexing, token identity, and persistence belong to the
specialized components that implement them.

## Components and responsibilities

- **GROBID / MinerU** extract usable document content from PDFs. Their
  backend-specific artifacts stay together in a per-document directory.
- **fsscanner** discovers files, preserves directory layout, maps input paths to
  output paths, and controls parallel processing. `pdf_to_text` does not
  implement its own directory walker or worker pool.
- **simplelexer** splits extracted UTF-8 text into lexical chunks. Whitespace
  chunks are discarded; the remaining chunk text is passed to `token_db`.
  `simplelexer` is intentionally a small lexer, not an NLP tokenizer. The
  choice of lexer/tokenizer is a pipeline component rather than PDF extraction
  logic.
- **token_db** owns token identity, token-database persistence, and merging.
  Tokenization produces one independent per-document `.tok` artifact. The
  corpus-wide `token_db.tdb` is then built by merging the token information
  from all document-local `.tok` artifacts.

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
                                +--> per-document .tok

all per-document .tok files
    |
    +--> token_db merge
            |
            +--> token_db.tdb
```

The stages form a build-style dependency chain:

```text
.pdf  ->  .md  ->  .tok  ->  token_db.tdb
          cpp -> object      objects -> linked result
```

A `.tok` file is the **individual document token artifact**. It must be
self-contained enough to be produced and kept independently of the merged
corpus database. `token_db.tdb` is the **merged corpus token database** built
from all current `.tok` files.

Consequently the intended incremental rules are the same as for compiled
artifacts:

- rebuild a document's extracted output when its PDF is newer;
- rebuild only that document's `.tok` when its extracted text is newer;
- rebuild `token_db.tdb` when it is missing or any `.tok` is newer.

When rebuilding `token_db.tdb`, all current `.tok` files participate in the
merge, just as a linker consumes all required object files even when only one
object caused relinking.

The important distinction is that `.tok` and `.tdb` are **not MinerU
outputs**. MinerU owns only the files inside its per-document artifact
directory. The token artifact is produced by the tokenization stage, and the
single `token_db.tdb` represents the merged token state for the whole corpus.

## Output layout

For MinerU, a corpus containing `bitcoin.pdf` is intended to look like:

```text
<output>/
├── bitcoin/
│   └── bitcoin.md          # MinerU artifact (plus any other MinerU artifacts)
├── bitcoin.md -> bitcoin/bitcoin.md
├── bitcoin.tok             # independent token artifact for this document
└── token_db.tdb            # one shared token database for the corpus
```

`<output_dir>` is used directly; `pdf_to_text` does not append an extra `text/` directory.\n\nThe outer `.md` path is the stable interface between extraction and
tokenization. The symlink is relative so the output tree can be moved as a
unit. GROBID follows the same separation of backend artifacts from downstream
token data, although its native primary format is not Markdown and its final
canonical-output handling is still being refined.

## Token artifacts and database

There is exactly one `.tok` artifact per processed document and exactly one
merged `token_db.tdb` per corpus.

A `.tok` file is a persisted document-local `TokenDb`: it contains the
unique tokens and occurrence counts for exactly one document. It therefore
does not depend on IDs from an already merged `token_db.tdb`.

The corpus `token_db.tdb` uses the same `TokenDb` persistence format, but
represents a different level of the pipeline: it is rebuilt by loading all
current `.tok` files and merging them with `TokenDb::merge`.

`token_db` is the authority for the document token representation, assigning
final IDs, counting tokens, merging document token data, and reading/writing
the binary database format. `pdf_to_text` should only orchestrate these
stages and their timestamp dependencies.

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

The project uses the repository convention that tests live under `tests/`,
mirror the relevant `src/` hierarchy, and use the `_test.rs` suffix.

## Design direction

`pdf_to_text` is an orchestrator, not a framework.

A useful rule for future changes is: if functionality can naturally live in
GROBID, MinerU, `fsscanner`, `simplelexer`, `token_db`, or a downstream
search/matrix crate, it should live there rather than grow a second
implementation here. This repository should mainly express the pipeline and
the contracts between its stages.

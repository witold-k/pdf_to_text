use std::fmt;
use std::io;
use std::process::ExitStatus;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Json(serde_json::Error),
    Xml(roxmltree::Error),
    Lex(simplelexer::error::LexError),
    TokenDb(token_db::Error),
    Postcard(postcard::Error),
    Http(ureq::Error),
    InvalidBackend,
    InvalidPath(&'static str),
    InvalidTokenId(token_db::TokenId),
    MissingGlobalToken(String),
    NoPrimaryArtifact,
    MultiplePrimaryArtifacts,
    MineruFailed(ExitStatus),
    ServiceFailed(ExitStatus),
    FsScanner,
    UnsupportedPlatform,
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "I/O error: {error}"),
            Self::Json(error) => write!(f, "JSON error: {error}"),
            Self::Xml(error) => write!(f, "XML error: {error}"),
            Self::Lex(error) => write!(f, "lexer error: {error}"),
            Self::TokenDb(error) => write!(f, "token database error: {error}"),
            Self::Postcard(error) => write!(f, "token stream serialization error: {error}"),
            Self::Http(error) => write!(f, "HTTP error: {error}"),
            Self::InvalidBackend => write!(f, "unknown PDF backend; expected 'grobid' or 'mineru'"),
            Self::InvalidPath(message) => write!(f, "invalid path: {message}"),
            Self::InvalidTokenId(id) => write!(f, "token stream contains unknown local token ID {}", id.get()),
            Self::MissingGlobalToken(token) => write!(f, "merged token database is missing token {token:?}"),
            Self::NoPrimaryArtifact => write!(f, "backend produced no primary artifact"),
            Self::MultiplePrimaryArtifacts => write!(f, "backend produced multiple primary artifacts"),
            Self::MineruFailed(status) => write!(f, "MinerU failed with status {status}"),
            Self::ServiceFailed(status) => write!(f, "service exited with status {status}"),
            Self::FsScanner => write!(f, "filesystem scanner failed"),
            Self::UnsupportedPlatform => write!(f, "canonical output links are supported only on Unix"),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self { Self::Io(error) }
}
impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self { Self::Json(error) }
}
impl From<roxmltree::Error> for Error {
    fn from(error: roxmltree::Error) -> Self { Self::Xml(error) }
}
impl From<simplelexer::error::LexError> for Error {
    fn from(error: simplelexer::error::LexError) -> Self { Self::Lex(error) }
}
impl From<token_db::Error> for Error {
    fn from(error: token_db::Error) -> Self { Self::TokenDb(error) }
}
impl From<postcard::Error> for Error {
    fn from(error: postcard::Error) -> Self { Self::Postcard(error) }
}
impl From<ureq::Error> for Error {
    fn from(error: ureq::Error) -> Self { Self::Http(error) }
}

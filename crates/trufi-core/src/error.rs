use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("rule pack `{pack}` is invalid: {message}")]
    InvalidRulePack { pack: String, message: String },

    #[error("unknown rule `{0}`")]
    UnknownRule(String),

    #[error("operation is not supported on this platform: {0}")]
    Unsupported(&'static str),

    #[error("administrator rights are required")]
    ElevationRequired,

    #[error("{0}")]
    Platform(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

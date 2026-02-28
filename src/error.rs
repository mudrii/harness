use thiserror::Error;

#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum HarnessError {
    #[error("not a git repository: {0}")]
    NotGitRepo(String),

    #[error("config file not found: {0}")]
    ConfigNotFound(String),

    #[error("config parse error: {0}")]
    ConfigParse(String),

    #[error("path does not exist: {0}")]
    PathNotFound(String),

    #[error("invalid profile target: {0}")]
    InvalidProfileTarget(String),

    #[error("bucket penalty exceeded maximum: {0}")]
    BucketPenaltyExceeded(String),

    #[error("forbidden tool access attempt: {0}")]
    ForbiddenToolAccess(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("toml parse error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, HarnessError>;

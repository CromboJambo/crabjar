use thiserror::Error;

#[derive(Error, Debug)]
pub enum TrainExtractError {
    #[error("database error: {0}")]
    Database(String),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("empty dataset: no entries matched filters")]
    EmptyDataset,

    #[error("export failed: {0}")]
    Export(String),

    #[error("invalid path: {0}")]
    InvalidPath(String),
}

impl From<rusqlite::Error> for TrainExtractError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Database(e.to_string())
    }
}

pub type TrainExtractResult<T> = Result<T, TrainExtractError>;

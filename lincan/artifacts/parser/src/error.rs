use std::io;
use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("cli error: {0}")]
    Cli(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("io error at {path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("cargo metadata error: {0}")]
    CargoMetadata(String),
    #[error("parse error in {path}: {message}")]
    ParseFile { path: PathBuf, message: String },
    #[error("serialization error: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("extraction error: {0}")]
    Extract(String),
}

#[derive(Debug, serde::Serialize)]
struct JsonError<'a> {
    kind: &'a str,
    message: String,
}

impl AppError {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Cli(_) => "cli_error",
            Self::InvalidInput(_) => "invalid_input",
            Self::Io { .. } => "io_error",
            Self::CargoMetadata(_) => "cargo_metadata_error",
            Self::ParseFile { .. } => "parse_error",
            Self::Serialize(_) => "serialize_error",
            Self::Extract(_) => "extract_error",
        }
    }
}

pub fn emit_stderr(err: &AppError) {
    eprintln!("error: {err}");
    let payload = JsonError {
        kind: err.kind(),
        message: err.to_string(),
    };
    match serde_json::to_string(&payload) {
        Ok(line) => eprintln!("{line}"),
        Err(ser_err) => eprintln!(
            "{{\"kind\":\"serialize_error\",\"message\":\"failed to serialize error payload: {ser_err}\"}}"
        ),
    }
}

use std::fs;
use std::path::Path;

use crate::error::AppError;
use crate::model::RustMapOutput;

pub fn write_output(path: &Path, data: &RustMapOutput) -> Result<(), AppError> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|source| AppError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let serialized = serde_json::to_string_pretty(data)?;
    fs::write(path, serialized).map_err(|source| AppError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(())
}

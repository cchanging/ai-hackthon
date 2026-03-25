pub mod cli;
pub mod emit;
pub mod error;
pub mod extract;
pub mod model;
pub mod workspace;

use std::path::Path;

use error::AppError;
use model::RustMapOutput;

pub fn run(input: &Path, output: &Path) -> Result<RustMapOutput, AppError> {
    let snapshot = workspace::load(input)?;
    let extracted = extract::extract_workspace(&snapshot)?;
    emit::write_output(output, &extracted)?;
    Ok(extracted)
}

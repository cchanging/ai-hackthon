use std::process::ExitCode;

fn main() -> ExitCode {
    match run_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            rustmap::error::emit_stderr(&err);
            ExitCode::from(1)
        }
    }
}

fn run_main() -> Result<(), rustmap::error::AppError> {
    let args = rustmap::cli::parse_from_env()?;
    rustmap::run(&args.input, &args.output)?;
    Ok(())
}

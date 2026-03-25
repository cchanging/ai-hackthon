use std::env;
use std::ffi::OsString;
use std::path::PathBuf;

use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct CliArgs {
    pub input: PathBuf,
    pub output: PathBuf,
}

pub fn parse_from_env() -> Result<CliArgs, AppError> {
    let args = env::args_os().skip(1).collect::<Vec<_>>();
    parse_from_args(args)
}

pub fn parse_from_args(mut args: Vec<OsString>) -> Result<CliArgs, AppError> {
    if matches!(args.first(), Some(first) if first == "rustmap") {
        args.remove(0);
    }

    let mut input: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;
    let mut idx = 0usize;
    while idx < args.len() {
        let token = args[idx].to_string_lossy();
        if token == "--output" {
            let Some(value) = args.get(idx + 1) else {
                return Err(AppError::Cli(
                    "missing value for --output. usage: cargo rustmap [path] [--output <file>]"
                        .to_string(),
                ));
            };
            output = Some(PathBuf::from(value));
            idx += 2;
            continue;
        }
        if token.starts_with("--") {
            return Err(AppError::Cli(format!(
                "unknown flag `{token}`. usage: cargo rustmap [path] [--output <file>]"
            )));
        }

        if input.is_none() {
            input = Some(PathBuf::from(&args[idx]));
            idx += 1;
            continue;
        }
        return Err(AppError::Cli(
            "too many positional arguments. usage: cargo rustmap [path] [--output <file>]"
                .to_string(),
        ));
    }

    Ok(CliArgs {
        input: input.unwrap_or_else(|| PathBuf::from(".")),
        output: output.unwrap_or_else(|| PathBuf::from("output/rustmap.json")),
    })
}

#[cfg(test)]
mod tests {
    use super::parse_from_args;
    use std::ffi::OsString;
    use std::path::PathBuf;

    #[test]
    fn parse_defaults() {
        let parsed = parse_from_args(Vec::new()).expect("parse should succeed");
        assert_eq!(parsed.input, PathBuf::from("."));
        assert_eq!(parsed.output, PathBuf::from("output/rustmap.json"));
    }

    #[test]
    fn parse_with_subcommand_prefix() {
        let parsed = parse_from_args(vec![
            OsString::from("rustmap"),
            OsString::from("examples/workspace_demo"),
            OsString::from("--output"),
            OsString::from("tmp/out.json"),
        ])
        .expect("parse should succeed");
        assert_eq!(parsed.input, PathBuf::from("examples/workspace_demo"));
        assert_eq!(parsed.output, PathBuf::from("tmp/out.json"));
    }
}

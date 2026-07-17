//! Argument parsing for the MCP server entry point.

use anyhow::{bail, Result};
use std::path::PathBuf;

const USAGE: &str = "Usage: cytoscnpy mcp-server [--root <PATH>]";

pub(crate) fn parse_root(args: &[String]) -> Result<Option<PathBuf>> {
    match args {
        [] => Ok(None),
        [flag, root] if flag == "--root" && !root.is_empty() => Ok(Some(PathBuf::from(root))),
        [flag] if flag == "--root" => bail!("Missing value for --root. {USAGE}"),
        _ => bail!("Invalid MCP server arguments. {USAGE}"),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_root;
    use std::path::PathBuf;

    #[test]
    fn defaults_to_process_working_directory() -> anyhow::Result<()> {
        assert_eq!(parse_root(&[])?, None);
        Ok(())
    }

    #[test]
    fn accepts_explicit_root() -> anyhow::Result<()> {
        let args = ["--root".to_owned(), "project".to_owned()];
        assert_eq!(parse_root(&args)?, Some(PathBuf::from("project")));
        Ok(())
    }

    #[test]
    fn rejects_missing_root_value() -> anyhow::Result<()> {
        match parse_root(&["--root".to_owned()]) {
            Err(error) => assert!(error.to_string().contains("Missing value")),
            Ok(value) => anyhow::bail!("expected missing root error, got {value:?}"),
        }
        Ok(())
    }

    #[test]
    fn rejects_unknown_arguments() -> anyhow::Result<()> {
        match parse_root(&["--unknown".to_owned()]) {
            Err(error) => assert!(error.to_string().contains("Invalid MCP server arguments")),
            Ok(value) => anyhow::bail!("expected invalid argument error, got {value:?}"),
        }
        Ok(())
    }
}

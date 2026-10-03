//! The CLI exposes one unified `DeSlopify` assessment command.
#![allow(clippy::unwrap_used)]

use clap::Parser;
use cytoscnpy::entry_point::run_with_args_to;

#[test]
fn top_level_help_focuses_on_primary_commands() {
    let mut buffer = Vec::new();
    let code = run_with_args_to(vec!["--help".to_owned()], &mut buffer).unwrap();
    assert_eq!(code, 0);
    let help = String::from_utf8(buffer).unwrap();
    let commands = help
        .split("Commands:")
        .nth(1)
        .unwrap()
        .split("Options:")
        .next()
        .unwrap();

    for name in [
        "deslop", "raw", "cc", "hal", "mi", "stats", "files", "deps", "init",
    ] {
        assert!(
            commands
                .lines()
                .any(|line| line.trim_start().starts_with(&format!("{name} "))),
            "missing {name}"
        );
    }
    for name in [
        "score",
        "graph",
        "context",
        "doctor",
        "searchability",
        "naming",
        "todos",
        "globals",
        "exceptions",
        "wildcards",
        "side-effects",
        "singletons",
        "anti-patterns",
        "duplicates",
        "unreferenced",
        "functions",
    ] {
        assert!(
            !commands
                .lines()
                .any(|line| line.trim_start().starts_with(&format!("{name} "))),
            "unexpected {name}"
        );
    }
}

#[test]
fn standalone_analysis_commands_are_removed() {
    for name in [
        "score",
        "graph",
        "context",
        "doctor",
        "searchability",
        "naming",
        "todos",
        "globals",
        "exceptions",
        "wildcards",
        "side-effects",
        "singletons",
        "anti-patterns",
        "duplicates",
        "unreferenced",
        "functions",
    ] {
        let cli = cytoscnpy::cli::Cli::try_parse_from(["cytoscnpy", name]).unwrap();
        assert!(cli.command.is_none(), "{name} should not be a command");
    }
}

//! Subcommand dispatcher for code metric commands (raw, cc, hal, mi).

use anyhow::Result;
use std::io::Write;

use super::super::handlers::{handle_cc, handle_hal, handle_mi, handle_raw, CcFlags, MiFlags};
use super::super::run::RuntimeContext;
use crate::cli::Commands;

pub(super) fn handle_metric_command<W: Write>(
    command: Commands,
    context: &RuntimeContext,
    verbose: bool,
    writer: &mut W,
) -> Result<i32> {
    match command {
        Commands::Raw { common, summary } => handle_raw(
            common,
            summary,
            &context.exclude_folders,
            &context.analysis_root,
            context.include_tests,
            verbose,
            writer,
        ),
        Commands::Cc {
            common,
            rank,
            average,
            total_average,
            show_complexity,
            order,
            no_assert,
            xml,
            fail_threshold,
        } => handle_cc(
            common,
            rank,
            CcFlags {
                average,
                total_average,
                show_complexity,
                order,
                no_assert,
                xml,
                fail_threshold,
            },
            &context.exclude_folders,
            &context.analysis_root,
            context.include_tests,
            verbose,
            writer,
        ),
        Commands::Hal { common, functions } => handle_hal(
            common,
            functions,
            &context.exclude_folders,
            &context.analysis_root,
            context.include_tests,
            verbose,
            writer,
        ),
        Commands::Mi {
            common,
            rank,
            multi,
            show,
            average,
            fail_threshold,
        } => handle_mi(
            common,
            rank,
            MiFlags {
                multi,
                show_hooks: show,
                average,
                fail_threshold,
            },
            &context.exclude_folders,
            &context.analysis_root,
            context.include_tests,
            verbose,
            writer,
        ),
        _ => unreachable!("handle_metric_command called with non-metric command"),
    }
}

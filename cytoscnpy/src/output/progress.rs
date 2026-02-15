use colored::Colorize;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use std::io::Write;
use std::time::Duration;

/// Print the exclusion list in styled format.
///
/// # Errors
///
/// Returns an error if writing to the output fails.
pub fn print_exclusion_list(writer: &mut impl Write, folders: &[String]) -> std::io::Result<()> {
    if folders.is_empty() {
        let defaults = crate::constants::DEFAULT_EXCLUDE_FOLDERS();
        let mut sorted_defaults: Vec<&str> = defaults.iter().copied().collect();
        sorted_defaults.sort_unstable();
        let list = sorted_defaults.join(", ");
        writeln!(
            writer,
            "{} {}",
            "[OK] Using default exclusions only:".green(),
            list.dimmed()
        )?;
    } else {
        let list = folders
            .iter()
            .map(std::string::String::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(writer, "{} {}", "Excluding:".yellow().bold(), list)?;
    }
    Ok(())
}

/// Create and return a spinner for analysis (used when file count is unknown).
///
/// In test mode, returns a hidden progress bar to avoid polluting test output.
///
/// # Panics
///
/// Panics if the progress style template is invalid (should never happen with hardcoded template).
#[must_use]
pub fn create_spinner() -> ProgressBar {
    if cfg!(test) {
        return ProgressBar::hidden();
    }

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.cyan} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    spinner.set_message("CytoScnPy analyzing your code…");
    spinner.enable_steady_tick(Duration::from_millis(100));
    spinner
}

/// Create a progress bar with file count (used when total files is known).
///
/// In test mode, returns a hidden progress bar to avoid polluting test output.
///
/// # Panics
///
/// Panics if the progress style template is invalid (should never happen with hardcoded template).
#[must_use]
pub fn create_progress_bar(total_files: u64) -> ProgressBar {
    if cfg!(test) {
        return ProgressBar::hidden();
    }

    let pb =
        ProgressBar::with_draw_target(Some(total_files), ProgressDrawTarget::stderr_with_hz(20));
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.cyan} [{bar:40.cyan/blue}] {pos}/{len} files ({percent}%) {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_bar())
            .progress_chars("█▓░"),
    );
    pb.set_message("analyzing...");
    pb.enable_steady_tick(Duration::from_millis(100));
    pb.tick();
    pb
}

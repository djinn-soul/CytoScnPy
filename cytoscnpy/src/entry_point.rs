mod config;
mod detection;
mod handlers;
mod paths;
mod run;

pub use detection::detect_entry_point_calls;
pub use run::{run_with_args, run_with_args_to};

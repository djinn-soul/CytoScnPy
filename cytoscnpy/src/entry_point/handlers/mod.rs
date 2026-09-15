pub(crate) mod analysis;
pub(crate) mod context;
pub(crate) mod deps;
mod deslop;
pub(crate) mod doctor;
pub(crate) mod graph;
mod metrics;
pub(crate) mod searchability;
mod stats;

pub(crate) use analysis::handle_analysis;
pub(crate) use context::{handle_context, ContextFlags};
pub(crate) use deps::{handle_deps, DepsCliArgs, DepsFlags};
pub(crate) use deslop::handle_deslop;
pub(crate) use doctor::{handle_doctor, DoctorFlags};
pub(crate) use graph::{handle_graph, GraphFlags};
pub(crate) use metrics::{handle_cc, handle_hal, handle_mi, handle_raw, CcFlags, MiFlags};
pub(crate) use searchability::{handle_searchability, SearchabilityFlags};
pub(crate) use stats::{handle_files, handle_stats};

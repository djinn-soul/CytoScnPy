//! Repository health checks for each distinct project root.

use rustc_hash::FxHashSet;

use super::ComprehensiveRequest;

pub(super) fn health_results(
    request: &ComprehensiveRequest<'_>,
) -> Vec<crate::doctor::DoctorResult> {
    let mut seen = FxHashSet::default();
    let cfg = crate::doctor::DoctorConfig {
        excludes: request.excludes.to_vec(),
        verbose: request.cli.output.verbose,
        fail_on_missing: false,
    };
    request
        .roots
        .iter()
        .map(|p| crate::doctor::resolve_doctor_target(p, request.analysis_root))
        .filter(|p| seen.insert(p.clone()))
        .map(|p| crate::doctor::run_doctor(&p, &cfg))
        .collect()
}

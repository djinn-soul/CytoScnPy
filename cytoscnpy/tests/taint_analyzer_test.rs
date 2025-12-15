//! Tests for the taint analyzer configuration and registry.
use cytoscnpy::taint::analyzer::{
    DjangoSourcePlugin, FlaskSourcePlugin, PluginRegistry, TaintAnalyzer,
};

#[test]
fn test_plugin_registry() {
    let mut registry = PluginRegistry::new();
    registry.register_source(FlaskSourcePlugin);
    registry.register_source(DjangoSourcePlugin);

    assert_eq!(registry.sources.len(), 2);
}

#[test]
fn test_analyzer_creation() {
    let analyzer = TaintAnalyzer::default();
    assert!(analyzer.config.intraprocedural);
    assert!(analyzer.config.interprocedural);
    assert!(analyzer.config.crossfile);
}

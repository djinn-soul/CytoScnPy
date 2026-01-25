use cytoscnpy::analyzer::CytoScnPy;
use cytoscnpy::visitor::UnusedCategory;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

fn project_tempdir() -> TempDir {
    let mut target_dir = std::env::current_dir().unwrap();
    target_dir.push("target");
    target_dir.push("test-categories-tmp");
    std::fs::create_dir_all(&target_dir).unwrap();
    tempfile::Builder::new()
        .prefix("categories_test_")
        .tempdir_in(target_dir)
        .unwrap()
}

#[test]
fn test_reporting_categories() {
    let dir = project_tempdir();
    let file_path = dir.path().join("categories.py");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r#"
# 1. DefinitelyUnused (High Confidence)
# Plain unused variable
UNUSED_PLAIN = 100

# 2. ProbablyUnused (Medium Confidence)
# Unused config constant (penalized)
CONFIG_VALUE = 200

# 3. PossiblyIntentional (Low confidence)
# Private method in a class (penalized heavily)
class Internal:
    def _inner(self):
        pass

# 4. Booster
# Suspicious variable (boosted)
temp_var = "debug"
"#
    )
    .unwrap();

    // Use confidence 0 to report EVERYTHING so we can check categories
    let mut analyzer = CytoScnPy::default().with_confidence(0).with_tests(false);
    let result = analyzer.analyze(dir.path());

    let vars = &result.unused_variables;
    let methods = &result.unused_methods;

    // Check UNUSED_PLAIN -> DefinitelyUnused
    let unused_plain = vars.iter().find(|d| d.simple_name == "UNUSED_PLAIN").unwrap();
    assert!(unused_plain.confidence >= 90);
    assert_eq!(unused_plain.category, UnusedCategory::DefinitelyUnused);

    // Check CONFIG_VALUE -> ConfigurationConstant
    // It should have confidence ~70 (95 - 25)
    let config_val = vars.iter().find(|d| d.simple_name == "CONFIG_VALUE").unwrap();
    // Base penalty for constant 15? -> 85.
    // Config penalty 25? -> 60?
    // Let's print confidence to be sure if test fails.
    println!("CONFIG_VALUE confidence: {}", config_val.confidence);
    assert_eq!(config_val.category, UnusedCategory::ConfigurationConstant);

    // Check _inner -> PossiblyIntentional
    // Private name penalty is huge (-80?), so confidence should be low.
    let inner_method = methods.iter().find(|d| d.simple_name == "_inner").unwrap();
    println!("_inner confidence: {}", inner_method.confidence);
    // 100 - 80 = 20. So it falls into default PossiblyIntentional (since < 40 is bucketed there too)
    // Or we might need to adjust buckets?
    // Current match arms:
    // 90..=100 => DefinitelyUnused,
    // 60..=89 => ProbablyUnused,
    // 40..=59 => PossiblyIntentional,
    // _ => PossiblyIntentional,
    assert_eq!(inner_method.category, UnusedCategory::PossiblyIntentional);

    // Check temp_var -> DefinitelyUnused
    // Boosted +15. 100 + 15 = 100 (clamped).
    let temp_var = vars.iter().find(|d| d.simple_name == "temp_var").unwrap();
    assert_eq!(temp_var.confidence, 100);
    assert_eq!(temp_var.category, UnusedCategory::DefinitelyUnused);
}

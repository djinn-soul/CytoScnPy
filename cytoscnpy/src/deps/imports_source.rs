//! Import extraction retains file read and parsing failures.

use super::imports_collect::collect_imports;
use super::imports_dynamic::DynamicImportAliases;
use super::imports_type_checking::TypeCheckingAliases;
use super::ImportScan;
use crate::analyzer::types::ParseError;
use crate::utils::LineIndex;
use std::path::Path;

pub(super) fn extract_imports_from_file(file: &Path, is_production: bool) -> ImportScan {
    let mut scan = ImportScan::empty();
    let content = match std::fs::read_to_string(file) {
        Ok(content) => content,
        Err(error) => {
            scan.scan_errors.push(ParseError {
                file: file.to_path_buf(),
                error: format!("Failed to read dependency source: {error}"),
            });
            return scan;
        }
    };
    let module = match ruff_python_parser::parse_module(&content) {
        Ok(parsed) => parsed.into_syntax(),
        Err(error) => {
            scan.scan_errors.push(ParseError {
                file: file.to_path_buf(),
                error: format!("Failed to parse dependency source: {error}"),
            });
            return scan;
        }
    };
    collect_imports(
        &module.body,
        &mut scan,
        &mut TypeCheckingAliases::default(),
        &mut DynamicImportAliases::default(),
        file,
        &LineIndex::new(&content),
        is_production,
        false,
    );
    if is_production {
        scan.production.extend(scan.all.iter().cloned());
    }
    scan
}

import * as path from "path";
import {
  CytoScnPyFinding,
  CytoScnPyAnalysisResult,
  RawCytoScnPyResult,
  RawCytoScnPyFinding,
  RawCloneFinding,
  ParseError,
} from "./analyzerTypes";
import { dependencySourcePath, pathIsDirectory } from "./analyzerPaths";

export function transformRawResult(
  rawResult: RawCytoScnPyResult,
  anchorPath = "",
): CytoScnPyAnalysisResult {
  const findings: CytoScnPyFinding[] = [];
  const parseErrors: ParseError[] = [];

  const normalizeSeverity = (
    severity: string | undefined,
  ): "error" | "warning" => {
    const upper = severity?.toUpperCase();
    return upper === "HIGH" || upper === "CRITICAL" ? "error" : "warning";
  };

  const processCategory = (
    categoryItems: RawCytoScnPyFinding[] | undefined,
    defaultRuleId: string,
    defaultCategory: string,
    messageFormatter: (finding: RawCytoScnPyFinding) => string,
    defaultSeverity: "error" | "warning" | "info",
  ) => {
    if (!categoryItems) {
      return;
    }

    for (const rawFinding of categoryItems) {
      findings.push({
        file_path: rawFinding.file,
        line_number: rawFinding.line,
        col: rawFinding.col,
        message: rawFinding.message || messageFormatter(rawFinding),
        rule_id: rawFinding.rule_id || defaultRuleId,
        category: rawFinding.category || defaultCategory,
        severity: normalizeSeverity(rawFinding.severity) || defaultSeverity,
        fix: rawFinding.fix,
      });
    }
  };

  // Unused code categories - use simple_name for cleaner display
  processCategory(
    rawResult.unused_functions,
    "unused-function",
    "Dead Code",
    (f) => `'${f.simple_name || f.name}' is defined but never used`,
    "warning",
  );
  processCategory(
    rawResult.unused_methods,
    "unused-method",
    "Dead Code",
    (f) => `Method '${f.simple_name || f.name}' is defined but never used`,
    "warning",
  );
  // Fallback formatters: when the CLI omits `message`, render the bare
  // `simple_name` rather than the module-qualified `name` (e.g. `pkg.foo.os`)
  // so user-visible diagnostics stay short and stable across analysis roots.
  processCategory(
    rawResult.unused_imports,
    "unused-import",
    "Dead Code",
    (f) => `'${f.simple_name || f.name}' is imported but never used`,
    "warning",
  );
  processCategory(
    rawResult.unused_classes,
    "unused-class",
    "Dead Code",
    (f) => `Class '${f.simple_name || f.name}' is defined but never used`,
    "warning",
  );
  processCategory(
    rawResult.unused_variables,
    "unused-variable",
    "Dead Code",
    (f) => `Variable '${f.simple_name || f.name}' is assigned but never used`,
    "warning",
  );
  processCategory(
    rawResult.unused_parameters,
    "unused-parameter",
    "Dead Code",
    (f) => `Parameter '${f.simple_name || f.name}' is never used`,
    "warning",
  );

  // Security categories
  processCategory(
    rawResult.secrets,
    "secret-detected",
    "Secrets",
    (f) => f.message || `Potential secret detected: ${f.simple_name || f.name}`,
    "error",
  );
  processCategory(
    rawResult.danger,
    "dangerous-code",
    "Security",
    (f) => f.message || `Dangerous code pattern: ${f.simple_name || f.name}`,
    "error",
  );
  processCategory(
    rawResult.quality,
    "quality-issue",
    "Quality",
    (f) => f.message || `Quality issue: ${f.simple_name || f.name}`,
    "warning",
  );

  if (rawResult.unused_dependencies) {
    for (const dep of rawResult.unused_dependencies) {
      findings.push({
        file_path: dependencySourcePath(dep, anchorPath),
        line_number: 1,
        message: `Dependency '${dep.package_name}' is declared but never imported`,
        rule_id: "unused-dependency",
        category: "Dependencies",
        severity: "warning",
      });
    }
  }

  if (rawResult.missing_dependencies) {
    const sourcePath = pathIsDirectory(anchorPath)
      ? path.join(anchorPath, "pyproject.toml")
      : anchorPath;
    for (const name of rawResult.missing_dependencies) {
      findings.push({
        file_path: sourcePath,
        line_number: 1,
        message: `Import '${name}' is used but is not declared as a dependency`,
        rule_id: "missing-dependency",
        category: "Dependencies",
        severity: "warning",
      });
    }
  }

  // Process taint findings separately because they have a different structure
  if (rawResult.taint_findings) {
    for (const f of rawResult.taint_findings) {
      const flowPath = f.flow_path ?? [];
      const flowStr =
        flowPath.length > 0
          ? `${f.source} -> ${flowPath.join(" -> ")} -> ${f.sink}`
          : `${f.source} -> ${f.sink}`;

      const message = `${f.vuln_type}: Tainted data from ${f.source} (line ${f.source_line}) reaches sink ${f.sink}.\n\nFlow: ${flowStr}\n\nRemediation: ${f.remediation}`;

      findings.push({
        file_path: f.file,
        line_number: f.sink_line,
        col: f.sink_col,
        message,
        rule_id: `taint-${f.vuln_type.toLowerCase()}`,
        category: "Security",
        severity: normalizeSeverity(f.severity),
      });
    }
  }

  // Process parse errors
  if (rawResult.parse_errors) {
    for (const err of rawResult.parse_errors) {
      parseErrors.push({
        file: err.file,
        // Rust provides a path and error text, so anchor the diagnostic to the file.
        line: 1,
        message: err.error,
      });
    }
  }

  // Process clone findings (displayed as warnings with navigation suggestions)
  // Clone detection uses AST-based hashing and edit distance (not CFG)
  // Deduplicate: keep only the highest-similarity clone per location
  if (rawResult.clones) {
    // Group by (file, line) and keep the best match
    const cloneMap = new Map<
      string,
      { clone: RawCloneFinding; similarity: number }
    >();

    for (const clone of rawResult.clones) {
      const key = `${clone.file}:${clone.line}`;
      const existing = cloneMap.get(key);
      if (!existing || clone.similarity > existing.similarity) {
        cloneMap.set(key, { clone, similarity: clone.similarity });
      }
    }

    // Create findings from deduplicated clones
    for (const { clone } of cloneMap.values()) {
      const similarityPercent = Math.round(clone.similarity * 100);
      const relatedFile = clone.related_clone.file.split(/[\\/]/).pop(); // basename
      const relatedLine = clone.related_clone.line;

      // Build a cleaner message with reference
      const message = `Similar to ${
        clone.related_clone.name || relatedFile
      }:${relatedLine} (${similarityPercent}% match). ${
        clone.suggestion || "Consider refactoring."
      }`;

      findings.push({
        file_path: clone.file,
        line_number: clone.line,
        message,
        rule_id: clone.rule_id,
        category: "Clones",
        severity: "warning",
      });
    }
  }

  return { findings, parseErrors };
}

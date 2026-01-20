// child_process.execFile is used inline in runCytoScnPyAnalysis

export interface CytoScnPyFinding {
  file_path: string;
  line_number: number;
  col?: number;
  message: string;
  rule_id: string;
  category: string;
  severity: "error" | "warning" | "info" | "hint";
  // CST-based fix suggestion (if available)
  fix?: {
    start_byte: number;
    end_byte: number;
    replacement: string;
  };
}

export interface CytoScnPyAnalysisResult {
  findings: CytoScnPyFinding[];
  parseErrors: ParseError[];
}

interface ParseError {
  file: string;
  line: number;
  message: string;
}

export interface CytoScnPyConfig {
  path: string;
  analysisMode: "workspace" | "file"; // workspace = full project, file = single file
  enableSecretsScan: boolean;
  enableDangerScan: boolean;
  enableQualityScan: boolean;
  enableCloneScan: boolean; // Enable code clone detection (--clones flag)
  confidenceThreshold: number;
  excludeFolders: string[];
  includeFolders: string[];
  includeTests: boolean;
  includeIpynb: boolean;
  maxComplexity: number;
  minMaintainabilityIndex: number;
  maxNesting: number;
  maxArguments: number;
  maxLines: number;
}

// This is the structure of the raw output from the cytoscnpy tool
interface RawCytoScnPyFinding {
  file: string;
  line: number;
  col?: number;
  message?: string;
  rule_id?: string;
  category?: string;
  severity?: string;
  name?: string;
  simple_name?: string;
  fix?: {
    start_byte: number;
    end_byte: number;
    replacement: string;
  };
}

interface RawTaintFinding {
  source: string;
  source_line: number;
  sink: string;
  sink_line: number;
  sink_col: number;
  flow_path: string[];
  vuln_type: string;
  severity: string;
  file: string;
  remediation: string;
}

interface RawCytoScnPyResult {
  unused_functions?: RawCytoScnPyFinding[];
  unused_methods?: RawCytoScnPyFinding[];
  unused_imports?: RawCytoScnPyFinding[];
  unused_classes?: RawCytoScnPyFinding[];
  unused_variables?: RawCytoScnPyFinding[];
  unused_parameters?: RawCytoScnPyFinding[];
  secrets?: RawCytoScnPyFinding[];
  danger?: RawCytoScnPyFinding[];
  quality?: RawCytoScnPyFinding[];
  taint_findings?: RawTaintFinding[];
  clone_findings?: RawCloneFinding[];
  parse_errors?: { file: string; line: number; message: string }[];
}

// Clone detection finding structure (matches Rust CloneFinding struct)
interface RawCloneFinding {
  rule_id: string;
  message: string;
  severity: string;
  file: string;
  line: number;
  end_line: number;
  start_byte: number;
  end_byte: number;
  clone_type: "Type1" | "Type2" | "Type3";
  similarity: number;
  name?: string;
  related_clone: {
    file: string;
    line: number;
    end_line: number;
    name?: string;
  };
  fix_confidence: number;
  is_duplicate: boolean;
  suggestion?: string;
  node_kind: "Function" | "AsyncFunction" | "Class" | "Method";
}

function transformRawResult(
  rawResult: RawCytoScnPyResult,
): CytoScnPyAnalysisResult {
  const findings: CytoScnPyFinding[] = [];
  const parseErrors: ParseError[] = [];

  const normalizeSeverity = (
    severity: string | undefined,
  ): "error" | "warning" | "info" => {
    switch (severity?.toUpperCase()) {
      case "HIGH":
      case "CRITICAL":
        return "error";
      case "MEDIUM":
        return "warning";
      case "LOW":
        return "info";
      default:
        return "warning";
    }
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
  processCategory(
    rawResult.unused_imports,
    "unused-import",
    "Dead Code",
    (f) => `'${f.name}' is imported but never used`,
    "warning",
  );
  processCategory(
    rawResult.unused_classes,
    "unused-class",
    "Dead Code",
    (f) => `Class '${f.name}' is defined but never used`,
    "warning",
  );
  processCategory(
    rawResult.unused_variables,
    "unused-variable",
    "Dead Code",
    (f) => `Variable '${f.name}' is assigned but never used`,
    "warning",
  );
  processCategory(
    rawResult.unused_parameters,
    "unused-parameter",
    "Dead Code",
    (f) => `Parameter '${f.name}' is never used`,
    "info",
  );

  // Security categories
  processCategory(
    rawResult.secrets,
    "secret-detected",
    "Secrets",
    (f) => f.message || `Potential secret detected: ${f.name}`,
    "error",
  );
  processCategory(
    rawResult.danger,
    "dangerous-code",
    "Security",
    (f) => f.message || `Dangerous code pattern: ${f.name}`,
    "error",
  );
  processCategory(
    rawResult.quality,
    "quality-issue",
    "Quality",
    (f) => f.message || `Quality issue: ${f.name}`,
    "warning",
  );

  // Process taint findings separately because they have a different structure
  if (rawResult.taint_findings) {
    for (const f of rawResult.taint_findings) {
      const flowStr =
        f.flow_path.length > 0
          ? `${f.source} -> ${f.flow_path.join(" -> ")} -> ${f.sink}`
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
        line: err.line,
        message: err.message,
      });
    }
  }

  // Process clone findings (displayed as hints with navigation suggestions)
  // Clone detection uses AST-based hashing and edit distance (not CFG)
  // Deduplicate: keep only the highest-similarity clone per location
  if (rawResult.clone_findings) {
    // Group by (file, line) and keep the best match
    const cloneMap = new Map<
      string,
      { clone: RawCloneFinding; similarity: number }
    >();

    for (const clone of rawResult.clone_findings) {
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
        severity: clone.is_duplicate ? "warning" : "hint",
      });
    }
  }

  return { findings, parseErrors };
}

export function runCytoScnPyAnalysis(
  filePath: string,
  config: CytoScnPyConfig,
): Promise<CytoScnPyAnalysisResult> {
  return new Promise((resolve, reject) => {
    // Build args array (avoids shell escaping issues on Windows)
    const args: string[] = [filePath, "--json"];

    if (config.enableSecretsScan) {
      args.push("--secrets");
    }
    if (config.enableDangerScan) {
      args.push("--danger");
    }
    if (config.enableCloneScan) {
      args.push("--clones");
    }

    if (config.confidenceThreshold > 0) {
      args.push("--confidence", config.confidenceThreshold.toString());
    }
    if (config.excludeFolders && config.excludeFolders.length > 0) {
      for (const folder of config.excludeFolders) {
        args.push("--exclude-folders", folder);
      }
    }
    if (config.includeFolders && config.includeFolders.length > 0) {
      for (const folder of config.includeFolders) {
        args.push("--include-folders", folder);
      }
    }
    if (config.includeTests) {
      args.push("--include-tests");
    }
    if (config.includeIpynb) {
      args.push("--include-ipynb");
    }

    if (config.enableQualityScan) {
      args.push("--quality");
      if (config.maxComplexity) {
        args.push("--max-complexity", config.maxComplexity.toString());
      }
      if (config.minMaintainabilityIndex) {
        args.push("--min-mi", config.minMaintainabilityIndex.toString());
      }
      if (config.maxNesting) {
        args.push("--max-nesting", config.maxNesting.toString());
      }
      if (config.maxArguments) {
        args.push("--max-args", config.maxArguments.toString());
      }
      if (config.maxLines) {
        args.push("--max-lines", config.maxLines.toString());
      }
    }

    const { execFile } = require("child_process");

    execFile(
      config.path,
      args,
      (error: Error | null, stdout: string, stderr: string) => {
        if (error) {
          // CLI exited with non-zero code, but might still have valid JSON
          // (e.g., gate thresholds failed but analysis succeeded)
          try {
            const rawResult: RawCytoScnPyResult = JSON.parse(stdout.trim());
            const result = transformRawResult(rawResult);
            resolve(result);
          } catch (parseError) {
            // JSON parsing failed - this is a real error
            console.error(
              `CytoScnPy analysis failed for ${filePath}: ${error.message}`,
            );
            if (stderr) {
              console.error(`Stderr: ${stderr}`);
            }
            reject(
              new Error(
                `Failed to run CytoScnPy analysis: ${error.message}. Stderr: ${stderr}`,
              ),
            );
          }
          return;
        }

        if (stderr) {
          console.warn(
            `CytoScnPy analysis for ${filePath} produced stderr: ${stderr}`,
          );
        }

        try {
          const rawResult: RawCytoScnPyResult = JSON.parse(stdout.trim());
          const result = transformRawResult(rawResult);
          resolve(result);
        } catch (parseError: any) {
          reject(
            new Error(
              `Failed to parse CytoScnPy JSON output for ${filePath}: ${parseError.message}. Output: ${stdout}`,
            ),
          );
        }
      },
    );
  });
}

/**
 * Run workspace-level analysis and return findings grouped by file path.
 * This provides cross-file reference tracking for accurate unused code detection.
 */
export function runWorkspaceAnalysis(
  workspacePath: string,
  config: CytoScnPyConfig,
): Promise<Map<string, CytoScnPyFinding[]>> {
  return new Promise((resolve, reject) => {
    const args: string[] = [workspacePath, "--json"];

    if (config.enableSecretsScan) {
      args.push("--secrets");
    }
    if (config.enableDangerScan) {
      args.push("--danger");
    }
    if (config.enableCloneScan) {
      args.push("--clones");
    }
    if (config.confidenceThreshold > 0) {
      args.push("--confidence", config.confidenceThreshold.toString());
    }
    if (config.excludeFolders && config.excludeFolders.length > 0) {
      for (const folder of config.excludeFolders) {
        args.push("--exclude-folders", folder);
      }
    }
    if (config.includeFolders && config.includeFolders.length > 0) {
      for (const folder of config.includeFolders) {
        args.push("--include-folders", folder);
      }
    }
    if (config.includeTests) {
      args.push("--include-tests");
    }
    if (config.includeIpynb) {
      args.push("--include-ipynb");
    }

    if (config.enableQualityScan) {
      args.push("--quality");
      if (config.maxComplexity) {
        args.push("--max-complexity", config.maxComplexity.toString());
      }
      if (config.minMaintainabilityIndex) {
        args.push("--min-mi", config.minMaintainabilityIndex.toString());
      }
      if (config.maxNesting) {
        args.push("--max-nesting", config.maxNesting.toString());
      }
      if (config.maxArguments) {
        args.push("--max-args", config.maxArguments.toString());
      }
      if (config.maxLines) {
        args.push("--max-lines", config.maxLines.toString());
      }
    }

    const { execFile } = require("child_process");

    execFile(
      config.path,
      args,
      { maxBuffer: 50 * 1024 * 1024 }, // 50MB buffer for large workspaces
      (error: Error | null, stdout: string, stderr: string) => {
        if (error && !stdout.trim()) {
          console.error(
            `CytoScnPy workspace analysis failed: ${error.message}`,
          );
          if (stderr) {
            console.error(`Stderr: ${stderr}`);
          }
          reject(new Error(`Workspace analysis failed: ${error.message}`));
          return;
        }

        try {
          const rawResult: RawCytoScnPyResult = JSON.parse(stdout.trim());
          const result = transformRawResult(rawResult);
          const path = require("path");

          // Group findings by file path, converting to absolute paths
          const findingsByFile = new Map<string, CytoScnPyFinding[]>();
          for (const finding of result.findings) {
            // Convert relative paths to absolute paths using workspace root
            let filePath = finding.file_path;
            if (!path.isAbsolute(filePath)) {
              filePath = path.resolve(workspacePath, filePath);
            }
            if (process.platform === "win32") {
              filePath = filePath.toLowerCase();
            }

            if (!findingsByFile.has(filePath)) {
              findingsByFile.set(filePath, []);
            }
            findingsByFile.get(filePath)!.push(finding);
          }

          resolve(findingsByFile);
        } catch (parseError: any) {
          reject(
            new Error(
              `Failed to parse workspace analysis output: ${parseError.message}`,
            ),
          );
        }
      },
    );
  });
}

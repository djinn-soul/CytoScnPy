// child_process.execFile is used inline in runCytoScnPyAnalysis

interface CytoScnPyFinding {
  file_path: string;
  line_number: number;
  col?: number;
  message: string;
  rule_id: string;
  severity: "error" | "warning" | "info";
}

interface CytoScnPyAnalysisResult {
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
  enableSecretsScan: boolean;
  enableDangerScan: boolean;
  enableQualityScan: boolean;
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
  severity?: string;
  name?: string;
  simple_name?: string; // Short name without class prefix (e.g., "method_a" instead of "ComplexClass.method_a")
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
  taint_findings?: RawCytoScnPyFinding[];
  parse_errors?: { file: string; line: number; message: string }[];
}

function transformRawResult(
  rawResult: RawCytoScnPyResult
): CytoScnPyAnalysisResult {
  const findings: CytoScnPyFinding[] = [];
  const parseErrors: ParseError[] = [];

  const normalizeSeverity = (
    severity: string | undefined
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
    category: RawCytoScnPyFinding[] | undefined,
    defaultRuleId: string,
    messageFormatter: (finding: RawCytoScnPyFinding) => string,
    defaultSeverity: "error" | "warning" | "info"
  ) => {
    if (!category) {
      return;
    }

    for (const rawFinding of category) {
      findings.push({
        file_path: rawFinding.file,
        line_number: rawFinding.line,
        col: rawFinding.col,
        message: rawFinding.message || messageFormatter(rawFinding),
        rule_id: rawFinding.rule_id || defaultRuleId,
        severity: normalizeSeverity(rawFinding.severity) || defaultSeverity,
      });
    }
  };

  // Unused code categories - use simple_name for cleaner display
  processCategory(
    rawResult.unused_functions,
    "unused-function",
    (f) => `'${f.simple_name || f.name}' is defined but never used`,
    "warning"
  );
  processCategory(
    rawResult.unused_methods,
    "unused-method",
    (f) => `Method '${f.simple_name || f.name}' is defined but never used`,
    "warning"
  );
  processCategory(
    rawResult.unused_imports,
    "unused-import",
    (f) => `'${f.name}' is imported but never used`,
    "warning"
  );
  processCategory(
    rawResult.unused_classes,
    "unused-class",
    (f) => `Class '${f.name}' is defined but never used`,
    "warning"
  );
  processCategory(
    rawResult.unused_variables,
    "unused-variable",
    (f) => `Variable '${f.name}' is assigned but never used`,
    "warning"
  );
  processCategory(
    rawResult.unused_parameters,
    "unused-parameter",
    (f) => `Parameter '${f.name}' is never used`,
    "info"
  );

  // Security categories
  processCategory(
    rawResult.secrets,
    "secret-detected",
    (f) => f.message || `Potential secret detected: ${f.name}`,
    "error"
  );
  processCategory(
    rawResult.danger,
    "dangerous-code",
    (f) => f.message || `Dangerous code pattern: ${f.name}`,
    "error"
  );
  processCategory(
    rawResult.quality,
    "quality-issue",
    (f) => f.message || `Quality issue: ${f.name}`,
    "warning"
  );
  processCategory(
    rawResult.taint_findings,
    "taint-vulnerability",
    (f) => f.message || `Potential vulnerability: ${f.name}`,
    "error"
  );

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

  return { findings, parseErrors };
}

export function runCytoScnPyAnalysis(
  filePath: string,
  config: CytoScnPyConfig
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
    if (config.enableQualityScan) {
      args.push("--quality");
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

    const { execFile } = require("child_process");

    execFile(
      config.path,
      args,
      (error: Error | null, stdout: string, stderr: string) => {
        if (error) {
          console.error(
            `CytoScnPy analysis failed for ${filePath}: ${error.message}`
          );
          console.error(`Stderr: ${stderr}`);
          try {
            const rawResult: RawCytoScnPyResult = JSON.parse(stdout.trim());
            const result = transformRawResult(rawResult);
            resolve(result);
          } catch (parseError) {
            reject(
              new Error(
                `Failed to run CytoScnPy analysis: ${error.message}. Stderr: ${stderr}`
              )
            );
          }
          return;
        }

        if (stderr) {
          console.warn(
            `CytoScnPy analysis for ${filePath} produced stderr: ${stderr}`
          );
        }

        try {
          const rawResult: RawCytoScnPyResult = JSON.parse(stdout.trim());
          const result = transformRawResult(rawResult);
          resolve(result);
        } catch (parseError: any) {
          reject(
            new Error(
              `Failed to parse CytoScnPy JSON output for ${filePath}: ${parseError.message}. Output: ${stdout}`
            )
          );
        }
      }
    );
  });
}

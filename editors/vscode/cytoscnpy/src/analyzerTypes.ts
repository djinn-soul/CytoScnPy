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

export interface ParseError {
  file: string;
  line: number;
  message: string;
}

export interface WorkspaceAnalysisResult {
  findingsByFile: Map<string, CytoScnPyFinding[]>;
  parseErrorsByFile: Map<string, ParseError[]>;
}

export interface CytoScnPyConfig {
  path: string;
  analysisMode: "workspace" | "file"; // workspace = full project, file = single file
  enableSecretsScan: boolean;
  enableDangerScan: boolean;
  enableQualityScan: boolean;
  enableCloneScan: boolean; // Enable code clone detection (--clones flag)
  confidenceThreshold?: number;
  excludeFolders?: string[];
  includeFolders?: string[];
  includeTests?: boolean;
  includeIpynb?: boolean;
  maxComplexity?: number;
  minMaintainabilityIndex?: number;
  maxNesting?: number;
  maxArguments?: number;
  maxLines?: number;
}

// This is the structure of the raw output from the cytoscnpy tool
export interface RawCytoScnPyFinding {
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

export interface RawTaintFinding {
  source: string;
  source_line: number;
  sink: string;
  sink_line: number;
  sink_col: number;
  flow_path?: string[];
  vuln_type: string;
  severity: string;
  file: string;
  remediation: string;
}

export interface RawDeclaredDependency {
  package_name: string;
  normalized_name: string;
  is_dev: boolean;
  source?: "Pyproject" | { Requirements: string };
}

export interface RawCytoScnPyResult {
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
  unused_dependencies?: RawDeclaredDependency[];
  missing_dependencies?: string[];
  clones?: RawCloneFinding[];
  parse_errors?: { file: string; error: string }[];
}

// Clone detection finding structure (matches Rust CloneFinding struct)
export interface RawCloneFinding {
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

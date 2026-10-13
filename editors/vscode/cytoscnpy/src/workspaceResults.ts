import { CytoScnPyFinding, ParseError, WorkspaceAnalysisResult } from "./analyzerTypes";
import { getCacheKey } from "./extensionCache";

/** Results from every workspace folder, keyed for cache lookups. */
export interface MergedWorkspaceResults {
  findingsByFile: Map<string, CytoScnPyFinding[]>;
  parseErrorsByFile: Map<string, ParseError[]>;
  /**
   * Cache keys are lowercased on Windows, so keep each file's real path for
   * the diagnostic URI; a lowercased URI duplicates the opened file's entry.
   */
  sourcePaths: Map<string, string>;
  scanErrors: ParseError[];
}

export function emptyWorkspaceResults(): MergedWorkspaceResults {
  return {
    findingsByFile: new Map(),
    parseErrorsByFile: new Map(),
    sourcePaths: new Map(),
    scanErrors: [],
  };
}

export function mergeWorkspaceResult(
  target: MergedWorkspaceResults,
  result: WorkspaceAnalysisResult,
): void {
  for (const [filePath, findings] of result.findingsByFile) {
    target.findingsByFile.set(getCacheKey(filePath), findings);
    target.sourcePaths.set(getCacheKey(filePath), filePath);
  }
  for (const [filePath, errors] of result.parseErrorsByFile) {
    target.parseErrorsByFile.set(getCacheKey(filePath), errors);
    target.sourcePaths.set(getCacheKey(filePath), filePath);
  }
  target.scanErrors.push(...result.scanErrors);
}

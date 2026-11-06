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

/**
 * Adds `result` to `target`, keeping only files `owns` accepts: when nested
 * projects are scanned on their own, each file's results come from the scan
 * of the project it belongs to.
 */
export function mergeWorkspaceResult(
  target: MergedWorkspaceResults,
  result: WorkspaceAnalysisResult,
  owns: (filePath: string) => boolean = () => true,
): void {
  for (const [filePath, findings] of result.findingsByFile) {
    if (owns(filePath)) {
      target.findingsByFile.set(getCacheKey(filePath), findings);
      target.sourcePaths.set(getCacheKey(filePath), filePath);
    }
  }
  for (const [filePath, errors] of result.parseErrorsByFile) {
    if (owns(filePath)) {
      target.parseErrorsByFile.set(getCacheKey(filePath), errors);
      target.sourcePaths.set(getCacheKey(filePath), filePath);
    }
  }
  target.scanErrors.push(...result.scanErrors.filter((error) => owns(error.file)));
}

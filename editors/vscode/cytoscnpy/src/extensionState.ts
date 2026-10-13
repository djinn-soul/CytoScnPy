import { CytoScnPyFinding, ParseError } from "./analyzer";
import { CoalescingTaskQueue, DebouncedTaskMap } from "./scanScheduler";

export const analysisState = {
  workspaceCache: null as Map<string, CytoScnPyFinding[]> | null,
  workspaceParseErrorsCache: null as Map<string, ParseError[]> | null,
  workspaceCacheTimestamp: 0,
  workspaceScanQueue: new CoalescingTaskQueue(),
  saveScanDebouncer: new DebouncedTaskMap(),
};

export interface AnalysisActions {
  runFullWorkspaceAnalysis(): Promise<void>;
  runIncrementalAnalysis(document: import("vscode").TextDocument): Promise<void>;
  refreshDiagnostics(document: import("vscode").TextDocument): Promise<void>;
}

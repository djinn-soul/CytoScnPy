export type AnalysisMode = "file" | "workspace";

/**
 * Periodic full scans only refresh workspace-mode results. File mode must
 * never scan the whole workspace on its own, and an unchanged workspace
 * does not need rescanning.
 */
export function periodicScanDue(
  openPythonModes: readonly AnalysisMode[],
  lastFileChangeTime: number,
  workspaceCacheTimestamp: number,
): boolean {
  return (
    openPythonModes.includes("workspace") &&
    lastFileChangeTime > workspaceCacheTimestamp
  );
}

/**
 * Workspace mode needs a workspace folder to scan. A file opened on its own,
 * or one outside every open folder, falls back to single-file analysis so it
 * is still analyzed instead of silently skipped.
 */
export function effectiveAnalysisMode(
  configured: AnalysisMode,
  inWorkspaceFolder: boolean,
): AnalysisMode {
  return configured === "workspace" && inWorkspaceFolder ? "workspace" : "file";
}

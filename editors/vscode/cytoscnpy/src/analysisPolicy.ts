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

import * as vscode from "vscode";
import { getCytoScnPyConfiguration } from "./configuration";
import { analysisState, AnalysisActions } from "./extensionState";
import { getCacheKey, fileCache, documentHashCache, clearDocumentAnalysis, invalidateWorkspaceCache } from "./extensionCache";
import { watchProjectConfiguration } from "./configWatcher";
import { cytoscnpyDiagnostics } from "./extensionUi";
import { periodicScanDue } from "./analysisPolicy";

export function registerAnalysisEvents(context: vscode.ExtensionContext, actions: AnalysisActions): void {
  const { runFullWorkspaceAnalysis, runIncrementalAnalysis, refreshDiagnostics } = actions;
  function openPythonDocuments(): vscode.TextDocument[] {
    return vscode.workspace.textDocuments.filter(
      (document) => document.languageId === "python",
    );
  }

  function analysisModeFor(document: vscode.TextDocument) {
    return getCytoScnPyConfiguration(context, document.uri).analysisMode;
  }

  function refreshAfterConfigurationChange() {
    invalidateWorkspaceCache();
    const pythonDocuments = openPythonDocuments();
    if (
      vscode.workspace.workspaceFolders?.length &&
      pythonDocuments.some(
        (document) => analysisModeFor(document) === "workspace",
      )
    ) {
      void runFullWorkspaceAnalysis();
    } else {
      pythonDocuments.forEach((document) => void refreshDiagnostics(document));
    }
  }

  // Initial analysis when a document is opened or becomes active
  if (vscode.window.activeTextEditor) {
    refreshDiagnostics(vscode.window.activeTextEditor.document);
  }

  // Periodic workspace re-scan: catches cross-file dependencies even if only
  // incremental scans ran. Skip the tick unless an open Python file uses
  // workspace mode and files changed since the last full analysis; both gates
  // apply in debug builds too so dev sessions are not flooded.
  let lastFileChangeTime = Date.now();
  const isDebug = context.extensionMode === vscode.ExtensionMode.Development;
  const PERIODIC_SCAN_INTERVAL_MS = isDebug ? 15 * 1000 : 5 * 60 * 1000;

  const periodicScanInterval = setInterval(async () => {
    if (
      !periodicScanDue(
        openPythonDocuments().map(analysisModeFor),
        lastFileChangeTime,
        analysisState.workspaceCacheTimestamp,
      )
    ) {
      return;
    }
    await runFullWorkspaceAnalysis();
  }, PERIODIC_SCAN_INTERVAL_MS);
  context.subscriptions.push({
    dispose: () => clearInterval(periodicScanInterval),
  });
  context.subscriptions.push(analysisState.saveScanDebouncer);

  context.subscriptions.push(
    vscode.workspace.onDidChangeTextDocument((event) => {
      if (event.document.languageId === "python" && event.document.isDirty) {
        clearDocumentAnalysis(event.document);
      }
    }),
  );

  // Analyze document on save - debounced incremental analysis (much faster than full workspace scan)
  context.subscriptions.push(
    vscode.workspace.onDidSaveTextDocument((document) => {
      if (document.languageId === "python") {
        // Update last change time
        lastFileChangeTime = Date.now();

        const config = getCytoScnPyConfiguration(context, document.uri);
        const debounceMs = config.analysisMode === "workspace" ? 3000 : 500;
        const debounceKey =
          config.analysisMode === "workspace"
            ? "workspace"
            : document.uri.toString();
        analysisState.saveScanDebouncer.schedule(debounceKey, debounceMs, () => {
          const currentConfig = getCytoScnPyConfiguration(
            context,
            document.uri,
          );

          if (currentConfig.analysisMode === "workspace") {
            // In workspace mode, run full analysis to maintain cross-file context correctness
            runFullWorkspaceAnalysis().catch((err) => {
              console.error(
                "[CytoScnPy] Workspace analysis on save failed:",
                err,
              );
            });
          } else {
            // Use incremental analysis - only re-scan the saved file
            // This is much faster than full workspace re-analysis
            runIncrementalAnalysis(document).catch((err) => {
              console.error("[CytoScnPy] Incremental analysis failed:", err);
            });
          }
        });
      }
    }),
  );

  // Re-run analysis when CytoScnPy settings change (e.g., settings.json saved)
  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (event.affectsConfiguration("cytoscnpy")) {
        refreshAfterConfigurationChange();
      }
    }),
  );

  context.subscriptions.push(
    watchProjectConfiguration(refreshAfterConfigurationChange),
  );

  context.subscriptions.push(
    // Folder changes rescan the workspace only for workspace-mode files.
    vscode.workspace.onDidChangeWorkspaceFolders(() =>
      refreshAfterConfigurationChange(),
    ),
  );

  // Analyze when the active editor changes (switching tabs)
  context.subscriptions.push(
    vscode.window.onDidChangeActiveTextEditor((editor) => {
      if (editor && editor.document.languageId === "python") {
        refreshDiagnostics(editor.document);
      }
    }),
  );

  // Clear diagnostics and cache when a document is closed
  context.subscriptions.push(
    vscode.workspace.onDidCloseTextDocument((document) => {
      const mode = getCytoScnPyConfiguration(
        context,
        document.uri,
      ).analysisMode;
      // In workspace mode we intentionally keep diagnostics for closed files
      // so the Problems view remains complete across the whole project.
      if (mode === "file") {
        cytoscnpyDiagnostics.delete(document.uri);
      }
      fileCache.delete(getCacheKey(document.uri.fsPath)); // Clear cache entry
      documentHashCache.delete(document.uri.toString());
    }),
  );
}

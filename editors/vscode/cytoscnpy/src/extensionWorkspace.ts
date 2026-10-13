import * as vscode from "vscode";
import { runWorkspaceAnalysis } from "./analyzer";
import { getCytoScnPyConfiguration } from "./configuration";
import { analysisState } from "./extensionState";
import { getCacheKey, fileCache, cacheDocumentAnalysis, invalidateWorkspaceCache } from "./extensionCache";
import { findingsToDiagnostics, parseErrorsToDiagnostics, buildClosedFileDiagnosticFromFinding, buildClosedFileDiagnosticFromParseError } from "./extensionDiagnostics";
import { emptyWorkspaceResults, mergeWorkspaceResult } from "./workspaceResults";
import { setStatus, cytoscnpyDiagnostics, cytoscnpyOutputChannel, applyGutterDecorations, reportAnalysisFailure, resetAnalysisFailure, setDiagnosticsBatched } from "./extensionUi";

export function createWorkspaceAnalysis(context: vscode.ExtensionContext): () => Promise<void> {
  const isDevBuild = context.extensionMode === vscode.ExtensionMode.Development;
  async function runFullWorkspaceAnalysis() {
    return analysisState.workspaceScanQueue.run(performFullWorkspaceAnalysis);
  }

  async function performFullWorkspaceAnalysis() {
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (!workspaceFolders || workspaceFolders.length === 0) {
      return;
    }

    setStatus("running", "scanning workspace");

    await vscode.window.withProgress(
      {
        location: vscode.ProgressLocation.Notification,
        title: "CytoScnPy: Analyzing workspace...",
        cancellable: true,
      },
      async (progress, token) => {
        const abort = new AbortController();
        const cancellation = token.onCancellationRequested(() => abort.abort());
        try {
          const startTime = Date.now();
          const merged = emptyWorkspaceResults();
          for (const folder of workspaceFolders) {
            progress.report({ message: `Scanning ${folder.name}...` });
            mergeWorkspaceResult(
              merged,
              await runWorkspaceAnalysis(
                folder.uri.fsPath,
                getCytoScnPyConfiguration(context, folder.uri),
                { signal: abort.signal },
              ),
            );
          }
          const { findingsByFile, parseErrorsByFile, sourcePaths, scanErrors } = merged;
          analysisState.workspaceCache = findingsByFile;
          analysisState.workspaceParseErrorsCache = parseErrorsByFile;
          analysisState.workspaceCacheTimestamp = Date.now();
          analysisState.workspaceScanFailed = false;
          resetAnalysisFailure();

          const duration = (Date.now() - startTime) / 1000;
          if (isDevBuild) {
            const fileCount = new Set<string>([
              ...analysisState.workspaceCache.keys(),
              ...(analysisState.workspaceParseErrorsCache?.keys() ?? []),
            ]).size;
            cytoscnpyOutputChannel.appendLine(
              `[CytoScnPy] Workspace analysis completed in ${duration.toFixed(
                2,
              )}s, findings in ${fileCount} files`,
            );
          }

          progress.report({ message: `Updating diagnostics...` });

          cytoscnpyDiagnostics.clear();
          const openDocuments = new Map(
            vscode.workspace.textDocuments
              .filter((document) => document.languageId === "python")
              .map((document) => [getCacheKey(document.uri.fsPath), document]),
          );
          const filesWithDiagnostics = new Set<string>([
            ...analysisState.workspaceCache.keys(),
            ...(analysisState.workspaceParseErrorsCache?.keys() ?? []),
          ]);

          const updates: [vscode.Uri, vscode.Diagnostic[]][] = [];
          for (const filePath of filesWithDiagnostics) {
            if (openDocuments.has(filePath)) {
              continue;
            }
            const uri = vscode.Uri.file(sourcePaths.get(filePath) ?? filePath);
            const findings = analysisState.workspaceCache.get(filePath) || [];
            const parseErrors =
              analysisState.workspaceParseErrorsCache?.get(filePath) || [];
            const diagnostics = [
              ...findings.map(buildClosedFileDiagnosticFromFinding),
              ...parseErrors.map(buildClosedFileDiagnosticFromParseError),
            ];
            updates.push([uri, diagnostics]);
          }

          for (const [filePath, document] of openDocuments) {
            if (document.isDirty) {
              fileCache.delete(filePath);
              continue;
            }
            const findings = analysisState.workspaceCache.get(filePath) || [];
            const parseErrors = analysisState.workspaceParseErrorsCache?.get(filePath) || [];
            const diagnostics = [
              ...findingsToDiagnostics(document, findings),
              ...parseErrorsToDiagnostics(document, parseErrors),
            ];
            updates.push([document.uri, diagnostics]);
            cacheDocumentAnalysis(document, diagnostics, findings);
          }
          setDiagnosticsBatched(updates);

          const activeEditor = vscode.window.activeTextEditor;
          if (activeEditor?.document.languageId === "python") {
            applyGutterDecorations(
              activeEditor,
              activeEditor.document.isDirty
                ? []
                : cytoscnpyDiagnostics.get(activeEditor.document.uri) || [],
            );
          }

          const findingsCount = Array.from(analysisState.workspaceCache.values()).reduce(
            (sum, list) => sum + list.length,
            0,
          );
          const summary = `${findingsCount} finding${findingsCount === 1 ? "" : "s"} (${duration.toFixed(1)}s)`;
          if (scanErrors.length > 0) {
            // These errors name a directory, not a file, so they cannot be
            // shown as Problems entries. Results are incomplete: say so.
            for (const scanError of scanErrors) {
              cytoscnpyOutputChannel.appendLine(
                `[CytoScnPy] Scan incomplete in ${scanError.file}: ${scanError.message}`,
              );
            }
            setStatus(
              "error",
              `scan incomplete (${scanErrors.length} error${scanErrors.length === 1 ? "" : "s"}, see output), ${summary}`,
            );
          } else {
            setStatus("idle", summary);
          }
        } catch (error: any) {
          if (token.isCancellationRequested) {
            // Keep the previous results; only stop scans from restarting
            // on their own until the user saves or re-runs analysis.
            analysisState.workspaceScanFailed = true;
            setStatus("idle", "scan cancelled, click to re-run");
            return;
          }
          invalidateWorkspaceCache();
          analysisState.workspaceScanFailed = true;
          console.error(
            `[CytoScnPy] Workspace analysis failed: ${error.message}`,
          );
          reportAnalysisFailure(error.message);
          setStatus("error", "analysis failed");
        } finally {
          cancellation.dispose();
        }
      },
    );
  }

  return runFullWorkspaceAnalysis;
}

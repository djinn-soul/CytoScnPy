import * as vscode from "vscode";
import * as path from "path";
import { runCytoScnPyAnalysis } from "./analyzer";
import { documentAnalysisMode, getCytoScnPyConfiguration } from "./configuration";
import { analysisState } from "./extensionState";
import { getCacheKey, clearDocumentAnalysis, cacheDocumentAnalysis } from "./extensionCache";
import { findingsToDiagnostics, parseErrorsToDiagnostics } from "./extensionDiagnostics";
import { setStatus, cytoscnpyDiagnostics, cytoscnpyOutputChannel, applyGutterDecorations, reportAnalysisFailure, resetAnalysisFailure } from "./extensionUi";

export function createFileAnalysis(
  context: vscode.ExtensionContext,
  runFullWorkspaceAnalysis: () => Promise<void>,
) {
  const isDevBuild = context.extensionMode === vscode.ExtensionMode.Development;
  async function runIncrementalAnalysis(document: vscode.TextDocument) {
    const filePath = document.uri.fsPath;
    if (document.isDirty) {
      clearDocumentAnalysis(document);
      return;
    }
    const documentVersion = document.version;
    const config = getCytoScnPyConfiguration(context, document.uri);
    setStatus("running", `scanning ${path.basename(filePath)}`);

    try {
      // Run single-file analysis
      const result = await runCytoScnPyAnalysis(filePath, config);
      if (document.isDirty || document.version !== documentVersion) {
        clearDocumentAnalysis(document);
        return;
      }
      const diagnostics = [
        ...findingsToDiagnostics(document, result.findings),
        ...parseErrorsToDiagnostics(document, result.parseErrors),
      ];

      // Update diagnostics for this file
      cytoscnpyDiagnostics.set(document.uri, diagnostics);

      cacheDocumentAnalysis(document, diagnostics, result.findings);
      resetAnalysisFailure();
      // Single-file results stay out of the workspace cache: they lack
      // cross-file references and would add dead-code false positives there.

      // Update sidebar and gutter decorations for active document
      if (
        vscode.window.activeTextEditor &&
        vscode.window.activeTextEditor.document.uri.toString() ===
        document.uri.toString()
      ) {
        applyGutterDecorations(vscode.window.activeTextEditor, diagnostics);
      }

      setStatus(
        "idle",
        `${diagnostics.length} finding${diagnostics.length === 1 ? "" : "s"}`,
      );

      if (isDevBuild) {
        cytoscnpyOutputChannel.appendLine(
          `[CytoScnPy] Incremental analysis completed for ${path.basename(
            filePath,
          )}`,
        );
      }
    } catch (error: any) {
      console.error(
        `[CytoScnPy] Incremental analysis failed for ${filePath}: ${error.message}`,
      );
      clearDocumentAnalysis(document);
      setStatus("error", "file analysis failed");
      reportAnalysisFailure(error.message);
    }
  }

  async function refreshDiagnostics(document: vscode.TextDocument) {
    if (document.languageId !== "python") {
      return; // Only analyze Python files
    }
    if (document.isDirty) {
      clearDocumentAnalysis(document);
      setStatus("idle", "save file to analyze");
      return;
    }

    // FILE MODE: Single file analysis (faster, but may have false positives)
    if (documentAnalysisMode(context, document.uri) === "file") {
      await runIncrementalAnalysis(document);
      return;
    }

    const filePath = getCacheKey(document.uri.fsPath);

    // WORKSPACE MODE: Full workspace analysis (accurate cross-file detection)
    // If we have a workspace cache, use it
    if (analysisState.workspaceCache) {
      const findings = analysisState.workspaceCache.get(filePath) || [];
      const parseErrors = analysisState.workspaceParseErrorsCache?.get(filePath) || [];
      const diagnostics = [
        ...findingsToDiagnostics(document, findings),
        ...parseErrorsToDiagnostics(document, parseErrors),
      ];
      cytoscnpyDiagnostics.set(document.uri, diagnostics);

      cacheDocumentAnalysis(document, diagnostics, findings);

      const editor = vscode.window.activeTextEditor;
      if (
        editor &&
        editor.document.uri.toString() === document.uri.toString()
      ) {
        applyGutterDecorations(editor, diagnostics);
      }
      return;
    }

    // No workspace cache - trigger workspace analysis, unless the last scan
    // failed: retrying on every opened or focused file repeats the failure.
    if (analysisState.workspaceScanFailed) {
      return;
    }
    await runFullWorkspaceAnalysis();
  }

  return { runIncrementalAnalysis, refreshDiagnostics };
}

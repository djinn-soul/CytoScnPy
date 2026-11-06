import * as vscode from "vscode";
import * as path from "path";
import { runCytoScnPyAnalysis } from "./analyzer";
import { documentAnalysisMode, getCytoScnPyConfiguration } from "./configuration";
import { analysisState } from "./extensionState";
import { getCacheKey, clearDocumentAnalysis, cacheDocumentAnalysis } from "./extensionCache";
import { findingsToDiagnostics, parseErrorsToDiagnostics } from "./extensionDiagnostics";
import { isUnsaved, notebookCellOf } from "./notebookCells";
import { setStatus, cytoscnpyDiagnostics, cytoscnpyOutputChannel, applyGutterDecorations, reportAnalysisFailure, resetAnalysisFailure } from "./extensionUi";

export function createFileAnalysis(
  context: vscode.ExtensionContext,
  runFullWorkspaceAnalysis: () => Promise<void>,
) {
  const isDevBuild = context.extensionMode === vscode.ExtensionMode.Development;
  // One analyzer run serves every cell of a notebook.
  const notebookRuns = new Map<string, Promise<void>>();

  /** The documents one analyzer run of `document`'s file updates. */
  function documentsOfFile(document: vscode.TextDocument): vscode.TextDocument[] {
    const cell = notebookCellOf(document);
    if (!cell) {
      return [document];
    }
    return cell.notebook
      .getCells()
      .filter((candidate) => candidate.kind === vscode.NotebookCellKind.Code)
      .map((candidate) => candidate.document)
      .filter((candidate) => candidate.languageId === "python");
  }

  async function runIncrementalAnalysis(document: vscode.TextDocument) {
    if (!notebookCellOf(document)) {
      return analyzeFile(document);
    }
    const key = document.uri.fsPath;
    let run = notebookRuns.get(key);
    if (!run) {
      run = analyzeFile(document).finally(() => notebookRuns.delete(key));
      notebookRuns.set(key, run);
    }
    return run;
  }

  async function analyzeFile(document: vscode.TextDocument) {
    const filePath = document.uri.fsPath;
    const documents = documentsOfFile(document);
    if (documents.some(isUnsaved)) {
      documents.forEach(clearDocumentAnalysis);
      return;
    }
    const versions = documents.map((candidate) => candidate.version);
    const config = getCytoScnPyConfiguration(context, document.uri);
    setStatus("running", `scanning ${path.basename(filePath)}`);

    try {
      // Run single-file analysis
      const result = await runCytoScnPyAnalysis(filePath, config);
      if (
        documents.some(
          (candidate, index) => isUnsaved(candidate) || candidate.version !== versions[index],
        )
      ) {
        documents.forEach(clearDocumentAnalysis);
        return;
      }
      let findingCount = 0;
      for (const target of documents) {
        const diagnostics = [
          ...findingsToDiagnostics(target, result.findings),
          ...parseErrorsToDiagnostics(target, result.parseErrors),
        ];
        findingCount += diagnostics.length;

        // Update diagnostics for this file
        cytoscnpyDiagnostics.set(target.uri, diagnostics);

        cacheDocumentAnalysis(target, diagnostics, result.findings);

        // Update sidebar and gutter decorations for active document
        if (
          vscode.window.activeTextEditor &&
          vscode.window.activeTextEditor.document.uri.toString() ===
          target.uri.toString()
        ) {
          applyGutterDecorations(vscode.window.activeTextEditor, diagnostics);
        }
      }
      resetAnalysisFailure();
      // Single-file results stay out of the workspace cache: they lack
      // cross-file references and would add dead-code false positives there.

      setStatus(
        "idle",
        `${findingCount} finding${findingCount === 1 ? "" : "s"}`,
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
      documents.forEach(clearDocumentAnalysis);
      setStatus("error", "file analysis failed");
      reportAnalysisFailure(error.message);
    }
  }

  async function refreshDiagnostics(document: vscode.TextDocument) {
    if (document.languageId !== "python") {
      return; // Only analyze Python files
    }
    if (isUnsaved(document)) {
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

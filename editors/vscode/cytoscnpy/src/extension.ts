import * as vscode from "vscode";
import { getCytoScnPyConfiguration } from "./configuration";
import { createWorkspaceAnalysis } from "./extensionWorkspace";
import { createFileAnalysis } from "./extensionFileAnalysis";
import { registerAnalysisEvents } from "./extensionEvents";
import { registerCommands } from "./extensionCommands";
import { registerMcpProvider } from "./extensionMcp";
import { initializeDecorations, cytoscnpyOutputChannel, disposeUi } from "./extensionUi";
import { QuickFixProvider } from "./quickFixProvider";

// Preserve the extension's public helpers used by integrations and tests.
export { CacheEntry, fileCache, computeHash, hashForDocument, getCacheKey } from "./extensionCache";
export { mapSeverity, buildClosedFileDiagnosticFromFinding, buildClosedFileDiagnosticFromParseError } from "./extensionDiagnostics";
export { runManualAnalysis } from "./extensionCommands";
export { QuickFixProvider } from "./quickFixProvider";

export function activate(context: vscode.ExtensionContext): void {
  const config = getCytoScnPyConfiguration(context);
  if (context.extensionMode === vscode.ExtensionMode.Development) {
    cytoscnpyOutputChannel.appendLine(
      `[CytoScnPy] Activated; binary=${config.path}, danger=${config.enableDangerScan}`,
    );
  }
  try {
    registerMcpProvider(context);
    initializeDecorations(context);
    const runFullWorkspaceAnalysis = createWorkspaceAnalysis(context);
    const fileAnalysis = createFileAnalysis(context, runFullWorkspaceAnalysis);
    const actions = { runFullWorkspaceAnalysis, ...fileAnalysis };
    registerAnalysisEvents(context, actions);
    registerCommands(context, actions);
    context.subscriptions.push(
      vscode.languages.registerCodeActionsProvider("python", new QuickFixProvider(), {
        providedCodeActionKinds: [vscode.CodeActionKind.QuickFix],
      }),
    );
  } catch (error) {
    console.error("Error during extension activation:", error);
  }
}

export function deactivate(): void {
  disposeUi();
}

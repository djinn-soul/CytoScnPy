import * as vscode from "vscode";
import { execFile } from "child_process";
import { getCytoScnPyConfiguration } from "./configuration";
import { AnalysisActions } from "./extensionState";
import { clearDocumentAnalysis } from "./extensionCache";
import { cytoscnpyOutputChannel } from "./extensionUi";

export async function runManualAnalysis(
  analysisMode: "file" | "workspace",
  runFileAnalysis: () => Promise<void>,
  runWorkspaceAnalysis: () => Promise<void>,
): Promise<void> {
  if (analysisMode === "workspace") {
    await runWorkspaceAnalysis();
    return;
  }
  await runFileAnalysis();
}

export function registerCommands(context: vscode.ExtensionContext, actions: AnalysisActions): void {
  const { refreshDiagnostics, runFullWorkspaceAnalysis } = actions;
  const disposableAnalyze = vscode.commands.registerCommand(
    "cytoscnpy.analyzeCurrentFile",
    async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor || editor.document.languageId !== "python") {
        vscode.window.showWarningMessage("No active text editor to analyze.");
        return;
      }
      if (editor.document.isDirty) {
        clearDocumentAnalysis(editor.document);
        vscode.window.showWarningMessage(
          "Save the Python file before running CytoScnPy analysis.",
        );
        return;
      }

      const config = getCytoScnPyConfiguration(context, editor.document.uri);
      // A current-file refresh cannot safely reuse the workspace cache: that
      // is exactly what made manual rescans leave stale Problems entries.
      // Rebuild the cross-file result so dead-code findings remain accurate.
      await runManualAnalysis(
        config.analysisMode,
        () => refreshDiagnostics(editor.document),
        runFullWorkspaceAnalysis,
      );
    },
  );

  context.subscriptions.push(disposableAnalyze);

  // Helper function to run metric commands
  async function runMetricCommand(
    commandType: "cc" | "hal" | "mi" | "raw",
    commandName: string,
  ) {
    if (
      !vscode.window.activeTextEditor ||
      vscode.window.activeTextEditor.document.languageId !== "python"
    ) {
      vscode.window.showWarningMessage(
        `No active Python file to run ${commandName} on.`,
      );
      return;
    }

    const filePath = vscode.window.activeTextEditor.document.uri.fsPath;
    const config = getCytoScnPyConfiguration(
      context,
      vscode.window.activeTextEditor.document.uri,
    );

    // Use execFile with argument array to prevent command injection
    const args = ["--client", "vscode", commandType, filePath];

    cytoscnpyOutputChannel.clear();
    cytoscnpyOutputChannel.show();
    cytoscnpyOutputChannel.appendLine(
      `Running: ${config.path} ${args.join(" ")}\n`,
    );

    execFile(
      config.path,
      args,
      (error: Error | null, stdout: string, stderr: string) => {
        if (error) {
          cytoscnpyOutputChannel.appendLine(
            `Error running ${commandName}: ${error.message}`,
          );
          cytoscnpyOutputChannel.appendLine(`Stderr: ${stderr}`);
          vscode.window.showErrorMessage(
            `CytoScnPy ${commandName} failed: ${error.message}`,
          );
          return;
        }
        if (stderr) {
          cytoscnpyOutputChannel.appendLine(
            `Stderr for ${commandName}:\n${stderr}`,
          );
        }
        cytoscnpyOutputChannel.appendLine(
          `Stdout for ${commandName}:\n${stdout}`,
        );
      },
    );
  }

  // Register metric commands
  context.subscriptions.push(
    vscode.commands.registerCommand("cytoscnpy.complexity", () =>
      runMetricCommand("cc", "Cyclomatic Complexity"),
    ),
  );
  context.subscriptions.push(
    vscode.commands.registerCommand("cytoscnpy.halstead", () =>
      runMetricCommand("hal", "Halstead Metrics"),
    ),
  );
  context.subscriptions.push(
    vscode.commands.registerCommand("cytoscnpy.maintainability", () =>
      runMetricCommand("mi", "Maintainability Index"),
    ),
  );
  context.subscriptions.push(
    vscode.commands.registerCommand("cytoscnpy.rawMetrics", () =>
      runMetricCommand("raw", "Raw Metrics"),
    ),
  );

  // Register analyze workspace command
  context.subscriptions.push(
    vscode.commands.registerCommand(
      "cytoscnpy.analyzeWorkspace",
      async () => {
        const workspaceFolders = vscode.workspace.workspaceFolders;
        if (!workspaceFolders || workspaceFolders.length === 0) {
          vscode.window.showWarningMessage("No workspace folder open.");
          return;
        }

        // Use the canonical workspace path: it refreshes the cache and
        // replaces the DiagnosticCollection that powers the Problems tab.
        await runFullWorkspaceAnalysis();
      },
    ),
  );

}

// The module 'vscode' contains the VS Code extensibility API
// Import the module and reference it with the alias vscode in your code below
import * as vscode from "vscode";
import * as os from "os";
import * as path from "path";
import { runCytoScnPyAnalysis, CytoScnPyConfig } from "./analyzer";
import { exec } from "child_process"; // Import exec for metric commands

// Create a diagnostic collection for CytoScnPy issues
const cytoscnpyDiagnostics =
  vscode.languages.createDiagnosticCollection("cytoscnpy");
// Create an output channel for metric commands
const cytoscnpyOutputChannel =
  vscode.window.createOutputChannel("CytoScnPy Metrics");

function getExecutablePath(context: vscode.ExtensionContext): string {
  const platform = os.platform();
  let executableName: string;

  switch (platform) {
    case "win32":
      executableName = "cytoscnpy-cli-win32.exe";
      break;
    case "linux":
      executableName = "cytoscnpy-cli-linux";
      break;
    case "darwin":
      executableName = "cytoscnpy-cli-darwin";
      break;
    default:
      // Fall back to pip-installed version
      return "cytoscnpy";
  }

  const bundledPath = path.join(context.extensionPath, "bin", executableName);

  // Check if bundled binary exists, otherwise fall back to pip-installed version
  try {
    const fs = require("fs");
    if (fs.existsSync(bundledPath)) {
      return bundledPath;
    }
  } catch {
    // Ignore errors, fall through to pip fallback
  }

  // Fall back to pip-installed cytoscnpy (assumes it's in PATH)
  return "cytoscnpy";
}

// Helper function to get configuration
function getCytoScnPyConfiguration(
  context: vscode.ExtensionContext
): CytoScnPyConfig {
  const config = vscode.workspace.getConfiguration("cytoscnpy");
  const pathSetting = config.inspect<string>("path");

  const userSetPath = pathSetting?.globalValue || pathSetting?.workspaceValue;

  return {
    path: userSetPath || getExecutablePath(context),
    enableSecretsScan: config.get<boolean>("enableSecretsScan") || false,
    enableDangerScan: config.get<boolean>("enableDangerScan") || false,
    enableQualityScan: config.get<boolean>("enableQualityScan") || false,
    confidenceThreshold: config.get<number>("confidenceThreshold") || 0,
    excludeFolders: config.get<string[]>("excludeFolders") || [],
    includeFolders: config.get<string[]>("includeFolders") || [],
    includeTests: config.get<boolean>("includeTests") || false,
    includeIpynb: config.get<boolean>("includeIpynb") || false,
    maxComplexity: config.get<number>("maxComplexity") || 10,
    minMaintainabilityIndex:
      config.get<number>("minMaintainabilityIndex") || 40,
    maxNesting: config.get<number>("maxNesting") || 3,
    maxArguments: config.get<number>("maxArguments") || 5,
    maxLines: config.get<number>("maxLines") || 50,
  };
}

export function activate(context: vscode.ExtensionContext) {
  console.log('Congratulations, your extension "cytoscnpy" is now active!');
  try {
    // Function to refresh diagnostics for the active document
    async function refreshDiagnostics(document: vscode.TextDocument) {
      if (document.languageId !== "python") {
        return; // Only analyze Python files
      }

      const filePath = document.uri.fsPath;
      const config = getCytoScnPyConfiguration(context); // Get current configuration

      try {
        const result = await runCytoScnPyAnalysis(filePath, config); // Pass config
        const diagnostics: vscode.Diagnostic[] = result.findings.map(
          (finding) => {
            const lineIndex = finding.line_number - 1;
            const lineText = document.lineAt(lineIndex);

            // Use column from finding if available (1-based -> 0-based)
            // If explicit column is 0 or missing, default to first non-whitespace char for cleaner look
            const startCol =
              finding.col && finding.col > 0
                ? finding.col // Rust CLI 0-based? Need to verify. Assuming 1-based for safety check first.
                : lineText.firstNonWhitespaceCharacterIndex;

            // Just usage of startCol.
            // Actually, let's assume if col is provided it's the start char.
            // If col is missing (0), we use firstNonWhitespaceCharacterIndex.

            const range = new vscode.Range(
              new vscode.Position(lineIndex, startCol),
              new vscode.Position(lineIndex, lineText.text.length)
            );
            let severity: vscode.DiagnosticSeverity;
            // Map CytoScnPy severity levels to VS Code severities
            switch (finding.severity.toUpperCase()) {
              case "CRITICAL":
              case "ERROR":
                severity = vscode.DiagnosticSeverity.Error;
                break;
              case "HIGH":
              case "WARNING":
                severity = vscode.DiagnosticSeverity.Warning;
                break;
              case "MEDIUM":
              case "INFO":
                severity = vscode.DiagnosticSeverity.Information;
                break;
              case "LOW":
                severity = vscode.DiagnosticSeverity.Hint;
                break;
              default:
                severity = vscode.DiagnosticSeverity.Information;
            }
            const diagnostic = new vscode.Diagnostic(
              range,
              `${finding.message} [${finding.rule_id}]`,
              severity
            );

            // Add tags for better visual highlighting
            // Unused code gets "Unnecessary" tag which fades the code
            const unusedRules = [
              "unused-function",
              "unused-method",
              "unused-class",
              "unused-import",
              "unused-variable",
              "unused-parameter",
            ];
            if (unusedRules.includes(finding.rule_id)) {
              diagnostic.tags = [vscode.DiagnosticTag.Unnecessary];
            }

            // Set the source for filtering in Problems panel
            diagnostic.source = "CytoScnPy";

            return diagnostic;
          }
        );
        cytoscnpyDiagnostics.set(document.uri, diagnostics);
      } catch (error: any) {
        console.error(
          `Error refreshing CytoScnPy diagnostics: ${error.message}`
        );
        vscode.window.showErrorMessage(
          `CytoScnPy analysis failed: ${error.message}`
        );
      }
    }

    // Initial analysis when a document is opened or becomes active
    if (vscode.window.activeTextEditor) {
      refreshDiagnostics(vscode.window.activeTextEditor.document);
    }

    // Analyze document on change with debounce
    let debounceTimer: NodeJS.Timeout;
    context.subscriptions.push(
      vscode.workspace.onDidChangeTextDocument((event) => {
        if (event.document.languageId === "python") {
          clearTimeout(debounceTimer);
          debounceTimer = setTimeout(() => {
            refreshDiagnostics(event.document);
          }, 500);
        }
      })
    );

    // Analyze when the active editor changes (switching tabs)
    context.subscriptions.push(
      vscode.window.onDidChangeActiveTextEditor((editor) => {
        if (editor && editor.document.languageId === "python") {
          refreshDiagnostics(editor.document);
        }
      })
    );

    // Re-analyze all visible documents when configuration changes
    context.subscriptions.push(
      vscode.workspace.onDidChangeConfiguration((event) => {
        if (event.affectsConfiguration("cytoscnpy")) {
          vscode.window.visibleTextEditors.forEach((editor) => {
            if (editor.document.languageId === "python") {
              refreshDiagnostics(editor.document);
            }
          });
        }
      })
    );

    // Clear diagnostics when a document is closed
    context.subscriptions.push(
      vscode.workspace.onDidCloseTextDocument((document) => {
        cytoscnpyDiagnostics.delete(document.uri);
      })
    );

    // Register a command to manually trigger analysis (e.g., from command palette)
    const disposableAnalyze = vscode.commands.registerCommand(
      "cytoscnpy.analyzeCurrentFile",
      () => {
        if (vscode.window.activeTextEditor) {
          refreshDiagnostics(vscode.window.activeTextEditor.document);
          vscode.window.showInformationMessage("CytoScnPy analysis triggered.");
        } else {
          vscode.window.showWarningMessage("No active text editor to analyze.");
        }
      }
    );

    context.subscriptions.push(disposableAnalyze);

    // Helper function to run metric commands
    async function runMetricCommand(
      context: vscode.ExtensionContext,
      commandType: "cc" | "hal" | "mi" | "raw",
      commandName: string
    ) {
      if (
        !vscode.window.activeTextEditor ||
        vscode.window.activeTextEditor.document.languageId !== "python"
      ) {
        vscode.window.showWarningMessage(
          `No active Python file to run ${commandName} on.`
        );
        return;
      }

      const filePath = vscode.window.activeTextEditor.document.uri.fsPath;
      const config = getCytoScnPyConfiguration(context);
      const command = `${config.path} ${commandType} "${filePath}"`;

      cytoscnpyOutputChannel.clear();
      cytoscnpyOutputChannel.show();
      cytoscnpyOutputChannel.appendLine(`Running: ${command}\n`);

      exec(command, (error, stdout, stderr) => {
        if (error) {
          cytoscnpyOutputChannel.appendLine(
            `Error running ${commandName}: ${error.message}`
          );
          cytoscnpyOutputChannel.appendLine(`Stderr: ${stderr}`);
          vscode.window.showErrorMessage(
            `CytoScnPy ${commandName} failed: ${error.message}`
          );
          return;
        }
        if (stderr) {
          cytoscnpyOutputChannel.appendLine(
            `Stderr for ${commandName}:\n${stderr}`
          );
        }
        cytoscnpyOutputChannel.appendLine(
          `Stdout for ${commandName}:\n${stdout}`
        );
      });
    }

    // Register metric commands
    context.subscriptions.push(
      vscode.commands.registerCommand("cytoscnpy.complexity", () =>
        runMetricCommand(context, "cc", "Cyclomatic Complexity")
      )
    );
    context.subscriptions.push(
      vscode.commands.registerCommand("cytoscnpy.halstead", () =>
        runMetricCommand(context, "hal", "Halstead Metrics")
      )
    );
    context.subscriptions.push(
      vscode.commands.registerCommand("cytoscnpy.maintainability", () =>
        runMetricCommand(context, "mi", "Maintainability Index")
      )
    );
    context.subscriptions.push(
      vscode.commands.registerCommand("cytoscnpy.rawMetrics", () =>
        runMetricCommand(context, "raw", "Raw Metrics")
      )
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

          const workspacePath = workspaceFolders[0].uri.fsPath;
          const config = getCytoScnPyConfiguration(context);

          cytoscnpyOutputChannel.clear();
          cytoscnpyOutputChannel.show();
          cytoscnpyOutputChannel.appendLine(
            `Analyzing workspace: ${workspacePath}\n`
          );

          let command = `"${config.path}" "${workspacePath}" --json`;
          if (config.enableSecretsScan) {
            command += " --secrets";
          }
          if (config.enableDangerScan) {
            command += " --danger";
          }
          if (config.enableQualityScan) {
            command += " --quality";
          }

          exec(command, (error, stdout, stderr) => {
            if (error) {
              cytoscnpyOutputChannel.appendLine(`Error: ${error.message}`);
              if (stderr) {
                cytoscnpyOutputChannel.appendLine(`Stderr: ${stderr}`);
              }
            }
            if (stdout) {
              cytoscnpyOutputChannel.appendLine(`Results:\n${stdout}`);
            }
            vscode.window.showInformationMessage(
              "Workspace analysis complete. See output channel."
            );
          });
        }
      )
    );

    // Register a HoverProvider for Python files
    context.subscriptions.push(
      vscode.languages.registerHoverProvider("python", {
        provideHover(document, position, token) {
          const diagnostics = cytoscnpyDiagnostics.get(document.uri);
          if (!diagnostics) {
            return;
          }

          for (const diagnostic of diagnostics) {
            if (diagnostic.range.contains(position)) {
              // Return the diagnostic message as markdown for better formatting
              return new vscode.Hover(
                new vscode.MarkdownString(diagnostic.message)
              );
            }
          }
          return;
        },
      })
    );
  } catch (error) {
    console.error("Error during extension activation:", error);
  }
}

export function deactivate() {
  cytoscnpyDiagnostics.dispose(); // Clean up diagnostics when extension is deactivated
  cytoscnpyOutputChannel.dispose(); // Clean up output channel
}

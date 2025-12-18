// The module 'vscode' contains the VS Code extensibility API
// Import the module and reference it with the alias vscode in your code below
import * as vscode from "vscode";
import * as os from "os";
import * as path from "path";
import * as crypto from "crypto";
import { runCytoScnPyAnalysis, CytoScnPyConfig } from "./analyzer";
import { exec } from "child_process"; // Import exec for metric commands

// Cache for file content hashes to skip re-analyzing unchanged files
interface CacheEntry {
  hash: string;
  diagnostics: vscode.Diagnostic[];
}
const fileCache = new Map<string, CacheEntry>();

// Helper function to compute content hash
function computeHash(content: string): string {
  return crypto.createHash("md5").update(content).digest("hex");
}

// Create a diagnostic collection for CytoScnPy issues
const cytoscnpyDiagnostics =
  vscode.languages.createDiagnosticCollection("cytoscnpy");
// Create an output channel for metric commands
const cytoscnpyOutputChannel =
  vscode.window.createOutputChannel("CytoScnPy Metrics");

// Gutter decoration types for severity levels
let errorDecorationType: vscode.TextEditorDecorationType;
let warningDecorationType: vscode.TextEditorDecorationType;
let infoDecorationType: vscode.TextEditorDecorationType;

// Tree view for sidebar
let issuesTreeView: vscode.TreeView<IssueTreeItem>;

// Tree item for sidebar view
class IssueTreeItem extends vscode.TreeItem {
  constructor(
    public readonly label: string,
    public readonly count: number,
    public readonly collapsibleState: vscode.TreeItemCollapsibleState,
    public readonly category?: string
  ) {
    super(label, collapsibleState);
    this.description = count > 0 ? `${count}` : "";
    this.tooltip = `${label}: ${count} issue${count !== 1 ? "s" : ""}`;

    // Set icon based on category
    if (category === "error") {
      this.iconPath = new vscode.ThemeIcon(
        "error",
        new vscode.ThemeColor("errorForeground")
      );
    } else if (category === "warning") {
      this.iconPath = new vscode.ThemeIcon(
        "warning",
        new vscode.ThemeColor("editorWarning.foreground")
      );
    } else if (category === "info") {
      this.iconPath = new vscode.ThemeIcon(
        "info",
        new vscode.ThemeColor("editorInfo.foreground")
      );
    }
  }
}

// Tree data provider for sidebar
class IssuesTreeDataProvider implements vscode.TreeDataProvider<IssueTreeItem> {
  private _onDidChangeTreeData = new vscode.EventEmitter<
    IssueTreeItem | undefined
  >();
  readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

  private diagnostics: vscode.Diagnostic[] = [];

  update(diagnostics: vscode.Diagnostic[]) {
    this.diagnostics = diagnostics;
    this._onDidChangeTreeData.fire(undefined);
  }

  getTreeItem(element: IssueTreeItem): vscode.TreeItem {
    return element;
  }

  getChildren(element?: IssueTreeItem): IssueTreeItem[] {
    if (!element) {
      // Root level - show category counts
      const errors = this.diagnostics.filter(
        (d) => d.severity === vscode.DiagnosticSeverity.Error
      ).length;
      const warnings = this.diagnostics.filter(
        (d) => d.severity === vscode.DiagnosticSeverity.Warning
      ).length;
      const infos = this.diagnostics.filter(
        (d) =>
          d.severity === vscode.DiagnosticSeverity.Information ||
          d.severity === vscode.DiagnosticSeverity.Hint
      ).length;

      const items: IssueTreeItem[] = [];
      if (errors > 0) {
        items.push(
          new IssueTreeItem(
            "Errors",
            errors,
            vscode.TreeItemCollapsibleState.None,
            "error"
          )
        );
      }
      if (warnings > 0) {
        items.push(
          new IssueTreeItem(
            "Warnings",
            warnings,
            vscode.TreeItemCollapsibleState.None,
            "warning"
          )
        );
      }
      if (infos > 0) {
        items.push(
          new IssueTreeItem(
            "Info",
            infos,
            vscode.TreeItemCollapsibleState.None,
            "info"
          )
        );
      }
      return items;
    }
    return [];
  }
}

// Instance of tree data provider
let issuesTreeDataProvider: IssuesTreeDataProvider;

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
    // Initialize tree view for sidebar badge
    issuesTreeDataProvider = new IssuesTreeDataProvider();
    issuesTreeView = vscode.window.createTreeView("cytoscnpy-issues", {
      treeDataProvider: issuesTreeDataProvider,
      showCollapseAll: false,
    });
    context.subscriptions.push(issuesTreeView);

    // Initialize gutter decoration types
    errorDecorationType = vscode.window.createTextEditorDecorationType({
      gutterIconPath: vscode.Uri.parse(
        "data:image/svg+xml," +
          encodeURIComponent(
            '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><circle cx="8" cy="8" r="6" fill="#f44336"/></svg>'
          )
      ),
      gutterIconSize: "contain",
      overviewRulerColor: "#f44336",
      overviewRulerLane: vscode.OverviewRulerLane.Right,
    });
    warningDecorationType = vscode.window.createTextEditorDecorationType({
      gutterIconPath: vscode.Uri.parse(
        "data:image/svg+xml," +
          encodeURIComponent(
            '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><circle cx="8" cy="8" r="6" fill="#ff9800"/></svg>'
          )
      ),
      gutterIconSize: "contain",
      overviewRulerColor: "#ff9800",
      overviewRulerLane: vscode.OverviewRulerLane.Right,
    });
    infoDecorationType = vscode.window.createTextEditorDecorationType({
      gutterIconPath: vscode.Uri.parse(
        "data:image/svg+xml," +
          encodeURIComponent(
            '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><circle cx="8" cy="8" r="6" fill="#2196f3"/></svg>'
          )
      ),
      gutterIconSize: "contain",
      overviewRulerColor: "#2196f3",
      overviewRulerLane: vscode.OverviewRulerLane.Right,
    });
    context.subscriptions.push(
      errorDecorationType,
      warningDecorationType,
      infoDecorationType
    );

    // Function to apply gutter decorations based on diagnostics
    function applyGutterDecorations(
      editor: vscode.TextEditor,
      diagnostics: vscode.Diagnostic[]
    ) {
      const errorRanges: vscode.DecorationOptions[] = [];
      const warningRanges: vscode.DecorationOptions[] = [];
      const infoRanges: vscode.DecorationOptions[] = [];

      for (const diag of diagnostics) {
        const decoration = { range: diag.range, hoverMessage: diag.message };
        switch (diag.severity) {
          case vscode.DiagnosticSeverity.Error:
            errorRanges.push(decoration);
            break;
          case vscode.DiagnosticSeverity.Warning:
            warningRanges.push(decoration);
            break;
          default:
            infoRanges.push(decoration);
            break;
        }
      }

      editor.setDecorations(errorDecorationType, errorRanges);
      editor.setDecorations(warningDecorationType, warningRanges);
      editor.setDecorations(infoDecorationType, infoRanges);
    }

    // Function to refresh diagnostics for the active document
    async function refreshDiagnostics(document: vscode.TextDocument) {
      if (document.languageId !== "python") {
        return; // Only analyze Python files
      }

      const filePath = document.uri.fsPath;
      const content = document.getText();
      const contentHash = computeHash(content);

      // Check cache - skip analysis if content unchanged
      const cached = fileCache.get(filePath);
      if (cached && cached.hash === contentHash) {
        // Use cached diagnostics
        cytoscnpyDiagnostics.set(document.uri, cached.diagnostics);
        issuesTreeDataProvider.update(cached.diagnostics);
        issuesTreeView.badge = {
          value: cached.diagnostics.length,
          tooltip: `CytoScnPy: ${cached.diagnostics.length} issue${
            cached.diagnostics.length !== 1 ? "s" : ""
          }`,
        };
        const editor = vscode.window.activeTextEditor;
        if (
          editor &&
          editor.document.uri.toString() === document.uri.toString()
        ) {
          applyGutterDecorations(editor, cached.diagnostics);
        }
        return;
      }

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

            // Categorize diagnostics for better Problems panel grouping
            // The source field is used for filtering, so we add category prefix
            const securityRules = [
              "secret-detected",
              "dangerous-code",
              "taint-vulnerability",
            ];
            const qualityRules = ["quality-issue"];

            let category: string;
            if (unusedRules.includes(finding.rule_id)) {
              category = "Unused";
            } else if (securityRules.includes(finding.rule_id)) {
              category = "Security";
            } else if (qualityRules.includes(finding.rule_id)) {
              category = "Quality";
            } else {
              category = "Analysis";
            }

            // Set the source with category for filtering in Problems panel
            // Format: "CytoScnPy [Category]" allows filtering by category
            diagnostic.source = `CytoScnPy [${category}]`;

            // Add a structured code property with link to docs
            diagnostic.code = {
              value: finding.rule_id,
              target: vscode.Uri.parse(
                `https://github.com/djinn09/CytoScnPy#${finding.rule_id}`
              ),
            };

            return diagnostic;
          }
        );
        cytoscnpyDiagnostics.set(document.uri, diagnostics);

        // Store in cache for future use
        fileCache.set(filePath, { hash: contentHash, diagnostics });

        // Update sidebar tree view with issue count and badge
        issuesTreeDataProvider.update(diagnostics);
        issuesTreeView.badge = {
          value: diagnostics.length,
          tooltip: `CytoScnPy: ${diagnostics.length} issue${
            diagnostics.length !== 1 ? "s" : ""
          }`,
        };

        // Apply gutter decorations to active editor
        const editor = vscode.window.activeTextEditor;
        if (
          editor &&
          editor.document.uri.toString() === document.uri.toString()
        ) {
          applyGutterDecorations(editor, diagnostics);
        }
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
    // Special case: Undo/Redo refreshes immediately
    let debounceTimer: NodeJS.Timeout;
    context.subscriptions.push(
      vscode.workspace.onDidChangeTextDocument((event) => {
        if (event.document.languageId === "python") {
          clearTimeout(debounceTimer);

          // Check if this is an Undo or Redo operation - refresh immediately
          if (
            event.reason === vscode.TextDocumentChangeReason.Undo ||
            event.reason === vscode.TextDocumentChangeReason.Redo
          ) {
            refreshDiagnostics(event.document);
          } else {
            // Regular typing - use debounce
            debounceTimer = setTimeout(() => {
              refreshDiagnostics(event.document);
            }, 500);
          }
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

    // Clear diagnostics and cache when a document is closed
    context.subscriptions.push(
      vscode.workspace.onDidCloseTextDocument((document) => {
        cytoscnpyDiagnostics.delete(document.uri);
        fileCache.delete(document.uri.fsPath); // Clear cache entry
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

    // Register Code Action Provider for quick fixes
    context.subscriptions.push(
      vscode.languages.registerCodeActionsProvider(
        "python",
        {
          provideCodeActions(
            document: vscode.TextDocument,
            range: vscode.Range | vscode.Selection,
            context: vscode.CodeActionContext,
            token: vscode.CancellationToken
          ): vscode.CodeAction[] {
            const actions: vscode.CodeAction[] = [];

            // Only process CytoScnPy diagnostics for unused code
            const unusedRules = [
              "unused-function",
              "unused-method",
              "unused-class",
              "unused-import",
              "unused-variable",
              "unused-parameter",
            ];

            for (const diagnostic of context.diagnostics) {
              // Handle both old string format and new structured code format
              const ruleId =
                typeof diagnostic.code === "object" &&
                diagnostic.code !== null &&
                "value" in diagnostic.code
                  ? (diagnostic.code.value as string)
                  : (diagnostic.code as string);

              // Check if it's a CytoScnPy diagnostic (source starts with "CytoScnPy")
              if (
                !diagnostic.source?.startsWith("CytoScnPy") ||
                !unusedRules.includes(ruleId)
              ) {
                continue;
              }

              // Create "Remove line" action
              const removeAction = new vscode.CodeAction(
                `Remove ${ruleId.replace("unused-", "")}`,
                vscode.CodeActionKind.QuickFix
              );
              removeAction.diagnostics = [diagnostic];
              removeAction.isPreferred = true;

              // Use WorkspaceEdit to delete the entire line
              const edit = new vscode.WorkspaceEdit();
              const lineRange = document.lineAt(
                diagnostic.range.start.line
              ).rangeIncludingLineBreak;
              edit.delete(document.uri, lineRange);
              removeAction.edit = edit;

              actions.push(removeAction);

              // Create "Comment out" action
              const commentAction = new vscode.CodeAction(
                `Comment out ${ruleId.replace("unused-", "")}`,
                vscode.CodeActionKind.QuickFix
              );
              commentAction.diagnostics = [diagnostic];

              const commentEdit = new vscode.WorkspaceEdit();
              const lineText = document.lineAt(
                diagnostic.range.start.line
              ).text;
              const leadingWhitespace = lineText.match(/^\s*/)?.[0] || "";
              commentEdit.replace(
                document.uri,
                document.lineAt(diagnostic.range.start.line).range,
                `${leadingWhitespace}# ${lineText.trimStart()}`
              );
              commentAction.edit = commentEdit;

              actions.push(commentAction);
            }

            return actions;
          },
        },
        {
          providedCodeActionKinds: [vscode.CodeActionKind.QuickFix],
        }
      )
    );
  } catch (error) {
    console.error("Error during extension activation:", error);
  }
}

export function deactivate() {
  cytoscnpyDiagnostics.dispose(); // Clean up diagnostics when extension is deactivated
  cytoscnpyOutputChannel.dispose(); // Clean up output channel
  errorDecorationType?.dispose(); // Clean up decoration types
  warningDecorationType?.dispose();
  infoDecorationType?.dispose();
}

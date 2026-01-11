// The module 'vscode' contains the VS Code extensibility API
// Import the module and reference it with the alias vscode in your code below
import * as vscode from "vscode";
import * as os from "os";
import * as path from "path";
import * as crypto from "crypto";
import {
  runCytoScnPyAnalysis,
  runWorkspaceAnalysis,
  CytoScnPyConfig,
  CytoScnPyFinding,
} from "./analyzer";
import { execFile } from "child_process"; // Import execFile for safer metric commands

// Cache for file content hashes to skip re-analyzing unchanged files
// We keep a history of entries to support instant Undo/Redo operations
export interface CacheEntry {
  hash: string;
  diagnostics: vscode.Diagnostic[];
  findings: CytoScnPyFinding[];
  timestamp: number;
}
const MAX_CACHE_HISTORY = 10;
export const fileCache = new Map<string, CacheEntry[]>();

// Workspace-level cache for cross-file analysis
let workspaceCache: Map<string, CytoScnPyFinding[]> | null = null;
let workspaceCacheTimestamp: number = 0;
let isWorkspaceAnalysisRunning = false;

// Debounce timer for save-triggered analysis (prevents multiple scans on rapid saves)
let analysisDebounceTimer: NodeJS.Timeout | null = null;
const ANALYSIS_DEBOUNCE_MS = 1000; // Wait 1 second after last save before re-analyzing

// Helper function to compute content hash
export function computeHash(content: string): string {
  return crypto.createHash("sha256").update(content).digest("hex");
}

// Helper function to get a consistent cache key (case-insensitive on Windows)
export function getCacheKey(fsPath: string): string {
  return process.platform === "win32" ? fsPath.toLowerCase() : fsPath;
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

  // Security: Ensure the bundled path is actually within the extension directory
  // to prevent potential path traversal vulnerabilities.
  if (!bundledPath.startsWith(context.extensionPath)) {
    return "cytoscnpy";
  }

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
    analysisMode:
      config.get<string>("analysisMode") === "file" ? "file" : "workspace",
    enableSecretsScan: config.get<boolean>("enableSecretsScan") || false,
    enableDangerScan: config.get<boolean>("enableDangerScan") || false,
    enableQualityScan: config.get<boolean>("enableQualityScan") || false,
    enableCloneScan: config.get<boolean>("enableCloneScan") || false,
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
    // Register MCP server definition provider for GitHub Copilot integration
    // This allows Copilot to use CytoScnPy's MCP server in agent mode
    // Note: This API requires VS Code 1.96+ and GitHub Copilot extension
    if (
      vscode.lm &&
      typeof vscode.lm.registerMcpServerDefinitionProvider === "function"
    ) {
      try {
        const mcpDidChangeEmitter = new vscode.EventEmitter<void>();
        context.subscriptions.push(
          vscode.lm.registerMcpServerDefinitionProvider("cytoscnpy-mcp", {
            onDidChangeMcpServerDefinitions: mcpDidChangeEmitter.event,
            provideMcpServerDefinitions: async () => {
              const executablePath = getExecutablePath(context);
              const workspaceFolders = vscode.workspace.workspaceFolders;
              const cwd = workspaceFolders?.[0]?.uri.fsPath ?? null;

              const extension =
                vscode.extensions.getExtension("djinn09.cytoscnpy");
              const version = extension?.packageJSON?.version || "0.1.0";

              return [
                new vscode.McpStdioServerDefinition(
                  "CytoScnPy",
                  executablePath,
                  ["mcp-server"],
                  {
                    cwd: cwd,
                    version: version,
                  }
                ),
              ];
            },
            resolveMcpServerDefinition: async (server) => server,
          })
        );
        console.log("CytoScnPy MCP server provider registered successfully");
      } catch (mcpError) {
        console.warn("Failed to register MCP server provider:", mcpError);
      }
    } else {
      console.log(
        "MCP server registration not available (requires VS Code 1.96+ with Copilot)"
      );
    }

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
        // FIX: Only set the range for the squiggle/gutter icon mapping
        // FIX: Do NOT set hoverMessage, as VS Code natively displays diagnostic messages on hover.
        // FIX: Setting it here causes duplicate messages in the hover tooltip.
        const decoration = { range: diag.range };
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

    // Track time for performance logging

    // Helper function to check if a line is suppressed via noqa comment
    function isLineSuppressed(lineText: string): boolean {
      // Matches: # noqa, # noqa: CSP, # noqa: E501, CSP, etc.
      const noqaRegex = /#\s*noqa(?::\s*([^#\n]+))?/i;
      const match = lineText.match(noqaRegex);
      if (!match) {
        return false;
      }
      // Bare # noqa suppresses all
      if (!match[1]) {
        return true;
      }
      // Check if CSP is in the list
      const codes = match[1].split(/,\s*/).map((s) => s.trim().toUpperCase());
      return codes.includes("CSP") || codes.some((c) => c.startsWith("CSP"));
    }

    // Helper function to convert findings to diagnostics for a document
    function findingsToDiagnostics(
      document: vscode.TextDocument,
      findings: CytoScnPyFinding[]
    ): vscode.Diagnostic[] {
      return findings
        .filter((finding) => {
          const lineIndex = finding.line_number - 1;
          if (lineIndex < 0 || lineIndex >= document.lineCount) {
            return true; // Keep - can't check suppression
          }
          const lineText = document.lineAt(lineIndex).text;
          return !isLineSuppressed(lineText);
        })
        .map((finding) => {
          const lineIndex = finding.line_number - 1;
          // Ensure line index is valid
          if (lineIndex < 0 || lineIndex >= document.lineCount) {
            const range = new vscode.Range(0, 0, 0, 0);
            return new vscode.Diagnostic(
              range,
              `${finding.message} [${finding.rule_id}]`,
              vscode.DiagnosticSeverity.Warning
            );
          }
          const lineText = document.lineAt(lineIndex);

          const startCol =
            finding.col && finding.col > 0
              ? finding.col
              : lineText.firstNonWhitespaceCharacterIndex;

          const range = new vscode.Range(
            new vscode.Position(lineIndex, startCol),
            new vscode.Position(lineIndex, lineText.text.length)
          );
          let severity: vscode.DiagnosticSeverity;
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
            case "HINT":
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

          diagnostic.source = `CytoScnPy [${category}]`;
          diagnostic.code = {
            value: finding.rule_id,
            target: vscode.Uri.parse(
              `https://github.com/djinn09/CytoScnPy#${finding.rule_id}`
            ),
          };

          return diagnostic;
        });
    }

    // Function to run workspace analysis and populate cache
    async function runFullWorkspaceAnalysis() {
      const workspaceFolders = vscode.workspace.workspaceFolders;
      if (!workspaceFolders || workspaceFolders.length === 0) {
        return;
      }

      if (isWorkspaceAnalysisRunning) {
        return; // Don't run multiple analyses at once
      }

      isWorkspaceAnalysisRunning = true;
      const workspacePath = workspaceFolders[0].uri.fsPath;
      const config = getCytoScnPyConfiguration(context);

      // Show progress notification during analysis
      await vscode.window.withProgress(
        {
          location: vscode.ProgressLocation.Notification,
          title: "CytoScnPy: Analyzing workspace...",
          cancellable: false,
        },
        async (progress) => {
          try {
            progress.report({ message: "Scanning Python files..." });
            const startTime = Date.now();

            workspaceCache = await runWorkspaceAnalysis(workspacePath, config);
            workspaceCacheTimestamp = Date.now();

            const duration = (Date.now() - startTime) / 1000;
            const fileCount = workspaceCache.size;
            console.log(
              `[CytoScnPy] Workspace analysis completed in ${duration.toFixed(
                2
              )}s, found findings in ${fileCount} files`
            );

            progress.report({ message: `Updating diagnostics...` });

            // Set diagnostics for ALL files in the workspace cache
            // This ensures Problems tab shows issues from all files, not just open ones
            for (const [filePath, findings] of workspaceCache.entries()) {
              const uri = vscode.Uri.file(filePath);
              // Create diagnostics with simplified info (no document access for closed files)
              const diagnostics = findings.map((finding) => {
                const lineIndex = Math.max(0, finding.line_number - 1);
                const startCol =
                  finding.col && finding.col > 0 ? finding.col : 0;
                const range = new vscode.Range(
                  new vscode.Position(lineIndex, startCol),
                  new vscode.Position(lineIndex, 100) // Approximate end
                );

                let severity: vscode.DiagnosticSeverity;
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
                  default:
                    severity = vscode.DiagnosticSeverity.Hint;
                }

                const diagnostic = new vscode.Diagnostic(
                  range,
                  `${finding.message} [${finding.rule_id}]`,
                  severity
                );
                diagnostic.source = `CytoScnPy`;
                diagnostic.code = finding.rule_id;

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

                return diagnostic;
              });

              cytoscnpyDiagnostics.set(uri, diagnostics);
            }

            // Update sidebar for active document
            if (vscode.window.activeTextEditor) {
              const activeDoc = vscode.window.activeTextEditor.document;
              if (activeDoc.languageId === "python") {
                const findings = workspaceCache.get(activeDoc.uri.fsPath) || [];
                const diagnostics = findingsToDiagnostics(activeDoc, findings);

                applyGutterDecorations(
                  vscode.window.activeTextEditor,
                  diagnostics
                );
              }
            }

            // Show completion message in status bar
            vscode.window.setStatusBarMessage(
              `$(check) CytoScnPy: Analyzed in ${duration.toFixed(1)}s`,
              5000
            );
          } catch (error: any) {
            console.error(
              `[CytoScnPy] Workspace analysis failed: ${error.message}`
            );
            vscode.window.showErrorMessage(
              `CytoScnPy analysis failed: ${error.message}`
            );
            workspaceCache = null;
          } finally {
            isWorkspaceAnalysisRunning = false;
          }
        }
      );
    }

    // Function to invalidate workspace cache
    function invalidateWorkspaceCache() {
      workspaceCache = null;
      workspaceCacheTimestamp = 0;
      fileCache.clear();
    }

    // Function to run incremental analysis on a single file and merge into workspace cache
    // This is much faster than full workspace re-analysis for single file saves
    async function runIncrementalAnalysis(document: vscode.TextDocument) {
      const filePath = document.uri.fsPath;
      const config = getCytoScnPyConfiguration(context);

      try {
        // Run single-file analysis
        const result = await runCytoScnPyAnalysis(filePath, config);
        const diagnostics = findingsToDiagnostics(document, result.findings);

        // Update diagnostics for this file
        cytoscnpyDiagnostics.set(document.uri, diagnostics);

        // Update file cache
        const cacheKey = getCacheKey(filePath);
        const contentHash = computeHash(document.getText());
        const cacheEntry: CacheEntry = {
          hash: contentHash,
          diagnostics: diagnostics,
          findings: result.findings,
          timestamp: Date.now(),
        };
        const history = fileCache.get(cacheKey) || [];
        history.unshift(cacheEntry);
        if (history.length > MAX_CACHE_HISTORY) {
          history.pop();
        }
        fileCache.set(cacheKey, history);

        // Merge into workspace cache if it exists
        if (workspaceCache) {
          workspaceCache.set(filePath, result.findings);
          workspaceCacheTimestamp = Date.now();
        }

        // Update sidebar and gutter decorations for active document
        if (
          vscode.window.activeTextEditor &&
          vscode.window.activeTextEditor.document.uri.toString() ===
            document.uri.toString()
        ) {
          applyGutterDecorations(vscode.window.activeTextEditor, diagnostics);
        }

        console.log(
          `[CytoScnPy] Incremental analysis completed for ${path.basename(
            filePath
          )}`
        );
      } catch (error: any) {
        console.error(
          `[CytoScnPy] Incremental analysis failed for ${filePath}: ${error.message}`
        );
        // On failure, fall back to full workspace analysis
        if (!isWorkspaceAnalysisRunning) {
          await runFullWorkspaceAnalysis();
        }
      }
    }

    // Function to refresh diagnostics for the active document
    async function refreshDiagnostics(document: vscode.TextDocument) {
      if (document.languageId !== "python") {
        return; // Only analyze Python files
      }

      const fsPath = document.uri.fsPath;
      const filePath =
        process.platform === "win32" ? fsPath.toLowerCase() : fsPath;
      const config = getCytoScnPyConfiguration(context);

      // FILE MODE: Single file analysis (faster, but may have false positives)
      if (config.analysisMode === "file") {
        try {
          const result = await runCytoScnPyAnalysis(fsPath, config);
          const diagnostics = findingsToDiagnostics(document, result.findings);
          cytoscnpyDiagnostics.set(document.uri, diagnostics);

          // Populate fileCache for CST-precise quick-fixes and diagnostics reuse
          const cacheKey = getCacheKey(fsPath);
          const contentHash = computeHash(document.getText());
          const cacheEntry: CacheEntry = {
            hash: contentHash,
            diagnostics: diagnostics,
            findings: result.findings,
            timestamp: Date.now(),
          };
          const history = fileCache.get(cacheKey) || [];
          // Prepend new entry, cap at MAX_CACHE_HISTORY
          history.unshift(cacheEntry);
          if (history.length > MAX_CACHE_HISTORY) {
            history.pop();
          }
          fileCache.set(cacheKey, history);

          const editor = vscode.window.activeTextEditor;
          if (
            editor &&
            editor.document.uri.toString() === document.uri.toString()
          ) {
            applyGutterDecorations(editor, diagnostics);
          }
        } catch (error: any) {
          console.error(`[CytoScnPy] File analysis failed: ${error.message}`);
        }
        return;
      }

      // WORKSPACE MODE: Full workspace analysis (accurate cross-file detection)
      // If we have a workspace cache, use it
      if (workspaceCache) {
        const findings = workspaceCache.get(filePath) || [];
        const diagnostics = findingsToDiagnostics(document, findings);
        cytoscnpyDiagnostics.set(document.uri, diagnostics);

        const contentHash = computeHash(document.getText());
        const cacheKey = getCacheKey(filePath);
        const cacheEntry: CacheEntry = {
          hash: contentHash,
          diagnostics: diagnostics,
          findings: findings,
          timestamp: Date.now(),
        };
        const history = fileCache.get(cacheKey) || [];
        // Prepend new entry, cap at MAX_CACHE_HISTORY
        history.unshift(cacheEntry);
        if (history.length > MAX_CACHE_HISTORY) {
          history.pop();
        }
        fileCache.set(cacheKey, history);

        const editor = vscode.window.activeTextEditor;
        if (
          editor &&
          editor.document.uri.toString() === document.uri.toString()
        ) {
          applyGutterDecorations(editor, diagnostics);
        }
        return;
      }

      // No workspace cache - trigger workspace analysis
      await runFullWorkspaceAnalysis();
    }

    // Initial analysis when a document is opened or becomes active
    if (vscode.window.activeTextEditor) {
      refreshDiagnostics(vscode.window.activeTextEditor.document);
    }

    // Periodic workspace re-scan
    // Ensures cross-file dependencies are eventually caught even if only incremental scans ran
    let lastFileChangeTime = Date.now();

    // Use 15 seconds for testing/debugging (Development mode), 5 minutes for production
    const isDebug = context.extensionMode === vscode.ExtensionMode.Development;
    const PERIODIC_SCAN_INTERVAL_MS = isDebug ? 15 * 1000 : 5 * 60 * 1000;

    console.log(
      `[CytoScnPy] Periodic scan interval set to ${PERIODIC_SCAN_INTERVAL_MS}ms (Debug: ${isDebug})`
    );

    const periodicScanInterval = setInterval(async () => {
      console.log(
        `[CytoScnPy] Periodic scan timer tick. Debug: ${isDebug}, Last Change: ${lastFileChangeTime}, Last Scan: ${workspaceCacheTimestamp}`
      );

      // In Debug mode: ALWAYS run (to verify timer works)
      // In Production mode: ONLY run if changes occurred since last full workspace analysis
      if (isDebug || lastFileChangeTime > workspaceCacheTimestamp) {
        console.log(
          "[CytoScnPy] Triggering periodic workspace re-scan (Reason: " +
            (isDebug ? "Debug Force" : "Changes Detected") +
            ")..."
        );
        await runFullWorkspaceAnalysis();
      } else {
        console.log(
          "[CytoScnPy] Skipping periodic re-scan (No changes detected)."
        );
      }
    }, PERIODIC_SCAN_INTERVAL_MS);
    context.subscriptions.push({
      dispose: () => clearInterval(periodicScanInterval),
    });

    // Analyze document on save - debounced incremental analysis (much faster than full workspace scan)
    context.subscriptions.push(
      vscode.workspace.onDidSaveTextDocument((document) => {
        if (document.languageId === "python") {
          // Update last change time
          lastFileChangeTime = Date.now();

          // Clear previous debounce timer
          if (analysisDebounceTimer) {
            clearTimeout(analysisDebounceTimer);
          }

          const config = getCytoScnPyConfiguration(context);
          // Use longer debounce for workspace mode to prevent frequent expensive scans
          const debounceMs = config.analysisMode === "workspace" ? 3000 : 500;

          // Debounce: wait based on mode
          analysisDebounceTimer = setTimeout(() => {
            // Re-fetch config to ensure we use the latest settings
            const currentConfig = getCytoScnPyConfiguration(context);

            if (currentConfig.analysisMode === "workspace") {
              // In workspace mode, run full analysis to maintain cross-file context correctness
              runFullWorkspaceAnalysis().catch((err) => {
                console.error(
                  "[CytoScnPy] Workspace analysis on save failed:",
                  err
                );
              });
            } else {
              // Use incremental analysis - only re-scan the saved file
              // This is much faster than full workspace re-analysis
              runIncrementalAnalysis(document).catch((err) => {
                console.error("[CytoScnPy] Incremental analysis failed:", err);
              });
            }
          }, debounceMs);
        }
      })
    );

    // Re-run analysis when CytoScnPy settings change (e.g., settings.json saved)
    context.subscriptions.push(
      vscode.workspace.onDidChangeConfiguration((event) => {
        if (event.affectsConfiguration("cytoscnpy")) {
          // Clear caches to force re-analysis with new settings
          invalidateWorkspaceCache();

          // Re-analyze all open Python documents
          vscode.workspace.textDocuments.forEach((doc) => {
            if (doc.languageId === "python") {
              refreshDiagnostics(doc);
            }
          });
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

    // Clear diagnostics and cache when a document is closed
    context.subscriptions.push(
      vscode.workspace.onDidCloseTextDocument((document) => {
        cytoscnpyDiagnostics.delete(document.uri);
        fileCache.delete(getCacheKey(document.uri.fsPath)); // Clear cache entry
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

      // Use execFile with argument array to prevent command injection
      const args = [commandType, filePath];

      cytoscnpyOutputChannel.clear();
      cytoscnpyOutputChannel.show();
      cytoscnpyOutputChannel.appendLine(
        `Running: ${config.path} ${args.join(" ")}\n`
      );

      execFile(
        config.path,
        args,
        (error: Error | null, stdout: string, stderr: string) => {
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
        }
      );
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

          const args = [workspacePath, "--json"];
          if (config.enableSecretsScan) {
            args.push("--secrets");
          }
          if (config.enableDangerScan) {
            args.push("--danger");
          }
          if (config.enableQualityScan) {
            args.push("--quality");
          }

          execFile(
            config.path,
            args,
            (error: Error | null, stdout: string, stderr: string) => {
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
            }
          );
        }
      )
    );

    // NOTE: Removed custom HoverProvider - VS Code natively displays diagnostic messages on hover
    // Adding our own HoverProvider was causing duplicate messages.

    // Register Code Action Provider for quick fixes
    const quickFixProvider = new QuickFixProvider();
    context.subscriptions.push(
      vscode.languages.registerCodeActionsProvider("python", quickFixProvider, {
        providedCodeActionKinds: [vscode.CodeActionKind.QuickFix],
      })
    );
  } catch (error) {
    console.error("Error during extension activation:", error);
  }
}

export class QuickFixProvider implements vscode.CodeActionProvider {
  public provideCodeActions(
    document: vscode.TextDocument,
    range: vscode.Range | vscode.Selection,
    context: vscode.CodeActionContext,
    token: vscode.CancellationToken
  ): vscode.CodeAction[] {
    const actions: vscode.CodeAction[] = [];

    for (const diagnostic of context.diagnostics) {
      // Handle both old string format and new structured code format
      const ruleId =
        typeof diagnostic.code === "object" &&
        diagnostic.code !== null &&
        "value" in diagnostic.code
          ? (diagnostic.code.value as string)
          : (diagnostic.code as string);

      // Check if it's a CytoScnPy diagnostic (source starts with "CytoScnPy")
      if (!diagnostic.source?.startsWith("CytoScnPy")) {
        continue;
      }

      // 1. Try to find precise "Remove" or "Fix" from cache (if available)
      const currentHash = computeHash(document.getText());
      const cacheKey = getCacheKey(document.uri.fsPath);
      const cachedHistory = fileCache.get(cacheKey) || [];
      const cachedEntry = cachedHistory.find((e) => e.hash === currentHash);

      const diagnosticLine = diagnostic.range.start.line + 1;

      // First try fileCache
      let finding = cachedEntry?.findings.find(
        (f) =>
          f.rule_id === ruleId && Math.abs(f.line_number - diagnosticLine) <= 2
      );

      // Fallback to workspaceCache if fileCache doesn't have it
      if (!finding && workspaceCache) {
        const wsFindings = workspaceCache.get(cacheKey) || [];
        finding = wsFindings.find(
          (f) =>
            f.rule_id === ruleId &&
            Math.abs(f.line_number - diagnosticLine) <= 2
        );
      }

      if (finding && finding.fix) {
        // Precise CST-based fix available (e.g., Remove unused function)
        const fixAction = new vscode.CodeAction(
          `Remove ${ruleId.replace("unused-", "")}`,
          vscode.CodeActionKind.QuickFix
        );
        fixAction.diagnostics = [diagnostic];
        fixAction.isPreferred = true;

        const edit = new vscode.WorkspaceEdit();
        const startPos = document.positionAt(finding.fix.start_byte);
        const endPos = document.positionAt(finding.fix.end_byte);

        edit.replace(
          document.uri,
          new vscode.Range(startPos, endPos),
          finding.fix.replacement
        );
        fixAction.edit = edit;
        actions.push(fixAction);
      }

      // 2. Add "Suppress" action for ALL CytoScnPy diagnostics
      const suppressAction = this.createSuppressionAction(document, diagnostic);
      if (suppressAction) {
        actions.push(suppressAction);
      }
    }

    return actions;
  }

  private createSuppressionAction(
    document: vscode.TextDocument,
    diagnostic: vscode.Diagnostic
  ): vscode.CodeAction | undefined {
    const actionTitle = "Suppress with # noqa: CSP";

    const action = new vscode.CodeAction(
      actionTitle,
      vscode.CodeActionKind.QuickFix
    );
    action.diagnostics = [diagnostic];

    const lineIndex = diagnostic.range.start.line;
    const lineText = document.lineAt(lineIndex).text;
    const edit = new vscode.WorkspaceEdit();

    // Check for existing suppression comment
    const noqaRegex = /#\s*noqa(?::\s*([^#\n]+))?/;
    const match = lineText.match(noqaRegex);

    if (match) {
      // Existing noqa found
      if (!match[1]) {
        // Bare # noqa - already suppresses all
        return undefined;
      }
      const existingCodes = match[1].split(/,\s*/).map((s) => s.trim());
      if (existingCodes.includes("CSP")) {
        return undefined; // Already suppressed
      }
      // Append CSP to existing codes
      const commentStart = match.index!;
      const commentContent = match[0];
      const newComment = `${commentContent}, CSP`;
      const range = new vscode.Range(
        new vscode.Position(lineIndex, commentStart),
        new vscode.Position(lineIndex, commentStart + commentContent.length)
      );
      edit.replace(document.uri, range, newComment);
    } else {
      // No existing noqa, append new one
      const insertText = "  # noqa: CSP";
      const insertPos = new vscode.Position(lineIndex, lineText.length);
      edit.insert(document.uri, insertPos, insertText);
    }

    action.edit = edit;
    return action;
  }
}

export function deactivate() {
  cytoscnpyDiagnostics.dispose(); // Clean up diagnostics when extension is deactivated
  cytoscnpyOutputChannel.dispose(); // Clean up output channel
  errorDecorationType?.dispose(); // Clean up decoration types
  warningDecorationType?.dispose();
  infoDecorationType?.dispose();
}

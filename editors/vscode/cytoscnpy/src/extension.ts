// The module 'vscode' contains the VS Code extensibility API
// Import the module and reference it with the alias vscode in your code below
import * as vscode from "vscode";
import * as os from "os";
import * as path from "path";
import * as fs from "fs";
import * as crypto from "crypto";
import {
  runCytoScnPyAnalysis,
  runWorkspaceAnalysis,
  CytoScnPyConfig,
  CytoScnPyFinding,
  ParseError,
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
let workspaceParseErrorsCache: Map<string, ParseError[]> | null = null;
let workspaceCacheTimestamp: number = 0;
let isWorkspaceAnalysisRunning = false;

// Debounce timer for save-triggered analysis (prevents multiple scans on rapid saves)
let analysisDebounceTimer: NodeJS.Timeout | null = null;
const ANALYSIS_DEBOUNCE_MS = 1000; // Wait 1 second after last save before re-analyzing

// Helper function to compute content hash
export function computeHash(content: string): string {
  return crypto.createHash("sha256").update(content).digest("hex");
}

// Per-document memo of the most recent (version, hash) pair so that
// `provideCodeActions` does not recompute SHA-256 over the full document
// every time the lightbulb is invoked. Keyed by VS Code URI string.
const documentHashCache = new Map<string, { version: number; hash: string }>();

export function hashForDocument(document: vscode.TextDocument): string {
  const key = document.uri.toString();
  const cached = documentHashCache.get(key);
  if (cached && cached.version === document.version) {
    return cached.hash;
  }
  const hash = computeHash(document.getText());
  documentHashCache.set(key, { version: document.version, hash });
  return hash;
}

// Single source of truth for translating CytoScnPy severity strings to VS Code
// DiagnosticSeverity. Both `findingsToDiagnostics` and the closed-file branch
// in `runFullWorkspaceAnalysis` route through this helper so that the two
// previously divergent switches cannot drift apart again.
//
// Behavior intentionally mirrors the pre-refactor switches: only CRITICAL/ERROR
// elevate to Error, everything else maps to Warning.
export function mapSeverity(severity: string): vscode.DiagnosticSeverity {
  const upper = severity.toUpperCase();
  if (upper === "CRITICAL" || upper === "ERROR") {
    return vscode.DiagnosticSeverity.Error;
  }
  return vscode.DiagnosticSeverity.Warning;
}

const UNUSED_RULE_IDS: ReadonlySet<string> = new Set([
  "unused-function",
  "unused-method",
  "unused-class",
  "unused-import",
  "unused-variable",
  "unused-parameter",
]);

// Builds the closed-file diagnostic for a single finding. Closed files have no
// document, so end-of-line is unknown; VS Code clamps the oversized end column
// down to the actual line length when the editor opens the file.
export function buildClosedFileDiagnosticFromFinding(
  finding: CytoScnPyFinding,
): vscode.Diagnostic {
  const lineIndex = Math.max(0, finding.line_number - 1);
  const startCol = finding.col && finding.col > 0 ? finding.col : 0;
  const range = new vscode.Range(
    new vscode.Position(lineIndex, startCol),
    new vscode.Position(lineIndex, Number.MAX_SAFE_INTEGER),
  );
  const diagnostic = new vscode.Diagnostic(
    range,
    `${finding.message} [${finding.rule_id}]`,
    mapSeverity(finding.severity),
  );
  diagnostic.source = "CytoScnPy";
  diagnostic.code = finding.rule_id;
  if (UNUSED_RULE_IDS.has(finding.rule_id)) {
    diagnostic.tags = [vscode.DiagnosticTag.Unnecessary];
  }
  return diagnostic;
}

export function buildClosedFileDiagnosticFromParseError(
  parseError: ParseError,
): vscode.Diagnostic {
  const lineIndex = Math.max(0, parseError.line - 1);
  const range = new vscode.Range(
    new vscode.Position(lineIndex, 0),
    new vscode.Position(lineIndex, Number.MAX_SAFE_INTEGER),
  );
  const diagnostic = new vscode.Diagnostic(
    range,
    `Parse error: ${parseError.message}`,
    vscode.DiagnosticSeverity.Error,
  );
  diagnostic.source = "CytoScnPy [Parse]";
  diagnostic.code = "parse-error";
  return diagnostic;
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

// Persistent status bar item showing current analyzer state — created once at
// module load so re-activation (e.g. window reload) does not multiply the item.
const statusBarItem = vscode.window.createStatusBarItem(
  vscode.StatusBarAlignment.Right,
  100,
);
statusBarItem.command = "cytoscnpy.analyzeWorkspace";

type StatusKind = "idle" | "running" | "error";
function setStatus(kind: StatusKind, detail: string): void {
  switch (kind) {
    case "running":
      statusBarItem.text = `$(sync~spin) CytoScnPy: ${detail}`;
      statusBarItem.tooltip = "CytoScnPy analysis running";
      break;
    case "error":
      statusBarItem.text = `$(error) CytoScnPy: ${detail}`;
      statusBarItem.tooltip = detail;
      break;
    case "idle":
    default:
      statusBarItem.text = `$(check) CytoScnPy: ${detail}`;
      statusBarItem.tooltip = "Click to re-run workspace analysis";
      break;
  }
  statusBarItem.show();
}

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

  // `executableName` comes from a hardcoded switch on `os.platform()`, so it
  // cannot contain `..` segments; no separate path-traversal check needed.
  const bundledPath = path.join(context.extensionPath, "bin", executableName);

  // Check if bundled binary exists, otherwise fall back to pip-installed version
  try {
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
  context: vscode.ExtensionContext,
): CytoScnPyConfig {
  const config = vscode.workspace.getConfiguration("cytoscnpy");
  const pathSetting = config.inspect<string>("path");

  const userSetPath = pathSetting?.globalValue || pathSetting?.workspaceValue;

  // Helper to get value only if explicitly set (Global, Workspace, or Folder)
  // If not set, return undefined so the analyzer uses values from pyproject.toml/.cytoscnpy.toml
  const getIfSet = <T>(key: string): T | undefined => {
    const inspect = config.inspect<T>(key);
    if (
      inspect &&
      (inspect.globalValue !== undefined ||
        inspect.workspaceValue !== undefined ||
        inspect.workspaceFolderValue !== undefined)
    ) {
      return config.get<T>(key);
    }
    return undefined;
  };

  return {
    path: userSetPath || getExecutablePath(context),
    analysisMode:
      config.get<string>("analysisMode") === "file" ? "file" : "workspace",
    enableSecretsScan: config.get<boolean>("enableSecretsScan") || false,
    enableDangerScan: config.get<boolean>("enableDangerScan") || false,
    enableQualityScan: config.get<boolean>("enableQualityScan") || false,
    enableCloneScan: config.get<boolean>("enableCloneScan") || false,
    confidenceThreshold: getIfSet<number>("confidenceThreshold"),
    excludeFolders: getIfSet<string[]>("excludeFolders"),
    includeFolders: getIfSet<string[]>("includeFolders"),
    includeTests: getIfSet<boolean>("includeTests"),
    includeIpynb: getIfSet<boolean>("includeIpynb"),
    maxComplexity: getIfSet<number>("maxComplexity"),
    minMaintainabilityIndex: getIfSet<number>("minMaintainabilityIndex"),
    maxNesting: getIfSet<number>("maxNesting"),
    maxArguments: getIfSet<number>("maxArguments"),
    maxLines: getIfSet<number>("maxLines"),
  };
}

export function activate(context: vscode.ExtensionContext) {
  const config = getCytoScnPyConfiguration(context);
  const isDevBuild =
    context.extensionMode === vscode.ExtensionMode.Development;
  if (isDevBuild) {
    cytoscnpyOutputChannel.appendLine(
      `[CytoScnPy] Activated; binary=${config.path}, danger=${config.enableDangerScan}`,
    );
  }
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
                  },
                ),
              ];
            },
            resolveMcpServerDefinition: async (server) => server,
          }),
        );
      } catch (mcpError) {
        console.warn("Failed to register MCP server provider:", mcpError);
      }
    }

    // Initialize gutter decoration types
    errorDecorationType = vscode.window.createTextEditorDecorationType({
      gutterIconPath: vscode.Uri.parse(
        "data:image/svg+xml," +
          encodeURIComponent(
            '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><circle cx="8" cy="8" r="6" fill="#f44336"/></svg>',
          ),
      ),
      gutterIconSize: "contain",
      overviewRulerColor: "#f44336",
      overviewRulerLane: vscode.OverviewRulerLane.Right,
    });
    warningDecorationType = vscode.window.createTextEditorDecorationType({
      gutterIconPath: vscode.Uri.parse(
        "data:image/svg+xml," +
          encodeURIComponent(
            '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><circle cx="8" cy="8" r="6" fill="#ff9800"/></svg>',
          ),
      ),
      gutterIconSize: "contain",
      overviewRulerColor: "#ff9800",
      overviewRulerLane: vscode.OverviewRulerLane.Right,
    });
    infoDecorationType = vscode.window.createTextEditorDecorationType({
      gutterIconPath: vscode.Uri.parse(
        "data:image/svg+xml," +
          encodeURIComponent(
            '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><circle cx="8" cy="8" r="6" fill="#2196f3"/></svg>',
          ),
      ),
      gutterIconSize: "contain",
      overviewRulerColor: "#2196f3",
      overviewRulerLane: vscode.OverviewRulerLane.Right,
    });
    context.subscriptions.push(
      errorDecorationType,
      warningDecorationType,
      infoDecorationType,
    );
    setStatus("idle", "ready");

    // Function to apply gutter decorations based on diagnostics
    function applyGutterDecorations(
      editor: vscode.TextEditor,
      diagnostics: vscode.Diagnostic[],
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
    function isLineSuppressed(lineText: string, ruleId: string): boolean {
      const pragmaRegex = /#\s*pragma:\s*no\s+cytoscnpy/i;
      if (pragmaRegex.test(lineText)) {
        return true;
      }

      // Matches: # noqa, # ignore, # noqa: CSP-D101, CSP, etc.
      const noqaRegex = /#\s*(?:noqa|ignore)(?::\s*([^#\n]+))?/i;
      const match = lineText.match(noqaRegex);
      if (!match) {
        return false;
      }
      // Bare # noqa suppresses all
      if (!match[1]) {
        return true;
      }
      const normalizedRule = ruleId.toUpperCase();
      const codes = match[1].split(/,\s*/).map((s) => s.trim().toUpperCase());
      if (codes.includes("CSP")) {
        return true;
      }
      return codes.includes(normalizedRule);
    }

    // Helper function to convert findings to diagnostics for a document
    function findingsToDiagnostics(
      document: vscode.TextDocument,
      findings: CytoScnPyFinding[],
    ): vscode.Diagnostic[] {
      return findings
        .filter((finding) => {
          const lineIndex = finding.line_number - 1;
          if (lineIndex < 0 || lineIndex >= document.lineCount) {
            return true; // Keep - can't check suppression
          }
          const lineText = document.lineAt(lineIndex).text;
          return !isLineSuppressed(lineText, finding.rule_id);
        })
        .map((finding) => {
          const lineIndex = finding.line_number - 1;
          // Ensure line index is valid
          if (lineIndex < 0 || lineIndex >= document.lineCount) {
            const range = new vscode.Range(0, 0, 0, 0);
            return new vscode.Diagnostic(
              range,
              `${finding.message} [${finding.rule_id}]`,
              vscode.DiagnosticSeverity.Warning,
            );
          }
          const lineText = document.lineAt(lineIndex);

          const startCol =
            finding.col && finding.col > 0
              ? finding.col
              : lineText.firstNonWhitespaceCharacterIndex;

          const range = new vscode.Range(
            new vscode.Position(lineIndex, startCol),
            new vscode.Position(lineIndex, lineText.text.length),
          );
          const diagnostic = new vscode.Diagnostic(
            range,
            `${finding.message} [${finding.rule_id}]`,
            mapSeverity(finding.severity),
          );

          if (finding.category === "Dead Code") {
            diagnostic.tags = [vscode.DiagnosticTag.Unnecessary];
          }

          diagnostic.source = `CytoScnPy [${finding.category}]`;
          diagnostic.code = finding.rule_id;

          return diagnostic;
        });
    }

    function parseErrorsToDiagnostics(
      document: vscode.TextDocument,
      parseErrors: ParseError[],
    ): vscode.Diagnostic[] {
      return parseErrors.map((parseError) => {
        const lineIndex = Math.max(
          0,
          Math.min(document.lineCount - 1, parseError.line - 1),
        );
        const lineText = document.lineAt(lineIndex);
        const range = new vscode.Range(
          new vscode.Position(
            lineIndex,
            lineText.firstNonWhitespaceCharacterIndex,
          ),
          new vscode.Position(lineIndex, lineText.text.length),
        );
        const diagnostic = new vscode.Diagnostic(
          range,
          `Parse error: ${parseError.message}`,
          vscode.DiagnosticSeverity.Error,
        );
        diagnostic.source = "CytoScnPy [Parse]";
        diagnostic.code = "parse-error";
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
      setStatus("running", "scanning workspace");
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

            const workspaceResult = await runWorkspaceAnalysis(
              workspacePath,
              config,
            );
            workspaceCache = workspaceResult.findingsByFile;
            workspaceParseErrorsCache = workspaceResult.parseErrorsByFile;
            workspaceCacheTimestamp = Date.now();

            const duration = (Date.now() - startTime) / 1000;
            if (isDevBuild) {
              const fileCount = new Set<string>([
                ...workspaceCache.keys(),
                ...(workspaceParseErrorsCache?.keys() ?? []),
              ]).size;
              cytoscnpyOutputChannel.appendLine(
                `[CytoScnPy] Workspace analysis completed in ${duration.toFixed(
                  2,
                )}s, findings in ${fileCount} files`,
              );
            }

            progress.report({ message: `Updating diagnostics...` });

            // Clear previous workspace diagnostics first; the analyzer output only
            // contains files with active findings, so this prevents stale entries
            // from lingering after a user fixes an issue.
            cytoscnpyDiagnostics.clear();

            // Set diagnostics for ALL files in workspace findings + parse errors
            // so the Problems view includes both categories.
            const filesWithDiagnostics = new Set<string>([
              ...workspaceCache.keys(),
              ...(workspaceParseErrorsCache?.keys() ?? []),
            ]);

            for (const filePath of filesWithDiagnostics) {
              const uri = vscode.Uri.file(filePath);
              const findings = workspaceCache.get(filePath) || [];
              const parseErrors =
                workspaceParseErrorsCache?.get(filePath) || [];
              const diagnostics = [
                ...findings.map(buildClosedFileDiagnosticFromFinding),
                ...parseErrors.map(buildClosedFileDiagnosticFromParseError),
              ];
              cytoscnpyDiagnostics.set(uri, diagnostics);
            }

            // Update sidebar for active document
            if (vscode.window.activeTextEditor) {
              const activeDoc = vscode.window.activeTextEditor.document;
              if (activeDoc.languageId === "python") {
                const findings = workspaceCache.get(activeDoc.uri.fsPath) || [];
                const parseErrors =
                  workspaceParseErrorsCache?.get(activeDoc.uri.fsPath) || [];
                const diagnostics = [
                  ...findingsToDiagnostics(activeDoc, findings),
                  ...parseErrorsToDiagnostics(activeDoc, parseErrors),
                ];

                applyGutterDecorations(
                  vscode.window.activeTextEditor,
                  diagnostics,
                );
              }
            }

            const findingsCount = Array.from(workspaceCache.values()).reduce(
              (sum, list) => sum + list.length,
              0,
            );
            setStatus(
              "idle",
              `${findingsCount} finding${findingsCount === 1 ? "" : "s"} (${duration.toFixed(1)}s)`,
            );
          } catch (error: any) {
            console.error(
              `[CytoScnPy] Workspace analysis failed: ${error.message}`,
            );
            vscode.window.showErrorMessage(
              `CytoScnPy analysis failed: ${error.message}`,
            );
            setStatus("error", "analysis failed");
            workspaceCache = null;
            workspaceParseErrorsCache = null;
          } finally {
            isWorkspaceAnalysisRunning = false;
          }
        },
      );
    }

    // Function to invalidate workspace cache.
    // Also clears the surfaced diagnostic collection so stale findings do not
    // linger in the Problems panel between a config change and the next analysis
    // completing (the closed-file workspace branch keeps diagnostics by default).
    function invalidateWorkspaceCache() {
      workspaceCache = null;
      workspaceParseErrorsCache = null;
      workspaceCacheTimestamp = 0;
      fileCache.clear();
      documentHashCache.clear();
      cytoscnpyDiagnostics.clear();
    }

    // Function to run incremental analysis on a single file and merge into workspace cache
    // This is much faster than full workspace re-analysis for single file saves
    async function runIncrementalAnalysis(document: vscode.TextDocument) {
      const filePath = document.uri.fsPath;
      const config = getCytoScnPyConfiguration(context);
      setStatus("running", `scanning ${path.basename(filePath)}`);

      try {
        // Run single-file analysis
        const result = await runCytoScnPyAnalysis(filePath, config);
        const diagnostics = [
          ...findingsToDiagnostics(document, result.findings),
          ...parseErrorsToDiagnostics(document, result.parseErrors),
        ];

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
          if (workspaceParseErrorsCache) {
            workspaceParseErrorsCache.set(filePath, result.parseErrors);
          }
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
        setStatus("error", "incremental failed");
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
          const diagnostics = [
            ...findingsToDiagnostics(document, result.findings),
            ...parseErrorsToDiagnostics(document, result.parseErrors),
          ];
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
        const parseErrors = workspaceParseErrorsCache?.get(filePath) || [];
        const diagnostics = [
          ...findingsToDiagnostics(document, findings),
          ...parseErrorsToDiagnostics(document, parseErrors),
        ];
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

    // Periodic workspace re-scan: catches cross-file dependencies even if only
    // incremental scans ran. Skip the tick entirely when no Python files are open
    // and when there have been no changes since the last full analysis — both
    // gates apply in debug builds too so dev sessions are not flooded.
    let lastFileChangeTime = Date.now();
    const isDebug = context.extensionMode === vscode.ExtensionMode.Development;
    const PERIODIC_SCAN_INTERVAL_MS = isDebug ? 15 * 1000 : 5 * 60 * 1000;

    function hasOpenPythonDocument(): boolean {
      return vscode.workspace.textDocuments.some(
        (d) => d.languageId === "python",
      );
    }

    const periodicScanInterval = setInterval(async () => {
      if (!hasOpenPythonDocument()) {
        return;
      }
      if (lastFileChangeTime <= workspaceCacheTimestamp) {
        return;
      }
      await runFullWorkspaceAnalysis();
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
          }, debounceMs);
        }
      }),
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
      }),
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
        const mode = getCytoScnPyConfiguration(context).analysisMode;
        // In workspace mode we intentionally keep diagnostics for closed files
        // so the Problems view remains complete across the whole project.
        if (mode === "file") {
          cytoscnpyDiagnostics.delete(document.uri);
        }
        fileCache.delete(getCacheKey(document.uri.fsPath)); // Clear cache entry
        documentHashCache.delete(document.uri.toString());
      }),
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
      },
    );

    context.subscriptions.push(disposableAnalyze);

    // Helper function to run metric commands
    async function runMetricCommand(
      context: vscode.ExtensionContext,
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
      const config = getCytoScnPyConfiguration(context);

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
        runMetricCommand(context, "cc", "Cyclomatic Complexity"),
      ),
    );
    context.subscriptions.push(
      vscode.commands.registerCommand("cytoscnpy.halstead", () =>
        runMetricCommand(context, "hal", "Halstead Metrics"),
      ),
    );
    context.subscriptions.push(
      vscode.commands.registerCommand("cytoscnpy.maintainability", () =>
        runMetricCommand(context, "mi", "Maintainability Index"),
      ),
    );
    context.subscriptions.push(
      vscode.commands.registerCommand("cytoscnpy.rawMetrics", () =>
        runMetricCommand(context, "raw", "Raw Metrics"),
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

          const workspacePath = workspaceFolders[0].uri.fsPath;
          const config = getCytoScnPyConfiguration(context);

          cytoscnpyOutputChannel.clear();
          cytoscnpyOutputChannel.show();
          cytoscnpyOutputChannel.appendLine(
            `Analyzing workspace: ${workspacePath}\n`,
          );

          const args = ["--client", "vscode", workspacePath, "--json"];
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
                "Workspace analysis complete. See output channel.",
              );
            },
          );
        },
      ),
    );

    // NOTE: Removed custom HoverProvider - VS Code natively displays diagnostic messages on hover
    // Adding our own HoverProvider was causing duplicate messages.

    // Register Code Action Provider for quick fixes
    const quickFixProvider = new QuickFixProvider();
    context.subscriptions.push(
      vscode.languages.registerCodeActionsProvider("python", quickFixProvider, {
        providedCodeActionKinds: [vscode.CodeActionKind.QuickFix],
      }),
    );
  } catch (error) {
    console.error("Error during extension activation:", error);
  }
}

const UNUSED_RULE_LABELS: Record<string, { singular: string; plural: string }> =
  {
    "unused-function": { singular: "function", plural: "functions" },
    "unused-method": { singular: "method", plural: "methods" },
    "unused-class": { singular: "class", plural: "classes" },
    "unused-import": { singular: "import", plural: "imports" },
    "unused-variable": { singular: "variable", plural: "variables" },
    "unused-parameter": { singular: "parameter", plural: "parameters" },
  };

function filterOverlappingFixes<T extends { finding: CytoScnPyFinding }>(
  items: T[],
): T[] {
  const sorted = [...items].sort((a, b) => {
    const aStart = a.finding.fix!.start_byte;
    const bStart = b.finding.fix!.start_byte;
    if (aStart !== bStart) {
      return aStart - bStart;
    }
    return b.finding.fix!.end_byte - a.finding.fix!.end_byte;
  });
  const filtered: T[] = [];
  let lastEnd = 0;
  for (const item of sorted) {
    const start = item.finding.fix!.start_byte;
    const end = item.finding.fix!.end_byte;
    if (start >= lastEnd) {
      filtered.push(item);
      lastEnd = end;
    }
  }
  return filtered;
}

function byteOffsetToUtf16Offset(text: string, byteOffset: number): number {
  const utf8 = Buffer.from(text, "utf8");
  const clamped = Math.max(0, Math.min(byteOffset, utf8.length));
  return utf8.subarray(0, clamped).toString("utf8").length;
}

function rangeFromFixBytes(
  document: vscode.TextDocument,
  startByte: number,
  endByte: number,
): vscode.Range | undefined {
  if (
    !Number.isInteger(startByte) ||
    !Number.isInteger(endByte) ||
    startByte < 0 ||
    endByte < startByte
  ) {
    return undefined;
  }

  const text = document.getText();
  const utf8Len = Buffer.byteLength(text, "utf8");
  if (endByte > utf8Len) {
    return undefined;
  }

  const startOffset = byteOffsetToUtf16Offset(text, startByte);
  const endOffset = byteOffsetToUtf16Offset(text, endByte);
  if (endOffset < startOffset) {
    return undefined;
  }

  return new vscode.Range(
    document.positionAt(startOffset),
    document.positionAt(endOffset),
  );
}

export class QuickFixProvider implements vscode.CodeActionProvider {
  public provideCodeActions(
    document: vscode.TextDocument,
    range: vscode.Range | vscode.Selection,
    context: vscode.CodeActionContext,
    token: vscode.CancellationToken,
  ): vscode.CodeAction[] {
    // Honour the cancellation token: VS Code routinely cancels stale invocations
    // when the user keeps typing. Doing the SHA-256 + diagnostic walk anyway is
    // wasted work and can starve the next valid request.
    if (token.isCancellationRequested) {
      return [];
    }

    const actions: vscode.CodeAction[] = [];

    // Collect all fixable findings for "Fix All" action
    const fixableByRule = new Map<
      string,
      { finding: CytoScnPyFinding; diagnostic: vscode.Diagnostic }[]
    >();
    const fixableUnused: {
      finding: CytoScnPyFinding;
      diagnostic: vscode.Diagnostic;
    }[] = [];

    // Reuse memoised SHA-256 keyed on `document.version`. Recomputing the hash
    // over the full document on every lightbulb invocation showed up as a
    // measurable hotspot on large files.
    const currentHash = hashForDocument(document);
    const cacheKey = getCacheKey(document.uri.fsPath);
    const cachedHistory = fileCache.get(cacheKey) || [];
    const cachedEntry = cachedHistory.find((e) => e.hash === currentHash);
    const getRuleId = (diagnostic: vscode.Diagnostic): string | undefined =>
      typeof diagnostic.code === "object" &&
      diagnostic.code !== null &&
      "value" in diagnostic.code
        ? (diagnostic.code.value as string)
        : (diagnostic.code as string);

    const findFindingForDiagnostic = (
      diagnostic: vscode.Diagnostic,
      ruleId: string | undefined,
    ): CytoScnPyFinding | undefined => {
      if (!ruleId) {
        return undefined;
      }
      const diagnosticLine = diagnostic.range.start.line + 1;
      const pickClosest = (
        findings: CytoScnPyFinding[],
      ): CytoScnPyFinding | undefined => {
        let best: CytoScnPyFinding | undefined;
        let bestDiff = Number.POSITIVE_INFINITY;
        for (const finding of findings) {
          if (finding.rule_id !== ruleId) {
            continue;
          }
          const diff = Math.abs(finding.line_number - diagnosticLine);
          if (diff > 2) {
            continue;
          }
          if (diff < bestDiff) {
            best = finding;
            bestDiff = diff;
            if (diff === 0) {
              break;
            }
          }
        }
        return best;
      };

      const fromCache = cachedEntry
        ? pickClosest(cachedEntry.findings)
        : undefined;
      if (fromCache) {
        return fromCache;
      }

      return undefined;
    };

    // Resolve file diagnostics for file-wide "Fix All" actions. Context
    // diagnostics are resolved separately because VS Code may pass equivalent
    // diagnostic objects that are not identical to the global collection.
    type ResolvedDiag = {
      diagnostic: vscode.Diagnostic;
      ruleId: string | undefined;
      finding: CytoScnPyFinding | undefined;
    };
    const isCytoScnPyDiagnostic = (diagnostic: vscode.Diagnostic): boolean =>
      diagnostic.source?.startsWith("CytoScnPy") ?? false;
    const resolveDiagnostic = (diagnostic: vscode.Diagnostic): ResolvedDiag => {
      const ruleId = getRuleId(diagnostic);
      const finding = findFindingForDiagnostic(diagnostic, ruleId);
      return { diagnostic, ruleId, finding };
    };
    const fileDiagnostics = vscode.languages.getDiagnostics(document.uri);
    for (const diagnostic of fileDiagnostics) {
      if (!isCytoScnPyDiagnostic(diagnostic)) {
        continue;
      }
      const resolvedDiagnostic = resolveDiagnostic(diagnostic);
      const { ruleId, finding } = resolvedDiagnostic;

      if (finding && finding.fix && ruleId && UNUSED_RULE_LABELS[ruleId]) {
        fixableUnused.push({ finding, diagnostic });
        if (!fixableByRule.has(ruleId)) {
          fixableByRule.set(ruleId, []);
        }
        fixableByRule.get(ruleId)!.push({ finding, diagnostic });
      }
    }

    for (const diagnostic of context.diagnostics) {
      if (!isCytoScnPyDiagnostic(diagnostic)) {
        continue;
      }
      const { ruleId, finding } = resolveDiagnostic(diagnostic);

      if (finding && finding.fix && ruleId) {
        const labels = UNUSED_RULE_LABELS[ruleId];
        if (labels) {
          // Extract symbol name from diagnostic message (e.g., "'ceil' is imported but never used")
          // Also try backticks for messages like "`name` is defined but never used"
          const symbolMatch =
            diagnostic.message.match(/'([^']+)'/) ||
            diagnostic.message.match(/`([^`]+)`/);

          const actionTitle = symbolMatch
            ? `Remove unused ${labels.singular} '${symbolMatch[1]}'`
            : `Remove unused ${labels.singular}`;

          const fixAction = new vscode.CodeAction(
            actionTitle,
            vscode.CodeActionKind.QuickFix,
          );
          fixAction.diagnostics = [diagnostic];
          fixAction.isPreferred = true;

          const range = rangeFromFixBytes(
            document,
            finding.fix.start_byte,
            finding.fix.end_byte,
          );
          if (range) {
            const edit = new vscode.WorkspaceEdit();
            edit.replace(document.uri, range, finding.fix.replacement);
            fixAction.edit = edit;
            actions.push(fixAction);
          }
        }
      }

      // "Suppress" action for every CytoScnPy diagnostic at the cursor.
      const suppressAction = this.createSuppressionAction(document, diagnostic);
      if (suppressAction) {
        actions.push(suppressAction);
      }
    }

    // 3. Add "Fix All" actions for rules with multiple findings
    for (const [ruleId, items] of fixableByRule.entries()) {
      const filteredItems = filterOverlappingFixes(items);
      if (filteredItems.length < 2) {
        continue;
      }

      const labels = UNUSED_RULE_LABELS[ruleId];
      if (!labels) {
        continue;
      }

      const fixAllAction = new vscode.CodeAction(
        `Remove all unused ${labels.plural} in this file`,
        vscode.CodeActionKind.QuickFix,
      );
      fixAllAction.diagnostics = filteredItems.map((i) => i.diagnostic);

      const edit = new vscode.WorkspaceEdit();
      // Sort by start_byte descending to apply fixes from end of file first
      // This prevents byte offset shifts from invalidating later fixes
      const sortedItems = [...filteredItems].sort(
        (a, b) => b.finding.fix!.start_byte - a.finding.fix!.start_byte,
      );

      let hasAllRanges = true;
      for (const { finding } of sortedItems) {
        const range = rangeFromFixBytes(
          document,
          finding.fix!.start_byte,
          finding.fix!.end_byte,
        );
        if (!range) {
          hasAllRanges = false;
          break;
        }
        edit.replace(document.uri, range, finding.fix!.replacement);
      }
      if (hasAllRanges) {
        fixAllAction.edit = edit;
        actions.push(fixAllAction);
      }
    }

    const filteredUnused = filterOverlappingFixes(fixableUnused);
    if (filteredUnused.length >= 2) {
      const fixAllDeadCodeAction = new vscode.CodeAction(
        "Remove all dead code in this file",
        vscode.CodeActionKind.QuickFix,
      );
      fixAllDeadCodeAction.diagnostics = filteredUnused.map(
        (i) => i.diagnostic,
      );

      const edit = new vscode.WorkspaceEdit();
      const sortedItems = [...filteredUnused].sort(
        (a, b) => b.finding.fix!.start_byte - a.finding.fix!.start_byte,
      );

      let hasAllRanges = true;
      for (const { finding } of sortedItems) {
        const range = rangeFromFixBytes(
          document,
          finding.fix!.start_byte,
          finding.fix!.end_byte,
        );
        if (!range) {
          hasAllRanges = false;
          break;
        }
        edit.replace(document.uri, range, finding.fix!.replacement);
      }
      if (hasAllRanges) {
        fixAllDeadCodeAction.edit = edit;
        actions.push(fixAllDeadCodeAction);
      }
    }

    return actions;
  }

  private createSuppressionAction(
    document: vscode.TextDocument,
    diagnostic: vscode.Diagnostic,
  ): vscode.CodeAction | undefined {
    const codeValue =
      typeof diagnostic.code === "object" &&
      diagnostic.code !== null &&
      "value" in diagnostic.code
        ? String(diagnostic.code.value)
        : typeof diagnostic.code === "string"
          ? diagnostic.code
          : undefined;
    const suppressionCode =
      codeValue && /^CSP-[A-Z]\d{3}$/i.test(codeValue) ? codeValue : "CSP";
    const actionTitle = `Suppress with # noqa: ${suppressionCode}`;

    const action = new vscode.CodeAction(
      actionTitle,
      vscode.CodeActionKind.QuickFix,
    );
    action.diagnostics = [diagnostic];

    const lineIndex = diagnostic.range.start.line;
    const lineText = document.lineAt(lineIndex).text;
    const edit = new vscode.WorkspaceEdit();
    const pragmaRegex = /#\s*pragma:\s*no\s+cytoscnpy/i;
    if (pragmaRegex.test(lineText)) {
      return undefined;
    }

    // Check for existing suppression comment
    const noqaRegex = /#\s*(?:noqa|ignore)(?::\s*([^#\n]+))?/i;
    const match = lineText.match(noqaRegex);

    if (match) {
      // Existing noqa found
      if (!match[1]) {
        // Bare # noqa - already suppresses all
        return undefined;
      }
      const existingCodes = match[1]
        .split(/,\s*/)
        .map((s) => s.trim().toUpperCase());
      if (
        existingCodes.includes("CSP") ||
        existingCodes.includes(suppressionCode.toUpperCase())
      ) {
        return undefined; // Already suppressed
      }
      // Append suppression code to existing codes
      const commentStart = match.index!;
      const commentContent = match[0];
      const newComment = `${commentContent}, ${suppressionCode}`;
      const range = new vscode.Range(
        new vscode.Position(lineIndex, commentStart),
        new vscode.Position(lineIndex, commentStart + commentContent.length),
      );
      edit.replace(document.uri, range, newComment);
    } else {
      // No existing noqa, append new one
      const insertText = `  # noqa: ${suppressionCode}`;
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
  statusBarItem.dispose();
  errorDecorationType?.dispose(); // Clean up decoration types
  warningDecorationType?.dispose();
  infoDecorationType?.dispose();
}

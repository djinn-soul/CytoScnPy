// The module 'vscode' contains the VS Code extensibility API
// Import the module and reference it with the alias vscode in your code below
import * as vscode from "vscode";
import * as path from "path";
import * as crypto from "crypto";
import {
  runCytoScnPyAnalysis,
  runWorkspaceAnalysis,
  CytoScnPyFinding,
  ParseError,
} from "./analyzer";
import { execFile } from "child_process"; // Import execFile for safer metric commands
import {
  getCytoScnPyConfiguration,
  getExecutablePath,
} from "./configuration";
import { resolveFinding } from "./findingResolver";
import { CoalescingTaskQueue, DebouncedTaskMap } from "./scanScheduler";

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
const workspaceScanQueue = new CoalescingTaskQueue();
const saveScanDebouncer = new DebouncedTaskMap();

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
          mcpDidChangeEmitter,
          vscode.workspace.onDidChangeWorkspaceFolders(() =>
            mcpDidChangeEmitter.fire(),
          ),
        );
        context.subscriptions.push(
          vscode.lm.registerMcpServerDefinitionProvider("cytoscnpy-mcp", {
            onDidChangeMcpServerDefinitions: mcpDidChangeEmitter.event,
            provideMcpServerDefinitions: async () => {
              const workspaceFolders = vscode.workspace.workspaceFolders;
              const extension =
                vscode.extensions.getExtension("djinn09.cytoscnpy");
              const version = extension?.packageJSON?.version || "0.1.0";
              if (!workspaceFolders || workspaceFolders.length === 0) {
                return [
                  new vscode.McpStdioServerDefinition(
                    "CytoScnPy",
                    getExecutablePath(context),
                    ["mcp-server"],
                    { cwd: null, version },
                  ),
                ];
              }
              return workspaceFolders.map(
                (folder) =>
                new vscode.McpStdioServerDefinition(
                  `CytoScnPy (${folder.name})`,
                  getExecutablePath(context, folder.uri),
                  ["mcp-server"],
                  { cwd: folder.uri.fsPath, version },
                ),
              );
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
      diagnostics: readonly vscode.Diagnostic[],
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

    function cacheDocumentAnalysis(
      document: vscode.TextDocument,
      diagnostics: vscode.Diagnostic[],
      findings: CytoScnPyFinding[],
    ): void {
      if (document.isDirty) {
        return;
      }
      const cacheKey = getCacheKey(document.uri.fsPath);
      const cacheEntry: CacheEntry = {
        hash: computeHash(document.getText()),
        diagnostics,
        findings,
        timestamp: Date.now(),
      };
      const history = fileCache.get(cacheKey) || [];
      history.unshift(cacheEntry);
      if (history.length > MAX_CACHE_HISTORY) {
        history.pop();
      }
      fileCache.set(cacheKey, history);
    }

    function clearDocumentAnalysis(document: vscode.TextDocument): void {
      cytoscnpyDiagnostics.delete(document.uri);
      fileCache.delete(getCacheKey(document.uri.fsPath));
      documentHashCache.delete(document.uri.toString());
      const editor = vscode.window.activeTextEditor;
      if (editor?.document.uri.toString() === document.uri.toString()) {
        applyGutterDecorations(editor, []);
      }
    }

    // Function to run workspace analysis and populate cache
    async function runFullWorkspaceAnalysis() {
      return workspaceScanQueue.run(performFullWorkspaceAnalysis);
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
          cancellable: false,
        },
        async (progress) => {
          try {
            const startTime = Date.now();
            const findingsByFile = new Map<string, CytoScnPyFinding[]>();
            const parseErrorsByFile = new Map<string, ParseError[]>();
            for (const folder of workspaceFolders) {
              progress.report({ message: `Scanning ${folder.name}...` });
              const result = await runWorkspaceAnalysis(
                folder.uri.fsPath,
                getCytoScnPyConfiguration(context, folder.uri),
              );
              for (const [filePath, findings] of result.findingsByFile) {
                findingsByFile.set(getCacheKey(filePath), findings);
              }
              for (const [filePath, errors] of result.parseErrorsByFile) {
                parseErrorsByFile.set(getCacheKey(filePath), errors);
              }
            }
            workspaceCache = findingsByFile;
            workspaceParseErrorsCache = parseErrorsByFile;
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

            cytoscnpyDiagnostics.clear();
            const openDocuments = new Map(
              vscode.workspace.textDocuments
                .filter((document) => document.languageId === "python")
                .map((document) => [getCacheKey(document.uri.fsPath), document]),
            );
            const filesWithDiagnostics = new Set<string>([
              ...workspaceCache.keys(),
              ...(workspaceParseErrorsCache?.keys() ?? []),
            ]);

            for (const filePath of filesWithDiagnostics) {
              if (openDocuments.has(filePath)) {
                continue;
              }
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

            for (const [filePath, document] of openDocuments) {
              if (document.isDirty) {
                fileCache.delete(filePath);
                continue;
              }
              const findings = workspaceCache.get(filePath) || [];
              const parseErrors = workspaceParseErrorsCache?.get(filePath) || [];
              const diagnostics = [
                ...findingsToDiagnostics(document, findings),
                ...parseErrorsToDiagnostics(document, parseErrors),
              ];
              cytoscnpyDiagnostics.set(document.uri, diagnostics);
              cacheDocumentAnalysis(document, diagnostics, findings);
            }

            const activeEditor = vscode.window.activeTextEditor;
            if (activeEditor?.document.languageId === "python") {
              applyGutterDecorations(
                activeEditor,
                activeEditor.document.isDirty
                  ? []
                  : cytoscnpyDiagnostics.get(activeEditor.document.uri) || [],
              );
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
            workspaceCacheTimestamp = 0;
            fileCache.clear();
            documentHashCache.clear();
            cytoscnpyDiagnostics.clear();
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

        // Merge into workspace cache if it exists
        if (workspaceCache) {
          workspaceCache.set(getCacheKey(filePath), result.findings);
          if (workspaceParseErrorsCache) {
            workspaceParseErrorsCache.set(
              getCacheKey(filePath),
              result.parseErrors,
            );
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
        clearDocumentAnalysis(document);
        setStatus("error", "file analysis failed");
        vscode.window.showErrorMessage(
          `CytoScnPy analysis failed: ${error.message}`,
        );
      }
    }

    // Function to refresh diagnostics for the active document
    async function refreshDiagnostics(document: vscode.TextDocument) {
      if (document.languageId !== "python") {
        return; // Only analyze Python files
      }
      if (document.isDirty) {
        clearDocumentAnalysis(document);
        setStatus("idle", "save file to analyze");
        return;
      }

      const fsPath = document.uri.fsPath;
      const filePath =
        process.platform === "win32" ? fsPath.toLowerCase() : fsPath;
      const config = getCytoScnPyConfiguration(context, document.uri);

      // FILE MODE: Single file analysis (faster, but may have false positives)
      if (config.analysisMode === "file") {
        try {
          const documentVersion = document.version;
          const result = await runCytoScnPyAnalysis(fsPath, config);
          if (document.isDirty || document.version !== documentVersion) {
            clearDocumentAnalysis(document);
            return;
          }
          const diagnostics = [
            ...findingsToDiagnostics(document, result.findings),
            ...parseErrorsToDiagnostics(document, result.parseErrors),
          ];
          cytoscnpyDiagnostics.set(document.uri, diagnostics);

          cacheDocumentAnalysis(document, diagnostics, result.findings);

          const editor = vscode.window.activeTextEditor;
          if (
            editor &&
            editor.document.uri.toString() === document.uri.toString()
          ) {
            applyGutterDecorations(editor, diagnostics);
          }
        } catch (error: any) {
          console.error(`[CytoScnPy] File analysis failed: ${error.message}`);
          clearDocumentAnalysis(document);
          setStatus("error", "file analysis failed");
          vscode.window.showErrorMessage(
            `CytoScnPy analysis failed: ${error.message}`,
          );
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
    context.subscriptions.push(saveScanDebouncer);

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
          saveScanDebouncer.schedule(debounceKey, debounceMs, () => {
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
          // Clear caches to force re-analysis with new settings
          invalidateWorkspaceCache();
          const pythonDocuments = vscode.workspace.textDocuments.filter(
            (doc) => doc.languageId === "python",
          );
          if (
            vscode.workspace.workspaceFolders?.length &&
            pythonDocuments.some(
              (doc) =>
                getCytoScnPyConfiguration(context, doc.uri).analysisMode ===
                "workspace",
            )
          ) {
            void runFullWorkspaceAnalysis();
          } else {
            pythonDocuments.forEach((doc) => void refreshDiagnostics(doc));
          }
        }
      }),
    );

    context.subscriptions.push(
      vscode.workspace.onDidChangeWorkspaceFolders(() => {
        invalidateWorkspaceCache();
        void runFullWorkspaceAnalysis();
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

    // Register a command to manually trigger analysis (e.g., from command palette)
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

          // Use the canonical workspace path: it refreshes the cache and
          // replaces the DiagnosticCollection that powers the Problems tab.
          await runFullWorkspaceAnalysis();
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
      if (!cachedEntry) {
        return undefined;
      }
      return resolveFinding(cachedEntry.findings, {
        ruleId,
        line: diagnostic.range.start.line + 1,
        message: diagnostic.message,
      });
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

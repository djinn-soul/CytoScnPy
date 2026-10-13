import * as vscode from "vscode";

export const cytoscnpyDiagnostics =
  vscode.languages.createDiagnosticCollection("cytoscnpy");

// VS Code stops syncing a batched `set` to the Problems view once it passes
// 1.1x its per-file limit (1,000), dropping the remaining files silently.
// Stay well under that per batch.
const DIAGNOSTICS_PER_BATCH = 500;

/**
 * Sets diagnostics for many files in a few batched updates instead of one
 * message to the main process per file.
 */
export function setDiagnosticsBatched(
  entries: readonly [vscode.Uri, vscode.Diagnostic[]][],
  collection: Pick<vscode.DiagnosticCollection, "set"> = cytoscnpyDiagnostics,
): void {
  let batch: [vscode.Uri, vscode.Diagnostic[]][] = [];
  let batchSize = 0;
  for (const entry of entries) {
    if (batch.length > 0 && batchSize + entry[1].length > DIAGNOSTICS_PER_BATCH) {
      collection.set(batch);
      batch = [];
      batchSize = 0;
    }
    batch.push(entry);
    batchSize += entry[1].length;
  }
  if (batch.length > 0) {
    collection.set(batch);
  }
}

export const cytoscnpyOutputChannel =
  vscode.window.createOutputChannel("CytoScnPy Metrics");

const statusBarItem = vscode.window.createStatusBarItem(
  vscode.StatusBarAlignment.Right,
  100,
);

statusBarItem.command = "cytoscnpy.analyzeWorkspace";

type StatusKind = "idle" | "running" | "error";

export function setStatus(kind: StatusKind, detail: string): void {
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

// One popup per failure streak: a broken binary would otherwise raise a
// new error on every tab switch. The status bar keeps showing the failure.
let failurePopupShown = false;

export function reportAnalysisFailure(message: string): void {
  if (failurePopupShown) {
    return;
  }
  failurePopupShown = true;
  void vscode.window.showErrorMessage(`CytoScnPy analysis failed: ${message}`);
}

export function resetAnalysisFailure(): void {
  failurePopupShown = false;
}

let errorDecorationType: vscode.TextEditorDecorationType;

let warningDecorationType: vscode.TextEditorDecorationType;

let infoDecorationType: vscode.TextEditorDecorationType;

/**
 * Creates the gutter decorations and hands every UI object to the extension
 * context, which disposes each one exactly once on deactivation.
 */
export function initializeUi(context: vscode.ExtensionContext): void {
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
    cytoscnpyDiagnostics,
    cytoscnpyOutputChannel,
    statusBarItem,
    errorDecorationType,
    warningDecorationType,
    infoDecorationType,
  );
  setStatus("idle", "ready");

}

export function applyGutterDecorations(
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

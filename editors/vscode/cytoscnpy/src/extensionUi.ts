import * as vscode from "vscode";

export const cytoscnpyDiagnostics =
  vscode.languages.createDiagnosticCollection("cytoscnpy");

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

let errorDecorationType: vscode.TextEditorDecorationType;

let warningDecorationType: vscode.TextEditorDecorationType;

let infoDecorationType: vscode.TextEditorDecorationType;

export function initializeDecorations(context: vscode.ExtensionContext): void {
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

export function disposeUi() {
  cytoscnpyDiagnostics.dispose(); // Clean up diagnostics when extension is deactivated
  cytoscnpyOutputChannel.dispose(); // Clean up output channel
  statusBarItem.dispose();
  errorDecorationType?.dispose(); // Clean up decoration types
  warningDecorationType?.dispose();
  infoDecorationType?.dispose();
}

import * as vscode from "vscode";
import { CytoScnPyFinding, ParseError } from "./analyzer";

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

export function findingsToDiagnostics(
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
      // A stale line number anchors to the file start but keeps the
      // CytoScnPy source and code, so suppression actions still apply.
      let range = new vscode.Range(0, 0, 0, 0);
      if (lineIndex >= 0 && lineIndex < document.lineCount) {
        const lineText = document.lineAt(lineIndex);
        const startCol =
          finding.col && finding.col > 0
            ? finding.col
            : lineText.firstNonWhitespaceCharacterIndex;
        range = new vscode.Range(
          new vscode.Position(lineIndex, startCol),
          new vscode.Position(lineIndex, lineText.text.length),
        );
      }
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

export function parseErrorsToDiagnostics(
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

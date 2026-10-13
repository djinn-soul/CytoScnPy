import * as vscode from "vscode";
import { CytoScnPyFinding, ParseError } from "./analyzer";
import { Directive, Suppression, covers, directivesByLine } from "./suppression";

export function mapSeverity(severity: string): vscode.DiagnosticSeverity {
  const upper = severity.toUpperCase();
  if (upper === "CRITICAL" || upper === "ERROR") {
    return vscode.DiagnosticSeverity.Error;
  }
  return vscode.DiagnosticSeverity.Warning;
}

export const UNUSED_RULE_IDS: ReadonlySet<string> = new Set([
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

const directiveCache = new WeakMap<
  vscode.TextDocument,
  { version: number; directives: Map<number, Directive> }
>();

/** Suppression directives per line (0-based), cached per document version. */
export function documentDirectives(
  document: vscode.TextDocument,
): Map<number, Directive> {
  const cached = directiveCache.get(document);
  if (cached && cached.version === document.version) {
    return cached.directives;
  }
  const directives = directivesByLine(document.getText());
  directiveCache.set(document, { version: document.version, directives });
  return directives;
}

/** Rule ID that suppresses every kind of unused definition, as in the CLI. */
export const DEAD_CODE_RULE_ID = "CSP-V001";

/** Matches the CLI: dead code also answers to `CSP-V001`. */
export function suppressesFinding(suppression: Suppression, ruleId: string): boolean {
  return (
    covers(suppression, ruleId) ||
    (UNUSED_RULE_IDS.has(ruleId) && covers(suppression, DEAD_CODE_RULE_ID))
  );
}

export function findingsToDiagnostics(
  document: vscode.TextDocument,
  findings: CytoScnPyFinding[],
): vscode.Diagnostic[] {
  const directives = documentDirectives(document);
  return findings
    .filter((finding) => {
      const suppression = directives.get(finding.line_number - 1)?.suppression;
      return !suppression || !suppressesFinding(suppression, finding.rule_id);
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

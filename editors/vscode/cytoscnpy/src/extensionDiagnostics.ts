import * as vscode from "vscode";
import { findingsForDocument, parseErrorsForDocument } from "./notebookCells";
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
  diagnosticSuppressionCodes.set(diagnostic, findingSuppressionCodes(finding));
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

/** Codes whose `# noqa` suppresses each diagnostic built from a finding. */
const diagnosticSuppressionCodes = new WeakMap<vscode.Diagnostic, readonly string[]>();

/**
 * Every code whose `# noqa` suppresses the finding. The CLI decides these
 * (e.g. `CSP-V001` and `F401` for an unused import); without them only the
 * rule ID does.
 */
export function findingSuppressionCodes(finding: CytoScnPyFinding): readonly string[] {
  return finding.suppression_codes ?? [finding.rule_id];
}

/** Suppression codes of a diagnostic built from a finding, if known. */
export function suppressionCodesForDiagnostic(
  diagnostic: vscode.Diagnostic,
): readonly string[] | undefined {
  return diagnosticSuppressionCodes.get(diagnostic);
}

export function suppressesFinding(
  suppression: Suppression,
  codes: readonly string[],
): boolean {
  return codes.some((code) => covers(suppression, code));
}

export function findingsToDiagnostics(
  document: vscode.TextDocument,
  findings: CytoScnPyFinding[],
): vscode.Diagnostic[] {
  const directives = documentDirectives(document);
  return findingsForDocument(document, findings)
    .filter((finding) => {
      const suppression = directives.get(finding.line_number - 1)?.suppression;
      return !suppression || !suppressesFinding(suppression, findingSuppressionCodes(finding));
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
      diagnosticSuppressionCodes.set(diagnostic, findingSuppressionCodes(finding));

      return diagnostic;
    });
}

export function parseErrorsToDiagnostics(
  document: vscode.TextDocument,
  parseErrors: ParseError[],
): vscode.Diagnostic[] {
  return parseErrorsForDocument(document, parseErrors).map((parseError) => {
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

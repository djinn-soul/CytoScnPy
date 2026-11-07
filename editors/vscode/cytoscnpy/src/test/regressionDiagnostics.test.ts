// Regression tests locking behaviors surfaced by audit before any refactor.
// Each block targets one audit item so future fixes can prove they did not regress
// the surrounding behavior.

import * as assert from "assert";
import * as vscode from "vscode";
import {
  computeHash,
  hashForDocument,
  mapSeverity,
  buildClosedFileDiagnosticFromFinding,
  buildClosedFileDiagnosticFromParseError,
} from "../extension";
import { CytoScnPyFinding } from "../analyzer";

suite("Regression: mapSeverity", () => {
  test("CRITICAL / ERROR map to Error", () => {
    assert.strictEqual(mapSeverity("CRITICAL"), vscode.DiagnosticSeverity.Error);
    assert.strictEqual(mapSeverity("error"), vscode.DiagnosticSeverity.Error);
  });

  test("HIGH / WARNING / MEDIUM / INFO / LOW / HINT / unknown collapse to Warning", () => {
    for (const s of ["HIGH", "WARNING", "MEDIUM", "INFO", "LOW", "HINT", "asdf"]) {
      assert.strictEqual(
        mapSeverity(s),
        vscode.DiagnosticSeverity.Warning,
        `expected ${s} to map to Warning`,
      );
    }
  });
});

suite("Regression: hashForDocument memoization", () => {
  test("returns the same hash for the same version without recomputing", async () => {
    const doc = await vscode.workspace.openTextDocument({
      language: "python",
      content: "x = 1\n",
    });
    const h1 = hashForDocument(doc);
    const h2 = hashForDocument(doc);
    assert.strictEqual(h1, h2);
    assert.strictEqual(h1, computeHash(doc.getText()));
  });
});

suite("Regression: buildClosedFileDiagnostic*", () => {
  test("finding diagnostic spans full line via MAX_SAFE_INTEGER end column", () => {
    const finding: CytoScnPyFinding = {
      file_path: "foo.py",
      line_number: 3,
      col: 4,
      message: "unused",
      rule_id: "unused-function",
      category: "Dead Code",
      severity: "warning",
    };
    const diag = buildClosedFileDiagnosticFromFinding(finding);
    assert.strictEqual(diag.range.start.line, 2);
    assert.strictEqual(diag.range.start.character, 4);
    assert.strictEqual(diag.range.end.character, Number.MAX_SAFE_INTEGER);
    assert.strictEqual(diag.source, "CytoScnPy");
    assert.strictEqual(diag.code, "unused-function");
    assert.deepStrictEqual(diag.tags, [vscode.DiagnosticTag.Unnecessary]);
  });

  test("non-dead-code rule does not get Unnecessary tag", () => {
    const finding: CytoScnPyFinding = {
      file_path: "foo.py",
      line_number: 1,
      message: "danger",
      rule_id: "CSP-D102",
      category: "Security",
      severity: "error",
    };
    const diag = buildClosedFileDiagnosticFromFinding(finding);
    assert.strictEqual(diag.severity, vscode.DiagnosticSeverity.Error);
    assert.strictEqual(diag.tags, undefined);
  });

  test("missing col defaults to 0", () => {
    const finding: CytoScnPyFinding = {
      file_path: "foo.py",
      line_number: 1,
      message: "msg",
      rule_id: "unused-import",
      category: "Dead Code",
      severity: "warning",
    };
    const diag = buildClosedFileDiagnosticFromFinding(finding);
    assert.strictEqual(diag.range.start.character, 0);
  });

  test("parse error diagnostic always Error severity, full-line span", () => {
    const diag = buildClosedFileDiagnosticFromParseError({
      file: "foo.py",
      line: 2,
      message: "unexpected token",
    });
    assert.strictEqual(diag.severity, vscode.DiagnosticSeverity.Error);
    assert.strictEqual(diag.range.start.line, 1);
    assert.strictEqual(diag.range.start.character, 0);
    assert.strictEqual(diag.range.end.character, Number.MAX_SAFE_INTEGER);
    assert.strictEqual(diag.source, "CytoScnPy [Parse]");
    assert.strictEqual(diag.code, "parse-error");
  });
});

// Regression tests locking behaviors surfaced by audit before any refactor.
// Each block targets one audit item so future fixes can prove they did not regress
// the surrounding behavior.

import * as assert from "assert";
import * as vscode from "vscode";
import {
  QuickFixProvider,
  fileCache,
  getCacheKey,
  computeHash,
  CacheEntry,
} from "../extension";
import { makeContext, makeFinding } from "./regressionHelpers";

suite("Regression: byte range validation", () => {
  let provider: QuickFixProvider;
  let doc: vscode.TextDocument;
  let cacheKey: string;

  setup(async () => {
    fileCache.clear();
    provider = new QuickFixProvider();
    doc = await vscode.workspace.openTextDocument({
      language: "python",
      content: "import os\n",
    });
    cacheKey = getCacheKey(doc.uri.fsPath);
  });

  test("out-of-range end_byte suppresses the Remove action", () => {
    const text = doc.getText();
    const utf8Len = Buffer.byteLength(text, "utf8");
    const entry: CacheEntry = {
      hash: computeHash(text),
      diagnostics: [],
      findings: [makeFinding("unused-import", 0, utf8Len + 50) as any],
      timestamp: Date.now(),
    };
    fileCache.set(cacheKey, [entry]);

    const diag = new vscode.Diagnostic(
      new vscode.Range(0, 7, 0, 9),
      "'os' is imported but never used",
      vscode.DiagnosticSeverity.Warning,
    );
    diag.source = "CytoScnPy";
    diag.code = "unused-import";

    const actions = provider.provideCodeActions(
      doc,
      diag.range,
      makeContext(diag),
      new vscode.CancellationTokenSource().token,
    );
    // No Remove action — only Suppress.
    assert.ok(!actions.some((a) => a.title.startsWith("Remove")));
    assert.ok(actions.some((a) => a.title.includes("Suppress")));
  });

  test("negative start_byte suppresses the Remove action", () => {
    const entry: CacheEntry = {
      hash: computeHash(doc.getText()),
      diagnostics: [],
      findings: [makeFinding("unused-import", -1, 9) as any],
      timestamp: Date.now(),
    };
    fileCache.set(cacheKey, [entry]);

    const diag = new vscode.Diagnostic(
      new vscode.Range(0, 7, 0, 9),
      "'os' is imported but never used",
      vscode.DiagnosticSeverity.Warning,
    );
    diag.source = "CytoScnPy";
    diag.code = "unused-import";

    const actions = provider.provideCodeActions(
      doc,
      diag.range,
      makeContext(diag),
      new vscode.CancellationTokenSource().token,
    );
    assert.ok(!actions.some((a) => a.title.startsWith("Remove")));
  });

  test("end_byte < start_byte suppresses the Remove action", () => {
    const entry: CacheEntry = {
      hash: computeHash(doc.getText()),
      diagnostics: [],
      findings: [makeFinding("unused-import", 5, 2) as any],
      timestamp: Date.now(),
    };
    fileCache.set(cacheKey, [entry]);

    const diag = new vscode.Diagnostic(
      new vscode.Range(0, 7, 0, 9),
      "'os' is imported but never used",
      vscode.DiagnosticSeverity.Warning,
    );
    diag.source = "CytoScnPy";
    diag.code = "unused-import";

    const actions = provider.provideCodeActions(
      doc,
      diag.range,
      makeContext(diag),
      new vscode.CancellationTokenSource().token,
    );
    assert.ok(!actions.some((a) => a.title.startsWith("Remove")));
  });
});

suite("Regression: suppression action edge cases", () => {
  let provider: QuickFixProvider;

  setup(() => {
    provider = new QuickFixProvider();
  });

  test("skips when pragma: no cytoscnpy already present", async () => {
    const doc = await vscode.workspace.openTextDocument({
      language: "python",
      content: "x = 1  # pragma: no cytoscnpy\n",
    });
    const diag = new vscode.Diagnostic(
      new vscode.Range(0, 0, 0, 5),
      "Unused variable",
      vscode.DiagnosticSeverity.Warning,
    );
    diag.source = "CytoScnPy";
    diag.code = "unused-variable";

    const actions = provider.provideCodeActions(
      doc,
      diag.range,
      makeContext(diag),
      new vscode.CancellationTokenSource().token,
    );
    assert.strictEqual(
      actions.filter((a) => a.title.includes("Suppress")).length,
      0,
    );
  });

  test("skips when bare # noqa already covers the line", async () => {
    const doc = await vscode.workspace.openTextDocument({
      language: "python",
      content: "x = 1  # noqa\n",
    });
    const diag = new vscode.Diagnostic(
      new vscode.Range(0, 0, 0, 5),
      "Unused variable",
      vscode.DiagnosticSeverity.Warning,
    );
    diag.source = "CytoScnPy";
    diag.code = "unused-variable";

    const actions = provider.provideCodeActions(
      doc,
      diag.range,
      makeContext(diag),
      new vscode.CancellationTokenSource().token,
    );
    assert.strictEqual(
      actions.filter((a) => a.title.includes("Suppress")).length,
      0,
    );
  });

  test("skips when CSP token already in noqa list", async () => {
    const doc = await vscode.workspace.openTextDocument({
      language: "python",
      content: "x = 1  # noqa: CSP\n",
    });
    const diag = new vscode.Diagnostic(
      new vscode.Range(0, 0, 0, 5),
      "Unused variable",
      vscode.DiagnosticSeverity.Warning,
    );
    diag.source = "CytoScnPy";
    diag.code = "CSP-V001";

    const actions = provider.provideCodeActions(
      doc,
      diag.range,
      makeContext(diag),
      new vscode.CancellationTokenSource().token,
    );
    assert.strictEqual(
      actions.filter((a) => a.title.includes("Suppress")).length,
      0,
    );
  });

  test("invalid rule code falls back to 'CSP' bucket", async () => {
    const doc = await vscode.workspace.openTextDocument({
      language: "python",
      content: "x = 1\n",
    });
    const diag = new vscode.Diagnostic(
      new vscode.Range(0, 0, 0, 5),
      "Unused variable",
      vscode.DiagnosticSeverity.Warning,
    );
    diag.source = "CytoScnPy";
    diag.code = "not-a-csp-id";

    const actions = provider.provideCodeActions(
      doc,
      diag.range,
      makeContext(diag),
      new vscode.CancellationTokenSource().token,
    );
    const suppress = actions.find((a) => a.title.includes("Suppress"));
    assert.ok(suppress, "should still offer suppression with generic CSP");
    assert.ok(suppress!.title.endsWith("CSP"));
  });
});

suite("Regression: cache hash mismatch invalidates Remove action", () => {
  let provider: QuickFixProvider;

  setup(() => {
    fileCache.clear();
    provider = new QuickFixProvider();
  });

  test("cached entry whose hash does not match current text is ignored", async () => {
    const doc = await vscode.workspace.openTextDocument({
      language: "python",
      content: "import os\n",
    });
    const cacheKey = getCacheKey(doc.uri.fsPath);

    // Cache a finding whose hash refers to STALE content.
    const staleEntry: CacheEntry = {
      hash: computeHash("STALE CONTENT NOT MATCHING DOC"),
      diagnostics: [],
      findings: [makeFinding("unused-import", 0, 10) as any],
      timestamp: Date.now(),
    };
    fileCache.set(cacheKey, [staleEntry]);

    const diag = new vscode.Diagnostic(
      new vscode.Range(0, 7, 0, 9),
      "'os' is imported but never used",
      vscode.DiagnosticSeverity.Warning,
    );
    diag.source = "CytoScnPy";
    diag.code = "unused-import";

    const actions = provider.provideCodeActions(
      doc,
      diag.range,
      makeContext(diag),
      new vscode.CancellationTokenSource().token,
    );
    assert.ok(
      !actions.some((a) => a.title.startsWith("Remove")),
      "stale cache must not produce Remove action",
    );
  });
});

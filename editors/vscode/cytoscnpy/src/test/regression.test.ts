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
import {
  MAX_CACHE_HISTORY,
  makeContext,
  makeFinding,
} from "./regressionHelpers";

suite("Regression: cache key normalization", () => {
  test("getCacheKey lowercases on win32, otherwise identity", () => {
    const input = "C:/Users/Foo/Bar.PY";
    const key = getCacheKey(input);
    if (process.platform === "win32") {
      assert.strictEqual(key, input.toLowerCase());
    } else {
      assert.strictEqual(key, input);
    }
  });

  test("getCacheKey is idempotent", () => {
    const k1 = getCacheKey("/tmp/AbC.py");
    const k2 = getCacheKey(k1);
    assert.strictEqual(k1, k2);
  });
});

suite("Regression: computeHash determinism", () => {
  test("identical content yields identical hash", () => {
    const a = computeHash("def f(): pass\n");
    const b = computeHash("def f(): pass\n");
    assert.strictEqual(a, b);
  });

  test("content diff yields hash diff", () => {
    const a = computeHash("def f(): pass\n");
    const b = computeHash("def g(): pass\n");
    assert.notStrictEqual(a, b);
  });

  test("hash is hex of length 64 (SHA-256)", () => {
    const h = computeHash("anything");
    assert.match(h, /^[0-9a-f]{64}$/);
  });
});

suite("Regression: fileCache history capacity", () => {
  let doc: vscode.TextDocument;
  let cacheKey: string;

  setup(async () => {
    fileCache.clear();
    doc = await vscode.workspace.openTextDocument({
      language: "python",
      content: "x = 1\n",
    });
    cacheKey = getCacheKey(doc.uri.fsPath);
  });

  test("manual insertion respects MAX_CACHE_HISTORY when caller bounds it", () => {
    // Lock the assumption that callers cap history at MAX_CACHE_HISTORY entries.
    // The cache itself does not enforce — the producers in extension.ts do.
    const history: CacheEntry[] = [];
    for (let i = 0; i < 25; i++) {
      const entry: CacheEntry = {
        hash: computeHash(`iter-${i}`),
        diagnostics: [],
        findings: [],
        timestamp: Date.now() + i,
      };
      history.unshift(entry);
      if (history.length > MAX_CACHE_HISTORY) {
        history.pop();
      }
    }
    fileCache.set(cacheKey, history);
    assert.strictEqual(fileCache.get(cacheKey)!.length, MAX_CACHE_HISTORY);
  });
});

suite("Regression: QuickFix scope filter", () => {
  let provider: QuickFixProvider;
  let doc: vscode.TextDocument;

  setup(async () => {
    fileCache.clear();
    provider = new QuickFixProvider();
    doc = await vscode.workspace.openTextDocument({
      language: "python",
      content: "import os\n",
    });
  });

  test("ignores diagnostics not from CytoScnPy", () => {
    const foreign = new vscode.Diagnostic(
      new vscode.Range(0, 7, 0, 9),
      "from pylint",
      vscode.DiagnosticSeverity.Warning,
    );
    foreign.source = "pylint";
    foreign.code = "unused-import";

    const actions = provider.provideCodeActions(
      doc,
      foreign.range,
      makeContext(foreign),
      new vscode.CancellationTokenSource().token,
    );
    assert.strictEqual(actions.length, 0);
  });

  test("accepts diagnostics where source begins with 'CytoScnPy'", () => {
    const diag = new vscode.Diagnostic(
      new vscode.Range(0, 7, 0, 9),
      "'os' is imported but never used",
      vscode.DiagnosticSeverity.Warning,
    );
    diag.source = "CytoScnPy [Dead Code]";
    diag.code = "unused-import";

    const actions = provider.provideCodeActions(
      doc,
      diag.range,
      makeContext(diag),
      new vscode.CancellationTokenSource().token,
    );
    // Without a cache match, only suppress action is returned.
    assert.ok(actions.length >= 1);
    assert.ok(actions.some((a) => a.title.includes("Suppress")));
  });

  test("uses context diagnostics even when global diagnostics are different objects", () => {
    const text = doc.getText();
    const cacheKey = getCacheKey(doc.uri.fsPath);
    const entry: CacheEntry = {
      hash: computeHash(text),
      diagnostics: [],
      findings: [makeFinding("unused-import", 0, Buffer.byteLength(text, "utf8")) as any],
      timestamp: Date.now(),
    };
    fileCache.set(cacheKey, [entry]);

    const globalDiag = new vscode.Diagnostic(
      new vscode.Range(0, 7, 0, 9),
      "'os' is imported but never used",
      vscode.DiagnosticSeverity.Warning,
    );
    globalDiag.source = "CytoScnPy";
    globalDiag.code = "unused-import";

    const contextDiag = new vscode.Diagnostic(
      globalDiag.range,
      globalDiag.message,
      vscode.DiagnosticSeverity.Warning,
    );
    contextDiag.source = "CytoScnPy";
    contextDiag.code = "unused-import";

    const diagnosticCollection =
      vscode.languages.createDiagnosticCollection("cytoscnpy-regression");
    diagnosticCollection.set(doc.uri, [globalDiag]);

    const actions = provider.provideCodeActions(
      doc,
      contextDiag.range,
      makeContext(contextDiag),
      new vscode.CancellationTokenSource().token,
    );

    assert.ok(
      actions.some((a) => a.title.startsWith("Remove unused import")),
      "context diagnostic should produce a direct Remove action",
    );
    assert.ok(actions.some((a) => a.title.includes("Suppress")));

    diagnosticCollection.dispose();
  });
});

suite("Regression: QuickFix cancellation token", () => {
  let provider: QuickFixProvider;
  let doc: vscode.TextDocument;

  setup(async () => {
    provider = new QuickFixProvider();
    doc = await vscode.workspace.openTextDocument({
      language: "python",
      content: "import os\n",
    });
  });

  test("pre-cancelled token short-circuits to empty action list", () => {
    const diag = new vscode.Diagnostic(
      new vscode.Range(0, 7, 0, 9),
      "'os' is imported but never used",
      vscode.DiagnosticSeverity.Warning,
    );
    diag.source = "CytoScnPy";
    diag.code = "unused-import";

    const cts = new vscode.CancellationTokenSource();
    cts.cancel();

    const actions = provider.provideCodeActions(
      doc,
      diag.range,
      makeContext(diag),
      cts.token,
    );
    assert.deepStrictEqual(actions, []);
  });

  test("uncancelled token still produces actions", () => {
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
    assert.ok(actions.length >= 1);
  });
});

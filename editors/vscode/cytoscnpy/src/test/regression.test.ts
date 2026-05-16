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
  hashForDocument,
  mapSeverity,
  buildClosedFileDiagnosticFromFinding,
  buildClosedFileDiagnosticFromParseError,
  CacheEntry,
} from "../extension";
import {
  buildAnalyzerArgs,
  CytoScnPyConfig,
  CytoScnPyFinding,
} from "../analyzer";

const MAX_CACHE_HISTORY = 10;

function makeContext(diagnostic: vscode.Diagnostic): vscode.CodeActionContext {
  return {
    diagnostics: [diagnostic],
    triggerKind: vscode.CodeActionTriggerKind.Invoke,
    only: undefined,
  };
}

function makeFinding(
  ruleId: string,
  startByte: number,
  endByte: number,
  line = 1,
) {
  return {
    rule_id: ruleId,
    line_number: line,
    message: "stub",
    fix: { start_byte: startByte, end_byte: endByte, replacement: "" },
  };
}

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

suite("Regression: buildAnalyzerArgs", () => {
  const baseConfig: CytoScnPyConfig = {
    path: "cytoscnpy",
    analysisMode: "workspace",
    enableSecretsScan: false,
    enableDangerScan: false,
    enableQualityScan: false,
    enableCloneScan: false,
  };

  test("minimal config emits only the required prefix", () => {
    const args = buildAnalyzerArgs("/tmp/foo.py", baseConfig);
    assert.deepStrictEqual(args, [
      "--client",
      "vscode",
      "/tmp/foo.py",
      "--json",
    ]);
  });

  test("scan toggles append their flags", () => {
    const args = buildAnalyzerArgs("/tmp/foo.py", {
      ...baseConfig,
      enableSecretsScan: true,
      enableDangerScan: true,
      enableCloneScan: true,
    });
    assert.ok(args.includes("--secrets"));
    assert.ok(args.includes("--danger"));
    assert.ok(args.includes("--clones"));
  });

  test("quality toggles are only emitted when enableQualityScan is true", () => {
    const off = buildAnalyzerArgs("/tmp/foo.py", {
      ...baseConfig,
      maxComplexity: 7,
      maxNesting: 4,
    });
    assert.ok(!off.includes("--max-complexity"));
    assert.ok(!off.includes("--max-nesting"));

    const on = buildAnalyzerArgs("/tmp/foo.py", {
      ...baseConfig,
      enableQualityScan: true,
      maxComplexity: 7,
      maxNesting: 4,
    });
    assert.ok(on.includes("--quality"));
    assert.deepStrictEqual(
      on.slice(on.indexOf("--max-complexity"), on.indexOf("--max-complexity") + 2),
      ["--max-complexity", "7"],
    );
    assert.deepStrictEqual(
      on.slice(on.indexOf("--max-nesting"), on.indexOf("--max-nesting") + 2),
      ["--max-nesting", "4"],
    );
  });

  test("zero confidence threshold is dropped, positive value is emitted", () => {
    const zero = buildAnalyzerArgs("/tmp/foo.py", {
      ...baseConfig,
      confidenceThreshold: 0,
    });
    assert.ok(!zero.includes("--confidence"));

    const fifty = buildAnalyzerArgs("/tmp/foo.py", {
      ...baseConfig,
      confidenceThreshold: 50,
    });
    assert.deepStrictEqual(
      fifty.slice(fifty.indexOf("--confidence"), fifty.indexOf("--confidence") + 2),
      ["--confidence", "50"],
    );
  });

  test("exclude / include folders repeat the flag per entry", () => {
    const args = buildAnalyzerArgs("/tmp/foo.py", {
      ...baseConfig,
      excludeFolders: ["build", "dist"],
      includeFolders: ["src"],
    });
    const excludeFlagPositions = args.reduce<number[]>((acc, v, i) => {
      if (v === "--exclude-folders") {
        acc.push(i);
      }
      return acc;
    }, []);
    assert.strictEqual(excludeFlagPositions.length, 2);
    assert.strictEqual(args[excludeFlagPositions[0] + 1], "build");
    assert.strictEqual(args[excludeFlagPositions[1] + 1], "dist");
    assert.ok(args.includes("--include-folders"));
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

import * as assert from "assert";
import * as vscode from "vscode";
import {
  QuickFixProvider,
  fileCache,
  getCacheKey,
  computeHash,
  CacheEntry,
} from "../extension";
import { before } from "mocha";

suite("Quick Fix Provider Tests: Fix All skips and offsets", () => {
  let provider: QuickFixProvider;
  let doc: vscode.TextDocument;

  before(async () => {
    provider = new QuickFixProvider();
    // Use a dummy file path that won't interfere with real files
    doc = await vscode.workspace.openTextDocument({
      language: "python",
      content: "def unused_fn():\n    pass\n",
    });
  });


  test("Should skip Fix All without a combined plan", async () => {
    const overlapDoc = await vscode.workspace.openTextDocument({
      language: "python",
      content: "def outer():\n  def inner():\n    pass\n  return 1\n",
    });

    const hash = computeHash(overlapDoc.getText());
    const cacheKey = getCacheKey(overlapDoc.uri.fsPath);

    const mockFindings = [
      {
        rule_id: "unused-function",
        line_number: 1,
        message: "Unused function",
        fix: {
          start_byte: 0,
          end_byte: 40,
          replacement: "",
        },
      },
      {
        rule_id: "unused-function",
        line_number: 2,
        message: "Unused function",
        fix: {
          start_byte: 10,
          end_byte: 20,
          replacement: "",
        },
      },
    ];

    const entry: CacheEntry = {
      hash: hash,
      diagnostics: [],
      findings: mockFindings as any,
      timestamp: Date.now(),
    };

    fileCache.set(cacheKey, [entry]);

    const diagnosticOuter = new vscode.Diagnostic(
      new vscode.Range(0, 0, 0, 9),
      "'outer' is defined but never used",
      vscode.DiagnosticSeverity.Warning
    );
    diagnosticOuter.source = "CytoScnPy";
    diagnosticOuter.code = "unused-function";

    const diagnosticInner = new vscode.Diagnostic(
      new vscode.Range(1, 2, 1, 11),
      "'inner' is defined but never used",
      vscode.DiagnosticSeverity.Warning
    );
    diagnosticInner.source = "CytoScnPy";
    diagnosticInner.code = "unused-function";

    const diagnosticCollection =
      vscode.languages.createDiagnosticCollection("cytoscnpy-test");
    diagnosticCollection.set(overlapDoc.uri, [diagnosticOuter, diagnosticInner]);

    const context: vscode.CodeActionContext = {
      diagnostics: [diagnosticOuter, diagnosticInner],
      triggerKind: vscode.CodeActionTriggerKind.Invoke,
      only: undefined,
    };

    const actions = provider.provideCodeActions(
      overlapDoc,
      diagnosticOuter.range,
      context,
      new vscode.CancellationTokenSource().token
    );

    const fixAllAction = actions.find(
      (a) => a.title === "Remove all unused functions in this file"
    );
    assert.ok(
      !fixAllAction,
      "Fix All must not stitch single fixes together without a verified plan",
    );

    diagnosticCollection.dispose();
  });

  test("Should skip Fix All when its plan covers a suppressed finding", async () => {
    const doc = await vscode.workspace.openTextDocument({
      language: "python",
      content: "import os\nimport sys\nimport re  # noqa: unused-import\n",
    });
    const plan = [
      { start_byte: 0, end_byte: 10, replacement: "" },
      { start_byte: 10, end_byte: 21, replacement: "" },
      { start_byte: 21, end_byte: 52, replacement: "" },
    ];
    const finding = (line: number, start: number, end: number) => ({
      rule_id: "unused-import",
      line_number: line,
      message: "Unused import",
      fix: { start_byte: start, end_byte: end, replacement: "", rule_fix_all: plan },
    });
    fileCache.set(getCacheKey(doc.uri.fsPath), [
      {
        hash: computeHash(doc.getText()),
        diagnostics: [],
        findings: [finding(1, 0, 10), finding(2, 10, 21), finding(3, 21, 52)] as any,
        timestamp: Date.now(),
      },
    ]);

    // Line 3 is suppressed, so only two diagnostics are shown.
    const diagnostics = [
      new vscode.Diagnostic(new vscode.Range(0, 7, 0, 9), "'os' is imported but never used"),
      new vscode.Diagnostic(new vscode.Range(1, 7, 1, 10), "'sys' is imported but never used"),
    ];
    for (const diagnostic of diagnostics) {
      diagnostic.source = "CytoScnPy";
      diagnostic.code = "unused-import";
    }
    const collection = vscode.languages.createDiagnosticCollection("cytoscnpy-test");
    collection.set(doc.uri, diagnostics);

    const actions = provider.provideCodeActions(
      doc,
      diagnostics[0].range,
      { diagnostics, triggerKind: vscode.CodeActionTriggerKind.Invoke, only: undefined },
      new vscode.CancellationTokenSource().token,
    );

    assert.ok(
      !actions.some((a) => a.title === "Remove all unused imports in this file"),
      "Fix All must not remove a finding hidden by # noqa",
    );
    assert.ok(actions.some((a) => a.title === "Remove unused import 'os'"));
    collection.dispose();
  });

  test("Should map UTF-8 fix byte offsets correctly for remove action", async () => {
    const unicodeDoc = await vscode.workspace.openTextDocument({
      language: "python",
      content: "emoji = '😀'\nimport os\nprint('hello')\n",
    });

    const hash = computeHash(unicodeDoc.getText());
    const cacheKey = getCacheKey(unicodeDoc.uri.fsPath);
    const text = unicodeDoc.getText();
    const targetText = "import os\n";
    const utf16Start = text.indexOf(targetText);
    const utf16End = utf16Start + targetText.length;
    const startByte = Buffer.byteLength(text.slice(0, utf16Start), "utf8");
    const endByte = Buffer.byteLength(text.slice(0, utf16End), "utf8");

    const mockFinding = {
      rule_id: "unused-import",
      line_number: 2,
      message: "Unused import",
      fix: {
        start_byte: startByte,
        end_byte: endByte,
        replacement: "",
      },
    };

    const entry: CacheEntry = {
      hash: hash,
      diagnostics: [],
      findings: [mockFinding as any],
      timestamp: Date.now(),
    };
    fileCache.set(cacheKey, [entry]);

    const diagnostic = new vscode.Diagnostic(
      new vscode.Range(1, 7, 1, 9),
      "'os' is imported but never used",
      vscode.DiagnosticSeverity.Warning
    );
    diagnostic.source = "CytoScnPy";
    diagnostic.code = "unused-import";

    const context: vscode.CodeActionContext = {
      diagnostics: [diagnostic],
      triggerKind: vscode.CodeActionTriggerKind.Invoke,
      only: undefined,
    };

    const actions = provider.provideCodeActions(
      unicodeDoc,
      diagnostic.range,
      context,
      new vscode.CancellationTokenSource().token
    );
    const removeAction = actions.find(
      (a) => a.title === "Remove unused import 'os'"
    );
    assert.ok(removeAction, "Should have Remove import action");

    const edits = removeAction!.edit!.get(unicodeDoc.uri);
    assert.strictEqual(edits.length, 1, "Should emit one remove edit");
    assert.strictEqual(
      unicodeDoc.offsetAt(edits[0].range.start),
      utf16Start,
      "Edit start should match UTF-16 position of import line"
    );
    assert.strictEqual(
      unicodeDoc.offsetAt(edits[0].range.end),
      utf16End,
      "Edit end should match UTF-16 position of import line"
    );
  });
});

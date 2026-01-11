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

suite("Quick Fix Provider Tests", () => {
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

  test("Should provide precise fix when cache matches", async () => {
    const hash = computeHash(doc.getText());
    const cacheKey = getCacheKey(doc.uri.fsPath);

    // Mock a finding with a fix
    const mockFinding = {
      rule_id: "unused-function",
      line_number: 1,
      message: "Unused function",
      fix: {
        start_byte: 0,
        end_byte: 17, // "def unused_fn():\n"
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
      new vscode.Range(0, 4, 0, 13),
      "Unused function",
      vscode.DiagnosticSeverity.Warning
    );
    diagnostic.source = "CytoScnPy";
    diagnostic.code = "unused-function";

    const context: vscode.CodeActionContext = {
      diagnostics: [diagnostic],
      triggerKind: vscode.CodeActionTriggerKind.Invoke,
      only: undefined,
    };

    const actions = provider.provideCodeActions(
      doc,
      diagnostic.range,
      context,
      new vscode.CancellationTokenSource().token
    );

    assert.strictEqual(
      actions.length,
      2,
      "Should have 2 actions (Remove + Suppress)"
    );

    const removeTitle = `Remove ${mockFinding.rule_id.replace("unused-", "")}`;
    const removeAction = actions.find((a) => a.title === removeTitle);
    assert.ok(removeAction, "Should have remove action");

    assert.strictEqual(
      removeAction!.title,
      "Remove function",
      "Action title should be precise"
    );
    assert.ok(removeAction!.edit, "Action should have an edit");

    const edit = removeAction!.edit!;
    const entries = edit.get(doc.uri);
    assert.strictEqual(entries.length, 1, "Should have 1 edit entry");
    assert.strictEqual(
      doc.offsetAt(entries[0].range.start),
      0,
      "Start byte should match"
    );
    assert.strictEqual(
      doc.offsetAt(entries[0].range.end),
      17,
      "End byte should match"
    );
  });

  test("Should provide ONLY suppression fixes when cache missing", async () => {
    fileCache.clear();

    const diagnostic = new vscode.Diagnostic(
      new vscode.Range(0, 4, 0, 13),
      "Unused function",
      vscode.DiagnosticSeverity.Warning
    );
    diagnostic.source = "CytoScnPy";
    diagnostic.code = "unused-function";

    const context: vscode.CodeActionContext = {
      diagnostics: [diagnostic],
      triggerKind: vscode.CodeActionTriggerKind.Invoke,
      only: undefined,
    };

    const actions = provider.provideCodeActions(
      doc,
      diagnostic.range,
      context,
      new vscode.CancellationTokenSource().token
    );

    assert.strictEqual(
      actions.length,
      1,
      "Should provide 1 action (Suppression only) when cache is missing"
    );
    assert.ok(
      actions.some((a) => a.title.includes("Suppress")),
      "Should contain suppression actions"
    );
    assert.ok(
      !actions.some((a) => a.title.includes("Remove")),
      "Should NOT contain remove action"
    );
  });

  test("Should provide suppression items for security findings", async () => {
    const diagnostic = new vscode.Diagnostic(
      new vscode.Range(0, 0, 0, 10),
      "Potential SQL injection (dynamic raw SQL)",
      vscode.DiagnosticSeverity.Error
    );
    diagnostic.source = "CytoScnPy";
    diagnostic.code = "CSP-D102";

    const context: vscode.CodeActionContext = {
      diagnostics: [diagnostic],
      triggerKind: vscode.CodeActionTriggerKind.Invoke,
      only: undefined,
    };

    const actions = provider.provideCodeActions(
      doc,
      diagnostic.range,
      context,
      new vscode.CancellationTokenSource().token
    );

    assert.strictEqual(
      actions.length,
      1,
      "Should provide suppression action (rule only)"
    );

    // Verify suppression action
    const suppressAction = actions.find((a) => a.title.includes("# noqa: CSP"));
    assert.ok(suppressAction, "Should have CSP suppression action");
    const edit = suppressAction!.edit!.get(doc.uri);
    assert.strictEqual(edit.length, 1);
    assert.ok(
      edit[0].newText.includes("# noqa: CSP"),
      "Should append noqa: CSP"
    );
  });

  test("Should append to existing noqa comment", async () => {
    // Create a doc with existing noqa
    const docWithNoqa = await vscode.workspace.openTextDocument({
      language: "python",
      content: "x = 1  # noqa: E501",
    });

    const diagnostic = new vscode.Diagnostic(
      new vscode.Range(0, 0, 0, 5),
      "Unused variable",
      vscode.DiagnosticSeverity.Warning
    );
    diagnostic.source = "CytoScnPy";
    diagnostic.code = "unused-variable";

    const context: vscode.CodeActionContext = {
      diagnostics: [diagnostic],
      triggerKind: vscode.CodeActionTriggerKind.Invoke,
      only: undefined,
    };

    const actions = provider.provideCodeActions(
      docWithNoqa,
      diagnostic.range,
      context,
      new vscode.CancellationTokenSource().token
    );

    const suppressAction = actions.find((a) => a.title.includes("# noqa: CSP"));
    assert.ok(suppressAction, "Should identify action");

    const edit = suppressAction!.edit!.get(docWithNoqa.uri);
    assert.strictEqual(edit.length, 1);
    // It should append CSP to existing comment
    assert.ok(
      edit[0].newText.includes("# noqa: E501, CSP"),
      "Should match merged comment with CSP"
    );
  });
});

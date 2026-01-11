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

  test("Should provide Remove action for unused-method", async () => {
    const methodDoc = await vscode.workspace.openTextDocument({
      language: "python",
      content: "class Foo:\n    def unused_method(self):\n        pass\n",
    });

    const hash = computeHash(methodDoc.getText());
    const cacheKey = getCacheKey(methodDoc.uri.fsPath);

    const mockFinding = {
      rule_id: "unused-method",
      line_number: 2,
      message: "Unused method",
      fix: {
        start_byte: 12, // Start of "def"
        end_byte: 45,
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
      new vscode.Range(1, 4, 1, 17),
      "Unused method",
      vscode.DiagnosticSeverity.Warning
    );
    diagnostic.source = "CytoScnPy";
    diagnostic.code = "unused-method";

    const context: vscode.CodeActionContext = {
      diagnostics: [diagnostic],
      triggerKind: vscode.CodeActionTriggerKind.Invoke,
      only: undefined,
    };

    const actions = provider.provideCodeActions(
      methodDoc,
      diagnostic.range,
      context,
      new vscode.CancellationTokenSource().token
    );

    assert.strictEqual(
      actions.length,
      2,
      "Should have 2 actions (Remove + Suppress)"
    );
    const removeAction = actions.find((a) => a.title === "Remove method");
    assert.ok(removeAction, "Should have Remove method action");
  });

  test("Should provide Remove action for unused-class", async () => {
    const classDoc = await vscode.workspace.openTextDocument({
      language: "python",
      content: "class UnusedClass:\n    pass\n",
    });

    const hash = computeHash(classDoc.getText());
    const cacheKey = getCacheKey(classDoc.uri.fsPath);

    const mockFinding = {
      rule_id: "unused-class",
      line_number: 1,
      message: "Unused class",
      fix: {
        start_byte: 0,
        end_byte: 24,
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
      new vscode.Range(0, 6, 0, 17),
      "Unused class",
      vscode.DiagnosticSeverity.Warning
    );
    diagnostic.source = "CytoScnPy";
    diagnostic.code = "unused-class";

    const context: vscode.CodeActionContext = {
      diagnostics: [diagnostic],
      triggerKind: vscode.CodeActionTriggerKind.Invoke,
      only: undefined,
    };

    const actions = provider.provideCodeActions(
      classDoc,
      diagnostic.range,
      context,
      new vscode.CancellationTokenSource().token
    );

    assert.strictEqual(
      actions.length,
      2,
      "Should have 2 actions (Remove + Suppress)"
    );
    const removeAction = actions.find((a) => a.title === "Remove class");
    assert.ok(removeAction, "Should have Remove class action");
  });

  test("Should provide Remove action for unused-import", async () => {
    const importDoc = await vscode.workspace.openTextDocument({
      language: "python",
      content: "import os\n\nprint('hello')\n",
    });

    const hash = computeHash(importDoc.getText());
    const cacheKey = getCacheKey(importDoc.uri.fsPath);

    const mockFinding = {
      rule_id: "unused-import",
      line_number: 1,
      message: "Unused import",
      fix: {
        start_byte: 0,
        end_byte: 10, // "import os\n"
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
      new vscode.Range(0, 7, 0, 9),
      "Unused import",
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
      importDoc,
      diagnostic.range,
      context,
      new vscode.CancellationTokenSource().token
    );

    assert.strictEqual(
      actions.length,
      2,
      "Should have 2 actions (Remove + Suppress)"
    );
    const removeAction = actions.find((a) => a.title === "Remove import");
    assert.ok(removeAction, "Should have Remove import action");
  });

  test("Should provide Remove action for unused-variable", async () => {
    const varDoc = await vscode.workspace.openTextDocument({
      language: "python",
      content: "x = 42\nprint('hello')\n",
    });

    const hash = computeHash(varDoc.getText());
    const cacheKey = getCacheKey(varDoc.uri.fsPath);

    const mockFinding = {
      rule_id: "unused-variable",
      line_number: 1,
      message: "Unused variable",
      fix: {
        start_byte: 0,
        end_byte: 7, // "x = 42\n"
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
      new vscode.Range(0, 0, 0, 1),
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
      varDoc,
      diagnostic.range,
      context,
      new vscode.CancellationTokenSource().token
    );

    assert.strictEqual(
      actions.length,
      2,
      "Should have 2 actions (Remove + Suppress)"
    );
    const removeAction = actions.find((a) => a.title === "Remove variable");
    assert.ok(removeAction, "Should have Remove variable action");
  });
});

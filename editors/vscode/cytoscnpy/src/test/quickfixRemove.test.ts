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

suite("Quick Fix Provider Tests: Remove actions", () => {
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
      "Method 'unused_method' is defined but never used",
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
    const removeAction = actions.find(
      (a) => a.title === "Remove unused method 'unused_method'"
    );
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
      "Class 'UnusedClass' is defined but never used",
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
    const removeAction = actions.find(
      (a) => a.title === "Remove unused class 'UnusedClass'"
    );
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
    const removeAction = actions.find(
      (a) => a.title === "Remove unused import 'os'"
    );
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
      "Variable 'x' is assigned but never used",
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
    const removeAction = actions.find(
      (a) => a.title === "Remove unused variable 'x'"
    );
    assert.ok(removeAction, "Should have Remove variable action");
  });
});

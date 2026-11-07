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

suite("Quick Fix Provider Tests: Fix All", () => {
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


  test("Should provide Fix All action for multiple unused imports", async () => {
    const importDoc = await vscode.workspace.openTextDocument({
      language: "python",
      content: "import os\nimport sys\nprint('hello')\n",
    });

    const hash = computeHash(importDoc.getText());
    const cacheKey = getCacheKey(importDoc.uri.fsPath);

    const mockFindings = [
      {
        rule_id: "unused-import",
        line_number: 1,
        message: "Unused import",
        fix: {
          start_byte: 0,
          end_byte: 10, // "import os\n"
          replacement: "",
          rule_fix_all: [
            { start_byte: 0, end_byte: 10, replacement: "" },
            { start_byte: 10, end_byte: 21, replacement: "" },
          ],
        },
      },
      {
        rule_id: "unused-import",
        line_number: 2,
        message: "Unused import",
        fix: {
          start_byte: 10,
          end_byte: 21, // "import sys\n"
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

    const diagnosticOne = new vscode.Diagnostic(
      new vscode.Range(0, 7, 0, 9),
      "'os' is imported but never used",
      vscode.DiagnosticSeverity.Warning
    );
    diagnosticOne.source = "CytoScnPy";
    diagnosticOne.code = "unused-import";

    const diagnosticTwo = new vscode.Diagnostic(
      new vscode.Range(1, 7, 1, 10),
      "'sys' is imported but never used",
      vscode.DiagnosticSeverity.Warning
    );
    diagnosticTwo.source = "CytoScnPy";
    diagnosticTwo.code = "unused-import";

    const diagnosticCollection =
      vscode.languages.createDiagnosticCollection("cytoscnpy-test");
    diagnosticCollection.set(importDoc.uri, [diagnosticOne, diagnosticTwo]);

    const context: vscode.CodeActionContext = {
      diagnostics: [diagnosticOne, diagnosticTwo],
      triggerKind: vscode.CodeActionTriggerKind.Invoke,
      only: undefined,
    };

    const actions = provider.provideCodeActions(
      importDoc,
      diagnosticOne.range,
      context,
      new vscode.CancellationTokenSource().token
    );

    const fixAllAction = actions.find(
      (a) => a.title === "Remove all unused imports in this file"
    );
    assert.ok(fixAllAction, "Should have Fix All action");
    assert.strictEqual(
      fixAllAction!.diagnostics?.length,
      2,
      "Fix All should include both diagnostics"
    );

    const edits = fixAllAction!.edit!.get(importDoc.uri);
    assert.strictEqual(edits.length, 2, "Fix All should edit both imports");

    diagnosticCollection.dispose();
  });

  test("Should provide Fix All dead code action across unused rules", async () => {
    const deadCodeDoc = await vscode.workspace.openTextDocument({
      language: "python",
      content: "import os\nx = 1\nprint('hello')\n",
    });

    const hash = computeHash(deadCodeDoc.getText());
    const cacheKey = getCacheKey(deadCodeDoc.uri.fsPath);

    const mockFindings = [
      {
        rule_id: "unused-import",
        line_number: 1,
        message: "Unused import",
        fix: {
          start_byte: 0,
          end_byte: 10, // "import os\n"
          replacement: "",
          file_fix_all: [
            { start_byte: 0, end_byte: 10, replacement: "" },
            { start_byte: 10, end_byte: 11, replacement: "_" },
          ],
        },
      },
      {
        rule_id: "unused-variable",
        line_number: 2,
        message: "Unused variable",
        fix: {
          start_byte: 10,
          end_byte: 16, // "x = 1\n"
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

    const diagnosticImport = new vscode.Diagnostic(
      new vscode.Range(0, 7, 0, 9),
      "'os' is imported but never used",
      vscode.DiagnosticSeverity.Warning
    );
    diagnosticImport.source = "CytoScnPy";
    diagnosticImport.code = "unused-import";

    const diagnosticVariable = new vscode.Diagnostic(
      new vscode.Range(1, 0, 1, 1),
      "Variable 'x' is assigned but never used",
      vscode.DiagnosticSeverity.Warning
    );
    diagnosticVariable.source = "CytoScnPy";
    diagnosticVariable.code = "unused-variable";

    const diagnosticCollection =
      vscode.languages.createDiagnosticCollection("cytoscnpy-test");
    diagnosticCollection.set(deadCodeDoc.uri, [
      diagnosticImport,
      diagnosticVariable,
    ]);

    const context: vscode.CodeActionContext = {
      diagnostics: [diagnosticImport, diagnosticVariable],
      triggerKind: vscode.CodeActionTriggerKind.Invoke,
      only: undefined,
    };

    const actions = provider.provideCodeActions(
      deadCodeDoc,
      diagnosticImport.range,
      context,
      new vscode.CancellationTokenSource().token
    );

    const fixAllAction = actions.find(
      (a) => a.title === "Remove all dead code in this file"
    );
    assert.ok(fixAllAction, "Should have Fix All dead code action");
    assert.strictEqual(
      fixAllAction!.diagnostics?.length,
      2,
      "Fix All dead code should include both diagnostics"
    );

    const edits = fixAllAction!.edit!.get(deadCodeDoc.uri);
    assert.strictEqual(edits.length, 2, "Fix All dead code should edit both");
    assert.strictEqual(
      edits[1].newText,
      "_",
      "Fix All should apply the combined plan, not each finding's own fix",
    );

    diagnosticCollection.dispose();
  });
});

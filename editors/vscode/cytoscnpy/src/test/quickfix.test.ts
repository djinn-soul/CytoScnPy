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
      "'unused_fn' is defined but never used",
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

    const removeTitle = "Remove unused function 'unused_fn'";
    const removeAction = actions.find((a) => a.title === removeTitle);
    assert.ok(removeAction, "Should have remove action");

    assert.strictEqual(
      removeAction!.title,
      "Remove unused function 'unused_fn'",
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
    const suppressAction = actions.find((a) =>
      a.title.includes("# noqa: CSP-D102")
    );
    assert.ok(suppressAction, "Should have CSP suppression action");
    const edit = suppressAction!.edit!.get(doc.uri);
    assert.strictEqual(edit.length, 1);
    assert.ok(
      edit[0].newText.includes("# noqa: CSP-D102"),
      "Should append noqa: CSP-D102"
    );
  });

  for (const [content, code, expected] of [
    ["x = 1  # noqa: E501", "unused-variable", "x = 1  # noqa: E501, unused-variable"],
    // The code goes before an explanation so it is still read as a code.
    ["x = 1  # noqa: E501 long URL", "CSP-D101", "x = 1  # noqa: E501, CSP-D101 long URL"],
    ["x = 1  # noqa: because reasons", "unused-variable", "x = 1  # noqa: unused-variable, because reasons"],
    // Codes the CLI does not honor fall back to suppressing everything.
    ["x = 1  # noqa: E501", "unused-dependency", "x = 1  # noqa: E501, CSP"],
  ]) {
    test(`Should append ${code} to existing noqa comment: ${content}`, async () => {
      const docWithNoqa = await vscode.workspace.openTextDocument({
        language: "python",
        content,
      });

      const diagnostic = new vscode.Diagnostic(
        new vscode.Range(0, 0, 0, 5),
        "Unused variable",
        vscode.DiagnosticSeverity.Warning
      );
      diagnostic.source = "CytoScnPy";
      diagnostic.code = code;

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

      const suppressAction = actions.find((a) => a.title.startsWith("Suppress with # noqa:"));
      assert.ok(suppressAction, "Should identify action");
      assert.strictEqual(suppressAction!.edit!.get(docWithNoqa.uri).length, 1);

      assert.ok(await vscode.workspace.applyEdit(suppressAction!.edit!));
      assert.strictEqual(docWithNoqa.lineAt(0).text, expected);
    });
  }

  test("Should not add suppression when pragma no cytoscnpy exists", async () => {
    const pragmaDoc = await vscode.workspace.openTextDocument({
      language: "python",
      content: "value = user_input  # pragma: no cytoscnpy",
    });

    const diagnostic = new vscode.Diagnostic(
      new vscode.Range(0, 0, 0, 5),
      "Potential issue",
      vscode.DiagnosticSeverity.Warning
    );
    diagnostic.source = "CytoScnPy";
    diagnostic.code = "CSP-D402";

    const context: vscode.CodeActionContext = {
      diagnostics: [diagnostic],
      triggerKind: vscode.CodeActionTriggerKind.Invoke,
      only: undefined,
    };

    const actions = provider.provideCodeActions(
      pragmaDoc,
      diagnostic.range,
      context,
      new vscode.CancellationTokenSource().token
    );

    assert.ok(
      !actions.some((a) => a.title.includes("Suppress with # noqa")),
      "Should not offer suppression action when pragma suppression already exists"
    );
  });
});

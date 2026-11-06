import * as assert from "assert";
import * as vscode from "vscode";
import { QuickFixProvider, fileCache, computeHash } from "../extension";
import { transformRawResult } from "../analyzer";
import { documentCacheKey } from "../extensionCache";
import { findingsToDiagnostics } from "../extensionDiagnostics";
import { findingsForDocument, notebookCellOf } from "../notebookCells";

suite("Notebook cells", () => {
  async function openNotebook(): Promise<vscode.NotebookDocument> {
    const data = new vscode.NotebookData([
      new vscode.NotebookCellData(vscode.NotebookCellKind.Markup, "# Title", "markdown"),
      new vscode.NotebookCellData(vscode.NotebookCellKind.Code, "import os, sys\nprint(sys)", "python"),
      new vscode.NotebookCellData(
        vscode.NotebookCellKind.Code,
        "import re\nimport json\ndef f():\n    x = 1\n    return 2",
        "python",
      ),
    ]);
    return vscode.workspace.openNotebookDocument("jupyter-notebook", data);
  }

  // As the CLI reports them with `--ipynb-cells`: lines and fixes per cell.
  const findings = [
    {
      rule_id: "unused-import",
      line_number: 1,
      col: 7,
      message: "'os' is imported but never used",
      category: "Dead Code",
      severity: "warning",
      cell_number: 1,
      fix: { start_byte: 7, end_byte: 11, replacement: "" },
    },
    {
      rule_id: "unused-import",
      line_number: 1,
      col: 7,
      message: "'re' is imported but never used",
      category: "Dead Code",
      severity: "warning",
      cell_number: 2,
      fix: {
        start_byte: 0,
        end_byte: 10,
        replacement: "",
        rule_fix_all: [
          { start_byte: 0, end_byte: 10, replacement: "" },
          { start_byte: 10, end_byte: 22, replacement: "" },
        ],
      },
    },
    {
      rule_id: "unused-import",
      line_number: 2,
      col: 7,
      message: "'json' is imported but never used",
      category: "Dead Code",
      severity: "warning",
      cell_number: 2,
      fix: { start_byte: 10, end_byte: 22, replacement: "" },
    },
  ] as any[];

  test("each cell shows only its own findings, at cell lines", async () => {
    const notebook = await openNotebook();
    const [, first, second] = notebook.getCells();
    assert.strictEqual(notebookCellOf(first.document)?.index, 1);
    assert.deepStrictEqual(
      findingsForDocument(first.document, findings).map((f) => f.message),
      ["'os' is imported but never used"],
    );
    const diagnostics = findingsToDiagnostics(second.document, findings);
    assert.deepStrictEqual(
      diagnostics.map((d) => d.range.start.line),
      [0, 1],
    );
    // Cells of one notebook must not share a cache entry.
    assert.notStrictEqual(
      documentCacheKey(first.document),
      documentCacheKey(second.document),
    );
  });

  test("quick fixes edit the cell, and Fix All stays within it", async () => {
    const notebook = await openNotebook();
    const cell = notebook.cellAt(2).document;
    fileCache.set(documentCacheKey(cell), [
      {
        hash: computeHash(cell.getText()),
        diagnostics: [],
        findings: findingsForDocument(cell, findings),
        timestamp: Date.now(),
      },
    ]);
    const diagnostics = findingsToDiagnostics(cell, findings);
    const collection = vscode.languages.createDiagnosticCollection("cytoscnpy-notebook-test");
    collection.set(cell.uri, diagnostics);

    const actions = new QuickFixProvider().provideCodeActions(
      cell,
      diagnostics[0].range,
      { diagnostics, triggerKind: vscode.CodeActionTriggerKind.Invoke, only: undefined },
      new vscode.CancellationTokenSource().token,
    );

    const single = actions.find((a) => a.title === "Remove unused import 're'");
    assert.ok(single, "Should offer the cell's own fix");
    assert.deepStrictEqual(
      single!.edit!.entries().map(([uri]) => uri.toString()),
      [cell.uri.toString()],
    );
    const fixAll = actions.find((a) => a.title === "Remove all unused imports in this cell");
    assert.ok(fixAll, "Should offer a cell-scoped Fix All");
    const applied = await vscode.workspace.applyEdit(fixAll!.edit!);
    assert.ok(applied);
    assert.strictEqual(cell.getText(), "def f():\n    x = 1\n    return 2");
    assert.strictEqual(
      notebook.cellAt(1).document.getText(),
      "import os, sys\nprint(sys)",
      "Other cells are untouched",
    );
    collection.dispose();
  });

  test("security and taint findings land in their cells", async () => {
    const notebook = await openNotebook();
    const result = transformRawResult({
      danger: [
        {
          file: "n.ipynb",
          line: 2,
          col: 0,
          message: "Avoid using eval",
          rule_id: "CSP-D001",
          severity: "CRITICAL",
          cell_number: 2,
        },
      ],
      taint_findings: [
        {
          rule_id: "CSP-D001",
          source: "input",
          source_line: 1,
          source_cell_number: 1,
          sink: "eval",
          sink_line: 2,
          sink_col: 0,
          cell_number: 2,
          vuln_type: "CodeInjection",
          severity: "CRITICAL",
          file: "n.ipynb",
          remediation: "Validate input.",
        },
      ],
    } as any);
    const [danger, taint] = result.findings;
    assert.strictEqual(danger.cell_number, 2);
    assert.strictEqual(taint.cell_number, 2);
    assert.ok(
      taint.message.includes("(cell 1, line 1)"),
      "a source in another cell names that cell",
    );
    const cell = notebook.cellAt(2).document;
    assert.deepStrictEqual(
      findingsToDiagnostics(cell, result.findings).map((d) => d.range.start.line),
      [1, 1],
    );
    assert.deepStrictEqual(findingsToDiagnostics(notebook.cellAt(1).document, result.findings), []);
  });
});

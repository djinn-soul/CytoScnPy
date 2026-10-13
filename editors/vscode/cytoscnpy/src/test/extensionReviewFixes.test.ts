import * as assert from "assert";
import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import * as vscode from "vscode";
import { effectiveAnalysisMode, periodicScanDue } from "../analysisPolicy";
import { findingResolver } from "../findingResolver";
import { runCytoScnPyAnalysis, runWorkspaceAnalysis, transformRawResult } from "../analyzer";
import { CytoScnPyConfig, CytoScnPyFinding } from "../analyzerTypes";
import { bundledExecutableName } from "../configuration";
import {
  cacheDocumentAnalysis,
  clearDocumentAnalysis,
  fileCache,
  getCacheKey,
  restoreDocumentAnalysis,
} from "../extensionCache";
import { cytoscnpyDiagnostics, setDiagnosticsBatched } from "../extensionUi";
import { findingsToDiagnostics } from "../extensionDiagnostics";
import { emptyWorkspaceResults, mergeWorkspaceResult } from "../workspaceResults";

/** Writes a stand-in analyzer executable that prints `output` as JSON. */
function fakeAnalyzer(directory: string, output: unknown): string {
  const jsonPath = path.join(directory, "output.json");
  fs.writeFileSync(jsonPath, JSON.stringify(output));
  const scriptPath = path.join(directory, "fake-cytoscnpy");
  fs.writeFileSync(scriptPath, `#!/bin/sh\ncat '${jsonPath}'\nexit 1\n`);
  fs.chmodSync(scriptPath, 0o755);
  return scriptPath;
}

/** Writes a stand-in analyzer that never finishes on its own. */
function hangingAnalyzer(directory: string): string {
  const scriptPath = path.join(directory, "hanging-cytoscnpy");
  fs.writeFileSync(scriptPath, "#!/bin/sh\nexec sleep 30\n");
  fs.chmodSync(scriptPath, 0o755);
  return scriptPath;
}

function baseConfig(binary: string): CytoScnPyConfig {
  return {
    path: binary,
    analysisMode: "file",
    enableSecretsScan: false,
    enableDangerScan: false,
    enableQualityScan: false,
    enableCloneScan: false,
  };
}

suite("Extension review fixes", () => {
  test("files outside any workspace folder fall back to file mode", () => {
    assert.strictEqual(effectiveAnalysisMode("workspace", true), "workspace");
    assert.strictEqual(effectiveAnalysisMode("workspace", false), "file");
    assert.strictEqual(effectiveAnalysisMode("file", true), "file");
    assert.strictEqual(effectiveAnalysisMode("file", false), "file");
  });

  test("undoing back to analyzed content restores its results", async () => {
    const directory = fs.mkdtempSync(path.join(os.tmpdir(), "cytoscnpy-undo-"));
    const file = path.join(directory, "undo.py");
    fs.writeFileSync(file, "import os\n");
    const document = await vscode.workspace.openTextDocument(vscode.Uri.file(file));
    const diagnostic = new vscode.Diagnostic(new vscode.Range(0, 0, 0, 9), "unused");

    cacheDocumentAnalysis(document, [diagnostic], []);
    clearDocumentAnalysis(document);
    assert.strictEqual(cytoscnpyDiagnostics.get(document.uri)?.length ?? 0, 0);

    assert.ok(restoreDocumentAnalysis(document));
    assert.strictEqual(cytoscnpyDiagnostics.get(document.uri)?.length, 1);

    // Content that was never analyzed, as after saving new text, restores nothing.
    fileCache.get(getCacheKey(file))!.forEach((entry) => (entry.hash = "other"));
    clearDocumentAnalysis(document);
    assert.ok(!restoreDocumentAnalysis(document));
    assert.strictEqual(cytoscnpyDiagnostics.get(document.uri)?.length ?? 0, 0);
  });

  test("workspace diagnostics are sent in batches VS Code fully syncs", () => {
    const diagnostic = new vscode.Diagnostic(new vscode.Range(0, 0, 0, 1), "x");
    const entries: [vscode.Uri, vscode.Diagnostic[]][] = [];
    for (let i = 0; i < 3000; i++) {
      entries.push([vscode.Uri.file(`/ws/f${i}.py`), [diagnostic]]);
    }
    entries.push([vscode.Uri.file("/ws/big.py"), new Array(900).fill(diagnostic)]);

    const batches: (readonly [vscode.Uri, readonly vscode.Diagnostic[] | undefined][])[] = [];
    setDiagnosticsBatched(entries, {
      set: ((batch: readonly [vscode.Uri, readonly vscode.Diagnostic[] | undefined][]) => {
        batches.push(batch);
      }) as vscode.DiagnosticCollection["set"],
    });

    // Every file is sent once, and only a file that is large on its own may
    // push a batch past the budget (VS Code drops files beyond ~1,100).
    assert.strictEqual(batches.flat().length, entries.length);
    assert.ok(batches.length < 20, `${batches.length} batches`);
    for (const batch of batches) {
      const total = batch.reduce((sum, [, list]) => sum + (list?.length ?? 0), 0);
      assert.ok(total <= 500 || batch.length === 1, `batch of ${total}`);
    }
  });

  test("finding resolution matches exact, same-line and nearby findings", () => {
    const finding = (line: number, message: string): CytoScnPyFinding => ({
      file_path: "a.py",
      line_number: line,
      message,
      rule_id: "unused-import",
      category: "Dead Code",
      severity: "warning",
    });
    const findings = [finding(1, "a"), finding(1, "b"), finding(5, "c"), finding(20, "d"), finding(21, "e")];
    const resolve = findingResolver(findings);
    assert.strictEqual(resolve({ ruleId: "unused-import", line: 1, message: "b [unused-import]" }), findings[1]);
    assert.strictEqual(resolve({ ruleId: "unused-import", line: 6, message: "stale" }), findings[2]);
    // Two candidates nearby is ambiguous, so nothing is applied.
    assert.strictEqual(resolve({ ruleId: "unused-import", line: 1, message: "stale" }), undefined);
    assert.strictEqual(resolve({ ruleId: "unused-import", line: 22, message: "stale" }), undefined);
    assert.strictEqual(resolve({ ruleId: "unused-class", line: 5, message: "c [unused-import]" }), undefined);
    assert.strictEqual(findingResolver(findings), resolve, "built once per findings array");
  });

  test("a hung analyzer is killed after the timeout", async function () {
    if (process.platform === "win32") {
      this.skip();
    }
    const directory = fs.mkdtempSync(path.join(os.tmpdir(), "cytoscnpy-hang-"));
    const started = Date.now();
    await assert.rejects(
      runCytoScnPyAnalysis(path.join(directory, "a.py"), baseConfig(hangingAnalyzer(directory)), {
        timeoutMs: 200,
      }),
      /timed out after/,
    );
    assert.ok(Date.now() - started < 5000);
  });

  test("an aborted workspace scan stops the analyzer", async function () {
    if (process.platform === "win32") {
      this.skip();
    }
    const directory = fs.mkdtempSync(path.join(os.tmpdir(), "cytoscnpy-abort-"));
    const abort = new AbortController();
    const scan = runWorkspaceAnalysis(directory, baseConfig(hangingAnalyzer(directory)), {
      signal: abort.signal,
    });
    setTimeout(() => abort.abort(), 100);
    await assert.rejects(scan, /cancelled/);
  });

  test("periodic scans run only for changed workspace-mode files", () => {
    assert.strictEqual(periodicScanDue(["file"], 2, 1), false);
    assert.strictEqual(periodicScanDue([], 2, 1), false);
    assert.strictEqual(periodicScanDue(["file", "workspace"], 2, 1), true);
    assert.strictEqual(periodicScanDue(["workspace"], 1, 1), false);
  });

  test("directory errors become scan errors and paths keep their case", async function () {
    if (process.platform === "win32") {
      this.skip();
    }
    const workspace = fs.mkdtempSync(path.join(os.tmpdir(), "cytoscnpy-ws-"));
    const source = path.join(workspace, "Mixed", "Case.py");
    fs.mkdirSync(path.dirname(source));
    fs.writeFileSync(source, "def unused():\n    pass\n");
    const config: CytoScnPyConfig = {
      path: fakeAnalyzer(workspace, {
        unused_functions: [{ file: source, line: 1, name: "unused" }],
        parse_errors: [
          { file: workspace, error: "Path traversal detected: link.py" },
          { file: source, error: "unexpected indent" },
        ],
      }),
      analysisMode: "workspace",
      enableSecretsScan: false,
      enableDangerScan: false,
      enableQualityScan: false,
      enableCloneScan: false,
    };

    const result = await runWorkspaceAnalysis(workspace, config);
    assert.deepStrictEqual([...result.findingsByFile.keys()], [source]);
    assert.deepStrictEqual([...result.parseErrorsByFile.keys()], [source]);
    assert.strictEqual(result.scanErrors.length, 1);
    assert.strictEqual(result.scanErrors[0].file, workspace);
  });

  test("merged results keep each file's real path for its diagnostic URI", () => {
    const source = "C:\\Users\\Dev\\Project\\Mixed\\Case.py";
    const merged = emptyWorkspaceResults();
    mergeWorkspaceResult(merged, {
      findingsByFile: new Map([[source, []]]),
      parseErrorsByFile: new Map(),
      scanErrors: [{ file: "C:\\Users\\Dev\\Project", line: 1, message: "x" }],
    });
    const key = getCacheKey(source);
    assert.ok(merged.findingsByFile.has(key));
    assert.strictEqual(merged.sourcePaths.get(key), source);
    assert.strictEqual(merged.scanErrors.length, 1);
  });

  test("bundled CLI reports workspace paths in their original case", async function () {
    const binary = path.join(__dirname, "..", "..", "bin", bundledExecutableName() ?? "");
    if (!bundledExecutableName() || !fs.existsSync(binary)) {
      this.skip(); // CI copies the release CLI into bin/ before running tests.
    }
    const workspace = fs.mkdtempSync(path.join(os.tmpdir(), "cytoscnpy-case-"));
    fs.mkdirSync(path.join(workspace, "Mixed"));
    fs.writeFileSync(path.join(workspace, "Mixed", "Case.py"), "def unused_case():\n    pass\n");
    const result = await runWorkspaceAnalysis(workspace, {
      path: binary,
      analysisMode: "workspace",
      enableSecretsScan: false,
      enableDangerScan: false,
      enableQualityScan: false,
      enableCloneScan: false,
    });
    const files = [...result.findingsByFile.keys()];
    assert.ok(
      files.some((file) => file.endsWith(path.join("Mixed", "Case.py"))),
      `expected original-case path in ${JSON.stringify(files)}`,
    );
  });

  test("findings on stale lines keep CytoScnPy source and code", async () => {
    const document = await vscode.workspace.openTextDocument({
      language: "python",
      content: "x = 1\n",
    });
    const [diagnostic] = findingsToDiagnostics(document, [
      {
        file_path: "app.py",
        line_number: 99,
        message: "'old' is defined but never used",
        rule_id: "unused-function",
        category: "Dead Code",
        severity: "warning",
      },
    ]);
    assert.strictEqual(diagnostic.code, "unused-function");
    assert.ok(diagnostic.source?.startsWith("CytoScnPy"));
    assert.deepStrictEqual(diagnostic.tags, [vscode.DiagnosticTag.Unnecessary]);
  });

  test("findings without a severity use the category default", () => {
    const result = transformRawResult({
      secrets: [{ file: "app.py", line: 1, name: "token" }],
      quality: [{ file: "app.py", line: 2, name: "long", severity: "CRITICAL" }],
    });
    assert.deepStrictEqual(
      result.findings.map((finding) => finding.severity),
      ["error", "error"],
    );
  });
});

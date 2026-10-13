import * as assert from "assert";
import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import * as vscode from "vscode";
import { periodicScanDue } from "../analysisPolicy";
import { runWorkspaceAnalysis, transformRawResult } from "../analyzer";
import { CytoScnPyConfig } from "../analyzerTypes";
import { bundledExecutableName } from "../configuration";
import { getCacheKey } from "../extensionCache";
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

suite("Extension review fixes", () => {
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

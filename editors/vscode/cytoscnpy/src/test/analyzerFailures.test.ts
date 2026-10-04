import * as assert from "assert";
import * as os from "os";
import * as path from "path";
import { runCytoScnPyAnalysis, runWorkspaceAnalysis, transformRawResult } from "../analyzer";
import { CytoScnPyConfig, RawTaintFinding } from "../analyzerTypes";

suite("Analyzer failure handling", () => {
  const config: CytoScnPyConfig = {
    path: path.join(os.tmpdir(), "cytoscnpy-missing-executable", "scanner"),
    analysisMode: "workspace",
    enableSecretsScan: false,
    enableDangerScan: false,
    enableQualityScan: false,
    enableCloneScan: false,
  };

  for (const workspace of [false, true]) {
    test(`missing executable preserves error context (${workspace ? "workspace" : "file"})`, async () => {
      const target = path.join(os.tmpdir(), "project", "app.py");
      await assert.rejects(
        workspace ? runWorkspaceAnalysis(target, config) : runCytoScnPyAnalysis(target, config),
        (error: unknown) => {
          assert.ok(error instanceof Error);
          assert.ok(error.message.includes(target));
          assert.ok(error.message.includes("ENOENT"));
          assert.ok(error.cause instanceof Error);
          return true;
        },
      );
    });
  }

  for (const flowPath of [undefined, [], ["sanitize", "forward"]]) {
    test(`taint diagnostics support optional flow paths (${JSON.stringify(flowPath)})`, () => {
      const finding: RawTaintFinding = {
        source: "input", source_line: 1, sink: "eval", sink_line: 3, sink_col: 1,
        vuln_type: "Code Injection", severity: "HIGH", file: "app.py", remediation: "Validate input",
      };
      if (flowPath !== undefined) {
        finding.flow_path = flowPath;
      }
      const result = transformRawResult({ taint_findings: [finding] });
      assert.strictEqual(result.findings.length, 1);
      assert.ok(result.findings[0].message.includes(
        flowPath?.length ? "input -> sanitize -> forward -> eval" : "input -> eval",
      ));
    });
  }
});

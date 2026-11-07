// Regression tests locking behaviors surfaced by audit before any refactor.
// Each block targets one audit item so future fixes can prove they did not regress
// the surrounding behavior.

import * as assert from "assert";
import * as path from "path";
import * as vscode from "vscode";
import {
  buildAnalyzerArgs,
  CytoScnPyConfig,
  transformRawResult,
} from "../analyzer";

suite("Regression: buildAnalyzerArgs", () => {
  // Every optional setting unset, so the project configuration decides.
  const baseConfig: CytoScnPyConfig = {
    path: "cytoscnpy",
    analysisMode: "workspace",
  };

  test("minimal config emits only the required prefix", () => {
    const args = buildAnalyzerArgs("/tmp/foo.py", baseConfig);
    assert.deepStrictEqual(args, [
      "--client",
      "vscode",
      "/tmp/foo.py",
      "--json",
      "--force-exclude",
      "--ipynb-cells",
    ]);
  });

  test("explicitly disabled settings override the project configuration", () => {
    const args = buildAnalyzerArgs("/tmp/foo.py", {
      ...baseConfig,
      enableSecretsScan: false,
      enableDangerScan: false,
      enableQualityScan: false,
      enableCloneScan: false,
      includeTests: false,
      includeIpynb: false,
      maxComplexity: 5,
    });
    for (const flag of [
      "--no-secrets",
      "--no-danger",
      "--no-quality",
      "--no-clones",
      "--no-include-tests",
      "--no-include-ipynb",
    ]) {
      assert.ok(args.includes(flag), `${flag} should be passed`);
    }
    // A threshold would enable quality analysis, which is disabled.
    assert.ok(!args.includes("--max-complexity"));
  });

  test("a threshold is passed while quality is left to the project", () => {
    const args = buildAnalyzerArgs("/tmp/foo.py", { ...baseConfig, maxComplexity: 5 });
    assert.ok(args.includes("--max-complexity"));
    assert.ok(!args.includes("--quality") && !args.includes("--no-quality"));
  });

  test("scan toggles append their flags", () => {
    const args = buildAnalyzerArgs("/tmp/foo.py", {
      ...baseConfig,
      enableSecretsScan: true,
      enableDangerScan: true,
      enableCloneScan: true,
    });
    assert.ok(args.includes("--secrets"));
    assert.ok(args.includes("--danger"));
    assert.ok(args.includes("--clones"));
  });

  test("quality thresholds are dropped when quality is disabled", () => {
    const off = buildAnalyzerArgs("/tmp/foo.py", {
      ...baseConfig,
      enableQualityScan: false,
      maxComplexity: 7,
      maxNesting: 4,
    });
    assert.ok(!off.includes("--max-complexity"));
    assert.ok(!off.includes("--max-nesting"));

    const on = buildAnalyzerArgs("/tmp/foo.py", {
      ...baseConfig,
      enableQualityScan: true,
      maxComplexity: 7,
      maxNesting: 4,
    });
    assert.ok(on.includes("--quality"));
    assert.deepStrictEqual(
      on.slice(on.indexOf("--max-complexity"), on.indexOf("--max-complexity") + 2),
      ["--max-complexity", "7"],
    );
    assert.deepStrictEqual(
      on.slice(on.indexOf("--max-nesting"), on.indexOf("--max-nesting") + 2),
      ["--max-nesting", "4"],
    );
  });

  test("an explicit confidence threshold is emitted, zero included", () => {
    const zero = buildAnalyzerArgs("/tmp/foo.py", {
      ...baseConfig,
      confidenceThreshold: 0,
    });
    assert.deepStrictEqual(
      zero.slice(zero.indexOf("--confidence"), zero.indexOf("--confidence") + 2),
      ["--confidence", "0"],
    );
    assert.ok(!buildAnalyzerArgs("/tmp/foo.py", baseConfig).includes("--confidence"));

    const fifty = buildAnalyzerArgs("/tmp/foo.py", {
      ...baseConfig,
      confidenceThreshold: 50,
    });
    assert.deepStrictEqual(
      fifty.slice(fifty.indexOf("--confidence"), fifty.indexOf("--confidence") + 2),
      ["--confidence", "50"],
    );
  });

  test("exclude / include folders repeat the flag per entry", () => {
    const args = buildAnalyzerArgs("/tmp/foo.py", {
      ...baseConfig,
      excludeFolders: ["build", "dist"],
      includeFolders: ["src"],
    });
    const excludeFlagPositions = args.reduce<number[]>((acc, v, i) => {
      if (v === "--exclude-folders") {
        acc.push(i);
      }
      return acc;
    }, []);
    assert.strictEqual(excludeFlagPositions.length, 2);
    assert.strictEqual(args[excludeFlagPositions[0] + 1], "build");
    assert.strictEqual(args[excludeFlagPositions[1] + 1], "dist");
    assert.ok(args.includes("--include-folders"));
  });
});

suite("Regression: transformRawResult categories", () => {
  test("surfaces all CLI finding categories as diagnostics input", () => {
    const result = transformRawResult(
      {
        unused_functions: [
          {
            file: "app.py",
            line: 1,
            name: "unused_fn",
            simple_name: "unused_fn",
          },
        ],
        secrets: [
          {
            file: "app.py",
            line: 2,
            message: "secret",
            severity: "HIGH",
          },
        ],
        danger: [
          {
            file: "app.py",
            line: 3,
            message: "danger",
            severity: "CRITICAL",
          },
        ],
        quality: [
          {
            file: "app.py",
            line: 4,
            message: "quality",
          },
        ],
        taint_findings: [
          {
            source: "request.args",
            source_line: 5,
            sink: "os.system",
            sink_line: 6,
            sink_col: 2,
            flow_path: [],
            vuln_type: "Command Injection",
            severity: "HIGH",
            file: "app.py",
            remediation: "validate input",
          },
        ],
        unused_dependencies: [
          {
            package_name: "unused-dep",
            normalized_name: "unused_dep",
            is_dev: false,
            source: "Pyproject",
          },
        ],
        missing_dependencies: ["missing_dep"],
      },
      "pyproject.toml",
    );

    assert.deepStrictEqual(
      result.findings.map((finding) => finding.rule_id),
      [
        "unused-function",
        "secret-detected",
        "dangerous-code",
        "quality-issue",
        "unused-dependency",
        "missing-dependency",
        "taint-command injection",
      ],
    );
  });

  test("taint findings keep the CLI rule ID that noqa suppresses", () => {
    const result = transformRawResult(
      {
        taint_findings: [
          {
            rule_id: "CSP-D003",
            source: "input",
            source_line: 1,
            sink: "os.system",
            sink_line: 2,
            sink_col: 0,
            vuln_type: "Command Injection",
            severity: "HIGH",
            file: "app.py",
            remediation: "validate input",
          },
        ],
      },
      "app.py",
    );
    assert.deepStrictEqual(
      result.findings.map((finding) => finding.rule_id),
      ["CSP-D003"],
    );
  });

  test("anchors dependency diagnostics to files for directory targets", () => {
    const result = transformRawResult(
      {
        unused_dependencies: [
          {
            package_name: "unused-dep",
            normalized_name: "unused_dep",
            is_dev: false,
            source: undefined,
          },
        ],
        missing_dependencies: ["missing_dep"],
      },
      __dirname,
    );

    assert.deepStrictEqual(
      result.findings.map((finding) => finding.file_path),
      [
        path.join(__dirname, "pyproject.toml"),
        path.join(__dirname, "pyproject.toml"),
      ],
    );
  });
});

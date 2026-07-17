import * as assert from "assert";
import { bundledExecutableName } from "../configuration";
import { resolveFinding } from "../findingResolver";
import { CoalescingTaskQueue, DebouncedTaskMap } from "../scanScheduler";
import { CytoScnPyFinding } from "../analyzer";

function finding(
  name: string,
  line: number,
  startByte: number,
): CytoScnPyFinding {
  return {
    file_path: "sample.py",
    line_number: line,
    message: `'${name}' is imported but never used`,
    rule_id: "unused-import",
    category: "Dead Code",
    severity: "warning",
    fix: { start_byte: startByte, end_byte: startByte + name.length, replacement: "" },
  };
}

suite("Review fix regressions", () => {
  test("bundled executable names match packaged platform artifacts", () => {
    assert.strictEqual(
      bundledExecutableName("win32", "x64"),
      "cytoscnpy-cli-win32.exe",
    );
    assert.strictEqual(
      bundledExecutableName("linux", "x64"),
      "cytoscnpy-cli-linux-x64",
    );
    assert.strictEqual(
      bundledExecutableName("darwin", "x64"),
      "cytoscnpy-cli-darwin",
    );
    assert.strictEqual(
      bundledExecutableName("darwin", "arm64"),
      "cytoscnpy-cli-darwin-arm64",
    );
    assert.strictEqual(bundledExecutableName("linux", "arm64"), undefined);
  });

  test("same-line findings resolve by exact rendered message", () => {
    const osFinding = finding("os", 1, 7);
    const sysFinding = finding("sys", 1, 11);
    const resolved = resolveFinding([osFinding, sysFinding], {
      ruleId: "unused-import",
      line: 1,
      message: "'sys' is imported but never used [unused-import]",
    });
    assert.strictEqual(resolved, sysFinding);
  });

  test("ambiguous findings fail closed instead of applying the wrong fix", () => {
    const first = finding("same", 1, 0);
    const second = finding("same", 1, 10);
    assert.strictEqual(
      resolveFinding([first, second], {
        ruleId: "unused-import",
        line: 1,
        message: "'same' is imported but never used [unused-import]",
      }),
      undefined,
    );
  });

  test("per-document debounce does not cancel another document", async () => {
    const debouncer = new DebouncedTaskMap();
    const calls: string[] = [];
    debouncer.schedule("a.py", 5, () => calls.push("a"));
    debouncer.schedule("b.py", 5, () => calls.push("b"));
    await new Promise((resolve) => setTimeout(resolve, 30));
    debouncer.dispose();
    assert.deepStrictEqual(calls.sort(), ["a", "b"]);
  });

  test("workspace requests made during a scan are coalesced and rerun", async () => {
    const queue = new CoalescingTaskQueue();
    const calls: string[] = [];
    let releaseFirst: (() => void) | undefined;
    const blocker = new Promise<void>((resolve) => {
      releaseFirst = resolve;
    });
    const first = queue.run(async () => {
      calls.push("first-start");
      await blocker;
      calls.push("first-end");
    });
    const second = queue.run(async () => {
      calls.push("second");
    });
    releaseFirst?.();
    await Promise.all([first, second]);
    assert.deepStrictEqual(calls, ["first-start", "first-end", "second"]);
  });
});

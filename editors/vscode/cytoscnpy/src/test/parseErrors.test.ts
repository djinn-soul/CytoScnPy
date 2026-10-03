import * as assert from "assert";
import { transformRawResult } from "../analyzer";

suite("CLI parse error compatibility", () => {
  test("preserves the Rust error message and supplies a valid diagnostic line", () => {
    const result = transformRawResult({
      parse_errors: [{ file: "/workspace/broken.py", error: "Expected a parameter at byte range 11..12" }],
    });
    assert.deepStrictEqual(result.parseErrors, [{
      file: "/workspace/broken.py",
      line: 1,
      message: "Expected a parameter at byte range 11..12",
    }]);
  });

  test("handles output without parse errors", () => {
    assert.deepStrictEqual(transformRawResult({}).parseErrors, []);
  });
});

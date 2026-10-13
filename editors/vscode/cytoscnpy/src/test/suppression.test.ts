import * as assert from "assert";
import { suppressesFinding } from "../extensionDiagnostics";
import { covers, directivesByLine, parseDirective } from "../suppression";

// Mirrors cytoscnpy/tests/suppression_logic_test.rs so the editor and the
// CLI agree on which lines are suppressed.
function suppresses(source: string, ruleId: string, line = 0): boolean {
  const suppression = directivesByLine(source).get(line)?.suppression;
  return suppression !== undefined && covers(suppression, ruleId);
}

suite("Suppression comments", () => {
  test("existing CLI formats keep working", () => {
    assert.ok(suppresses("x = 1  # noqa: E501, W291, CSP", "CSP-D101"));
    assert.ok(suppresses("x = 1  # NoQa", "CSP-D101"));
    assert.ok(suppresses("x = 1  # noqa : CSP", "CSP-D101"));
    assert.ok(suppresses("x = 1  # noqa: CSP  -- false positive", "CSP-D101"));
    assert.ok(suppresses("x = 1  # noqa  this is intentional", "CSP-D101"));
    assert.ok(suppresses("x = 1  # ignore", "CSP-D101"));
    assert.ok(suppresses("x = 1  # pragma: no cytoscnpy", "unused-function"));
    assert.ok(suppresses("c = 'ad' #noqa: R102, CSP, dont implement any thing", "CSP-D101"));
    assert.ok(suppresses("def f(x=[]): # noqa: B006", "B006"));
    assert.ok(!suppresses("def f(x=[]): # noqa: B006", "E722"));
  });

  test("a # inside a string is not a comment", () => {
    assert.ok(!suppresses('x = "# noqa"', "CSP"));
    assert.ok(!suppresses("x = '# ignore'", "CSP"));
    assert.ok(!suppresses("x = f\"{y['k']} # noqa\"", "CSP"));
    assert.ok(!suppresses('x = "a\\" # noqa"', "CSP"));
    assert.ok(suppresses('x = "#"  # noqa', "CSP"));
  });

  test("multi-line strings are tracked across lines", () => {
    const docstring = 's = """\ntext # noqa\n"""\ny = 1  # noqa\n';
    assert.ok(!suppresses(docstring, "CSP", 1));
    assert.ok(suppresses(docstring, "CSP", 3));
    assert.ok(suppresses("s = '''\nabc'''  # noqa\n", "CSP", 1));
  });

  test("ignore must be a whole word", () => {
    assert.ok(!suppresses("x = 1  # ignored on purpose", "CSP"));
    assert.ok(!suppresses("x = 1  # noqa_like helper", "CSP"));
    assert.ok(!suppresses("x = 1  # type: ignore", "CSP"));
    assert.ok(suppresses("x = 1  # type: ignore  # noqa", "CSP"));
  });

  test("codes may be followed by an explanation", () => {
    const reviewed = "x = 1  # noqa: CSP-D101 reviewed, safe input";
    assert.ok(suppresses(reviewed, "CSP-D101"));
    assert.ok(!suppresses(reviewed, "CSP-D102"));
    assert.ok(suppresses("x = 1  # noqa: CSP-D101 CSP-Q202", "CSP-Q202"));
    const prose = "x = 1  # noqa: E501 not a CSP issue";
    assert.ok(suppresses(prose, "E501"));
    assert.ok(!suppresses(prose, "CSP-D101"));
    assert.strictEqual(directivesByLine("x = 1  # noqa: because reasons").get(0)?.suppression, undefined);
  });

  test("codesEnd points after the last code, before the explanation", () => {
    const comment = "# noqa: E501 reviewed";
    const directive = parseDirective(comment, 10);
    assert.strictEqual(directive?.codesEnd, 10 + comment.indexOf(" reviewed"));
  });

  test("dead code answers to CSP-V001 and its own kind, as in the CLI", () => {
    const suppression = (source: string) => directivesByLine(source).get(0)!.suppression!;
    assert.ok(suppressesFinding(suppression("import os  # noqa: CSP-V001"), "unused-import"));
    assert.ok(suppressesFinding(suppression("import os  # noqa: unused-import"), "unused-import"));
    assert.ok(!suppressesFinding(suppression("import os  # noqa: unused-function"), "unused-import"));
    // CSP-V001 is for dead code only.
    assert.ok(!suppressesFinding(suppression("x = 1  # noqa: CSP-V001"), "CSP-D101"));
  });
});

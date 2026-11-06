import * as assert from "assert";
import * as path from "path";
import { ownerRoot } from "../projectRoots";
import { emptyWorkspaceResults, mergeWorkspaceResult } from "../workspaceResults";
import { getCacheKey } from "../extensionCache";

suite("Nested project roots", () => {
  const root = path.resolve("/ws");
  const backend = path.join(root, "backend");
  const roots = [root, backend];

  test("each file belongs to the deepest project containing it", () => {
    assert.strictEqual(ownerRoot(path.join(backend, "api.py"), roots), backend);
    assert.strictEqual(ownerRoot(path.join(root, "app.py"), roots), root);
    // A sibling whose name only starts like a project is not inside it.
    assert.strictEqual(ownerRoot(path.join(root, "backend2", "x.py"), roots), root);
    assert.strictEqual(ownerRoot(path.resolve("/elsewhere/x.py"), roots), undefined);
  });

  test("the outer scan's results for a nested project are dropped", () => {
    const api = path.join(backend, "api.py");
    const app = path.join(root, "app.py");
    const finding = (message: string) => [{ message } as any];
    const merged = emptyWorkspaceResults();
    const scan = (results: [string, string][]) => ({
      findingsByFile: new Map(results.map(([file, message]) => [file, finding(message)])),
      parseErrorsByFile: new Map(),
      scanErrors: [],
    });
    // The outer scan reads only the root pyproject, so its backend result is
    // the one the backend's own configuration must replace.
    mergeWorkspaceResult(merged, scan([[app, "outer"], [api, "outer"]]), (file) =>
      ownerRoot(file, roots) === root,
    );
    mergeWorkspaceResult(merged, scan([[api, "backend"]]), (file) =>
      ownerRoot(file, roots) === backend,
    );
    assert.strictEqual(merged.findingsByFile.get(getCacheKey(app))?.[0].message, "outer");
    assert.strictEqual(merged.findingsByFile.get(getCacheKey(api))?.[0].message, "backend");
  });
});

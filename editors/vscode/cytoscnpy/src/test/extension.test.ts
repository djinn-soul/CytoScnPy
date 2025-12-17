import * as assert from "assert";
import * as vscode from "vscode";
import * as myExtension from "../../src/extension"; // Import the extension's main module
import { before } from "mocha";

suite("CytoScnPy Extension Test Suite", function () {
  this.timeout(10000);
  vscode.window.showInformationMessage("Start all CytoScnPy tests.");

  before(async function () {
    this.timeout(5000);
    // Open a Python document to trigger extension activation
    const doc = await vscode.workspace.openTextDocument({
      language: "python",
      content: 'print("hello world")',
    });
    await vscode.window.showTextDocument(doc);
    // Wait for extension to activate
    await new Promise((resolve) => setTimeout(resolve, 4000));
  });

  test("Extension should be active", async () => {
    const extension = vscode.extensions.getExtension("djinn09.cytoscnpy");
    assert.ok(extension, "Extension should be found");
    assert.strictEqual(extension.isActive, true, "Extension should be active");
  });

  test("All commands should be registered", async () => {
    const commands = await vscode.commands.getCommands(true);

    const expectedCommands = [
      "cytoscnpy.analyzeCurrentFile",
      "cytoscnpy.analyzeWorkspace",
      "cytoscnpy.complexity",
      "cytoscnpy.halstead",
      "cytoscnpy.maintainability",
      "cytoscnpy.rawMetrics",
    ];

    for (const cmd of expectedCommands) {
      assert.ok(
        commands.includes(cmd),
        `Command "${cmd}" should be registered`
      );
    }
  });

  // A more advanced test would involve mocking `runCytoScnPyAnalysis`,
  // opening a dummy document, saving it, and checking `vscode.languages.getDiagnostics`.
  // This requires more complex test setup not suitable for this context.
});

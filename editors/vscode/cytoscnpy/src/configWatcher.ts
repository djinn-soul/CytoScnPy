import * as path from "path";
import * as vscode from "vscode";

const PROJECT_CONFIG_NAMES = new Set([
  "pyproject.toml",
  ".cytoscnpy.toml",
]);

export function isProjectConfigPath(filePath: string): boolean {
  return PROJECT_CONFIG_NAMES.has(path.basename(filePath).toLowerCase());
}

export function watchProjectConfiguration(
  onChange: (uri: vscode.Uri) => void,
): vscode.Disposable {
  const watcher = vscode.workspace.createFileSystemWatcher(
    "**/{pyproject.toml,.cytoscnpy.toml}",
  );
  const notify = (uri: vscode.Uri) => {
    if (isProjectConfigPath(uri.fsPath)) {
      onChange(uri);
    }
  };

  return vscode.Disposable.from(
    watcher,
    watcher.onDidChange(notify),
    watcher.onDidCreate(notify),
    watcher.onDidDelete(notify),
  );
}

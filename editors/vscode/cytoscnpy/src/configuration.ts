import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import * as vscode from "vscode";
import { CytoScnPyConfig } from "./analyzer";

export function bundledExecutableName(
  platform = os.platform(),
  architecture = os.arch(),
): string | undefined {
  if (platform === "win32" && architecture === "x64") {
    return "cytoscnpy-cli-win32.exe";
  }
  if (platform === "linux" && architecture === "x64") {
    return "cytoscnpy-cli-linux-x64";
  }
  if (platform === "darwin" && architecture === "arm64") {
    return "cytoscnpy-cli-darwin-arm64";
  }
  if (platform === "darwin" && architecture === "x64") {
    return "cytoscnpy-cli-darwin";
  }
  return undefined;
}

function explicitlyConfiguredPath(resource?: vscode.Uri): string | undefined {
  const config = vscode.workspace.getConfiguration("cytoscnpy", resource);
  const inspected = config.inspect<string>("path");
  const hasExplicitValue =
    inspected?.workspaceFolderValue !== undefined ||
    inspected?.workspaceValue !== undefined ||
    inspected?.globalValue !== undefined;
  if (!hasExplicitValue) {
    return undefined;
  }
  const configured = config.get<string>("path")?.trim();
  return configured || undefined;
}

export function getExecutablePath(
  context: vscode.ExtensionContext,
  resource?: vscode.Uri,
): string {
  const configured = explicitlyConfiguredPath(resource);
  if (configured) {
    return configured;
  }

  const executableName = bundledExecutableName();
  if (executableName) {
    const bundledPath = path.join(context.extensionPath, "bin", executableName);
    try {
      if (fs.existsSync(bundledPath)) {
        return bundledPath;
      }
    } catch {
      // Fall through to the PATH-installed executable.
    }
  }
  return "cytoscnpy";
}

export function getCytoScnPyConfiguration(
  context: vscode.ExtensionContext,
  resource?: vscode.Uri,
): CytoScnPyConfig {
  const config = vscode.workspace.getConfiguration("cytoscnpy", resource);
  const getIfSet = <T>(key: string): T | undefined => {
    const inspect = config.inspect<T>(key);
    if (
      inspect &&
      (inspect.globalValue !== undefined ||
        inspect.workspaceValue !== undefined ||
        inspect.workspaceFolderValue !== undefined)
    ) {
      return config.get<T>(key);
    }
    return undefined;
  };

  return {
    path: getExecutablePath(context, resource),
    analysisMode:
      config.get<string>("analysisMode") === "file" ? "file" : "workspace",
    enableSecretsScan: config.get<boolean>("enableSecretsScan") || false,
    enableDangerScan: config.get<boolean>("enableDangerScan") || false,
    enableQualityScan: config.get<boolean>("enableQualityScan") || false,
    enableCloneScan: config.get<boolean>("enableCloneScan") || false,
    confidenceThreshold: getIfSet<number>("confidenceThreshold"),
    excludeFolders: getIfSet<string[]>("excludeFolders"),
    includeFolders: getIfSet<string[]>("includeFolders"),
    includeTests: getIfSet<boolean>("includeTests"),
    includeIpynb: getIfSet<boolean>("includeIpynb"),
    maxComplexity: getIfSet<number>("maxComplexity"),
    minMaintainabilityIndex: getIfSet<number>("minMaintainabilityIndex"),
    maxNesting: getIfSet<number>("maxNesting"),
    maxArguments: getIfSet<number>("maxArguments"),
    maxLines: getIfSet<number>("maxLines"),
  };
}

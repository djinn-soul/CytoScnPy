import * as vscode from "vscode";
import { getExecutablePath } from "./configuration";

export function registerMcpProvider(context: vscode.ExtensionContext): void {
  if (
    vscode.lm &&
    typeof vscode.lm.registerMcpServerDefinitionProvider === "function"
  ) {
    try {
      const mcpDidChangeEmitter = new vscode.EventEmitter<void>();
      context.subscriptions.push(
        mcpDidChangeEmitter,
        vscode.workspace.onDidChangeWorkspaceFolders(() =>
          mcpDidChangeEmitter.fire(),
        ),
      );
      context.subscriptions.push(
        vscode.lm.registerMcpServerDefinitionProvider("cytoscnpy-mcp", {
          onDidChangeMcpServerDefinitions: mcpDidChangeEmitter.event,
          provideMcpServerDefinitions: async () => {
            const workspaceFolders = vscode.workspace.workspaceFolders;
            const extension =
              vscode.extensions.getExtension("djinn09.cytoscnpy");
            const version = extension?.packageJSON?.version || "0.1.0";
            if (!workspaceFolders || workspaceFolders.length === 0) {
              return [
                new vscode.McpStdioServerDefinition(
                  "CytoScnPy",
                  getExecutablePath(context),
                  ["mcp-server"],
                  { cwd: null, version },
                ),
              ];
            }
            return workspaceFolders.map(
              (folder) =>
                new vscode.McpStdioServerDefinition(
                  `CytoScnPy (${folder.name})`,
                  getExecutablePath(context, folder.uri),
                  ["mcp-server"],
                  { cwd: folder.uri.fsPath, version },
                ),
            );
          },
          resolveMcpServerDefinition: async (server) => server,
        }),
      );
    } catch (mcpError) {
      console.warn("Failed to register MCP server provider:", mcpError);
    }
  }
}

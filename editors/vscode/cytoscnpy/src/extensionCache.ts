import * as vscode from "vscode";
import * as crypto from "crypto";
import { CytoScnPyFinding } from "./analyzer";
import { analysisState } from "./extensionState";
import { cytoscnpyDiagnostics, applyGutterDecorations } from "./extensionUi";

export interface CacheEntry {
  hash: string;
  diagnostics: vscode.Diagnostic[];
  findings: CytoScnPyFinding[];
  timestamp: number;
}

const MAX_CACHE_HISTORY = 10;

export const fileCache = new Map<string, CacheEntry[]>();

export const documentHashCache = new Map<string, { version: number; hash: string }>();

export function computeHash(content: string): string {
  return crypto.createHash("sha256").update(content).digest("hex");
}

export function hashForDocument(document: vscode.TextDocument): string {
  const key = document.uri.toString();
  const cached = documentHashCache.get(key);
  if (cached && cached.version === document.version) {
    return cached.hash;
  }
  const hash = computeHash(document.getText());
  documentHashCache.set(key, { version: document.version, hash });
  return hash;
}

export function getCacheKey(fsPath: string): string {
  return process.platform === "win32" ? fsPath.toLowerCase() : fsPath;
}

export function cacheDocumentAnalysis(
  document: vscode.TextDocument,
  diagnostics: vscode.Diagnostic[],
  findings: CytoScnPyFinding[],
): void {
  if (document.isDirty) {
    return;
  }
  const cacheKey = getCacheKey(document.uri.fsPath);
  const cacheEntry: CacheEntry = {
    hash: hashForDocument(document),
    diagnostics,
    findings,
    timestamp: Date.now(),
  };
  const history = fileCache.get(cacheKey) || [];
  history.unshift(cacheEntry);
  if (history.length > MAX_CACHE_HISTORY) {
    history.pop();
  }
  fileCache.set(cacheKey, history);
}

export function clearDocumentAnalysis(document: vscode.TextDocument): void {
  cytoscnpyDiagnostics.delete(document.uri);
  fileCache.delete(getCacheKey(document.uri.fsPath));
  documentHashCache.delete(document.uri.toString());
  const editor = vscode.window.activeTextEditor;
  if (editor?.document.uri.toString() === document.uri.toString()) {
    applyGutterDecorations(editor, []);
  }
}

export function invalidateWorkspaceCache() {
  analysisState.workspaceCache = null;
  analysisState.workspaceParseErrorsCache = null;
  analysisState.workspaceCacheTimestamp = 0;
  fileCache.clear();
  documentHashCache.clear();
  cytoscnpyDiagnostics.clear();
}

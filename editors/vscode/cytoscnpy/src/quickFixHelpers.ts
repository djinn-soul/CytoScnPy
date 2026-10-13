import * as vscode from "vscode";
import { CytoScnPyFinding } from "./analyzer";

export const UNUSED_RULE_LABELS: Record<string, { singular: string; plural: string }> =
  {
    "unused-function": { singular: "function", plural: "functions" },
    "unused-method": { singular: "method", plural: "methods" },
    "unused-class": { singular: "class", plural: "classes" },
    "unused-import": { singular: "import", plural: "imports" },
    "unused-variable": { singular: "variable", plural: "variables" },
    "unused-parameter": { singular: "parameter", plural: "parameters" },
  };

export function filterOverlappingFixes<T extends { finding: CytoScnPyFinding }>(
  items: T[],
): T[] {
  const sorted = [...items].sort((a, b) => {
    const aStart = a.finding.fix!.start_byte;
    const bStart = b.finding.fix!.start_byte;
    if (aStart !== bStart) {
      return aStart - bStart;
    }
    return b.finding.fix!.end_byte - a.finding.fix!.end_byte;
  });
  const filtered: T[] = [];
  let lastEnd = 0;
  for (const item of sorted) {
    const start = item.finding.fix!.start_byte;
    const end = item.finding.fix!.end_byte;
    if (start >= lastEnd) {
      filtered.push(item);
      lastEnd = end;
    }
  }
  return filtered;
}

function byteOffsetToUtf16Offset(text: string, byteOffset: number): number {
  const utf8 = Buffer.from(text, "utf8");
  const clamped = Math.max(0, Math.min(byteOffset, utf8.length));
  return utf8.subarray(0, clamped).toString("utf8").length;
}

export function rangeFromFixBytes(
  document: vscode.TextDocument,
  startByte: number,
  endByte: number,
): vscode.Range | undefined {
  if (
    !Number.isInteger(startByte) ||
    !Number.isInteger(endByte) ||
    startByte < 0 ||
    endByte < startByte
  ) {
    return undefined;
  }

  const text = document.getText();
  const utf8Len = Buffer.byteLength(text, "utf8");
  if (endByte > utf8Len) {
    return undefined;
  }

  const startOffset = byteOffsetToUtf16Offset(text, startByte);
  const endOffset = byteOffsetToUtf16Offset(text, endByte);
  if (endOffset < startOffset) {
    return undefined;
  }

  return new vscode.Range(
    document.positionAt(startOffset),
    document.positionAt(endOffset),
  );
}

export function createSuppressionAction(
  document: vscode.TextDocument,
  diagnostic: vscode.Diagnostic,
): vscode.CodeAction | undefined {
  const codeValue =
    typeof diagnostic.code === "object" &&
    diagnostic.code !== null &&
    "value" in diagnostic.code
      ? String(diagnostic.code.value)
      : typeof diagnostic.code === "string"
        ? diagnostic.code
        : undefined;
  const suppressionCode =
    codeValue && /^CSP-[A-Z]\d{3}$/i.test(codeValue) ? codeValue : "CSP";
  const actionTitle = `Suppress with # noqa: ${suppressionCode}`;

  const action = new vscode.CodeAction(
    actionTitle,
    vscode.CodeActionKind.QuickFix,
  );
  action.diagnostics = [diagnostic];

  const lineIndex = diagnostic.range.start.line;
  const lineText = document.lineAt(lineIndex).text;
  const edit = new vscode.WorkspaceEdit();
  const pragmaRegex = /#\s*pragma:\s*no\s+cytoscnpy/i;
  if (pragmaRegex.test(lineText)) {
    return undefined;
  }

  // Check for existing suppression comment
  const noqaRegex = /#\s*(?:noqa|ignore)(?::\s*([^#\n]+))?/i;
  const match = lineText.match(noqaRegex);

  if (match) {
    // Existing noqa found
    if (!match[1]) {
      // Bare # noqa - already suppresses all
      return undefined;
    }
    const existingCodes = match[1]
      .split(/,\s*/)
      .map((s) => s.trim().toUpperCase());
    if (
      existingCodes.includes("CSP") ||
      existingCodes.includes(suppressionCode.toUpperCase())
    ) {
      return undefined; // Already suppressed
    }
    // Append suppression code to existing codes
    const commentStart = match.index!;
    const commentContent = match[0];
    const newComment = `${commentContent}, ${suppressionCode}`;
    const range = new vscode.Range(
      new vscode.Position(lineIndex, commentStart),
      new vscode.Position(lineIndex, commentStart + commentContent.length),
    );
    edit.replace(document.uri, range, newComment);
  } else {
    // No existing noqa, append new one
    const insertText = `  # noqa: ${suppressionCode}`;
    const insertPos = new vscode.Position(lineIndex, lineText.length);
    edit.insert(document.uri, insertPos, insertText);
  }

  action.edit = edit;
  return action;
}

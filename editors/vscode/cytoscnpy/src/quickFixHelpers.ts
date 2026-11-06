import * as vscode from "vscode";
import { CytoScnPyFinding } from "./analyzer";
import {
  UNUSED_RULE_IDS,
  documentDirectives,
  suppressesFinding,
  suppressionCodesForDiagnostic,
} from "./extensionDiagnostics";

export const UNUSED_RULE_LABELS: Record<string, { singular: string; plural: string }> =
  {
    "unused-function": { singular: "function", plural: "functions" },
    "unused-method": { singular: "method", plural: "methods" },
    "unused-class": { singular: "class", plural: "classes" },
    "unused-import": { singular: "import", plural: "imports" },
    "unused-variable": { singular: "variable", plural: "variables" },
    "unused-parameter": { singular: "parameter", plural: "parameters" },
  };

type FixRangeMapper = (startByte: number, endByte: number) => vscode.Range | undefined;

/**
 * Maps analyzer byte offsets to ranges in `document`. Encoding the document
 * once and reusing it for every fix keeps "Fix All" from re-encoding the
 * whole file several times per finding.
 */
export function fixRangeMapper(document: vscode.TextDocument): FixRangeMapper {
  const text = document.getText();
  const utf8 = Buffer.from(text, "utf8");
  const toUtf16 = (byteOffset: number) =>
    utf8.toString("utf8", 0, byteOffset).length;

  return (startByte, endByte) => {
    if (
      !Number.isInteger(startByte) ||
      !Number.isInteger(endByte) ||
      startByte < 0 ||
      endByte < startByte ||
      endByte > utf8.length
    ) {
      return undefined;
    }
    const startOffset = toUtf16(startByte);
    const endOffset = toUtf16(endByte);
    if (endOffset < startOffset) {
      return undefined;
    }
    return new vscode.Range(
      document.positionAt(startOffset),
      document.positionAt(endOffset),
    );
  };
}

export function rangeFromFixBytes(
  document: vscode.TextDocument,
  startByte: number,
  endByte: number,
): vscode.Range | undefined {
  return fixRangeMapper(document)(startByte, endByte);
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
  // Prefer the finding's own code so other findings on the line stay
  // visible; fall back to CSP (everything) only for codes the CLI ignores.
  const suppressionCode =
    codeValue &&
    (/^CSP-[A-Z]\d{3}$/i.test(codeValue) || UNUSED_RULE_IDS.has(codeValue))
      ? codeValue
      : "CSP";
  const actionTitle = `Suppress with # noqa: ${suppressionCode}`;

  const action = new vscode.CodeAction(
    actionTitle,
    vscode.CodeActionKind.QuickFix,
  );
  action.diagnostics = [diagnostic];

  const lineIndex = diagnostic.range.start.line;
  const directive = documentDirectives(document).get(lineIndex);
  if (
    directive?.suppression &&
    suppressesFinding(
      directive.suppression,
      suppressionCodesForDiagnostic(diagnostic) ?? [suppressionCode],
    )
  ) {
    return undefined; // Already suppressed
  }

  const edit = new vscode.WorkspaceEdit();
  if (directive?.codesEnd !== undefined) {
    // Add the code after the listed ones, ahead of any explanation.
    edit.insert(
      document.uri,
      new vscode.Position(lineIndex, directive.codesEnd),
      `, ${suppressionCode}`,
    );
  } else if (directive?.codesStart !== undefined) {
    // `# noqa: reason` lists no codes yet.
    edit.insert(
      document.uri,
      new vscode.Position(lineIndex, directive.codesStart),
      `${suppressionCode}, `,
    );
  } else {
    // No existing noqa, append new one
    const lineText = document.lineAt(lineIndex).text;
    edit.insert(
      document.uri,
      new vscode.Position(lineIndex, lineText.length),
      `  # noqa: ${suppressionCode}`,
    );
  }

  action.edit = edit;
  return action;
}

import * as vscode from "vscode";
import { CytoScnPyFinding, ParseError } from "./analyzerTypes";

/**
 * Notebook cells are text documents whose `fsPath` is the notebook's. The
 * analyzer (run with `--ipynb-cells`) tags each finding with its cell index
 * and a cell-relative line, and plans dead-code fixes within that cell.
 */
export interface NotebookCellLocation {
  notebook: vscode.NotebookDocument;
  /** Index among all of the notebook's cells, as nbformat counts them. */
  index: number;
}

export function notebookCellOf(
  document: vscode.TextDocument,
): NotebookCellLocation | undefined {
  if (document.uri.scheme !== "vscode-notebook-cell") {
    return undefined;
  }
  const uri = document.uri.toString();
  for (const notebook of vscode.workspace.notebookDocuments) {
    const cell = notebook
      .getCells()
      .find((candidate) => candidate.document.uri.toString() === uri);
    if (cell) {
      return { notebook, index: cell.index };
    }
  }
  return undefined;
}

/** True when the text on screen may differ from the file the analyzer read. */
export function isUnsaved(document: vscode.TextDocument): boolean {
  return document.isDirty || (notebookCellOf(document)?.notebook.isDirty ?? false);
}

/**
 * The findings that belong in `document`. A cell shows only its own; a
 * notebook finding without a cell (such as a file-level one) has a line of
 * the joined cells, which no cell can place.
 */
export function findingsForDocument(
  document: vscode.TextDocument,
  findings: CytoScnPyFinding[],
): CytoScnPyFinding[] {
  const cell = notebookCellOf(document);
  if (!cell) {
    return findings;
  }
  return findings.filter((finding) => finding.cell_number === cell.index);
}

/**
 * Parse errors of a notebook name lines of the joined cells; show them once,
 * at the top of the first code cell.
 */
export function parseErrorsForDocument(
  document: vscode.TextDocument,
  parseErrors: ParseError[],
): ParseError[] {
  const cell = notebookCellOf(document);
  if (!cell) {
    return parseErrors;
  }
  const firstCode = cell.notebook
    .getCells()
    .find((candidate) => candidate.kind === vscode.NotebookCellKind.Code);
  if (firstCode?.index !== cell.index) {
    return [];
  }
  return parseErrors.map((error) => ({ ...error, line: 1 }));
}

/** Key for per-document caches; cells of one notebook share an `fsPath`. */
export function documentKey(
  document: vscode.TextDocument,
  fileKey: (fsPath: string) => string,
): string {
  return document.uri.scheme === "vscode-notebook-cell"
    ? document.uri.toString()
    : fileKey(document.uri.fsPath);
}

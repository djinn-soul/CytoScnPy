import * as vscode from "vscode";
import { CytoScnPyFinding } from "./analyzer";
import { findingResolver } from "./findingResolver";
import { fileCache, getCacheKey, hashForDocument } from "./extensionCache";
import { UNUSED_RULE_LABELS, filterOverlappingFixes, fixRangeMapper, createSuppressionAction } from "./quickFixHelpers";

export class QuickFixProvider implements vscode.CodeActionProvider {
  public provideCodeActions(
    document: vscode.TextDocument,
    range: vscode.Range | vscode.Selection,
    context: vscode.CodeActionContext,
    token: vscode.CancellationToken,
  ): vscode.CodeAction[] {
    // Honour the cancellation token: VS Code routinely cancels stale invocations
    // when the user keeps typing. Doing the SHA-256 + diagnostic walk anyway is
    // wasted work and can starve the next valid request.
    if (token.isCancellationRequested) {
      return [];
    }

    const actions: vscode.CodeAction[] = [];

    // Collect all fixable findings for "Fix All" action
    const fixableByRule = new Map<
      string,
      { finding: CytoScnPyFinding; diagnostic: vscode.Diagnostic }[]
    >();
    const fixableUnused: {
      finding: CytoScnPyFinding;
      diagnostic: vscode.Diagnostic;
    }[] = [];

    // Reuse memoised SHA-256 keyed on `document.version`. Recomputing the hash
    // over the full document on every lightbulb invocation showed up as a
    // measurable hotspot on large files.
    const currentHash = hashForDocument(document);
    const cacheKey = getCacheKey(document.uri.fsPath);
    const cachedHistory = fileCache.get(cacheKey) || [];
    const cachedEntry = cachedHistory.find((e) => e.hash === currentHash);
    const resolve = cachedEntry ? findingResolver(cachedEntry.findings) : undefined;
    // Created on first use: requests without fixes skip encoding the text.
    let mapFixRange: ReturnType<typeof fixRangeMapper> | undefined;
    const rangeFromFixBytes = (startByte: number, endByte: number) =>
      (mapFixRange ??= fixRangeMapper(document))(startByte, endByte);
    const getRuleId = (diagnostic: vscode.Diagnostic): string | undefined =>
      typeof diagnostic.code === "object" &&
        diagnostic.code !== null &&
        "value" in diagnostic.code
        ? (diagnostic.code.value as string)
        : (diagnostic.code as string);

    const findFindingForDiagnostic = (
      diagnostic: vscode.Diagnostic,
      ruleId: string | undefined,
    ): CytoScnPyFinding | undefined => {
      return resolve?.({
        ruleId,
        line: diagnostic.range.start.line + 1,
        message: diagnostic.message,
      });
    };

    // Resolve file diagnostics for file-wide "Fix All" actions. Context
    // diagnostics are resolved separately because VS Code may pass equivalent
    // diagnostic objects that are not identical to the global collection.
    type ResolvedDiag = {
      diagnostic: vscode.Diagnostic;
      ruleId: string | undefined;
      finding: CytoScnPyFinding | undefined;
    };
    const isCytoScnPyDiagnostic = (diagnostic: vscode.Diagnostic): boolean =>
      diagnostic.source?.startsWith("CytoScnPy") ?? false;
    const resolveDiagnostic = (diagnostic: vscode.Diagnostic): ResolvedDiag => {
      const ruleId = getRuleId(diagnostic);
      const finding = findFindingForDiagnostic(diagnostic, ruleId);
      return { diagnostic, ruleId, finding };
    };
    const fileDiagnostics = vscode.languages.getDiagnostics(document.uri);
    for (const diagnostic of fileDiagnostics) {
      if (!isCytoScnPyDiagnostic(diagnostic)) {
        continue;
      }
      const resolvedDiagnostic = resolveDiagnostic(diagnostic);
      const { ruleId, finding } = resolvedDiagnostic;

      if (finding && finding.fix && ruleId && UNUSED_RULE_LABELS[ruleId]) {
        fixableUnused.push({ finding, diagnostic });
        if (!fixableByRule.has(ruleId)) {
          fixableByRule.set(ruleId, []);
        }
        fixableByRule.get(ruleId)!.push({ finding, diagnostic });
      }
    }

    for (const diagnostic of context.diagnostics) {
      if (!isCytoScnPyDiagnostic(diagnostic)) {
        continue;
      }
      const { ruleId, finding } = resolveDiagnostic(diagnostic);

      if (finding && finding.fix && ruleId) {
        const labels = UNUSED_RULE_LABELS[ruleId];
        if (labels) {
          // Extract symbol name from diagnostic message (e.g., "'ceil' is imported but never used")
          // Also try backticks for messages like "`name` is defined but never used"
          const symbolMatch =
            diagnostic.message.match(/'([^']+)'/) ||
            diagnostic.message.match(/`([^`]+)`/);

          const actionTitle = symbolMatch
            ? `Remove unused ${labels.singular} '${symbolMatch[1]}'`
            : `Remove unused ${labels.singular}`;

          const fixAction = new vscode.CodeAction(
            actionTitle,
            vscode.CodeActionKind.QuickFix,
          );
          fixAction.diagnostics = [diagnostic];
          fixAction.isPreferred = true;

          const range = rangeFromFixBytes(
            finding.fix.start_byte,
            finding.fix.end_byte,
          );
          if (range) {
            const edit = new vscode.WorkspaceEdit();
            edit.replace(document.uri, range, finding.fix.replacement);
            fixAction.edit = edit;
            actions.push(fixAction);
          }
        }
      }

      // "Suppress" action for every CytoScnPy diagnostic at the cursor.
      const suppressAction = createSuppressionAction(document, diagnostic);
      if (suppressAction) {
        actions.push(suppressAction);
      }
    }

    // 3. Add "Fix All" actions for rules with multiple findings
    for (const [ruleId, items] of fixableByRule.entries()) {
      const filteredItems = filterOverlappingFixes(items);
      if (filteredItems.length < 2) {
        continue;
      }

      const labels = UNUSED_RULE_LABELS[ruleId];
      if (!labels) {
        continue;
      }

      const fixAllAction = new vscode.CodeAction(
        `Remove all unused ${labels.plural} in this file`,
        vscode.CodeActionKind.QuickFix,
      );
      fixAllAction.diagnostics = filteredItems.map((i) => i.diagnostic);

      const edit = new vscode.WorkspaceEdit();
      // Sort by start_byte descending to apply fixes from end of file first
      // This prevents byte offset shifts from invalidating later fixes
      const sortedItems = [...filteredItems].sort(
        (a, b) => b.finding.fix!.start_byte - a.finding.fix!.start_byte,
      );

      let hasAllRanges = true;
      for (const { finding } of sortedItems) {
        const range = rangeFromFixBytes(
          finding.fix!.start_byte,
          finding.fix!.end_byte,
        );
        if (!range) {
          hasAllRanges = false;
          break;
        }
        edit.replace(document.uri, range, finding.fix!.replacement);
      }
      if (hasAllRanges) {
        fixAllAction.edit = edit;
        actions.push(fixAllAction);
      }
    }

    const filteredUnused = filterOverlappingFixes(fixableUnused);
    if (filteredUnused.length >= 2) {
      const fixAllDeadCodeAction = new vscode.CodeAction(
        "Remove all dead code in this file",
        vscode.CodeActionKind.QuickFix,
      );
      fixAllDeadCodeAction.diagnostics = filteredUnused.map(
        (i) => i.diagnostic,
      );

      const edit = new vscode.WorkspaceEdit();
      const sortedItems = [...filteredUnused].sort(
        (a, b) => b.finding.fix!.start_byte - a.finding.fix!.start_byte,
      );

      let hasAllRanges = true;
      for (const { finding } of sortedItems) {
        const range = rangeFromFixBytes(
          finding.fix!.start_byte,
          finding.fix!.end_byte,
        );
        if (!range) {
          hasAllRanges = false;
          break;
        }
        edit.replace(document.uri, range, finding.fix!.replacement);
      }
      if (hasAllRanges) {
        fixAllDeadCodeAction.edit = edit;
        actions.push(fixAllDeadCodeAction);
      }
    }

    return actions;
  }

}

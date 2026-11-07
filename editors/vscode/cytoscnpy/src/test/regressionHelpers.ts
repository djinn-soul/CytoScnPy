// Shared fakes for the regression suites.

import * as vscode from "vscode";

export const MAX_CACHE_HISTORY = 10;

export function makeContext(diagnostic: vscode.Diagnostic): vscode.CodeActionContext {
  return {
    diagnostics: [diagnostic],
    triggerKind: vscode.CodeActionTriggerKind.Invoke,
    only: undefined,
  };
}

export function makeFinding(
  ruleId: string,
  startByte: number,
  endByte: number,
  line = 1,
) {
  return {
    rule_id: ruleId,
    line_number: line,
    message: "stub",
    fix: { start_byte: startByte, end_byte: endByte, replacement: "" },
  };
}

import { CytoScnPyFinding } from "./analyzer";

export interface DiagnosticIdentity {
  ruleId: string | undefined;
  line: number;
  message: string;
}

function renderedMessage(finding: CytoScnPyFinding): string {
  return `${finding.message} [${finding.rule_id}]`;
}

export function resolveFinding(
  findings: readonly CytoScnPyFinding[],
  diagnostic: DiagnosticIdentity,
): CytoScnPyFinding | undefined {
  if (!diagnostic.ruleId) {
    return undefined;
  }
  const sameRule = findings.filter(
    (finding) => finding.rule_id === diagnostic.ruleId,
  );
  const exact = sameRule.filter(
    (finding) =>
      finding.line_number === diagnostic.line &&
      renderedMessage(finding) === diagnostic.message,
  );
  if (exact.length === 1) {
    return exact[0];
  }

  const sameLine = sameRule.filter(
    (finding) => finding.line_number === diagnostic.line,
  );
  if (sameLine.length === 1) {
    return sameLine[0];
  }

  const nearby = sameRule.filter(
    (finding) => Math.abs(finding.line_number - diagnostic.line) <= 2,
  );
  return nearby.length === 1 ? nearby[0] : undefined;
}

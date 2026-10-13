import { CytoScnPyFinding } from "./analyzer";

export interface DiagnosticIdentity {
  ruleId: string | undefined;
  line: number;
  message: string;
}

function renderedMessage(finding: CytoScnPyFinding): string {
  return `${finding.message} [${finding.rule_id}]`;
}

type FindingResolver = (diagnostic: DiagnosticIdentity) => CytoScnPyFinding | undefined;

// Quick fixes resolve every diagnostic in the file on each lightbulb request;
// index the findings once per cached result instead of scanning them all for
// each diagnostic.
const resolvers = new WeakMap<readonly CytoScnPyFinding[], FindingResolver>();

/** Returns a resolver over `findings`, built once per findings array. */
export function findingResolver(
  findings: readonly CytoScnPyFinding[],
): FindingResolver {
  let resolver = resolvers.get(findings);
  if (!resolver) {
    resolver = createResolver(findings);
    resolvers.set(findings, resolver);
  }
  return resolver;
}

export function resolveFinding(
  findings: readonly CytoScnPyFinding[],
  diagnostic: DiagnosticIdentity,
): CytoScnPyFinding | undefined {
  return findingResolver(findings)(diagnostic);
}

function createResolver(findings: readonly CytoScnPyFinding[]): FindingResolver {
  const byRuleAndLine = new Map<string, CytoScnPyFinding[]>();
  const key = (ruleId: string, line: number) => `${line}\u0000${ruleId}`;
  for (const finding of findings) {
    const findingKey = key(finding.rule_id, finding.line_number);
    const group = byRuleAndLine.get(findingKey);
    if (group) {
      group.push(finding);
    } else {
      byRuleAndLine.set(findingKey, [finding]);
    }
  }

  return (diagnostic) => {
    const { ruleId, line, message } = diagnostic;
    if (!ruleId) {
      return undefined;
    }
    const sameLine = byRuleAndLine.get(key(ruleId, line)) ?? [];
    const exact = sameLine.filter(
      (finding) => renderedMessage(finding) === message,
    );
    if (exact.length === 1) {
      return exact[0];
    }
    if (sameLine.length === 1) {
      return sameLine[0];
    }

    const nearby: CytoScnPyFinding[] = [];
    for (let offset = -2; offset <= 2; offset++) {
      nearby.push(...(byRuleAndLine.get(key(ruleId, line + offset)) ?? []));
    }
    return nearby.length === 1 ? nearby[0] : undefined;
  };
}

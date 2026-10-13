/**
 * Inline suppression comments (`# noqa`, `# ignore`, `# pragma: no cytoscnpy`).
 *
 * Mirrors `cytoscnpy/src/utils/suppression.rs` so the editor hides exactly
 * what the CLI hides; keep the two in sync.
 */

export type Suppression =
  | { kind: "all" }
  | { kind: "specific"; codes: ReadonlySet<string> };

/** A suppression directive found in a line's comment. */
export interface Directive {
  /** Undefined when the directive lists only an explanation, no codes. */
  suppression: Suppression | undefined;
  /** Column where codes start after `noqa:`; undefined for bare directives. */
  codesStart?: number;
  /** Column just after the last listed code. */
  codesEnd?: number;
}

const DIRECTIVE_RE =
  /#\s*(?:(pragma:\s*no\s*cytoscnpy)|(?:noqa|ignore)\b(?:\s*:\s*([^#\n]*))?)/i;

export function covers(suppression: Suppression, ruleId: string): boolean {
  return (
    suppression.kind === "all" || suppression.codes.has(ruleId.toUpperCase())
  );
}

/**
 * Returns each commented line (0-based) with its comment's start column and
 * text. String literals are tracked across lines, so a `#` inside a string,
 * including a triple-quoted string spanning lines, is not a comment.
 */
export function commentsByLine(
  source: string,
): Map<number, { column: number; text: string }> {
  const found = new Map<number, { column: number; text: string }>();
  let line = 0;
  let lineStart = 0;
  let open: { quote: string; triple: boolean } | undefined;
  let i = 0;
  while (i < source.length) {
    const char = source[i];
    if (char === "\n") {
      line += 1;
      lineStart = i + 1;
      // A single-quoted string cannot span lines without a backslash.
      if (open && !open.triple) {
        open = undefined;
      }
      i += 1;
      continue;
    }
    if (open) {
      if (char === "\\") {
        // Skip the escaped character; a line continuation still starts a
        // new line.
        const skip = source.startsWith("\r\n", i + 1) ? 3 : 2;
        if (source[i + skip - 1] === "\n") {
          line += 1;
          lineStart = i + skip;
        }
        i += skip;
        continue;
      }
      if (char === open.quote) {
        if (!open.triple) {
          open = undefined;
        } else if (source.startsWith(char.repeat(3), i)) {
          open = undefined;
          i += 3;
          continue;
        }
      }
      i += 1;
      continue;
    }
    if (char === "#") {
      const newline = source.indexOf("\n", i);
      const end = newline === -1 ? source.length : newline;
      found.set(line, {
        column: i - lineStart,
        text: source.slice(i, end).replace(/\r$/, ""),
      });
      i = end;
      continue;
    }
    if (char === '"' || char === "'") {
      const triple = source.startsWith(char.repeat(3), i);
      open = { quote: char, triple };
      i += triple ? 3 : 1;
      continue;
    }
    i += 1;
  }
  return found;
}

/**
 * A token that can name a rule (`CSP`, `CSP-D101`, `E501`, `unused-import`).
 * The first token that cannot is where the explanation begins.
 */
function isRuleCode(token: string): boolean {
  return (
    /^[A-Za-z][A-Za-z0-9]*(?:-[A-Za-z0-9]+)*$/.test(token) &&
    (token.toUpperCase() === "CSP" || token.includes("-") || /\d/.test(token))
  );
}

/** Parses the suppression directive in a comment starting at `column`. */
export function parseDirective(
  comment: string,
  column = 0,
): Directive | undefined {
  const match = DIRECTIVE_RE.exec(comment);
  if (!match) {
    return undefined;
  }
  if (match[1] !== undefined) {
    return { suppression: { kind: "all" } };
  }
  const codesText = match[2];
  if (codesText === undefined) {
    // Bare `# noqa` or `# ignore`.
    return { suppression: { kind: "all" } };
  }
  const codesStart = column + match.index + match[0].length - codesText.length;
  if (codesText.trim() === "") {
    return { suppression: { kind: "all" }, codesStart };
  }
  // `# noqa: CSP-D101, E501 reason` lists codes, then explains them.
  const codes = new Set<string>();
  let codesEnd: number | undefined;
  for (const token of codesText.matchAll(/[^,\s]+/g)) {
    if (!isRuleCode(token[0])) {
      break;
    }
    const code = token[0].toUpperCase();
    if (code === "CSP") {
      return { suppression: { kind: "all" }, codesStart };
    }
    codes.add(code);
    codesEnd = codesStart + token.index! + token[0].length;
  }
  return {
    suppression: codes.size > 0 ? { kind: "specific", codes } : undefined,
    codesStart,
    codesEnd,
  };
}

/** Returns the suppression directive on each line (0-based) of `source`. */
export function directivesByLine(source: string): Map<number, Directive> {
  const directives = new Map<number, Directive>();
  for (const [line, comment] of commentsByLine(source)) {
    const directive = parseDirective(comment.text, comment.column);
    if (directive) {
      directives.set(line, directive);
    }
  }
  return directives;
}

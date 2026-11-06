import * as path from "path";
import { getCacheKey } from "./extensionCache";

function contains(root: string, filePath: string): boolean {
  const relative = path.relative(getCacheKey(root), getCacheKey(filePath));
  return !relative.startsWith("..") && !path.isAbsolute(relative);
}

/**
 * The deepest of `roots` (as `cytoscnpy projects` lists them) containing
 * `filePath`: the project whose scan, with its own configuration, owns it.
 */
export function ownerRoot(filePath: string, roots: string[]): string | undefined {
  let owner: string | undefined;
  for (const root of roots) {
    if (contains(root, filePath) && (!owner || root.length > owner.length)) {
      owner = root;
    }
  }
  return owner;
}

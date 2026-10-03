import * as fs from "fs";
import * as path from "path";
import { RawDeclaredDependency } from "./analyzerTypes";

export function dependencyAnchorPath(target: string): string {
  try {
    if (fs.existsSync(target) && fs.statSync(target).isDirectory()) {
      for (const filename of [
        "pyproject.toml",
        ".cytoscnpy.toml",
        "requirements.txt",
        "requirements-dev.txt",
      ]) {
        const candidate = path.join(target, filename);
        if (fs.existsSync(candidate)) {
          return candidate;
        }
      }
    }
  } catch {
    // Keep diagnostics visible even if the target cannot be inspected.
  }
  return target;
}

export function dependencySourcePath(
  dep: RawDeclaredDependency,
  anchorPath: string,
): string {
  const source = dep.source;
  const isDir = pathIsDirectory(anchorPath);
  const baseDir = isDir ? anchorPath : path.dirname(anchorPath);

  if (source === "Pyproject") {
    return path.join(baseDir, "pyproject.toml");
  }
  if (source && typeof source === "object" && "Requirements" in source) {
    return path.isAbsolute(source.Requirements)
      ? source.Requirements
      : path.join(baseDir, source.Requirements);
  }
  return isDir ? path.join(anchorPath, "pyproject.toml") : anchorPath;
}

export function pathIsDirectory(target: string): boolean {
  try {
    return fs.existsSync(target) && fs.statSync(target).isDirectory();
  } catch {
    return false;
  }
}

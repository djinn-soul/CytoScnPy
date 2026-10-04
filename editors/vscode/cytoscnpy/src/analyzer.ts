import { spawn } from "child_process";
import * as path from "path";

export * from "./analyzerTypes";
export { transformRawResult } from "./analyzerResult";
import { transformRawResult } from "./analyzerResult";
import { dependencyAnchorPath } from "./analyzerPaths";
import {
  CytoScnPyConfig,
  CytoScnPyAnalysisResult,
  RawCytoScnPyResult,
  WorkspaceAnalysisResult,
  CytoScnPyFinding,
  ParseError,
} from "./analyzerTypes";

/**
 * Runs the analyzer and collects stdout via a streaming pipe instead of
 * `execFile`'s in-memory buffer. Workspaces with many findings used to throw
 * `ERR_CHILD_PROCESS_STDIO_MAXBUFFER` once stdout crossed 50MB; the streaming
 * variant has no fixed cap.
 */
function runAnalyzerStreaming(
  binaryPath: string,
  args: string[],
): Promise<{ stdout: string; stderr: string; code: number | null }> {
  return new Promise((resolve, reject) => {
    const child = spawn(binaryPath, args, {
      windowsHide: true,
      shell: false,
    });
    const stdoutChunks: Buffer[] = [];
    const stderrChunks: Buffer[] = [];
    child.stdout.on("data", (chunk: Buffer) => stdoutChunks.push(chunk));
    child.stderr.on("data", (chunk: Buffer) => stderrChunks.push(chunk));
    child.on("error", reject);
    child.on("close", (code) => {
      resolve({
        stdout: Buffer.concat(stdoutChunks).toString("utf8"),
        stderr: Buffer.concat(stderrChunks).toString("utf8"),
        code,
      });
    });
  });
}

/**
 * Builds the CLI argument vector shared by single-file and workspace analysis.
 * Exported for test snapshotting — keep deterministic and pure.
 */
export function buildAnalyzerArgs(
  target: string,
  config: CytoScnPyConfig,
): string[] {
  const args: string[] = ["--client", "vscode", target, "--json"];

  if (config.enableSecretsScan) {
    args.push("--secrets");
  }
  if (config.enableDangerScan) {
    args.push("--danger");
  }
  if (config.enableCloneScan) {
    args.push("--clones");
  }
  if (
    config.confidenceThreshold !== undefined &&
    config.confidenceThreshold > 0
  ) {
    args.push("--confidence", config.confidenceThreshold.toString());
  }
  if (config.excludeFolders && config.excludeFolders.length > 0) {
    for (const folder of config.excludeFolders) {
      args.push("--exclude-folders", folder);
    }
  }
  if (config.includeFolders && config.includeFolders.length > 0) {
    for (const folder of config.includeFolders) {
      args.push("--include-folders", folder);
    }
  }
  if (config.includeTests) {
    args.push("--include-tests");
  }
  if (config.includeIpynb) {
    args.push("--include-ipynb");
  }

  if (config.enableQualityScan) {
    args.push("--quality");
    if (config.maxComplexity !== undefined) {
      args.push("--max-complexity", config.maxComplexity.toString());
    }
    if (config.minMaintainabilityIndex !== undefined) {
      args.push("--min-mi", config.minMaintainabilityIndex.toString());
    }
    if (config.maxNesting !== undefined) {
      args.push("--max-nesting", config.maxNesting.toString());
    }
    if (config.maxArguments !== undefined) {
      args.push("--max-args", config.maxArguments.toString());
    }
    if (config.maxLines !== undefined) {
      args.push("--max-lines", config.maxLines.toString());
    }
  }

  return args;
}

export async function runCytoScnPyAnalysis(
  filePath: string,
  config: CytoScnPyConfig,
): Promise<CytoScnPyAnalysisResult> {
  const args = buildAnalyzerArgs(filePath, config);
  let stderr = "";
  let code: number | null = null;
  try {
    const output = await runAnalyzerStreaming(config.path, args);
    stderr = output.stderr;
    code = output.code;
    if (stderr) {
      console.warn(`CytoScnPy analysis for ${filePath} produced stderr: ${stderr}`);
    }
    const rawResult: RawCytoScnPyResult = JSON.parse(output.stdout.trim());
    return transformRawResult(rawResult, dependencyAnchorPath(filePath));
  } catch (error: unknown) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(
      `CytoScnPy analysis failed for ${filePath} (exit ${code}): ${message}. Stderr: ${stderr}`,
      { cause: error },
    );
  }
}

/**
 * Run workspace-level analysis and return findings grouped by file path.
 * This provides cross-file reference tracking for accurate unused code detection.
 */
function normalizeAbsolutePath(filePath: string, workspacePath: string): string {
  let resolved = filePath;
  if (!path.isAbsolute(resolved)) {
    resolved = path.resolve(workspacePath, resolved);
  }
  if (process.platform === "win32") {
    resolved = resolved.toLowerCase();
  }
  return resolved;
}

export async function runWorkspaceAnalysis(
  workspacePath: string,
  config: CytoScnPyConfig,
): Promise<WorkspaceAnalysisResult> {
  const args = buildAnalyzerArgs(workspacePath, config);

  // Streaming pipe — large workspaces routinely exceeded the prior 50MB
  // `execFile` cap once secrets/danger output was enabled.
  let stderr = "";
  let code: number | null = null;
  try {
    const output = await runAnalyzerStreaming(config.path, args);
    stderr = output.stderr;
    code = output.code;
    if (code !== 0 && !output.stdout.trim()) {
      throw new Error(`Workspace analysis failed (exit ${code})`);
    }
    const rawResult: RawCytoScnPyResult = JSON.parse(output.stdout.trim());
    const result = transformRawResult(rawResult, dependencyAnchorPath(workspacePath));

    const findingsByFile = new Map<string, CytoScnPyFinding[]>();
    for (const finding of result.findings) {
      const filePath = normalizeAbsolutePath(finding.file_path, workspacePath);
      if (!findingsByFile.has(filePath)) {
        findingsByFile.set(filePath, []);
      }
      findingsByFile.get(filePath)!.push(finding);
    }

    const parseErrorsByFile = new Map<string, ParseError[]>();
    for (const parseError of result.parseErrors) {
      const filePath = normalizeAbsolutePath(parseError.file, workspacePath);
      if (!parseErrorsByFile.has(filePath)) {
        parseErrorsByFile.set(filePath, []);
      }
      parseErrorsByFile.get(filePath)!.push(parseError);
    }

    return { findingsByFile, parseErrorsByFile };
  } catch (error: unknown) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(
      `CytoScnPy workspace analysis failed for ${workspacePath} (exit ${code}): ${message}. Stderr: ${stderr}`,
      { cause: error },
    );
  }
}

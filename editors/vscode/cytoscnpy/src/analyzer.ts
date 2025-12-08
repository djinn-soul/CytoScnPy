import { exec } from 'child_process';

interface CytoScnPyFinding {
    file_path: string;
    line_number: number;
    message: string;
    rule_id: string;
    severity: 'error' | 'warning' | 'info';
}

interface CytoScnPyAnalysisResult {
    findings: CytoScnPyFinding[];
}

export interface CytoScnPyConfig {
    path: string;
    enableSecretsScan: boolean;
    enableDangerScan: boolean;
    enableQualityScan: boolean;
    confidenceThreshold: string;
}

// This is the structure of the raw output from the cytoscnpy tool
interface RawCytoScnPyFinding {
    file: string;
    line: number;
    message?: string;
    rule_id?: string;
    severity?: string;
    name?: string; 
}

interface RawCytoScnPyResult {
    unused_functions?: RawCytoScnPyFinding[];
    unused_imports?: RawCytoScnPyFinding[];
    unused_classes?: RawCytoScnPyFinding[];
    unused_variables?: RawCytoScnPyFinding[];
    unused_parameters?: RawCytoScnPyFinding[];
    secrets?: RawCytoScnPyFinding[];
    danger?: RawCytoScnPyFinding[];
    quality?: RawCytoScnPyFinding[];
}

function transformRawResult(rawResult: RawCytoScnPyResult): CytoScnPyAnalysisResult {
    const findings: CytoScnPyFinding[] = [];

    const normalizeSeverity = (severity: string | undefined): 'error' | 'warning' | 'info' => {
        switch (severity?.toUpperCase()) {
            case 'HIGH':
                return 'error';
            case 'MEDIUM':
                return 'warning';
            case 'LOW':
                return 'info';
            default:
                return 'warning';
        }
    };

    const processCategory = (category: RawCytoScnPyFinding[] | undefined, defaultRuleId: string, defaultMessagePrefix: string, defaultSeverity: 'error' | 'warning' | 'info') => {
                if (!category) {
            return;
        }

        for (const rawFinding of category) {
            findings.push({
                file_path: rawFinding.file,
                line_number: rawFinding.line,
                message: rawFinding.message || `${defaultMessagePrefix}: ${rawFinding.name}`,
                rule_id: rawFinding.rule_id || defaultRuleId,
                severity: normalizeSeverity(rawFinding.severity) || defaultSeverity,
            });
        }
    };

    processCategory(rawResult.unused_functions, 'unused-function', 'Unused function', 'warning');
    processCategory(rawResult.unused_imports, 'unused-import', 'Unused import', 'warning');
    processCategory(rawResult.unused_classes, 'unused-class', 'Unused class', 'warning');
    processCategory(rawResult.unused_variables, 'unused-variable', 'Unused variable', 'warning');
    processCategory(rawResult.unused_parameters, 'unused-parameter', 'Unused parameter', 'warning');
    processCategory(rawResult.secrets, 'secret-detected', 'Secret detected', 'error');
    processCategory(rawResult.danger, 'dangerous-code', 'Dangerous code detected', 'error');
    processCategory(rawResult.quality, 'quality-issue', 'Quality issue detected', 'warning');
    
    return { findings };
}


export function runCytoScnPyAnalysis(filePath: string, config: CytoScnPyConfig): Promise<CytoScnPyAnalysisResult> {
    return new Promise((resolve, reject) => {
        let command = `${config.path} "${filePath}" --json`;

        if (config.enableSecretsScan) {
            command += ' --secrets';
        }
        if (config.enableDangerScan) {
            command += ' --danger';
        }
        if (config.enableQualityScan) {
            command += ' --quality';
        }
        if (config.confidenceThreshold !== 'all') {
            command += ` --confidence ${config.confidenceThreshold}`;
        }
        
        exec(command, (error, stdout, stderr) => {
            if (error) {
                console.error(`CytoScnPy analysis failed for ${filePath}: ${error.message}`);
                console.error(`Stderr: ${stderr}`);
                try {
                    const rawResult: RawCytoScnPyResult = JSON.parse(stdout.trim());
                    const result = transformRawResult(rawResult);
                    resolve(result);
                } catch (parseError) {
                    reject(new Error(`Failed to run CytoScnPy analysis: ${error.message}. Stderr: ${stderr}`));
                }
                return;
            }

            if (stderr) {
                console.warn(`CytoScnPy analysis for ${filePath} produced stderr: ${stderr}`);
            }

            try {
                const rawResult: RawCytoScnPyResult = JSON.parse(stdout.trim());
                const result = transformRawResult(rawResult);
                resolve(result);
            } catch (parseError: any) {
                reject(new Error(`Failed to parse CytoScnPy JSON output for ${filePath}: ${parseError.message}. Output: ${stdout}`));
            }
        });
    });
}

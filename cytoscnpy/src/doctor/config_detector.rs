use super::types::{ConfigCategory, DetectedConfig};
use std::fs;
use std::path::{Path, PathBuf};

struct ConfigPattern {
    names: &'static [&'static str],
    category: ConfigCategory,
}

const CONFIG_PATTERNS: &[ConfigPattern] = &[
    ConfigPattern {
        names: &[
            "ruff.toml",
            ".ruff.toml",
            ".prettierrc",
            ".prettierrc.json",
            ".prettierrc.yml",
            ".prettierrc.yaml",
            ".prettierrc.js",
            "prettier.config.js",
            "rustfmt.toml",
            ".rustfmt.toml",
            ".clang-format",
        ],
        category: ConfigCategory::Formatter,
    },
    ConfigPattern {
        names: &[
            "ruff.toml",
            ".ruff.toml",
            ".flake8",
            ".pylintrc",
            "pylintrc",
            ".eslintrc",
            ".eslintrc.json",
            ".eslintrc.js",
            "eslint.config.js",
            "eslint.config.mjs",
            ".clippy.toml",
            "clippy.toml",
        ],
        category: ConfigCategory::Linter,
    },
    ConfigPattern {
        names: &[
            "mypy.ini",
            ".mypy.ini",
            "pyrightconfig.json",
            "tsconfig.json",
            "jsconfig.json",
        ],
        category: ConfigCategory::TypeChecker,
    },
    ConfigPattern {
        names: &[
            "pytest.ini",
            "conftest.py",
            "jest.config.js",
            "jest.config.ts",
            "vitest.config.js",
            "vitest.config.ts",
        ],
        category: ConfigCategory::TestFramework,
    },
    ConfigPattern {
        names: &[
            ".github/workflows",
            ".gitlab-ci.yml",
            ".circleci",
            "Jenkinsfile",
            "azure-pipelines.yml",
            ".travis.yml",
        ],
        category: ConfigCategory::CI,
    },
    ConfigPattern {
        names: &[
            "Dockerfile",
            "docker-compose.yml",
            "docker-compose.yaml",
            "compose.yml",
            "compose.yaml",
            ".dockerignore",
        ],
        category: ConfigCategory::Docker,
    },
    ConfigPattern {
        names: &[
            "pyproject.toml",
            "setup.py",
            "setup.cfg",
            "requirements.txt",
            "Pipfile",
            "Cargo.toml",
            "package.json",
            "go.mod",
        ],
        category: ConfigCategory::DependencyManager,
    },
    ConfigPattern {
        names: &[
            "uv.lock",
            "poetry.lock",
            "Pipfile.lock",
            "requirements.lock",
            "Cargo.lock",
            "package-lock.json",
            "pnpm-lock.yaml",
            "yarn.lock",
        ],
        category: ConfigCategory::Lockfile,
    },
    ConfigPattern {
        names: &[
            "Makefile",
            "makefile",
            "Justfile",
            "justfile",
            "Taskfile.yml",
            "build.py",
        ],
        category: ConfigCategory::BuildScript,
    },
    ConfigPattern {
        names: &[
            "README.md",
            "README.rst",
            "README.txt",
            "README",
            "CONTRIBUTING.md",
            "ARCHITECTURE.md",
            "docs",
        ],
        category: ConfigCategory::Documentation,
    },
    ConfigPattern {
        names: &[".gitignore"],
        category: ConfigCategory::GitIgnore,
    },
    ConfigPattern {
        names: &[".editorconfig", ".vscode"],
        category: ConfigCategory::Editor,
    },
];

fn is_valid_config_path(category: ConfigCategory, name: &str, path: &Path) -> bool {
    if !path.exists() {
        return false;
    }
    if category == ConfigCategory::CI {
        if name == ".github/workflows" {
            if let Ok(entries) = fs::read_dir(path) {
                return entries.filter_map(Result::ok).any(|e| {
                    let p = e.path();
                    p.is_file()
                        && p.extension()
                            .is_some_and(|ext| ext == "yml" || ext == "yaml")
                });
            }
            return false;
        }
        if name == ".circleci" {
            return path.join("config.yml").is_file() || path.join("config.yaml").is_file();
        }
    }
    true
}

/// Scans the repository root for known configuration, tooling, and setup files.
pub fn detect_configurations(repo_root: &Path) -> Vec<DetectedConfig> {
    let mut detected = Vec::new();

    for pattern in CONFIG_PATTERNS {
        for &name in pattern.names {
            let path = repo_root.join(name);
            if is_valid_config_path(pattern.category, name, &path) {
                let display_name = PathBuf::from(name)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(name)
                    .to_owned();

                detected.push(DetectedConfig {
                    category: pattern.category,
                    name: display_name,
                    path: path.clone(),
                    details: None,
                });
            }
        }
    }

    detected
}

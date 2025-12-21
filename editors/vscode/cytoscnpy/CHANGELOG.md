# Change Log

All notable changes to the "cytoscnpy" extension will be documented in this file.

Check [Keep a Changelog](http://keepachangelog.com/) for recommendations on how to structure this file.

## [0.1.2] - 2025-12-19

### Added

- **MCP Server Integration**: Automatic registration with GitHub Copilot via `mcpServerDefinitionProviders`
- **Copilot Support**: Ask Copilot to use CytoScnPy tools directly (e.g., "Run a quick security scan using CytoScnPy")
- **New MCP Tool**: `quick_scan` for fast security-focused analysis (secrets & dangerous patterns only)

### Changed

- Extension now spawns bundled CLI binary in MCP mode for Copilot integration
- Improved MCP tool descriptions for better LLM context

### Fixed

- Removed distracting "14 issues" badge from Explorer icon

## [0.1.0] - 2025-12-08

### Added

- Initial release of CytoScnPy VS Code extension
- Real-time dead code analysis for Python files
- Security scanning (secrets, dangerous code patterns)
- Taint analysis (SQL injection, command injection, code execution)
- Code quality metrics (Cyclomatic Complexity, Halstead, Maintainability Index)
- Inline diagnostics with severity-based coloring
- Commands: Analyze Current File, Calculate CC, Calculate Halstead, Calculate MI
- Bundled Windows binary (`cytoscnpy-cli-win32.exe`)
- Configurable settings for security scans and confidence thresholds

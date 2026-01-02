# History

## Development History

### Phase 8: Advanced Framework Support & CFG Integration

**Status: Done**

**Features:**

- **Control Flow Graph (CFG) Integration:**

  - Full CFG construction from Python AST.
  - Control flow handling for `if`, `for`, `while`, `try`, `match`, etc.
  - CFG-based clone detection validation (secondary filter).
  - Enabled via `--features cfg`.

- **Framework Support:**
  - Dedicated support for Django, FastAPI, and Pydantic patterns.
  - Reduced false positives for framework-specific constructs.

### Phase 7.6: Accuracy Improvements (In Progress/Partial)

**Status: In Progress**

**Completed Fixes:**

- Return annotations and `TYPE_CHECKING` block handling.
- Class-Method Linking: Automatic detection of methods within unused classes.
- Improved variable scope analysis.

### Phase 7.5: Performance Optimizations

**Status: Done**

**Optimizations:**

- Reduced unnecessary `.clone()` calls.
- Improved string handling and interning (planned/partial).
- Profile-Guided Optimization support.

### Phase 7: Infrastructure & Quality

**Status: Done**

**Improvements:**

- **Parser Migration:** Migrated from `rustpython-parser` to `ruff_python_parser` for better performance and CST support.
- **Error Handling:** Enhanced error reporting and context.

### Phase 6: Editor Integration

**Status: Done**

**Features:**

- **VS Code Extension:**
  - Full integration with diagnostics.
  - Quick fixes for dead code removal.
  - Sidebar badge and findings explorer.
  - File caching for performance.
- **UX Enhancements:**
  - Gutter decorations for severity.
  - Problem grouping.

## Legacy & Deprecated

- **rustpython-parser:** Replaced by `ruff_python_parser`.
- **Legacy CLI flags:** Some older flags may be deprecated in favor of subcommand structure (check `CLI.md` for current syntax).

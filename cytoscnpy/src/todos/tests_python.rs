//! Unit tests for Python script detection and commented-out import heuristics.

use super::run;

#[test]
fn skips_debug_print_in_main_guarded_script() {
    let r = run(
        "tools/report.py",
        "def main():\n    print('done')\n\nif __name__ == \"__main__\":\n    main()\n",
    );
    assert_eq!(r.stats.debug_print_count, 0);
}

#[test]
fn skips_debug_print_in_argparse_script() {
    let r = run("tools/report.py", "import argparse\nprint('usage')\n");
    assert_eq!(r.stats.debug_print_count, 0);
}

#[test]
fn floor_division_is_not_a_comment() {
    let r = run("src/math_ops.py", "half = total // 2  # TODO: round\n");
    assert_eq!(r.stats.todo_count, 1);
    assert_eq!(r.stats.commented_code_count, 0);
}

#[test]
fn detects_commented_out_imports() {
    let r = run(
        "src/engine.py",
        "# import os.path as osp\n# from . import models, views\n",
    );
    assert_eq!(r.stats.commented_code_count, 2);
}

#[test]
fn does_not_flag_import_prose() {
    let r = run(
        "src/engine.py",
        "# from X import Y pattern\n# import X pattern\n",
    );
    assert_eq!(r.stats.commented_code_count, 0);
}

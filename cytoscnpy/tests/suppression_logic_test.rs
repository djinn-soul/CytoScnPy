//! Tests for the enhanced suppression logic (noqa, ignore, and pragma).
use cytoscnpy::utils::get_ignored_lines;

#[test]
fn test_suppression_logic_scenarios() {
    // 1. Multiple tools in one noqa
    let s1 = "x = 1  # noqa: E501, W291, CSP";
    assert!(get_ignored_lines(s1).contains(&1));

    // 2. Mixed case variants
    let s2 = "x = 1  # NOQA\ny = 1  # NoQa\nz = 1  # noqa : CSP";
    let res2 = get_ignored_lines(s2);
    assert!(res2.contains(&1));
    assert!(res2.contains(&2));
    assert!(res2.contains(&3));

    // 3. noqa with extra comments
    let s3 = "x = 1  # noqa: CSP  -- false positive";
    assert!(get_ignored_lines(s3).contains(&1));

    // 4. noqa not at end of line (but in comment)
    let s4 = "x = 1  # noqa  this is intentional";
    assert!(get_ignored_lines(s4).contains(&1));

    // 5. Bare ignore
    let s5 = "x = 1  # ignore";
    assert!(get_ignored_lines(s5).contains(&1));

    // 6. pragma + noqa together
    let s6 = "x = 1  # pragma: no cytoscnpy # noqa";
    assert!(get_ignored_lines(s6).contains(&1));

    // 7. noqa for a different tool only -> Should NOT ignore
    let s7 = "x = 1  # noqa: E501";
    assert!(!get_ignored_lines(s7).contains(&1));

    // 8. Multiple ignores with CSP
    let s8 = "x = 1; y = 2  # noqa: CSP, E501";
    assert!(get_ignored_lines(s8).contains(&1));

    // 9. ignore with CSP code
    let s9 = "x = 1  # ignore: CSP";
    assert!(get_ignored_lines(s9).contains(&1));

    // 10. Wrong or unknown codes -> Should NOT ignore
    let s10 = "x = 1  # noqa: XYZ123";
    assert!(!get_ignored_lines(s10).contains(&1));

    // 11. User special case:
    let s_user = "c = 'ad' #noqa: R102, CSP, dont implemen any thing";
    assert!(get_ignored_lines(s_user).contains(&1));
}

#[test]
fn test_pragma_legacy() {
    let s = "def f(): # pragma: no cytoscnpy\n    pass";
    assert!(get_ignored_lines(s).contains(&1));
}

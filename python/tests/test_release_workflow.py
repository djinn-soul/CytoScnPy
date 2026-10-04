"""Exercise the release workflow's production guard and macOS rename script."""

import re
import shutil
import subprocess
from pathlib import Path

import pytest

WORKFLOW = Path(__file__).resolve().parents[2] / ".github/workflows/publish.yml"


def job_body(name: str) -> str:
    """Read one top-level job without a YAML parser dependency."""
    text = WORKFLOW.read_text(encoding="utf-8")
    match = re.search(
        rf"^  {name}:\n(.*?)(?=^  [\w-]+:|\Z)", text, re.MULTILINE | re.DOTALL
    )
    assert match is not None
    return match.group(1)


@pytest.mark.parametrize("job", ["publish-pypi", "github-release-prod"])
def test_production_jobs_exclude_all_prerelease_suffixes(job: str) -> None:
    """Both production jobs require tag refs without any hyphen suffix."""
    condition = re.search(r"^    if: (.+)$", job_body(job), re.MULTILINE)
    assert condition is not None
    assert condition.group(1) == (
        "startsWith(github.ref, 'refs/tags/') && !contains(github.ref_name, '-')"
    )


@pytest.mark.parametrize("missing_arm64", [False, True])
def test_macos_artifacts_are_distinct_and_missing_binary_fails(
    tmp_path: Path, *, missing_arm64: bool
) -> None:
    """Run the actual rename step with distinct executable contents."""
    job = job_body("github-release-prod")
    for architecture in ["x64", "arm64"]:
        assert re.search(
            rf"name: cli-macos-{architecture}\n\s+path: macos-assets/{architecture}/",
            job,
        )
        directory = tmp_path / "macos-assets" / architecture
        directory.mkdir(parents=True)
        if architecture != "arm64" or not missing_arm64:
            (directory / "cytoscnpy-cli").write_text(architecture, encoding="utf-8")
    (tmp_path / "release-assets").mkdir()
    match = re.search(
        r"- name: Rename macOS binaries\n\s+run: \|\n(.*?)(?=\n\n)", job, re.DOTALL
    )
    assert match is not None
    script = "\n".join(line.strip() for line in match.group(1).splitlines())
    bash = shutil.which("bash")
    assert bash is not None
    # Execute the repository workflow step with no interpolated user input.
    result = subprocess.run(  # noqa: S603
        [bash, "-e", "-c", script], cwd=tmp_path, capture_output=True, check=False
    )
    if missing_arm64:
        assert result.returncode != 0
    else:
        assert result.returncode == 0, result.stderr
        for architecture in ["x64", "arm64"]:
            artifact = (
                tmp_path / "release-assets" / f"cytoscnpy-cli-darwin-{architecture}"
            )
            assert artifact.read_text(encoding="utf-8") == architecture

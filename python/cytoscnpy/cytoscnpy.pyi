from collections.abc import Sequence

def run(args: Sequence[str]) -> int: ...
def scan_json(
    paths: Sequence[str] = ...,
    confidence: int | None = None,
    secrets: bool | None = None,
    danger: bool | None = None,
    quality: bool | None = None,
    include_tests: bool | None = None,
    exclude_folders: Sequence[str] = ...,
    include_folders: Sequence[str] = ...,
    include_ipynb: bool | None = None,
    clones: bool | None = None,
    clone_similarity: float | None = None,
) -> str: ...
def scan_code_json(
    code: str,
    filename: str = ...,
    confidence: int = 60,
    secrets: bool = True,
    danger: bool = True,
    quality: bool = True,
) -> str: ...

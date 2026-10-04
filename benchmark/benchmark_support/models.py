from __future__ import annotations

from collections.abc import Callable, Iterable, Mapping
from typing import Protocol, TypedDict

Finding = tuple[str, int | None, str, str]


class ToolStatus(TypedDict):
    available: bool
    reason: str


class ToolConfigRequired(TypedDict):
    name: str
    command: list[str]


class ToolConfig(ToolConfigRequired, total=False):
    env: Mapping[str, str]
    cwd: str


ToolCheckResults = dict[str, ToolStatus]


class ToolCheckInfo(TypedDict):
    module: str
    arg: str


class BenchmarkResult(TypedDict):
    name: str
    time: float
    memory_mb: float
    issues: int
    output: str
    stdout: str


class MetricResult(TypedDict):
    TP: int
    FP: int
    FN: int
    Precision: float
    Recall: float
    F1: float
    missed_items: list[str]


VerificationValue = MetricResult | str
VerificationResult = dict[str, VerificationValue]
MetricResults = dict[str, MetricResult]


class FinalReportEntry(TypedDict):
    name: str
    time: float
    memory_mb: float
    issues: int
    f1_score: float
    precision: float
    recall: float
    stats: Mapping[str, object]


class FinalReport(TypedDict):
    timestamp: float
    platform: str
    results: list[FinalReportEntry]


class Args(Protocol):
    list: bool
    check: bool
    include: list[str] | None
    exclude: list[str] | None
    save_json: str | None
    compare_json: str | None
    threshold: float


class MemoryInfo(Protocol):
    rss: int


class PsutilProcessLike(Protocol):
    def memory_info(self) -> MemoryInfo: ...

    def children(self, *, recursive: bool = ...) -> Iterable[PsutilProcessLike]: ...


class PsutilModule(Protocol):
    NoSuchProcess: type[BaseException]
    AccessDenied: type[BaseException]

    # psutil.Process is a callable/class; model it as an attribute to avoid
    # style warnings about method naming while keeping the external API shape.
    Process: Callable[[int], PsutilProcessLike]

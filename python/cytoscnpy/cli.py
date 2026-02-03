import sys

from cytoscnpy import (
    run,  # type: ignore[reportAttributeAccessIssue, reportUnknownVariableType]
)


def main() -> None:
    """Main entry point for CLI."""
    args = sys.argv[1:]
    try:
        rc: int = run(args)  # type: ignore[reportUnknownVariableType, reportUnknownArgumentType]
        raise SystemExit(int(rc))  # type: ignore[reportUnknownArgumentType]
    except KeyboardInterrupt:
        raise SystemExit(130) from None
    except Exception as e:
        print(f"cytoscnpy error: {e}", file=sys.stderr)  # noqa: T201
        raise SystemExit(1) from e


if __name__ == "__main__":
    main()

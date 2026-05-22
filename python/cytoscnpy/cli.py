import sys

from . import run


def main() -> None:
    """Main entry point for CLI."""
    args = sys.argv[1:]
    try:
        rc = run(args)
        raise SystemExit(int(rc))
    except KeyboardInterrupt:
        raise SystemExit(130) from None
    except Exception as e:
        print(f"cytoscnpy error: {e}", file=sys.stderr)  # noqa: T201
        raise SystemExit(1) from e


if __name__ == "__main__":
    main()

import sys
from cytoscnpy import run


def main():
    """Main entry point for CLI."""
    args = sys.argv[1:]
    try:
        rc = run(args)
        raise SystemExit(int(rc))
    except KeyboardInterrupt:
        raise SystemExit(130)
    except Exception as e:
        print(f"cytoscnpy error: {e}", file=sys.stderr)
        raise SystemExit(1)


if __name__ == "__main__":
    main()

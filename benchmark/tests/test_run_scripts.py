import json
import sys
from pathlib import Path
from unittest.mock import MagicMock, patch

# Add benchmark directory to sys.path
BENCHMARK_DIR = Path(__file__).resolve().parent.parent
sys.path.append(str(BENCHMARK_DIR))

import run_baseline_benchmark  # noqa: E402
import run_benchmarks  # noqa: E402


def test_run_benchmarks_main(tmp_path):
    """Test run_benchmarks.py main flow."""
    # Setup mocks
    mock_run = MagicMock()
    mock_run.returncode = 0

    with patch("sys.argv", ["script", "--skip-regression"]):
        # Mock subprocess.run for hyperfine check and execution (if any leaked)
        with patch("subprocess.run", return_value=mock_run):
            # Mock get_cytoscnpy_binary
            with patch(
                "run_benchmarks.get_cytoscnpy_binary", return_value=Path("mock_bin")
            ):
                # Mock get_file_stats
                with patch("run_benchmarks.get_file_stats", return_value=(10, 1000)):
                    # Mock run_hyperfine_benchmark
                    with patch(
                        "run_benchmarks.run_hyperfine_benchmark",
                        return_value={
                            "results": [
                                {"mean": 1.0, "stddev": 0.1, "min": 0.9, "max": 1.1}
                            ]
                        },
                    ):
                        # Mock run_comparison
                        with patch("run_benchmarks.run_comparison") as mock_comp:
                            # Mock os.chdir to avoid changing test directory
                            with patch("os.chdir"):
                                # Mock Path.exists and iterdir to simulate datasets
                                with patch("pathlib.Path.exists", return_value=True):
                                    with patch(
                                        "pathlib.Path.is_dir", return_value=True
                                    ):
                                        # Make iterdir return a few mock paths
                                        mock_p = MagicMock(spec=Path)
                                        mock_p.name = "dataset1"
                                        mock_p.is_dir.return_value = True

                                        with patch(
                                            "pathlib.Path.iterdir",
                                            return_value=[mock_p],
                                        ):
                                            with patch("run_benchmarks.save_results"):
                                                with patch("builtins.print"):
                                                    run_benchmarks.main()

                                            mock_comp.assert_called()


def test_run_baseline_benchmark(tmp_path):
    """Test run_baseline_benchmark.py main flow."""
    # Create valid dummy corpus and binary
    corpus = tmp_path / "corpus"
    corpus.mkdir()
    (corpus / "test.py").write_text("print('hello')")

    binary = tmp_path / "cytoscnpy"
    binary.touch()

    # We need to rely on run_benchmark returning a dict
    dummy_result = {
        "avg_time": 0.1,
        "stddev": 0.01,
        "min_time": 0.09,
        "max_time": 0.11,
        "peak_memory_mb": [10.0],
        "avg_memory_mb": 10.0,
        "num_files": 1,
        "throughput_files_per_sec": 100,
        "command": "cmd",
        "flags": [],
    }

    with patch(
        "sys.argv",
        ["script", "--corpus", str(corpus), "--binary", str(binary), "--quick"],
    ):
        with patch(
            "run_baseline_benchmark.run_benchmark", return_value=dummy_result
        ) as mock_run:
            with patch("builtins.print"):
                with patch("run_baseline_benchmark.save_results"):
                    run_baseline_benchmark.main()

            mock_run.assert_called()


def test_get_cytoscnpy_binary(tmp_path):
    """Test binary resolution."""
    # Test fallback by ensuring NOTHING exists
    with patch("pathlib.Path.exists", return_value=False):
        p = run_benchmarks.get_cytoscnpy_binary()
        assert p.name == "cytoscnpy"

    # Test finding it
    with patch("pathlib.Path.exists", side_effect=[True]):
        p = run_benchmarks.get_cytoscnpy_binary()
        assert "target" in str(p) or "cytoscnpy-bin" in str(p)


def test_run_benchmark_inner(tmp_path):
    """Test actual run_benchmark logic in run_baseline_benchmark.py."""
    corpus = tmp_path / "corpus"
    binary = tmp_path / "bin"

    # Mock psutil and subprocess.Popen
    with patch("psutil.Process") as mock_psutil:
        mock_proc = MagicMock()
        mock_proc.memory_info.return_value.rss = 1024 * 1024 * 50  # 50 MB
        mock_proc.children.return_value = []
        mock_psutil.return_value = mock_proc

        with patch("subprocess.Popen") as mock_popen:
            process_mock = MagicMock()

            # Use a side effect function to ensure it doesn't loop infinitely
            def poll_se():
                if not hasattr(poll_se, "called"):
                    poll_se.called = True
                    return None
                return 0

            process_mock.poll.side_effect = poll_se
            process_mock.communicate.return_value = ("", "")
            process_mock.returncode = 0
            process_mock.pid = 1234
            mock_popen.return_value = process_mock

            with patch("time.sleep"):  # Fast sleep
                with patch("builtins.print"):
                    result = run_baseline_benchmark.run_benchmark(
                        corpus, binary, iterations=1
                    )

            assert result["avg_time"] >= 0
            assert result["peak_memory_mb"][0] == 50.0


def test_run_hyperfine_benchmark(tmp_path):
    """Test hyperfine execution wrapper."""
    dataset = tmp_path / "dataset"
    binary = tmp_path / "bin"

    # Hyperfine output
    hf_json = {"results": [{"mean": 1.5, "stddev": 0.1, "min": 1.4, "max": 1.6}]}

    with patch("subprocess.run") as mock_run:
        mock_run.return_value.returncode = 0

        # We need to write the json output file that the script expects
        # The script does: json_output = Path(f"benchmark/results_{name}.json")
        # We should patch open or ensure the file exists.
        # It's better to patch open/json.load

        with patch("pathlib.Path.open", new_callable=MagicMock):
            with patch("json.load", return_value=hf_json):
                # We also need to patch exists to return True
                with patch("pathlib.Path.exists", return_value=True):
                    with patch("builtins.print"):
                        res = run_benchmarks.run_hyperfine_benchmark(
                            binary, dataset, runs=1
                        )

    assert res == hf_json


def test_run_comprehensive_benchmarks(tmp_path):
    """Test run_baseline_benchmark.run_comprehensive_benchmarks."""
    corpus = tmp_path
    binary = tmp_path / "bin"

    dummy_res = {
        "avg_time": 1.0,
        "stddev": 0.1,
        "min_time": 0.9,
        "max_time": 1.1,
        "avg_memory_mb": 10.0,
        "throughput_files_per_sec": 100,
    }

    with patch("run_baseline_benchmark.run_benchmark", return_value=dummy_res):
        with patch("builtins.print"):
            results = run_baseline_benchmark.run_comprehensive_benchmarks(
                corpus, binary
            )

    assert len(results) >= 5  # basic, secrets, etc.


def test_check_regression(tmp_path):
    """Test run_benchmarks.check_regression."""

    # Run BenchmarkResult dataclass
    from run_benchmarks import BenchmarkResult

    current = [BenchmarkResult("d1", 10, 100, 1.0, 0.1, 0.9, 1.1, 10)]
    baseline_file = tmp_path / "base.json"

    # 1. No baseline
    with patch("builtins.print"):
        assert not run_benchmarks.check_regression(current, baseline_file)

    # 2. Baseline exists, no regression
    baseline_data = {
        "results": [
            {
                "dataset": "d1",
                "mean_seconds": 1.2,
                "lines": 100,
            }  # Slower baseline -> no regression
        ]
    }
    baseline_file.write_text(json.dumps(baseline_data))

    with patch("builtins.print"):
        assert not run_benchmarks.check_regression(current, baseline_file)

    # 3. Regression
    baseline_data_reg = {
        "results": [
            {
                "dataset": "d1",
                "mean_seconds": 0.5,
                "lines": 100,
            }  # Faster baseline -> regression
        ]
    }
    baseline_file.write_text(json.dumps(baseline_data_reg))

    with patch("builtins.print"):
        assert run_benchmarks.check_regression(current, baseline_file, threshold=0.1)


def test_get_cytoscnpy_binary_variants(tmp_path):
    """Test various binary locations."""
    # 1. target/release/cytoscnpy-bin.exe
    with patch("pathlib.Path.exists") as mock_exists:
        # Side effect: True for first item in list (release/exe), False for others?
        # The function uses a list of paths.
        # We need to simulate True on specific calls.

        # It's easier to patch the class and specific instances?
        # Let's mock the list iteration inside the function simply by patching 'possible_paths' ?
        # No, 'possible_paths' is local variable.

        # We can use side_effect on exists().
        # Calls:
        # 1. target/release/cytoscnpy-bin.exe
        # 2. target/release/cytoscnpy-bin
        # 3. target/debug/cytoscnpy-bin.exe
        # 4. target/debug/cytoscnpy-bin

        # Case A: Found first
        mock_exists.side_effect = [True, False, False, False]
        p = run_benchmarks.get_cytoscnpy_binary()
        assert "release" in str(p) and "exe" in str(p)

    # Case B: Found last
    with patch("pathlib.Path.exists", side_effect=[False, False, False, True]):
        p = run_benchmarks.get_cytoscnpy_binary()
        assert "debug" in str(p) and "cytoscnpy-bin" in str(p)


def test_get_file_stats_failure(tmp_path):
    """Test get_file_stats when command fails or output is bad."""
    binary = tmp_path / "bin"
    dataset = tmp_path / "data"

    # 1. Command fails (Exception)
    with patch("subprocess.run", side_effect=OSError("Boom")):
        files, lines = run_benchmarks.get_file_stats(binary, dataset)
        assert files == 0
        assert lines == 0

    # 2. Command succeeds but bad JSON
    with patch("subprocess.run") as mock_run:
        mock_run.return_value.stdout = "Not JSON"
        files, lines = run_benchmarks.get_file_stats(binary, dataset)
        assert files == 0
        assert lines == 0

    # 3. Command succeeds, JSON valid, but missing fields
    with patch("subprocess.run") as mock_run:
        mock_run.return_value.stdout = json.dumps({"other": "data"})
        files, lines = run_benchmarks.get_file_stats(binary, dataset)
        assert files == 0
        assert lines == 0

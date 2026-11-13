"""Read synthetic CyanBird imports with the installed Antigravity language server.
No credentials are copied. All application data lives in a temporary directory.
Usage: python scripts/verify-antigravity-native.py --binary PATH [--report PATH]
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import socket
import sqlite3
import subprocess
import tempfile
import time
import urllib.request
import uuid

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--binary", required=True, type=Path)
parser.add_argument("--report", default=".qa/antigravity-native-report.json", type=Path)
args = parser.parse_args()
repository = Path(__file__).resolve().parent.parent
binary = args.binary.resolve(strict=True)
cargo = shutil.which("cargo") or str(Path.home() / ".cargo/bin/cargo.exe")
token = str(uuid.uuid4())
conversation = "e4095dd8-520c-47f6-8c6f-0441c208c1af"

def free_port():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]

with tempfile.TemporaryDirectory(prefix="cyanbird-antigravity-native-") as temporary:
    root = Path(temporary)
    port, tls_port = free_port(), free_port()
    flags = subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0
    process = None
    output = (root / "server.log").open("wb")

    def rpc(method, data):
        request = urllib.request.Request(
            f"http://127.0.0.1:{port}/exa.language_server_pb.LanguageServerService/{method}",
            data=json.dumps(data).encode(),
            headers={"Content-Type": "application/json", "x-codeium-csrf-token": token},
        )
        with urllib.request.urlopen(request, timeout=5) as response:
            return json.load(response)

    def start():
        global process
        process = subprocess.Popen([
            str(binary), "--standalone", f"--gemini_dir={root}",
            "--app_data_dir=antigravity", f"--http_server_port={port}",
            f"--https_server_port={tls_port}", f"--csrf_token={token}",
            "--disable_telemetry", "--subclient_type=hub", "--override_ide_name=antigravity",
        ], stdout=output, stderr=output, creationflags=flags)
        deadline = time.monotonic() + 45
        while time.monotonic() < deadline:
            if process.poll() is not None:
                raise RuntimeError("Native language server exited; inspect server log")
            try:
                rpc("ListProjects", {})
                # The listener starts before database migrations finish.
                state = root / "antigravity/antigravity_state.pbtxt"
                if state.exists() and "MIGRATION_STATUS_COMPLETED" in state.read_text():
                    time.sleep(1)
                    return
            except (OSError, ValueError):
                pass
            time.sleep(0.25)
        raise RuntimeError("Native language server did not become ready")

    def stop():
        global process
        if process is not None:
            process.terminate()
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=10)
            process = None

    try:
        start()
        stop()
        environment = os.environ.copy()
        environment["CYANBIRD_ANTIGRAVITY_QA"] = str(root / "antigravity")
        subprocess.run([cargo, "test", "--manifest-path", str(repository / "src-tauri/Cargo.toml"),
                        "antigravity::tests::native_fixture_and_collision_rollback", "--", "--nocapture"],
                       cwd=repository, env=environment, check=True, creationflags=flags)
        with sqlite3.connect(root / f"antigravity/conversations/{conversation}.db") as database:
            expected_trajectory = database.execute("SELECT trajectory_id FROM trajectory_meta").fetchone()[0]
        database.close()
        passes = []
        for _ in range(2):
            start()
            steps = rpc("GetCascadeTrajectorySteps", {"cascadeId": conversation})["steps"]
            summaries = rpc("GetAllCascadeTrajectories", {})["trajectorySummaries"]
            summary = summaries[conversation]
            projects = rpc("ListProjects", {})["projectIds"]
            assert steps[0]["userInput"]["userResponse"] == "CYANBIRD_USER_ALPHA"
            items = steps[0]["userInput"].get("items") or []
            assert any(item.get("text") == "CYANBIRD_USER_ALPHA" for item in items), f"missing userInput.items: {steps[0]['userInput']}"
            reply = steps[1]["plannerResponse"]["response"]
            assert "CYANBIRD_PROGRESS_FIRST" in reply
            assert "CYANBIRD_PROGRESS_MIDDLE" in reply
            assert "CYANBIRD_ASSISTANT_BETA" in reply
            assert "TOOL_ARCHIVE_OMIT" not in reply
            assert "THINKING_ARCHIVE_OMIT" not in reply
            assert "Historical tool event" not in reply
            assert "```rust\nfn main() {}\n```" in reply
            assert reply.index("CYANBIRD_PROGRESS_FIRST") < reply.index("CYANBIRD_PROGRESS_MIDDLE") < reply.index("CYANBIRD_ASSISTANT_BETA")
            assert summary["trajectoryId"] == expected_trajectory
            assert summary["summary"] == "CyanBird fixture"
            metadata = summary["trajectoryMetadata"]
            assert metadata["projectId"] in projects
            assert metadata["projectId"] != "outside-of-project"
            assert "Project%20%E4%B8%AD%E6%96%87" in metadata["workspaceUris"][0]
            passes.append({"steps": steps, "summary": summary})
            stop()
        report = {"result": "passed", "binarySha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
                  "checks": ["native messages", "title", "trajectory ID", "Chinese project path", "two restarts"],
                  "passes": passes}
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps(report, indent=2), encoding="utf-8")
        print(f"Native Antigravity oracle passed twice: {args.report.resolve()}")
    finally:
        stop()
        output.close()

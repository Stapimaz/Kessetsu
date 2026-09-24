from __future__ import annotations

import json
from pathlib import Path
import subprocess
import sys
import unittest
from unittest.mock import patch

from kessetsu import KessetsuClient, KessetsuCommandError, KessetsuContractError


class ClientTests(unittest.TestCase):
    def setUp(self) -> None:
        self.client = KessetsuClient(sys.executable, timeout=5)

    @patch("kessetsu.client.subprocess.run")
    def test_commands_use_argv_without_a_shell_and_validate_command(self, run) -> None:
        run.return_value = subprocess.CompletedProcess(
            [],
            0,
            stdout=json.dumps(
                {
                    "schema_version": "kessetsu.cli.v1",
                    "command": "check",
                    "status": "success",
                    "domain_versions": {},
                    "diagnostics": [],
                    "summary": {},
                    "measurements": {},
                    "assertions": None,
                    "requirements": None,
                    "artifacts": [],
                }
            ),
            stderr="",
        )
        self.client.check_file("folder with spaces/circuit.kess", parameters={"gain": "2"})
        arguments = run.call_args.args[0]
        self.assertEqual(arguments[0], self.client.executable)
        self.assertIn(str(Path("folder with spaces/circuit.kess").resolve()), arguments)
        self.assertIn("gain=2", arguments)
        self.assertFalse(run.call_args.kwargs["shell"])

    @patch("kessetsu.client.subprocess.run")
    def test_nonzero_exit_retains_structured_diagnostics(self, run) -> None:
        report = {
            "schema_version": "kessetsu.cli.v1",
            "command": "check",
            "diagnostics": [{"code": "KES-E003", "message": "Floating pin"}],
        }
        run.return_value = subprocess.CompletedProcess([], 1, stdout=json.dumps(report), stderr="")
        with self.assertRaises(KessetsuCommandError) as caught:
            self.client.check_file("bad.kess")
        self.assertEqual(caught.exception.exit_code, 1)
        self.assertEqual(caught.exception.report["diagnostics"][0]["code"], "KES-E003")

    @patch("kessetsu.client.subprocess.run")
    def test_success_without_one_json_object_fails_closed(self, run) -> None:
        run.return_value = subprocess.CompletedProcess([], 0, stdout="progress\n{}", stderr="")
        with self.assertRaisesRegex(KessetsuContractError, "one JSON object"):
            self.client.check_file("circuit.kess")


if __name__ == "__main__":
    unittest.main()

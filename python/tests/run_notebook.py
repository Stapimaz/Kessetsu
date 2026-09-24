from __future__ import annotations

import argparse
import asyncio
from pathlib import Path
import sys

import nbformat
from nbclient import NotebookClient


def main() -> None:
    if sys.platform == "win32":
        asyncio.set_event_loop_policy(asyncio.WindowsSelectorEventLoopPolicy())
    parser = argparse.ArgumentParser()
    parser.add_argument("notebook", type=Path)
    parser.add_argument("--working-directory", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    print(f"Executing {args.notebook}", flush=True)
    notebook = nbformat.read(args.notebook, as_version=4)
    NotebookClient(
        notebook,
        timeout=180,
        kernel_name="python3",
        resources={"metadata": {"path": str(args.working_directory.resolve())}},
    ).execute()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    nbformat.write(notebook, args.output)
    print(f"Executed notebook written to {args.output}", flush=True)


if __name__ == "__main__":
    main()

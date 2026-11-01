#!/usr/bin/env python3
"""Run a built API with private JSON configuration, without printing credentials."""
import json
import os
from pathlib import Path
import sys

root = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else Path(__file__).resolve().parents[1]
config = json.loads((root / '.runtime/api.json').read_text())
env = dict(os.environ)
env.update(config['environment'])
os.chdir(root)
binary = root / 'target/debug' / config['binary']
os.execve(str(binary), [str(binary), *sys.argv[2:]], env)


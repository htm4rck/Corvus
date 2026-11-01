#!/usr/bin/env python3
"""Compatibility launcher; acceptance now exercises the generic multi-company product."""
from pathlib import Path
import subprocess
subprocess.run(['python3',str(Path(__file__).with_name('verify_composable.py'))],check=True)

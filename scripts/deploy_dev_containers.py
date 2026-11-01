#!/usr/bin/env python3
"""Run the compiled DEV artifacts on the platform network; credentials remain private."""
import json
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[1]
for database in ['tordo-analytics-dev-postgres', 'tordo-analytics-dev-mysql']:
    inspect = subprocess.run(['docker', 'inspect', database], capture_output=True, text=True)
    if inspect.returncode == 0:
        networks = json.loads(inspect.stdout)[0]['NetworkSettings']['Networks']
        if 'tordo_dev_network' not in networks:
            subprocess.run(['docker', 'network', 'connect', 'tordo_dev_network', database], check=True)
for project in [root, root.parent / 'Tordo.Reports']:
    config = json.loads((project / '.runtime/api.json').read_text())
    environment = config['environment']
    environment['DATABASE_URL'] = environment['DATABASE_URL'].replace('127.0.0.1:55436', 'tordo-analytics-dev-postgres:5432')
    environment['UI_DIR'] = '/app/ui'
    if 'TORDO_SOURCE_SECRETS' in environment:
        secrets = json.loads(environment['TORDO_SOURCE_SECRETS'])
        for secret in secrets.values():
            secret['url'] = secret['url'].replace('127.0.0.1:55436', 'tordo-analytics-dev-postgres:5432').replace('127.0.0.1:55437', 'tordo-analytics-dev-mysql:3306')
        environment['TORDO_SOURCE_SECRETS'] = json.dumps(secrets)
    path = project / '.runtime/container.env'
    path.write_text(''.join(f'{key}={value}\n' for key, value in environment.items()))
    path.chmod(0o600)
# Retire only this task's native DEV services if they were previously installed.
subprocess.run(['systemctl', '--user', 'disable', '--now', 'tordo-corvus-dev.service', 'tordo-reports-dev.service'], capture_output=True)
subprocess.run(['docker', 'compose', '-f', str(root / 'infra/compose.api.dev.yml'), 'up', '-d', '--force-recreate'], check=True)
print('APIs Rust DEV ejecutándose en la red de Tordo.Platform.')

#!/usr/bin/env python3
"""Optional MySQL 8 reader fixture for Reports connector acceptance."""
import json
from pathlib import Path
import secrets
import subprocess
import time

root = Path(__file__).resolve().parents[1]
runtime = root / '.runtime'
state_file = runtime / 'mysql.json'
state = json.loads(state_file.read_text()) if state_file.exists() else {'root': secrets.token_hex(24), 'reader': secrets.token_hex(24)}
state_file.write_text(json.dumps(state)); state_file.chmod(0o600)
env = runtime / 'mysql.env'
env.write_text(f"MYSQL_ROOT_PASSWORD={state['root']}\n"); env.chmod(0o600)
name = 'tordo-analytics-dev-mysql'
if subprocess.run(['docker', 'inspect', name], capture_output=True).returncode:
    subprocess.run(['docker', 'run', '-d', '--name', name, '--restart', 'unless-stopped', '--env-file', str(env), '-p', '127.0.0.1:55437:3306', '-v', 'tordo-analytics-dev-mysql-data:/var/lib/mysql', 'mysql:8.0'], check=True, capture_output=True)
for _ in range(60):
    ready = subprocess.run(['docker', 'exec', '-e', f"MYSQL_PWD={state['root']}", name, 'mysql', '-uroot', '-e', 'SELECT 1'], capture_output=True)
    if ready.returncode == 0: break
    time.sleep(1)
else: raise SystemExit('MySQL DEV no disponible')
sql = f"""CREATE DATABASE IF NOT EXISTS analytics;
CREATE OR REPLACE VIEW analytics.sales AS SELECT 'Lima' AS region, CAST(125.50 AS DECIMAL(18,2)) AS amount UNION ALL SELECT 'Arequipa', CAST(80.00 AS DECIMAL(18,2));
CREATE USER IF NOT EXISTS 'analytics_reader'@'%' IDENTIFIED BY '{state['reader']}';
GRANT SELECT ON analytics.sales TO 'analytics_reader'@'%';
"""
subprocess.run(['docker', 'exec', '-i', '-e', f"MYSQL_PWD={state['root']}", name, 'mysql', '-uroot'], input=sql, text=True, check=True, capture_output=True)
config_path = root.parent / 'Tordo.Reports/.runtime/api.json'
config = json.loads(config_path.read_text())
registry = json.loads(config['environment']['TORDO_SOURCE_SECRETS'])
registry['ventas_mysql'] = dict(tenantId='B20123456789', engine='my_sql', url=f"mysql://analytics_reader:{state['reader']}@127.0.0.1:55437/analytics", allowedViews=['analytics.sales'], readOnly=True, tenantIsolated=True)
config['environment']['TORDO_SOURCE_SECRETS'] = json.dumps(registry)
config_path.write_text(json.dumps(config)); config_path.chmod(0o600)
subprocess.run(['python3', str(root / 'scripts/deploy_dev_containers.py')], check=True)
print('Fixture MySQL 8 preparada; referencia ventas_mysql registrada en DEV.')

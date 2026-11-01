#!/usr/bin/env python3
"""Provision only the isolated analytics DEV databases and the two user services."""
import json
import os
from pathlib import Path
import secrets
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
REPORTS = ROOT.parent / 'Tordo.Reports'
RUNTIME = ROOT / '.runtime'
RUNTIME.mkdir(mode=0o700, exist_ok=True)

def private(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(value)
    path.chmod(0o600)

state_file = RUNTIME / 'stack.json'
if state_file.exists():
    state = json.loads(state_file.read_text())
else:
    state = {key: secrets.token_hex(24) for key in ['postgres', 'corvus', 'reports', 'reader', 'readerB', 'tokenA', 'tokenB']}
    private(state_file, json.dumps(state))
private(RUNTIME / 'postgres.env', f"POSTGRES_PASSWORD={state['postgres']}\n")

def command(args, **kwargs):
    return subprocess.run(args, check=True, **kwargs)

command(['docker', 'compose', '-f', str(ROOT / 'infra/docker-compose.yml'), 'up', '-d'])
for attempt in range(30):
    if subprocess.run(['docker', 'exec', 'tordo-analytics-dev-postgres', 'pg_isready', '-U', 'postgres'], capture_output=True).returncode == 0:
        break
    time.sleep(1)
else:
    raise SystemExit('PostgreSQL DEV no disponible')

def sql(database, statement):
    return subprocess.run(['docker', 'exec', '-i', 'tordo-analytics-dev-postgres', 'psql', '-U', 'postgres', '-d', database, '-v', 'ON_ERROR_STOP=1', '-At'], input=statement, text=True, check=True, capture_output=True).stdout.strip()

for role, key in [('corvus_app', 'corvus'), ('reports_app', 'reports'), ('analytics_reader_a', 'reader'), ('analytics_reader_b', 'readerB')]:
    if sql('postgres', f"SELECT 1 FROM pg_roles WHERE rolname='{role}'") != '1':
        sql('postgres', f"CREATE ROLE {role} LOGIN PASSWORD '{state[key]}'")
for database, owner in [('corvus_dev', 'corvus_app'), ('reports_dev', 'reports_app'), ('analytics_a', 'postgres'), ('analytics_b', 'postgres')]:
    if sql('postgres', f"SELECT 1 FROM pg_database WHERE datname='{database}'") != '1':
        sql('postgres', f'CREATE DATABASE {database} OWNER {owner}')
for database, reader in [('analytics_a', 'analytics_reader_a'), ('analytics_b', 'analytics_reader_b')]:
    sql(database, f"CREATE SCHEMA IF NOT EXISTS analytics; CREATE OR REPLACE VIEW analytics.sales AS SELECT * FROM (VALUES ('Lima'::text,125.50::numeric),('Arequipa'::text,80.00::numeric)) AS v(region,amount); REVOKE CONNECT ON DATABASE {database} FROM PUBLIC; GRANT CONNECT ON DATABASE {database} TO {reader}; GRANT USAGE ON SCHEMA analytics TO {reader}; GRANT SELECT ON analytics.sales TO {reader};")

capabilities = ['corvus:ingest', 'corvus:query', 'corvus:read', 'corvus:write', 'reports:read', 'reports:admin', 'reports:author', 'reports:execute', 'reports:download']
clients = [dict(id='dev-a', token=state['tokenA'], companies=['B20123456789'], capabilities=capabilities), dict(id='dev-b', token=state['tokenB'], companies=['B20999999999'], capabilities=capabilities)]
registry = {
    'ventas_pg': dict(tenantId='B20123456789', engine='postgre_sql', url=f"postgresql://analytics_reader_a:{state['reader']}@127.0.0.1:55436/analytics_a", allowedViews=['analytics.sales'], readOnly=True, tenantIsolated=True),
    'ventas_pg_b': dict(tenantId='B20999999999', engine='postgre_sql', url=f"postgresql://analytics_reader_b:{state['readerB']}@127.0.0.1:55436/analytics_b", allowedViews=['analytics.sales'], readOnly=True, tenantIsolated=True),
}
for project, binary, key, port in [(ROOT, 'corvus-api', 'corvus', 8086), (REPORTS, 'reports-api', 'reports', 8087)]:
    ui_name = 'corvus-ui' if key == 'corvus' else 'reports-ui'
    existing = project / '.runtime/api.json'
    previous = json.loads(existing.read_text()) if existing.exists() else {}
    environment = dict(DATABASE_URL=f"postgresql://{key}_app:{state[key]}@127.0.0.1:55436/{key}_dev", BIND_ADDRESS=f'0.0.0.0:{port}', UI_DIR=str(project / f'apps/ui/dist/{ui_name}/browser'), TORDO_SERVICE_CLIENTS=json.dumps(clients), RUST_LOG='info')
    for name in ['CORVUS_ANALYTICS_SOURCES','CORVUS_SERVING_SOURCES']:
        if name in previous.get('environment',{}):environment[name]=previous['environment'][name]
    prior_clients=json.loads(previous.get('environment',{}).get('TORDO_SERVICE_CLIENTS','[]'))
    environment['TORDO_SERVICE_CLIENTS']=json.dumps(clients+[c for c in prior_clients if c['id'] not in {'dev-a','dev-b'}])
    if key == 'reports':
        existing = project / '.runtime/api.json'
        previous = json.loads(existing.read_text()) if existing.exists() else {}
        previous_sources = json.loads(previous.get('environment', {}).get('TORDO_SOURCE_SECRETS', '{}'))
        environment['TORDO_SOURCE_SECRETS'] = json.dumps({**previous_sources, **registry})
    private(project / '.runtime/api.json', json.dumps(dict(binary=binary, environment=environment)))
    command(['python3', str(ROOT / 'scripts/run_api.py'), str(project), 'migrate'])
subprocess.run(['python3', str(ROOT / 'scripts/deploy_dev_containers.py')], check=True)
print('DEV disponible: Corvus :8086 y Reports :8087. Credenciales privadas en Corvus/.runtime/stack.json.')

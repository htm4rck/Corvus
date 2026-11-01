#!/usr/bin/env python3
"""Provision only Acme DEV read views and a least-privilege extraction principal."""
import json
from pathlib import Path
import secrets
import subprocess

root=Path(__file__).resolve().parents[1]
runtime=root/'.runtime'
state_file=runtime/'acme-source.json'
state=json.loads(state_file.read_text()) if state_file.exists() else {'password':secrets.token_hex(24)}
state_file.write_text(json.dumps(state));state_file.chmod(0o600)
inspect=json.loads(subprocess.check_output(['docker','inspect','tordo_dev-postgres-1'],text=True))[0]
settings=dict(value.split('=',1) for value in inspect['Config']['Env'] if '=' in value)
assert settings.get('POSTGRES_DB') in ('tordo_dev',None), 'Use DEV PostgreSQL only'

def sql(text):
    subprocess.run(['docker','exec','-i','tordo_dev-postgres-1','sh','-c','exec psql -U "$POSTGRES_USER" -d 20123456789_business -v ON_ERROR_STOP=1'],input=text,text=True,check=True,capture_output=True)

sql((root/'source/operation_sales_export.sql').read_text())
sql(f"""DO $$ BEGIN IF NOT EXISTS(SELECT 1 FROM pg_roles WHERE rolname='corvus_acme_reader') THEN CREATE ROLE corvus_acme_reader LOGIN PASSWORD '{state['password']}' NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT; END IF; END $$;
GRANT CONNECT ON DATABASE "20123456789_business" TO corvus_acme_reader;
GRANT USAGE ON SCHEMA analytics_export TO corvus_acme_reader;
GRANT SELECT ON analytics_export.invoice_headers,analytics_export.invoice_lines,analytics_export.order_headers,analytics_export.order_lines TO corvus_acme_reader;
ALTER ROLE corvus_acme_reader SET default_transaction_read_only=on;
""")
config_path=runtime/'api.json';config=json.loads(config_path.read_text())
registry=json.loads(config['environment'].get('CORVUS_ANALYTICS_SOURCES','{}'))
registry['acme-operation-dev']={'tenantId':'B20123456789','url':f"postgresql://corvus_acme_reader:{state['password']}@tordo_dev-postgres-1:5432/20123456789_business",'deploymentEnvironment':'dev'}
config['environment']['CORVUS_ANALYTICS_SOURCES']=json.dumps(registry)
clients=json.loads(config['environment']['TORDO_SERVICE_CLIENTS'])
for client in clients:
    for capability in ['corvus:ingest','corvus:query']:
        if capability not in client['capabilities']:client['capabilities'].append(capability)
config['environment']['TORDO_SERVICE_CLIENTS']=json.dumps(clients)
config_path.write_text(json.dumps(config));config_path.chmod(0o600)
print('Vistas Acme DEV y lector restringido preparados; no se modificaron comprobantes o pedidos.')

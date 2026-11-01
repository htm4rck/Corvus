#!/usr/bin/env python3
"""Configure private DEV bindings and an empty second-tenant QA export source."""
import hashlib,json,subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[1];reports=root.parent/'Tordo.Reports';state=json.loads((root/'.runtime/stack.json').read_text())
p=root/'.runtime/api.json';config=json.loads(p.read_text());env=config['environment'];sources=json.loads(env['CORVUS_ANALYTICS_SOURCES']);sources['qa-sales-b']={'tenantId':'B20999999999','url':f"postgresql://analytics_reader_b:{state['readerB']}@tordo-analytics-dev-postgres:5432/analytics_b",'deploymentEnvironment':'dev'};env['CORVUS_ANALYTICS_SOURCES']=json.dumps(sources);p.write_text(json.dumps(config));p.chmod(0o600)
sql='''CREATE SCHEMA IF NOT EXISTS analytics_export;
CREATE OR REPLACE VIEW analytics_export.invoice_headers AS SELECT NULL::uuid AS id,NULL::date AS business_date,NULL::jsonb AS body WHERE false;
CREATE OR REPLACE VIEW analytics_export.order_headers AS SELECT NULL::uuid AS id,NULL::date AS business_date,NULL::jsonb AS body WHERE false;
CREATE OR REPLACE VIEW analytics_export.invoice_lines AS SELECT NULL::uuid AS id,NULL::uuid AS invoice_id,NULL::jsonb AS body WHERE false;
CREATE OR REPLACE VIEW analytics_export.order_lines AS SELECT NULL::uuid AS id,NULL::uuid AS order_id,NULL::jsonb AS body WHERE false;
GRANT USAGE ON SCHEMA analytics_export TO analytics_reader_b;
GRANT SELECT ON ALL TABLES IN SCHEMA analytics_export TO analytics_reader_b;
'''
subprocess.run(['docker','exec','-i','tordo-analytics-dev-postgres','psql','-U','postgres','-d','analytics_b','-v','ON_ERROR_STOP=1'],input=sql,text=True,check=True,capture_output=True)
p=reports/'.runtime/api.json';config=json.loads(p.read_text());registry=json.loads(config['environment']['CORVUS_SERVING_SOURCES'])
for value in registry.values():value.setdefault('pipelineId',str(__import__('uuid').UUID(hashlib.md5(('sales-documents:'+value['tenantId']).encode()).hexdigest())))
config['environment']['CORVUS_SERVING_SOURCES']=json.dumps(registry);p.write_text(json.dumps(config));p.chmod(0o600)
print('Bindings por pipeline preparados. El segundo origen de QA está vacío; no se generaron ventas.')

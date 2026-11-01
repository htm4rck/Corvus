#!/usr/bin/env python3
"""Functional acceptance against the isolated stack created by bootstrap_dev.py."""
import concurrent.futures
import csv
import io
import json
from pathlib import Path
import subprocess
import urllib.request
import urllib.error
import uuid
import zipfile

ROOT = Path(__file__).resolve().parents[1]
state = json.loads((ROOT / '.runtime/stack.json').read_text())
A, B = 'B20123456789', 'B20999999999'
checks = []

def request(port, path, body=None, tenant=A, token='tokenA', method=None, expected=200, raw=False):
    headers = {'business-code': tenant, 'Content-Type': 'application/json'}
    if token:
        headers['x-tordo-service-token'] = state[token]
    req = urllib.request.Request(f'http://127.0.0.1:{port}{path}', data=None if body is None else json.dumps(body).encode(), headers=headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=15) as response:
            status, data = response.status, response.read()
    except urllib.error.HTTPError as error:
        status, data = error.code, error.read()
    assert status in ([expected] if isinstance(expected, int) else expected), (path, status, data[:400])
    return data if raw else json.loads(data)

for port in (8086, 8087):
    assert request(port, '/health')['status'] == 'ready'
    path = '/api/v1/import-batches' if port == 8086 else '/api/v1/reports'
    request(port, path, token=None, expected=401)
    request(port, path, tenant=B, expected=403)
checks.append('health and authentication boundaries')

event = dict(eventId=str(uuid.uuid4()), eventType='sale.completed', eventVersion='1.0', occurredAt='2026-10-30T00:00:00Z', tenantId=A, companyId=A, sourceSystem='verification', data={'amount': '125.50'})
with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:
    receipts = list(pool.map(lambda _: request(8086, '/api/v1/events', event, expected=202), range(6)))
assert len({r['id'] for r in receipts}) == 1
request(8086, '/api/v1/events', {**event, 'data': {'amount': '126.50'}}, expected=409)
checks.append('concurrent event replay and conflicting payload')

batch_input = dict(tenantId=A, companyId=A, domain='verification', sourceSystem='verification', aggregationLevel='line')
batch = request(8086, '/api/v1/import-batches', batch_input, expected=201)
base = '/api/v1/import-batches/' + batch['id']
request(8086, base, tenant=B, token='tokenB', expected=404)
request(8086, base + '/confirm', {}, expected=409)
rejected = request(8086, base + '/validate', {'rows': [{'amount': '10.00'}, None]})
assert rejected['rejectedRecords'] == 1 and rejected['status'] == 'REJECTED'
request(8086, base + '/confirm', {}, expected=409)
request(8086, base + '/validate', {'rows': [{'amount': '10.00'}, {'amount': '20.00'}]})
confirmed = request(8086, base + '/confirm', {})
assert confirmed['status'] == 'CONFIRMED'
assert request(8086, base + '/confirm', {})['id'] == batch['id']
snapshot = request(8086, '/api/v1/datasets/' + batch['id'])
assert snapshot['rowCount'] == 2 and snapshot['rows'][0]['amount'] == '10.00'
request(8086, '/api/v1/datasets/' + batch['id'], tenant=B, token='tokenB', expected=404)
request(8086, base + '/validate', {'rows': [{'amount': '99'}]}, expected=409)
checks.append('batch validation, immutable snapshots and tenant isolation')

# Deliberately cause the snapshot INSERT to fail, proving the batch transition rolls back too.
rollback = request(8086, '/api/v1/import-batches', batch_input, expected=201)
rid = rollback['id']
request(8086, f'/api/v1/import-batches/{rid}/validate', {'rows': [{'amount': '1'}]})
def sql(statement):
    subprocess.run(['docker', 'exec', '-i', 'tordo-analytics-dev-postgres', 'psql', '-U', 'postgres', '-d', 'corvus_dev', '-v', 'ON_ERROR_STOP=1'], input=statement, text=True, capture_output=True, check=True)
sql(f"INSERT INTO corvus.datasets(tenant_id,id,body) VALUES('{A}','{rid}','{{}}')")
try:
    request(8086, f'/api/v1/import-batches/{rid}/confirm', {}, expected=409)
    assert request(8086, f'/api/v1/import-batches/{rid}')['status'] == 'VALIDATED'
finally:
    sql(f"DELETE FROM corvus.datasets WHERE tenant_id='{A}' AND id='{rid}'")
checks.append('real PostgreSQL rollback of batch and snapshot transaction')

source = request(8087, '/api/v1/sources', dict(name='Ventas DEV', engine='postgre_sql', secretRef='ventas_pg'), expected=201)
source2 = request(8087, '/api/v1/sources', dict(name='Segunda conexión PG', engine='postgre_sql', secretRef='ventas_pg'), expected=201)
assert source2['id'] != source['id']
request(8087, '/api/v1/sources', dict(name='Invalid tenant reference', engine='postgre_sql', secretRef='ventas_pg'), tenant=B, token='tokenB', expected=403)
checks.append('multiple connections to the same engine and secret scope')

def create_report(sql, params=None, expected=201):
    return request(8087, '/api/v1/reports', dict(name='Verificación', sourceId=source['id'], sql=sql, parameters=params or [], visualization=dict(kind='table', dimension=None, metric=None)), expected=expected)
def execute(report, expected=200):
    return request(8087, f"/api/v1/reports/{report['id']}/execute", {}, expected=expected)

report = create_report('SELECT region, amount FROM analytics.sales ORDER BY region')
job = execute(report)
assert job['result']['rows'] == [['Arequipa', '80.00'], ['Lima', '125.50']]
request(8087, f"/api/v1/reports/{report['id']}", tenant=B, token='tokenB', expected=404)
request(8087, f"/api/v1/executions/{job['id']}", tenant=B, token='tokenB', expected=404)
revision = request(8087, f"/api/v1/reports/{report['id']}", dict(expectedVersion=1, name='Revisión', sourceId=source['id'], sql='SELECT region, amount FROM analytics.sales', parameters=[], visualization=dict(kind='table', dimension=None, metric=None)), method='PUT')
assert revision['version'] == 2
checks.append('saved queries, versions, real reader connection and tenant isolation')

quoted = execute(create_report('SELECT $1::text AS label', ["O'Brien"]))
assert quoted['result']['rows'] == [["O'Brien"]]
duplicate = execute(create_report('SELECT 1 AS x, 2 AS x'))['result']
assert [c['name'] for c in duplicate['columns']] == ['x', 'x'] and duplicate['rows'] == [['1', '2']]
empty = execute(create_report('SELECT region, amount FROM analytics.sales WHERE false'))['result']
assert len(empty['columns']) == 2 and empty['rows'] == []
checks.append('native parameter binding, duplicate columns and zero-row schema')

execute(create_report('SELECT * FROM pg_catalog.pg_class'), expected=403)
execute(create_report('DELETE FROM analytics.sales'), expected=400)
execute(create_report('SELECT 1; SELECT 2'), expected=400)
checks.append('SQL statement and relation admission')

csv_data = request(8087, f"/api/v1/executions/{job['id']}/export?format=csv", raw=True).decode()
assert list(csv.reader(io.StringIO(csv_data)))[0] == ['region', 'amount']
xlsx_data = request(8087, f"/api/v1/executions/{job['id']}/export?format=xlsx", raw=True)
with zipfile.ZipFile(io.BytesIO(xlsx_data)) as book:
    assert b'<f>' not in book.read('xl/worksheets/sheet1.xml')
formula = execute(create_report('SELECT $1::text AS label', ['=1+1']))
formula_csv = request(8087, f"/api/v1/executions/{formula['id']}/export?format=csv", raw=True).decode()
assert "'=1+1" in formula_csv
request(8087, f"/api/v1/executions/{job['id']}/export?format=csv", tenant=B, token='tokenB', expected=404)
checks.append('CSV/XLSX precision, formula safety and download authorization')

request(8087, '/api/v1/dashboards', dict(name='DEV', reportIds=[report['id']]), expected=201)
request(8087, '/api/v1/dashboards', dict(name='Cross tenant', reportIds=[report['id']]), tenant=B, token='tokenB', expected=404)
checks.append('dashboard references scoped to the same tenant')

mysql_config = json.loads((ROOT.parent / 'Tordo.Reports/.runtime/api.json').read_text())
if 'ventas_mysql' in json.loads(mysql_config['environment']['TORDO_SOURCE_SECRETS']):
    mysql_source = request(8087, '/api/v1/sources', dict(name='MySQL verificación', engine='my_sql', secretRef='ventas_mysql'), expected=201)
    mysql_report = request(8087, '/api/v1/reports', dict(name='MySQL verificación', sourceId=mysql_source['id'], sql='SELECT region, amount FROM analytics.sales ORDER BY region', parameters=[], visualization=dict(kind='table', dimension=None, metric=None)), expected=201)
    assert execute(mysql_report)['result']['rows'] == [['Arequipa', '80.00'], ['Lima', '125.50']]
    mysql_parameter = request(8087, '/api/v1/reports', dict(name='MySQL parámetros', sourceId=mysql_source['id'], sql='SELECT CAST(? AS CHAR) AS label', parameters=["O'Brien · Perú"], visualization=dict(kind='table', dimension=None, metric=None)), expected=201)
    assert execute(mysql_parameter)['result']['rows'] == [["O'Brien · Perú"]]
    checks.append('MySQL 8 real reader, decimal values, Unicode and native parameter binding')
preview = execute(create_report('SELECT generate_series(1,1100) AS n'))
assert preview['result']['completeness'] == 'preview' and len(preview['result']['rows']) == 1000
request(8087, f"/api/v1/executions/{preview['id']}/export?format=csv", expected=409)
checks.append('bounded preview cannot be exported as a complete result')

output = dict(status='passed', groups=checks, services=['corvus', 'reports'], database='isolated analytics DEV')
(ROOT / '.runtime/verification.json').write_text(json.dumps(output, indent=2) + '\n')
print(json.dumps(output, indent=2))

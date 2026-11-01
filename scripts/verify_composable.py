#!/usr/bin/env python3
"""DEV acceptance for company-independent ingestion and composable BI resources."""
import concurrent.futures,datetime,io,json,subprocess,urllib.request,urllib.error,uuid,zipfile
from pathlib import Path
from decimal import Decimal
R=Path(__file__).resolve().parents[1];state=json.loads((R/'.runtime/stack.json').read_text());A='B20123456789';B='B20999999999';checks=[]
def call(port,path,body=None,tenant=A,expected=200,method=None,raw=False):
 req=urllib.request.Request(f'http://127.0.0.1:{port}/api/v1/'+path,data=None if body is None else json.dumps(body).encode(),method=method,headers={'business-code':tenant,'x-tordo-service-token':state['tokenA' if tenant==A else 'tokenB'],'Content-Type':'application/json'})
 try:
  with urllib.request.urlopen(req,timeout=55) as r:status,data=r.status,r.read()
 except urllib.error.HTTPError as e:status,data=e.code,e.read()
 assert status in ([expected] if isinstance(expected,int) else expected),(path,status,data[:400]);return data if raw else json.loads(data)
def sql(database,statement):return subprocess.run(['docker','exec','-i','tordo-analytics-dev-postgres','psql','-U','postgres','-d',database,'-v','ON_ERROR_STOP=1','-At'],input=statement,text=True,capture_output=True,check=True).stdout.strip()
def command(key=None):return {'runKey':key or str(uuid.uuid4()),'from':'2026-10-01','to':'2026-10-31'}
def ingest(p,body=None,tenant=A,expected=200):return call(8086,f"pipelines/{p['id']}/ingestions",body or command(),tenant,expected)
def head(p,tenant=A):return call(8086,f"pipelines/{p['id']}/dataset",tenant=tenant)
pa=next(p for p in call(8086,'pipelines') if p['domain']=='sales_documents');m=ingest(pa)
assert m['datasetCode']=='sales.v1' and m['pipelineId']==pa['id'];assert [v['layer'] for v in m['layers']]==['bronze','silver','gold'];assert m['quality']['approvedInvoiceTotals']=={'PEN':'186.00'}
snapshot=m['snapshotId'];assert sql('corvus_dev',f"SELECT sum(billed_total) FROM corvus_gold.billing_daily WHERE tenant_id='{A}' AND snapshot_id='{snapshot}'")=='186.00'
assert sql('corvus_dev',f"SELECT DISTINCT currency FROM corvus_silver.order_headers WHERE tenant_id='{A}' AND snapshot_id='{snapshot}'")=='PEN'
assert 'SOL' in sql('corvus_dev',f"SELECT body->'orders'->0->>'currency' FROM corvus_bronze.batches WHERE tenant_id='{A}' AND run_id='{snapshot}'")
checks.append('domain pipeline publishes real PEN186 through separate bronze, normalized silver and materialized gold')
for model in call(8086,'models'):
 filters={'currency':'PEN'}
 if 'fiscalEnvironment' in model['requiredFilters']:filters['fiscalEnvironment']='BETA'
 table=call(8086,'sales/query',{'snapshotId':snapshot,'pipelineId':pa['id'],'query':{'datasetCode':model['datasetCode'],'queryId':model['queryId'],'filters':filters}})
 assert [c['name'] for c in table['columns']]==model['columns']
checks.append('source discovery and gold model schemas are served as metadata')
call(8086,'pipelines',{'name':'Denied source','domain':'sales_documents','sourceRef':'qa-sales-b'},expected=403)
source_b=next(s for s in call(8086,'ingestion-sources',tenant=B) if s['sourceRef']=='qa-sales-b')
pb=call(8086,'pipelines',{'name':'QA · Comprobantes B','domain':pa['domain'],'sourceRef':source_b['sourceRef']},tenant=B,expected=201)
pb2=call(8086,'pipelines',{'name':'QA · Segundo flujo B','domain':pa['domain'],'sourceRef':source_b['sourceRef']},tenant=B,expected=201)
key=str(uuid.uuid4());cmd=command(key);b1=ingest(pb,cmd,B);b2=ingest(pb2,cmd,B)
assert b1['version']==b2['version']==1 and b1['snapshotId']!=b2['snapshotId'];assert b1['quality']['invoiceCount']==0
assert ingest(pb,cmd,B)['snapshotId']==b1['snapshotId'];ingest(pb,{**cmd,'from':'2026-10-02'},B,409)
assert head(pa)['snapshotId']==snapshot
call(8086,f"pipelines/{pb['id']}/dataset",expected=404)
call(8086,'sales/query',{'snapshotId':snapshot,'query':{'datasetCode':'sales.v1','queryId':'sales.billing.daily.v1','filters':{'fiscalEnvironment':'BETA'}}},tenant=B,expected=404)
checks.append('same domain and same run key work in two independent pipelines; second company cannot read the first')
# A malformed QA export is retained in bronze; it must never replace silver/gold.
empty_view="CREATE OR REPLACE VIEW analytics_export.invoice_headers AS SELECT NULL::uuid AS id,NULL::date AS business_date,NULL::jsonb AS body WHERE false;"
fixture={'id':str(uuid.uuid4()),'orderId':None,'number':'QA-NOT-A-BUSINESS-INVOICE','documentType':'BOLETA','businessDate':'2026-10-30','status':'PENDING','currency':'SOL','taxBase':'1.00','taxAmount':'0.00','total':'1.00','originApplication':'unknown','salesChannel':'QA','fiscalEnvironment':'BETA','fiscalStatus':'UNKNOWN','branchCode':None,'classification':'TECHNICAL_FIXTURE','originEvidence':'QA_INVALID_EXPORT','deleted':False}
try:
 body=json.dumps(fixture);sql('analytics_b',f"CREATE OR REPLACE VIEW analytics_export.invoice_headers AS SELECT '{fixture['id']}'::uuid AS id,'2026-10-30'::date AS business_date,'{body}'::jsonb AS body;")
 bad=command();response=ingest(pb,bad,B,409);assert response['error']['code']=='QUALITY_FAILED'
 assert head(pb,B)['snapshotId']==b1['snapshotId']
 row=sql('corvus_dev',f"SELECT r.state||'|'||r.error_code||'|'||(b.body->'invoices'->0->>'currency') FROM corvus.ingestion_runs r JOIN corvus_bronze.batches b ON b.tenant_id=r.tenant_id AND b.run_id=r.id WHERE r.tenant_id='{B}' AND r.pipeline_id='{pb['id']}' AND r.run_key='{bad['runKey']}'")
 assert row=='failed|QUALITY_FAILED|SOL',row
finally:sql('analytics_b',empty_view)
checks.append('quality failure retains unmodified QA bronze and leaves the previous silver/gold head unchanged')
# A failure after gold and head writes must roll back everything except the already captured bronze.
sql('corvus_dev',f"CREATE FUNCTION corvus.composition_fail() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN IF NEW.tenant_id='{B}' AND NEW.action='SalesSnapshotPublished' THEN RAISE EXCEPTION 'QA rollback'; END IF; RETURN NEW; END$$; CREATE TRIGGER composition_fail BEFORE INSERT ON corvus.audit_log FOR EACH ROW EXECUTE FUNCTION corvus.composition_fail();")
try:
 before=head(pb,B);bronze=int(sql('corvus_dev',f"SELECT count(*) FROM corvus_bronze.batches WHERE tenant_id='{B}' AND pipeline_id='{pb['id']}'"));ingest(pb,tenant=B,expected=503)
 assert head(pb,B)['snapshotId']==before['snapshotId'];assert int(sql('corvus_dev',f"SELECT count(*) FROM corvus_bronze.batches WHERE tenant_id='{B}' AND pipeline_id='{pb['id']}'"))==bronze+1
 assert sql('corvus_dev',f"SELECT snapshot_id FROM corvus.sales_checkpoints WHERE tenant_id='{B}' AND pipeline_id='{pb['id']}'")==before['snapshotId']
finally:sql('corvus_dev','DROP TRIGGER composition_fail ON corvus.audit_log;DROP FUNCTION corvus.composition_fail();')
checks.append('PostgreSQL promotion rollback preserves bronze and rolls back silver, gold, head, checkpoint and audit')
sql('corvus_dev',f"CREATE FUNCTION corvus.composition_slow() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN IF NEW.tenant_id='{B}' AND NEW.action='SalesSnapshotPublished' THEN PERFORM pg_sleep(0.4); END IF;RETURN NEW;END$$;CREATE TRIGGER composition_slow BEFORE INSERT ON corvus.audit_log FOR EACH ROW EXECUTE FUNCTION corvus.composition_slow();")
try:
 with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:outcomes=list(pool.map(lambda _:ingest(pb,tenant=B,expected=[200,409]),range(2)))
 assert sum('snapshotId' in v for v in outcomes)==1
finally:sql('corvus_dev','DROP TRIGGER composition_slow ON corvus.audit_log;DROP FUNCTION corvus.composition_slow();')
assert head(pb2,B)['snapshotId']==b2['snapshotId'];checks.append('snapshot CAS serializes a pipeline without changing another pipeline head')
# No sales/company assumption: a SQL question powers bar, pie and number charts in another company.
source=call(8087,'sources',{'name':'QA general connection','engine':'postgre_sql','secretRef':'ventas_pg_b'},tenant=B,expected=201)
qbody={'name':'QA · Cualquier capacidad','sourceId':source['id'],'sql':'SELECT region, amount FROM analytics.sales ORDER BY region','parameters':[]}
q=call(8087,'queries',qbody,tenant=B,expected=201)
charts=[]
for kind in ['bar','pie','number']:
 charts.append(call(8087,'visualizations',{'name':'QA · '+kind,'queryId':q['id'],'settings':{'kind':kind,'dimension':'region' if kind!='number' else None,'metric':'amount','series':[]}},tenant=B,expected=201))
cards=[{'id':str(uuid.uuid4()),'visualizationId':v['id'],'span':1 if i<2 else 2} for i,v in enumerate(charts)]
dbody={'name':'QA · Tablero libre','capability':'Compras','cards':cards};d=call(8087,'dashboards',dbody,tenant=B,expected=201)
d2=call(8087,'dashboards',{'name':'QA · Otro tablero','capability':'Inventario','cards':cards[:1]},tenant=B,expected=201)
run=call(8087,f"dashboards/{d['id']}/execute",{},tenant=B);assert [v['visualization']['settings']['kind'] for v in run['cards']]==['bar','pie','number'];assert len({v['execution']['reportId'] for v in run['cards']})==1
assert sum(Decimal(row[1]) for row in run['cards'][0]['execution']['result']['rows'])==Decimal('205.50')
checks.append('one arbitrary SQL question powers bar/pie/number visualizations reused by dashboards for different capabilities')
p=call(8087,f"dashboards/{d['id']}/publish",{'expectedVersion':d['version']},tenant=B,expected=201)
q2=call(8087,f"queries/{q['id']}",{**qbody,'sql':'SELECT region, amount * 2 AS amount FROM analytics.sales ORDER BY region','expectedVersion':q['version']},tenant=B,method='PUT')
chart2=call(8087,f"visualizations/{charts[1]['id']}",{'name':'QA · Edited line','queryId':q['id'],'settings':{'kind':'line','dimension':'region','metric':'amount','series':[]},'expectedVersion':1},tenant=B,method='PUT')
updated=call(8087,f"dashboards/{d['id']}",{**dbody,'cards':[cards[1]],'expectedVersion':d['version']},tenant=B,method='PUT')
frozen=call(8087,f"publications/{p['id']}/execute",{},tenant=B);live=call(8087,f"dashboards/{d['id']}/execute",{},tenant=B)
assert len(frozen['cards'])==3 and frozen['cards'][1]['visualization']['settings']['kind']=='pie';assert frozen['cards'][1]['execution']['reportVersion']==q['version'];assert sum(Decimal(row[1]) for row in frozen['cards'][1]['execution']['result']['rows'])==Decimal('205.50')
assert len(live['cards'])==1 and live['cards'][0]['visualization']['settings']['kind']=='line';assert live['cards'][0]['execution']['reportVersion']==q2['version'];assert sum(Decimal(row[1]) for row in live['cards'][0]['execution']['result']['rows'])==Decimal('411.00')
call(8087,f"dashboards/{d['id']}",{**dbody,'expectedVersion':1},tenant=B,method='PUT',expected=409)
call(8087,f"dashboards/{d['id']}/publish",{'expectedVersion':1},tenant=B,expected=409)
checks.append('published dashboard freezes query/chart/dashboard versions; draft edits and optimistic conflicts preserve the publication')
for path in [f"queries/{q['id']}",f"visualizations/{charts[0]['id']}",f"dashboards/{d['id']}",f"publications/{p['id']}",f"dashboard-runs/{run['id']}"]:
 call(8087,path,expected=404)
call(8087,'visualizations',{'name':'Denied','queryId':q['id'],'settings':{'kind':'table','series':[]}},expected=404)
call(8087,'dashboards',{'name':'Denied','cards':cards},expected=404)
call(8087,f"dashboard-runs/{run['id']}/export?format=xlsx",expected=404)
checks.append('query, visualization, dashboard, publication, execution and export references are company-scoped')
archive=zipfile.ZipFile(io.BytesIO(call(8087,f"dashboard-runs/{frozen['id']}/export?format=csv",tenant=B,raw=True)));assert len([n for n in archive.namelist() if n.endswith('.csv')])==3;assert 'metadata.json' in archive.namelist();assert all('125.50' in archive.read(n).decode() for n in archive.namelist() if n.endswith('.csv'))
xlsx=zipfile.ZipFile(io.BytesIO(call(8087,f"dashboard-runs/{frozen['id']}/export?format=xlsx",tenant=B,raw=True)));assert len([n for n in xlsx.namelist() if n.startswith('xl/worksheets/sheet')])==4
checks.append('dashboard exports contain all card tables plus provenance metadata, with exact decimals')
sql('reports_dev',f"CREATE FUNCTION reports.composition_fail() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN IF NEW.tenant_id='{B}' AND NEW.body->>'action'='reports.chart.saved' THEN RAISE EXCEPTION 'QA rollback'; END IF;RETURN NEW;END$$;CREATE TRIGGER composition_fail BEFORE INSERT ON reports.audit_log FOR EACH ROW EXECUTE FUNCTION reports.composition_fail();")
try:
 before=sql('reports_dev',f"SELECT count(*) FROM reports.resources WHERE tenant_id='{B}' AND kind='chart'")
 call(8087,'visualizations',{'name':'QA rollback','queryId':q['id'],'settings':{'kind':'table','series':[]}},tenant=B,expected=503)
 assert sql('reports_dev',f"SELECT count(*) FROM reports.resources WHERE tenant_id='{B}' AND kind='chart'")==before
finally:sql('reports_dev','DROP TRIGGER composition_fail ON reports.audit_log;DROP FUNCTION reports.composition_fail();')
checks.append('new BI definitions, immutable versions and audit share a PostgreSQL unit of work')
# Existing Acme is only catalog data; the generic executor pins all three cards to one Corvus pipeline cut.
da=next(d for d in call(8087,'dashboards') if d['name']=='Acme · Facturación y pedidos');arun=call(8087,f"dashboards/{da['id']}/execute",{});assert len(arun['cards'])==3;assert {c['execution']['result']['snapshotId'] for c in arun['cards']}=={snapshot}
subprocess.run(['docker','stop','corvus-rust-dev'],capture_output=True,check=True)
try:
 error=call(8087,f"dashboards/{da['id']}/execute",{},expected=503);assert error['error']['code']=='CORVUS_UNAVAILABLE'
finally:subprocess.run(['docker','start','corvus-rust-dev'],capture_output=True,check=True)
checks.append('example dashboard is generic catalog data, pins one Corvus cut and never falls back to the operational database')
result={'verifiedAt':datetime.datetime.now(datetime.timezone.utc).isoformat(),'environment':'DEV','checks':checks,'realCompanySnapshot':snapshot,'exampleDashboardId':da['id'],'exampleRunId':arun['id'],'secondCompany':{'pipelineId':pb['id'],'dashboardId':d['id'],'publicationId':p['id']}}
(R/'docs/evidence/composable-analytics-acceptance.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n');(R/'.runtime/composable-acceptance.json').write_text(json.dumps(result))
print('\n'.join('PASS '+v for v in checks))

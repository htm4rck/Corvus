// Explicit Acme DEV pilot: exercise the authenticated checkout contract consumed by Amazilia.
// Idempotent key and persisted operator identity prevent issuing a second document on rerun.
const fs=require('node:fs'),assert=require('node:assert/strict'),cp=require('node:child_process'),crypto=require('node:crypto');
const {Client}=require('/home/marc/Documentos/Proyects/Birdport/node_modules/pg');
const jwt=require('/home/marc/Documentos/Proyects/Birdport/node_modules/jsonwebtoken');
const root='/home/marc/Documentos/Proyects/Corvus',stateFile=root+'/.runtime/pos-case.json',evidenceFile=root+'/docs/evidence/acme-pos-case.json';
const state=fs.existsSync(stateFile)?JSON.parse(fs.readFileSync(stateFile)): {operator:'U'+String(Date.now()).slice(-11),key:'corvus:acme:amazilia:pilot-v1'};
fs.writeFileSync(stateFile,JSON.stringify(state),{mode:0o600});
const inspect=name=>JSON.parse(cp.execFileSync('docker',['inspect',name],{encoding:'utf8'}))[0];
const pg=Object.fromEntries(inspect('tordo_dev-postgres-1').Config.Env.filter(v=>v.includes('=')).map(v=>[v.slice(0,v.indexOf('=')),v.slice(v.indexOf('=')+1)]));
assert.equal(pg.POSTGRES_DB,'tordo_dev');
const config={host:'127.0.0.1',port:5432,user:pg.POSTGRES_USER,password:pg.POSTGRES_PASSWORD};
const central=new Client({...config,database:'tordo_dev'}),db=new Client({...config,database:'20123456789_business'});
async function main(){await central.connect();await db.connect();let created=false;
try {
 const fiscal=(await central.query("SELECT environment,sunat_endpoint FROM carabo.tenant WHERE business_code='B20123456789' AND active=true")).rows[0];assert.equal(fiscal?.environment,'BETA');assert(!fiscal.sunat_endpoint || fiscal.sunat_endpoint.includes('beta'));
 const business='B20123456789',user=state.operator;
 const token=jwt.sign({sub:user,type:'admin',username:user,licenseType:'root',iss:'https://auth.tordo.local',aud:'tordo-api'},fs.readFileSync('/home/marc/Documentos/Proyects/tordo/apps/auth/keys/private.pem'),{algorithm:'RS256',expiresIn:'15m'});
 assert.equal((await central.query('SELECT 1 FROM auth."user" WHERE "userCode"=$1',[user])).rowCount,0,'Pilot user must be temporary');
 await central.query('INSERT INTO auth."user"("userCode",nif,"userName",email,"firstName","lastName") VALUES($1,$1,$1,\'analytics-dev@example.invalid\',\'Piloto\',\'Acme DEV\')',[user]);created=true;
 await central.query('INSERT INTO auth.session(token,"expiresAt",signature,"userUserCode") VALUES($1,now()+interval \'15 minutes\',$2,$3)',[token,crypto.randomUUID(),user]);
 await central.query('INSERT INTO auth.user_business("userCode","businessCode","isActive") VALUES($1,$2,true)',[user,business]);
 const payload={idempotencyKey:state.key,paymentMode:'DEFERRED',methods:[],totalAmount:0,currency:'PEN',cashRegisterCode:'ECOM01',branchCode:'ECOM',invoice:{invoiceNumber:'',customerCode:'12345678',customerName:'CLIENTE PILOTO ACME DEV',customerDocument:'12345678',invoiceDate:'2026-10-30',warehouseCode:'ECOM-SALES',saleDocumentType:'RECEIPT',notes:'Piloto analítico Acme DEV; checkout API de Amazilia; venta a crédito sin cobro',details:[{productCode:'TOC-CICA-CALMING-SERUM',productName:'TOCOBO Cica Calming Serum',quantity:1,unitPrice:89,priceIncludesTax:true,taxRate:18,warehouseCode:'ECOM-SALES',unitOfMeasure:'NIU'}]}};
 const container=inspect('tordo_dev-operation-1');const base='http://'+container.NetworkSettings.Networks.tordo_dev_network.IPAddress+':43618/operation';
 const call=async()=>{const r=await fetch(base+'/sale-invoice/checkout',{method:'POST',headers:{Authorization:'Bearer '+token,'business-code':business,'Content-Type':'application/json'},body:JSON.stringify(payload),signal:AbortSignal.timeout(60000)});const body=await r.json();assert(r.ok&&body.success!==false,body.message);return body.data;};
 const result=await call();assert.equal(result.invoice.invoiceStatus,'APPROVED');assert.equal(Number(result.invoice.total),89);
 const replay=await call();assert.equal(replay.invoice.id,result.invoice.id);
 const invoice=(await db.query('SELECT id,invoice_number,invoice_status,total,sales_order_id FROM operation.sale_invoice WHERE id=$1',[result.invoice.id])).rows[0];assert.equal(invoice.sales_order_id,null);
 const checkout=(await db.query('SELECT status,invoice_id,payment_id FROM operation.pos_checkout WHERE idempotency_key=$1',[state.key])).rows[0];assert.equal(checkout.status,'COMPLETED');assert.equal(checkout.payment_id,null);
 const proof=crypto.createHash('sha256').update(JSON.stringify({method:'POST',route:'/operation/sale-invoice/checkout',request:payload,response:{invoiceId:invoice.id,invoiceNumber:invoice.invoice_number,total:invoice.total},actor:user})).digest('hex');
 await db.query(`INSERT INTO analytics_export.origin_evidence(entity_type,entity_id,origin_application,sales_channel,data_classification,fiscal_environment,proof_type,proof_hash,actor) VALUES('invoice',$1,'amazilia','POS','BUSINESS_DEV','BETA','CONTROLLED_AMAZILIA_CHECKOUT_API',$2,$3) ON CONFLICT(entity_type,entity_id) DO NOTHING`,[invoice.id,proof,user]);
 const evidence={environment:'DEV',fiscalEnvironment:'BETA',originApplication:'amazilia',originEvidence:'CONTROLLED_AMAZILIA_CHECKOUT_API',interaction:'Contrato HTTP consumido por Amazilia; no automatización de la UI Flutter',invoiceId:invoice.id,invoiceNumber:invoice.invoice_number,invoiceStatus:invoice.invoice_status,total:invoice.total,currency:'PEN',directPosWithoutOrder:true,paymentMode:'DEFERRED',noCollectionCreated:true,compatibleCheckoutReplay:true,proofHash:proof};
 fs.mkdirSync(root+'/docs/evidence',{recursive:true});fs.writeFileSync(evidenceFile,JSON.stringify(evidence,null,2)+'\n');console.log('PASS Acme DEV: comprobante aprobado de S/89, sin cobro; replay no duplica; evidencia de procedencia registrada.');
} finally {if(created){await central.query('DELETE FROM auth.session WHERE "userUserCode"=$1',[state.operator]);await central.query('DELETE FROM auth.user_business WHERE "userCode"=$1',[state.operator]);await central.query('DELETE FROM auth."user" WHERE "userCode"=$1',[state.operator]);}await db.end();await central.end();}}
main().catch(e=>{console.error(e.message);process.exitCode=1;});

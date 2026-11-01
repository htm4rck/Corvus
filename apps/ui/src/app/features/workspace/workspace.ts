import {Component,inject,signal} from '@angular/core';
import {FormField,form} from '@angular/forms/signals';
import {Ingestions} from '../ingestions/ingestions';
import {Api} from '../../core/api';
interface Batch{id:string;domain:string;sourceSystem:string;status:string;receivedRecords:number;validRecords:number;}
interface Dataset{datasetId:string;domain:string;rowCount:number;}
@Component({selector:'app-workspace',imports:[FormField,Ingestions],templateUrl:'./workspace.html',styleUrl:'./workspace.css'})
export class Workspace{
  readonly api=inject(Api);readonly tabs=['Ingestas','Lotes','Datasets','Alcance'];readonly tab=signal('Ingestas');
  readonly ready=signal(false);readonly status=signal('Comprobando servicio');readonly busy=signal(false);readonly error=signal('');readonly selected=signal('');
  readonly batches=signal<Batch[]>([]);readonly datasets=signal<Dataset[]>([]);
  readonly settings=signal({tenant:'',token:''});readonly access=form(this.settings);
  readonly model=signal({domain:'sales',sourceSystem:'dev',rows:'[{"region":"Lima","amount":"125.50"},{"region":"Arequipa","amount":"80.00"}]'});readonly batchForm=form(this.model);
  constructor(){void this.api.get<{status:string}>('health',true).then(v=>{this.ready.set(v.status==='ready');this.status.set(v.status==='ready'?'Servicio disponible':'Servicio no preparado');}).catch(()=>this.status.set('Servicio no disponible'));}
  async run(action:()=>Promise<void>){this.busy.set(true);this.error.set('');try{await action();}catch(e){this.error.set(this.api.error(e));}finally{this.busy.set(false);}}
  async refresh(){const[b,d]=await Promise.all([this.api.get<Batch[]>('api/v1/import-batches'),this.api.get<Dataset[]>('api/v1/datasets')]);this.batches.set(b);this.datasets.set(d);}
  connect(){void this.run(async()=>{this.batches.set([]);this.datasets.set([]);this.selected.set('');this.api.session.set({...this.settings()});await this.refresh();});}
  create(){void this.run(async()=>{const m=this.model();const t=this.api.session().tenant;const v=await this.api.post<Batch>('api/v1/import-batches',{tenantId:t,companyId:t,domain:m.domain,sourceSystem:m.sourceSystem,aggregationLevel:'line',schemaVersion:'1.0'});this.selected.set(v.id);await this.refresh();});}
  validate(){void this.run(async()=>{const rows:unknown=JSON.parse(this.model().rows);if(!Array.isArray(rows))throw new Error('Las filas deben formar una lista JSON.');await this.api.post(`api/v1/import-batches/${this.selected()}/validate`,{rows});await this.refresh();});}
  confirm(v:Batch){void this.run(async()=>{await this.api.post(`api/v1/import-batches/${v.id}/confirm`);await this.refresh();});}
}

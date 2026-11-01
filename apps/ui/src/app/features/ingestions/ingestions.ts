import {Component,effect,inject,signal} from '@angular/core';
import {FormField,form} from '@angular/forms/signals';
import {HttpErrorResponse} from '@angular/common/http';
import {Api} from '../../core/api';
interface Pipeline {id:string;name:string;domain:string;sourceRef:string;version:number;}
interface Source {sourceRef:string;connector:string;}
interface Domain {code:string;name:string;datasetCode:string;layers:string[];}
interface Manifest {snapshotId:string;pipelineId:string;version:number;datasetCode:string;observedThrough:string;layers:{layer:string;recordCount:number;state:string}[];quality:{warnings:string[];approvedInvoiceTotals:Record<string,string>};}
interface Run {id:string;runKey:string;state:string;errorCode:string|null;bronzeCaptured:boolean;snapshotId:string|null;}
@Component({selector:'app-ingestions',imports:[FormField],templateUrl:'./ingestions.html',styleUrl:'./ingestions.css'})
export class Ingestions {
 readonly api=inject(Api);readonly pipelines=signal<Pipeline[]>([]);readonly sources=signal<Source[]>([]);readonly domains=signal<Domain[]>([]);readonly runs=signal<Run[]>([]);readonly manifest=signal<Manifest|null>(null);readonly busy=signal(false);readonly error=signal('');
 readonly definition=signal({name:'',domain:'',sourceRef:''});readonly definitionForm=form(this.definition);
 readonly command=signal({pipelineId:'',runKey:String(crypto.randomUUID()),from:'',to:''});readonly commandForm=form(this.command);
 constructor(){effect(()=>{const session=this.api.session();this.pipelines.set([]);this.sources.set([]);this.domains.set([]);this.runs.set([]);this.manifest.set(null);this.error.set('');if(session.token)void this.load();});}
 async act(work:()=>Promise<void>){this.busy.set(true);this.error.set('');try{await work();}catch(e){this.error.set(this.api.error(e));}finally{this.busy.set(false);}}
 async load(){await this.act(async()=>{const[p,s,d]=await Promise.all([this.api.get<Pipeline[]>('api/v1/pipelines'),this.api.get<Source[]>('api/v1/ingestion-sources'),this.api.get<Domain[]>('api/v1/ingestion-domains')]);this.pipelines.set(p);this.sources.set(s);this.domains.set(d);this.definition.update(v=>({...v,domain:d[0]?.code??'',sourceRef:s[0]?.sourceRef??''}));this.command.update(v=>({...v,pipelineId:p[0]?.id??''}));});}
 create(){void this.act(async()=>{const value=await this.api.post<Pipeline>('api/v1/pipelines',this.definition());this.pipelines.update(v=>[value,...v]);this.command.update(v=>({...v,pipelineId:value.id}));});}
 async inspect(){this.manifest.set(null);this.runs.set([]);const id=this.command().pipelineId;if(!id)return;const[r,m]=await Promise.all([this.api.get<Run[]>(`api/v1/pipelines/${id}/runs`),this.api.get<Manifest>(`api/v1/pipelines/${id}/dataset`).catch(e=>{if(e instanceof HttpErrorResponse&&e.status===404)return null;throw e;})]);this.runs.set(r);this.manifest.set(m);}
 view(){void this.act(()=>this.inspect());}
 run(){void this.act(async()=>{const {pipelineId,...command}=this.command();this.manifest.set(await this.api.post<Manifest>(`api/v1/pipelines/${pipelineId}/ingestions`,command));this.runs.set(await this.api.get<Run[]>(`api/v1/pipelines/${pipelineId}/runs`));});}
 next(){this.command.update(v=>({...v,runKey:String(crypto.randomUUID())}));}
 totals(m:Manifest){return Object.entries(m.quality.approvedInvoiceTotals).map(([currency,total])=>`${currency} ${total}`).join(' · ');}
}

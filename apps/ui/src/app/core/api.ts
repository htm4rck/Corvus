import { Service, inject, signal } from '@angular/core';
import { HttpClient, HttpHeaders, HttpErrorResponse } from '@angular/common/http';
import { firstValueFrom } from 'rxjs';
@Service()
export class Api {
  private readonly http=inject(HttpClient);
  readonly session=signal({tenant:'',token:''});
  private url(path:string) { const url=new URL(path.replace(/^\//,''),document.baseURI);return url.pathname+url.search; }
  private headers() { const s=this.session();return new HttpHeaders({'business-code':s.tenant,'x-tordo-service-token':s.token}); }
  private scoped<T>(value:Promise<T>,scope:ReturnType<typeof this.session>):Promise<T>{return value.then(v=>{if(this.session()!==scope)throw new Error('El espacio cambió; vuelve a cargar los datos.');return v;});}
  get<T>(path:string,publicRoute=false) { const scope=this.session();const promise=firstValueFrom(this.http.get<T>(this.url(path),{headers:publicRoute?undefined:this.headers()}));return publicRoute?promise:this.scoped(promise,scope); }
  post<T>(path:string,body:unknown={}) { const scope=this.session();return this.scoped(firstValueFrom(this.http.post<T>(this.url(path),body,{headers:this.headers()})),scope); }
  put<T>(path:string,body:unknown) { const scope=this.session();return this.scoped(firstValueFrom(this.http.put<T>(this.url(path),body,{headers:this.headers()})),scope); }
  async download(path:string,name:string) {
    const scope=this.session();const blob=await this.scoped(firstValueFrom(this.http.get(this.url(path),{headers:this.headers(),responseType:'blob'})),scope);
    const url=URL.createObjectURL(blob);const link=document.createElement('a');link.href=url;link.download=name;document.body.append(link);link.click();link.remove();setTimeout(()=>URL.revokeObjectURL(url),1000);
  }
  error(error:unknown):string {
    if(error instanceof HttpErrorResponse) return error.error?.error?.message ?? (error.status===401?'Identidad inválida. Revisa el acceso.':error.status===403?'No tienes permiso para esta empresa u operación.':'No se pudo completar la solicitud. Revisa el servicio e inténtalo de nuevo.');
    return error instanceof Error?error.message:'No se pudo completar la solicitud.';
  }
}

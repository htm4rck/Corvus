import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting, HttpTestingController } from '@angular/common/http/testing';
import { Workspace } from './workspace';
describe('Workspace access boundary',()=>{
 it('clears previously loaded data before a different scope is requested',async()=>{
  TestBed.configureTestingModule({imports:[Workspace],providers:[provideHttpClient(),provideHttpClientTesting()]});
  const fixture=TestBed.createComponent(Workspace);const component=fixture.componentInstance;
  const http=TestBed.inject(HttpTestingController);
  http.expectOne('/health').flush({status:'ready'});
  component.batches.set([{id:'old',domain:'old',sourceSystem:'old',status:'CREATED',receivedRecords:0,validRecords:0}]);
  component.settings.set({tenant:'B20999999999',token:'test-token'});component.connect();
  expect(component.batches()).toEqual([]);expect(component.datasets()).toEqual([]);
  http.expectOne('/api/v1/import-batches').flush([]);
http.expectOne('/api/v1/datasets').flush([]);
  await fixture.whenStable();
  expect(component.batches()).toEqual([]);expect(component.datasets()).toEqual([]);
  for(const path of ['pipelines','ingestion-sources','ingestion-domains']){const request=http.expectOne('/api/v1/'+path);expect(request.request.headers.get('business-code')).toBe('B20999999999');request.flush([]);}
  await fixture.whenStable();
  http.verify();
 });
});

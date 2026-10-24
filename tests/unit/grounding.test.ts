import { expect,it } from 'vitest';
import { validateEvidence } from '../../src/lib/server/research/grounding';
import { makeContent } from '../fixtures/knowledge';
const content=makeContent(),sources=content.sources.map(s=>({...s,text:'Flash retains data without power. A controller manages flash storage.'}));
it('requires real known sources and exact evidence rather than invented quotes',()=>{
  expect(()=>validateEvidence(content,sources)).not.toThrow();
  const bad=structuredClone(content);bad.summary[0].evidence[0].quote='Invented evidence';
  expect(()=>validateEvidence(bad,sources)).toThrow();
  bad.summary[0].evidence[0]={sourceId:'unknown',quote:'Flash retains data without power.'};expect(()=>validateEvidence(bad,sources)).toThrow();
  const other=structuredClone(content);other.sources[0].url='https://invented.example.org';expect(()=>validateEvidence(other,sources)).toThrow();
});

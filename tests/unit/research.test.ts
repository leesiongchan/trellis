import { expect,it,vi } from 'vitest';
import { researchTopic } from '../../src/lib/server/research/service';
import type { ResearchDependencies } from '../../src/lib/server/research/types';
import { makeContent,makeRequest } from '../fixtures/knowledge';
const content=makeContent();
const deps=():ResearchDependencies=>({
 search:vi.fn(async()=>content.sources.map(s=>({...s,text:'Flash retains data without power. A controller manages flash storage.'}))),
 generate:vi.fn(async()=>({kind:'topic' as const,content})),verify:vi.fn(async()=>({unsupportedParagraphIds:[],unsupportedBranchIndexes:[]}))
});
it('returns insufficient evidence without generating unsupported content',async()=>{
 const d=deps();d.search=vi.fn(async()=>[]);expect((await researchTopic(makeRequest(),new AbortController().signal,d)).kind).toBe('insufficient-evidence');expect(d.generate).not.toHaveBeenCalled();
});
it('returns one clarification before applying factual-content checks',async()=>{
 const d=deps();d.generate=vi.fn(async()=>({kind:'clarification' as const,question:'Which Java do you mean?'}));
 expect(await researchTopic(makeRequest({question:'Java'}),new AbortController().signal,d)).toEqual({kind:'clarification',question:'Which Java do you mean?'});expect(d.verify).not.toHaveBeenCalled();
});
it('rejects malformed output and evidence-check failures instead of labeling them researched',async()=>{
 const d=deps();d.verify=vi.fn(async()=>({unsupportedParagraphIds:['p2'],unsupportedBranchIndexes:[]}));
 expect((await researchTopic(makeRequest(),new AbortController().signal,d)).kind).toBe('insufficient-evidence');
 d.generate=vi.fn(async()=>({kind:'topic',content:{title:'incomplete'}} as never));await expect(researchTopic(makeRequest(),new AbortController().signal,d)).rejects.toThrow();
});
it('returns only validated content and propagates cancellation to providers',async()=>{
 const d=deps(),controller=new AbortController();expect(await researchTopic(makeRequest(),controller.signal,d)).toEqual({kind:'topic',content});
 controller.abort();await expect(researchTopic(makeRequest(),controller.signal,d)).rejects.toThrow();
});

import type { TopicContent } from '#lib/knowledge/types.ts';
import type { RetrievedSource } from './types';
import { ServiceError } from '../config';
const normalize=(text:string)=>text.replace(/\s+/gu,' ').trim();
export function validateEvidence(content:TopicContent,sources:RetrievedSource[]):void{
  const known=new Map(sources.map(s=>[s.id,s]));
  for(const source of content.sources)if(known.get(source.id)?.url!==source.url)throw new ServiceError('INVALID_EVIDENCE','A source could not be verified.');
  const publicIds=new Set(content.sources.map(s=>s.id));
  for(const paragraph of [...content.summary,...content.sections.flatMap(s=>s.paragraphs),...content.branches.map(b=>b.reason)]){
    if(!paragraph.evidence.length)throw new ServiceError('INVALID_EVIDENCE','The explanation is missing supporting evidence.');
    for(const ref of paragraph.evidence){const source=known.get(ref.sourceId);if(!source||!publicIds.has(ref.sourceId)||!normalize(source.text).includes(normalize(ref.quote)))throw new ServiceError('INVALID_EVIDENCE','A quoted passage could not be found in the retrieved sources.');}
  }
}

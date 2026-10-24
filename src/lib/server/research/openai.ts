import OpenAI from 'openai';
import { zodTextFormat } from 'openai/helpers/zod';
import { z } from 'zod';
import { newId } from '$lib/knowledge/normalize';
import { TopicContentSchema } from '$lib/knowledge/schemas';
import type { ResearchRequest,TopicContent } from '$lib/knowledge/types';
import { readConfig,ServiceError } from '../config';
import { EXPLAIN_PROMPT,VERIFY_PROMPT } from './prompts';
import type { DraftResult,GroundingVerdict,RetrievedSource } from './types';
const Evidence=z.object({sourceId:z.string(),quote:z.string()});
const Paragraph=z.object({text:z.string(),evidence:z.array(Evidence)});
const Draft=z.object({result:z.discriminatedUnion('kind',[
  z.object({kind:z.literal('topic'),title:z.string(),summary:z.array(Paragraph),sections:z.array(z.object({heading:z.string(),paragraphs:z.array(Paragraph)})),analogy:z.string().nullable(),limitations:z.string().nullable(),branches:z.array(z.object({title:z.string(),question:z.string(),relationship:z.string(),reason:Paragraph}))}),
  z.object({kind:z.literal('clarification'),question:z.string()}),z.object({kind:z.literal('insufficient-evidence'),message:z.string()})
])});
const Verdict=z.object({unsupportedParagraphIds:z.array(z.string()),unsupportedBranchIndexes:z.array(z.number().int())});
function client(){const config=readConfig();if(!config.openaiKey)throw new ServiceError('SETUP_REQUIRED','Explanations are not connected. Add OPENAI_API_KEY on the server.',503,false);return new OpenAI({apiKey:config.openaiKey,maxRetries:0});}
export async function generateTopic(input:ResearchRequest,sources:RetrievedSource[],signal:AbortSignal):Promise<DraftResult>{
  const response=await client().responses.parse({model:readConfig().model,store:false,reasoning:{effort:'low'},max_output_tokens:8000,
    input:[{role:'system',content:EXPLAIN_PROMPT},{role:'user',content:JSON.stringify({date:new Date().toISOString().slice(0,10),topic:input,sources})}],text:{format:zodTextFormat(Draft,'topic_explanation')}},{signal});
  if(response.status!=='completed'||!response.output_parsed)throw new ServiceError('INVALID_EXPLANATION','The explanation could not be completed. Please retry.');
  const draft=response.output_parsed.result;if(draft.kind!=='topic')return draft;
  const paragraph=(p:z.infer<typeof Paragraph>)=>({...p,id:newId()});
  return {kind:'topic',content:TopicContentSchema.parse({title:draft.title,summary:draft.summary.map(paragraph),sections:draft.sections.map(s=>({...s,paragraphs:s.paragraphs.map(paragraph)})),
    analogy:draft.analogy,limitations:draft.limitations,branches:draft.branches.map(b=>({...b,reason:paragraph(b.reason)})),sources:sources.map(({text,...source})=>source),researchedAt:new Date().toISOString()})};
}
export async function verifyGrounding(draft:TopicContent,sources:RetrievedSource[],signal:AbortSignal):Promise<GroundingVerdict>{
  const response=await client().responses.parse({model:readConfig().model,store:false,reasoning:{effort:'low'},max_output_tokens:2000,
    input:[{role:'system',content:VERIFY_PROMPT},{role:'user',content:JSON.stringify({draft,sources})}],text:{format:zodTextFormat(Verdict,'evidence_check')}},{signal});
  if(response.status!=='completed'||!response.output_parsed)throw new ServiceError('GROUNDING_FAILED','The source check could not finish. Please retry.');
  return Verdict.parse(response.output_parsed);
}

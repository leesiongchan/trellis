import type { RequestHandler } from '@sveltejs/kit';
import { createHash,randomUUID } from 'node:crypto';
import { ResearchRequestSchema } from '#lib/knowledge/schemas';
import type { ResearchResult } from '#lib/knowledge/types';
import { ACCESS_COOKIE,authorizeRequest } from '../access';
import { readConfig,ServiceError,type AppConfig } from '../config';
import { readBoundedJson } from '../request-guard';
import { logResearch } from '../log';
import { researchTopic } from './service';
import type { ResearchDependencies } from './types';
export function createResearchHandler(deps:ResearchDependencies,config:()=>AppConfig=readConfig):RequestHandler{
  const jobs=new Map<string,Promise<ResearchResult>>();let starts:number[]=[];
  return async({request,cookies})=>{
    const candidate=request.headers.get('x-request-id'),requestId=candidate&&/^[a-zA-Z0-9_-]{1,100}$/.test(candidate)?candidate:randomUUID();
    const headers={'content-type':'application/json','cache-control':'no-store','x-request-id':requestId},started=Date.now();
    const reply=(body:unknown,status=200)=>new Response(JSON.stringify(body),{status,headers});
    try{
      const settings=config();
      if(!authorizeRequest(request,cookies.get(ACCESS_COOKIE),settings.accessKey))throw new ServiceError('ACCESS_REQUIRED','Open your private access link to enable research.',401,false);
      const url=new URL(request.url),expected=new URL(settings.origin);
      if(request.headers.get('origin')!==expected.origin||url.host!==expected.host||(request.headers.get('host')??url.host)!==expected.host)throw new ServiceError('ORIGIN_REJECTED','This request must come from your private app address.',403,false);
      const parsed=ResearchRequestSchema.safeParse(await readBoundedJson(request,settings.limits.bodyBytes));
      if(!parsed.success)throw new ServiceError('INVALID_QUESTION','Enter a question of 1–500 characters with a shorter context.',422,false);
      if(!settings.openaiKey||!settings.tavilyKey)throw new ServiceError('SETUP_REQUIRED','Live research needs OPENAI_API_KEY and TAVILY_API_KEY in the server’s .env file. Saved explorations remain available.',503,false);
      const input=parsed.data,key=createHash('sha256').update(JSON.stringify(input)).digest('hex');
      let job=jobs.get(key);
      if(!job){
        starts=starts.filter(time=>time>Date.now()-60000);
        if(jobs.size>=settings.limits.concurrent||starts.length>=settings.limits.perMinute)throw new ServiceError('RESEARCH_LIMIT','A few topics are already being researched. Try again shortly.',429);
        starts.push(Date.now());logResearch('research_started',requestId);
        const signal=AbortSignal.timeout(settings.limits.researchMs);
        const instrumented:ResearchDependencies={
          search:async(query,s)=>{const t=Date.now(),sources=await deps.search(query,s);logResearch('search_completed',requestId,{stage:'search',durationMs:Date.now()-t,sourceCount:sources.length});return sources;},
          generate:async(i,sources,s)=>{const t=Date.now(),result=await deps.generate(i,sources,s);logResearch('generation_completed',requestId,{stage:'generation',durationMs:Date.now()-t});return result;},verify:deps.verify
        };
        job=Promise.resolve().then(()=>new Promise<ResearchResult>((resolve,reject)=>{
          const aborted=()=>reject(new ServiceError('RESEARCH_TIMEOUT','Research took too long. Please retry.',504));
          signal.addEventListener('abort',aborted,{once:true});
          researchTopic(input,signal,instrumented).then(resolve,reject).finally(()=>signal.removeEventListener('abort',aborted));
        })).finally(()=>jobs.delete(key));jobs.set(key,job);
      }
      const result=await job;logResearch('research_completed',requestId,{durationMs:Date.now()-started});return reply(result);
    }catch(error){
      const known=error instanceof ServiceError?error:null;
      const timeout=error instanceof Error&&['TimeoutError','AbortError','APIUserAbortError'].includes(error.name);
      const failure=known??new ServiceError(timeout?'RESEARCH_TIMEOUT':'UPSTREAM_FAILURE',timeout?'Research took too long. Please retry.':'Research could not finish. Please try again shortly.',timeout?504:502);
      logResearch('research_failed',requestId,{code:failure.code,durationMs:Date.now()-started});
      return reply({error:{code:failure.code,message:failure.message,retryable:failure.retryable,requestId}},failure.status);
    }
  };
}

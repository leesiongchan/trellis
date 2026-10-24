export const LIMITS = { bodyBytes:65536, researchMs:120000, searchMs:20000, concurrent:2, perMinute:10 } as const;
export function readConfig(env:NodeJS.ProcessEnv=process.env){
  return {openaiKey:env.OPENAI_API_KEY??'',tavilyKey:env.TAVILY_API_KEY??'',model:env.OPENAI_MODEL||'gpt-5.6-terra',
    origin:env.APP_ORIGIN||'http://127.0.0.1:5173',accessKey:env.APP_ACCESS_KEY??'',limits:LIMITS};
}
export class ServiceError extends Error {
  constructor(public code:string,message:string,public status=502,public retryable=true){super(message);}
}

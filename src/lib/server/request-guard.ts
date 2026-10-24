import { ServiceError } from './config';
export async function readBoundedJson(request:Request,maxBytes:number):Promise<unknown>{
  if(!request.headers.get('content-type')?.split(';')[0].trim().endsWith('/json'))throw new ServiceError('INVALID_CONTENT_TYPE','Send a JSON request.',400,false);
  if(Number(request.headers.get('content-length'))>maxBytes)throw new ServiceError('BODY_TOO_LARGE','The question and context are too large.',413,false);
  const reader=request.body?.getReader();if(!reader)throw new ServiceError('INVALID_JSON','A question is required.',400,false);
  const chunks:Uint8Array[]=[],decoder=new TextDecoder('utf-8',{fatal:true});let total=0;
  try{
    while(true){const {value,done}=await reader.read();if(done)break;total+=value.byteLength;if(total>maxBytes){void reader.cancel();throw new ServiceError('BODY_TOO_LARGE','The question and context are too large.',413,false);}chunks.push(value);}
    const bytes=new Uint8Array(total);let offset=0;for(const chunk of chunks){bytes.set(chunk,offset);offset+=chunk.byteLength;}
    try{return JSON.parse(decoder.decode(bytes));}catch{throw new ServiceError('INVALID_JSON','The request could not be read. Please try again.',400,false);}
  }finally{reader.releaseLock();}
}

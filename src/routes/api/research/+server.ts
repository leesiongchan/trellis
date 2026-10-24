import { createResearchHandler } from '#lib/server/research/handler.ts';
import { searchEvidence } from '#lib/server/research/tavily.ts';
import { generateTopic,verifyGrounding } from '#lib/server/research/openai.ts';
export const POST=createResearchHandler({search:searchEvidence,generate:generateTopic,verify:verifyGrounding});

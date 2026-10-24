import { createResearchHandler } from '#lib/server/research/handler';
import { searchEvidence } from '#lib/server/research/tavily';
import { generateTopic,verifyGrounding } from '#lib/server/research/openai';
export const POST=createResearchHandler({search:searchEvidence,generate:generateTopic,verify:verifyGrounding});

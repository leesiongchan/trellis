import { ResearchRequestSchema, ResearchResultSchema } from '#lib/knowledge/schemas.ts';
import type { ResearchRequest, ResearchResult } from '#lib/knowledge/types.ts';
import { LIMITS } from '../config';
import { validateEvidence } from './grounding';
import type { ResearchDependencies } from './types';
export async function researchTopic(
  input: ResearchRequest,
  parentSignal: AbortSignal,
  deps: ResearchDependencies
): Promise<ResearchResult> {
  const signal = AbortSignal.any([parentSignal, AbortSignal.timeout(LIMITS.researchMs)]);
  signal.throwIfAborted();
  const request = ResearchRequestSchema.parse(input);
  const context = request.ancestors.map((a) => a.title).join(' → ');
  const sources = await deps.search(
    [request.question, request.clarification, context ? 'Context: ' + context : '']
      .filter(Boolean)
      .join(' — '),
    signal
  );
  signal.throwIfAborted();
  if (!sources.length)
    return {
      kind: 'insufficient-evidence',
      message:
        'I could not retrieve enough reliable source text for this topic. Try a more specific question.'
    };
  const result = ResearchResultSchema.parse(await deps.generate(request, sources, signal));
  signal.throwIfAborted();
  if (result.kind !== 'topic') return result;
  try {
    validateEvidence(result.content, sources);
  } catch {
    return {
      kind: 'insufficient-evidence',
      message: 'The explanation did not pass its source check. Try again or narrow the topic.'
    };
  }
  const verdict = await deps.verify(result.content, sources, signal);
  signal.throwIfAborted();
  if (verdict.unsupportedParagraphIds.length || verdict.unsupportedBranchIndexes.length)
    return {
      kind: 'insufficient-evidence',
      message:
        'Some claims were not sufficiently supported by the retrieved sources. Please retry or make the question more specific.'
    };
  return result;
}

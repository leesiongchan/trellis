import { ApiErrorSchema, ResearchResultSchema } from './schemas';
import { newId } from './normalize';
import type { ResearchClient } from './types';
export const requestResearch: ResearchClient = async (input, signal) => {
  const requestId = newId();
  let response: Response;
  try {
    response = await fetch('/api/research', {
      method: 'POST',
      headers: { 'content-type': 'application/json', 'x-request-id': requestId },
      body: JSON.stringify(input),
      signal
    });
  } catch (error) {
    if (signal.aborted) throw error;
    throw {
      code: 'OFFLINE',
      message: 'Cannot reach the research service. Your saved topics are still here.',
      retryable: true,
      requestId
    };
  }
  let body: unknown;
  try {
    body = await response.json();
  } catch {
    throw {
      code: 'INVALID_RESPONSE',
      message: 'The research service sent an unreadable response. Please retry.',
      retryable: true,
      requestId
    };
  }
  if (!response.ok) {
    const parsed = ApiErrorSchema.safeParse((body as { error?: unknown })?.error);
    throw parsed.success
      ? parsed.data
      : {
          code: 'RESEARCH_FAILED',
          message: 'Research could not finish. Please retry.',
          retryable: true,
          requestId
        };
  }
  const result = ResearchResultSchema.safeParse(body);
  if (!result.success)
    throw {
      code: 'INVALID_RESPONSE',
      message: 'The explanation was incomplete. Please retry.',
      retryable: true,
      requestId
    };
  return result.data;
};

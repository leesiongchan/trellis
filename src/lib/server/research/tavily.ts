import { z } from 'zod';
import { SourceSchema } from '#lib/knowledge/schemas.ts';
import { isPublicSourceUrl, newId } from '#lib/knowledge/normalize.ts';
import { readConfig, ServiceError, LIMITS } from '../config';
import type { RetrievedSource } from './types';
const SearchResponse = z.object({ results: z.array(z.unknown()) });
const SearchItem = z.object({
  title: z.string().min(1),
  url: z.string(),
  raw_content: z.string().min(1)
});
export function parseSearchResults(raw: unknown): RetrievedSource[] {
  const results = SearchResponse.parse(raw),
    sources: RetrievedSource[] = [],
    urls = new Set<string>();
  for (const item of results.results) {
    const parsed = SearchItem.safeParse(item);
    if (!parsed.success) continue;
    const { title, url, raw_content } = parsed.data;
    if (!isPublicSourceUrl(url) || urls.has(url) || !raw_content.trim()) continue;
    const source = SourceSchema.safeParse({
      id: newId(),
      title: title.slice(0, 500),
      url,
      publisher: new URL(url).hostname,
      retrievedAt: new Date().toISOString()
    });
    if (!source.success) continue;
    urls.add(url);
    sources.push({ ...source.data, text: raw_content.slice(0, 12000) });
    if (sources.length === 5) break;
  }
  return sources;
}
export async function searchEvidence(
  query: string,
  signal: AbortSignal
): Promise<RetrievedSource[]> {
  const config = readConfig();
  if (!config.tavilyKey)
    throw new ServiceError(
      'SETUP_REQUIRED',
      'Web research is not connected. Add TAVILY_API_KEY on the server.',
      503,
      false
    );
  const response = await fetch('https://api.tavily.com/search', {
    method: 'POST',
    redirect: 'error',
    headers: { Authorization: 'Bearer ' + config.tavilyKey, 'Content-Type': 'application/json' },
    body: JSON.stringify({
      query,
      search_depth: 'advanced',
      max_results: 5,
      include_raw_content: 'text',
      include_answer: false,
      include_images: false
    }),
    signal: AbortSignal.any([signal, AbortSignal.timeout(LIMITS.searchMs)])
  });
  if (!response.ok)
    throw new ServiceError(
      response.status === 429 ? 'PROVIDER_LIMIT' : 'SEARCH_FAILED',
      'The web research service could not complete this search. Please try again later.',
      response.status === 429 ? 429 : 502
    );
  return parseSearchResults(await response.json());
}

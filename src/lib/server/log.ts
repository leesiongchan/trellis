export function logResearch(
  event: string,
  requestId: string,
  fields: { stage?: string; durationMs?: number; sourceCount?: number; code?: string } = {}
) {
  console.info(JSON.stringify({ event, requestId, entryPoint: 'research_endpoint', ...fields }));
}

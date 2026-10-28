import { z } from 'zod';
import { isPublicSourceUrl } from './normalize';

const id = z.string().min(1).max(200);
const timestamp = z.iso.datetime();
const question = z.string().trim().min(1).max(500);
export const SourceSchema = z.object({
  id,
  title: z.string().min(1).max(500),
  url: z.string().max(4096).refine(isPublicSourceUrl, 'A public HTTPS source URL is required'),
  publisher: z.string().max(500).nullable(),
  retrievedAt: timestamp
});
export const EvidenceRefSchema = z.object({ sourceId: id, quote: z.string().min(1).max(600) });
export const ParagraphSchema = z.object({
  id,
  text: z.string().min(1).max(6000),
  evidence: z.array(EvidenceRefSchema).min(1).max(5)
});
export const BranchSchema = z.object({
  title: z.string().min(1).max(160),
  question,
  relationship: z.string().min(1).max(100),
  reason: ParagraphSchema
});
export const TopicContentSchema = z.object({
  title: z.string().min(1).max(160),
  summary: z.array(ParagraphSchema).min(1).max(5),
  sections: z
    .array(
      z.object({
        heading: z.string().min(1).max(160),
        paragraphs: z.array(ParagraphSchema).min(1).max(12)
      })
    )
    .min(1)
    .max(12),
  analogy: z.string().max(3000).nullable(),
  limitations: z.string().max(3000).nullable(),
  sources: z.array(SourceSchema).min(1).max(5),
  branches: z.array(BranchSchema).max(6),
  researchedAt: timestamp
});
export const ApiErrorSchema = z.object({
  code: z.string(),
  message: z.string(),
  retryable: z.boolean(),
  requestId: z.string()
});
export const TopicNodeSchema = z.object({
  id,
  parentId: id.nullable(),
  question,
  relationship: z.string().max(100),
  teaser: z.string().max(6000),
  status: z.enum(['suggested', 'researching', 'ready', 'failed']),
  content: TopicContentSchema.nullable(),
  childIds: z.array(id),
  lastError: ApiErrorSchema.nullable(),
  isRefreshing: z.boolean()
});
export const ViewSchema = z.object({
  selectedNodeId: id,
  expandedNodeIds: z.array(id).max(2000),
  viewport: z.object({
    x: z.number().finite().min(-10_000_000).max(10_000_000),
    y: z.number().finite().min(-10_000_000).max(10_000_000),
    zoom: z.number().min(0.1).max(3)
  }),
  mobilePane: z.enum(['map', 'reader'])
});
export const ExplorationSchema = z
  .object({
    schemaVersion: z.literal(1),
    mode: z.literal('research').default('research'),
    id,
    originalPrompt: question,
    rootId: id,
    nodes: z.record(id, TopicNodeSchema),
    view: ViewSchema,
    revision: z.number().int().min(0),
    createdAt: timestamp,
    updatedAt: timestamp
  })
  .superRefine((v, ctx) => {
    const fail = () =>
      ctx.addIssue({
        code: 'custom',
        message: 'The saved exploration has invalid tree references.'
      });
    if (
      !v.nodes[v.rootId] ||
      v.nodes[v.rootId].parentId !== null ||
      !v.nodes[v.view.selectedNodeId] ||
      v.view.expandedNodeIds.some((n) => !v.nodes[n])
    )
      return fail();
    const seen = new Set<string>(),
      stack = [v.rootId];
    while (stack.length) {
      const key = stack.pop()!,
        n = v.nodes[key];
      if (!n || seen.has(key) || n.id !== key || (n.status === 'ready' && !n.content))
        return fail();
      seen.add(key);
      for (const child of n.childIds) {
        if (!v.nodes[child] || v.nodes[child].parentId !== key) return fail();
        stack.push(child);
      }
    }
    if (seen.size !== Object.keys(v.nodes).length) fail();
  });
export const ResearchRequestSchema = z
  .object({
    explorationId: id,
    nodeId: id,
    question,
    clarification: question.nullable(),
    ancestors: z
      .array(z.object({ title: z.string().max(160), summary: z.string().max(6000) }))
      .max(50)
  })
  .refine(
    (v) => v.ancestors.reduce((n, a) => n + a.summary.length, 0) <= 6000,
    'Ancestor context is too long'
  );
export const ResearchResultSchema = z.discriminatedUnion('kind', [
  z.object({ kind: z.literal('topic'), content: TopicContentSchema }),
  z.object({ kind: z.literal('clarification'), question }),
  z.object({ kind: z.literal('insufficient-evidence'), message: z.string().min(1).max(2000) })
]);

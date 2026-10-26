import { z } from 'zod';
import {
  EvidenceRefSchema,
  ParagraphSchema,
  ResearchResultSchema,
  TopicContentSchema,
  SourceSchema
} from './schemas';
export const OpaqueIdSchema = z
  .string()
  .min(1)
  .max(200)
  .regex(/^[A-Za-z0-9_-]+$/);
const id = OpaqueIdSchema;
const integer = z.number().int().min(0).max(Number.MAX_SAFE_INTEGER);
const epoch = integer.min(1);
const timestamp = z.iso.datetime();
const question = z
  .string()
  .trim()
  .min(1)
  .max(500)
  .refine((v) => v.length <= 500);
const locale = z
  .string()
  .min(1)
  .max(35)
  .regex(/^[A-Za-z0-9-]+$/);
export const commandSchema = <T extends z.ZodType>(payload: T) =>
  z.object({ operationId: id, libraryEpoch: epoch, expectedRevision: integer.nullable(), payload });
export const CreateExplorationCommandSchema = commandSchema(z.object({ question, locale }));
export const AddBranchCommandSchema = commandSchema(z.object({ parentAppearanceId: id, question }));
export const ResearchCommandSchema = commandSchema(
  z.object({ appearanceId: id, refresh: z.boolean(), clarification: question.nullable() })
);
export const ChooseAnswerCommandSchema = commandSchema(
  z.object({ appearanceId: id, answerVersionId: id })
);
export const DismissConnectionCommandSchema = commandSchema(z.object({ connectionId: id }));
export const DeleteExplorationCommandSchema = commandSchema(z.object({ explorationId: id }));
export const PURGE_CONFIRMATION =
  'Delete all server library content; offline copies and retained backups require separate clearing';
export const PurgeLibraryCommandSchema = commandSchema(
  z.object({ confirmation: z.literal(PURGE_CONFIRMATION) })
);
export const LibraryInfoSchema = z.object({
  protocolVersion: z.literal(2),
  schemaVersion: z.literal(2),
  libraryId: id,
  libraryEpoch: epoch,
  discovery: z.enum(['ready', 'unavailable', 'indexing'])
});
export const ExplorationSummarySchema = z.object({
  id,
  originalPrompt: question,
  rootId: id,
  revision: integer,
  createdAt: timestamp,
  updatedAt: timestamp
});
export const AppearanceSchema = z.object({
  id,
  explorationId: id,
  parentId: id.nullable(),
  topicId: id,
  question,
  locale,
  clarification: question.nullable(),
  ancestors: z
    .array(z.object({ title: z.string().max(160), summary: z.string().max(6000) }))
    .max(50),
  interpretation: z.string().max(500),
  relationship: z.string().max(100),
  reason: ParagraphSchema.nullable(),
  reasonSources: z.array(SourceSchema).max(5),
  reasonAnswerVersionId: id.nullable(),
  answerVersionId: id.nullable(),
  childIds: z.array(id).max(2000)
});
export const AnswerVersionSchema = z.object({
  id,
  topicId: id,
  revision: integer,
  content: TopicContentSchema,
  createdAt: timestamp
});
export const ExplorationSnapshotSchema = z.object({
  schemaVersion: z.literal(2),
  libraryEpoch: epoch,
  exploration: ExplorationSummarySchema,
  appearances: z.array(AppearanceSchema).min(1).max(2000),
  answers: z.array(AnswerVersionSchema).max(2000)
});
export const TopicViewSchema = z.object({
  id,
  question,
  currentAnswerVersionId: id.nullable(),
  selectedAnswer: AnswerVersionSchema.nullable(),
  versions: z.array(z.object({ id, revision: integer, createdAt: timestamp }))
});
export const PassageLocationSchema = z.object({
  answerVersionId: id,
  paragraphId: z.string().min(1).max(200),
  sectionIndex: integer.nullable()
});
export const ConnectionViewSchema = z.object({
  id,
  fromTopicId: id,
  toTopicId: id,
  kind: z.enum([
    'branches_to',
    'equivalent_question',
    'similar_question',
    'overlapping_answer',
    'related_topic'
  ]),
  status: z.enum(['candidate', 'accepted', 'dismissed']),
  reason: z.string().max(2000),
  method: z.string().max(100),
  modelVersion: z.string().max(200).nullable(),
  score: z.number().finite().min(-1).max(1).nullable(),
  passages: z.array(PassageLocationSchema).max(6),
  evidence: z.array(EvidenceRefSchema).max(10),
  citationSources:z.array(z.object({topicId:id,answerVersionId:id,paragraphId:z.string().min(1).max(200),source:SourceSchema,quote:z.string().min(1).max(600)})).max(10)
});
export const pageSchema = <T extends z.ZodType>(item: T) =>
  z.object({ items: z.array(item).max(100), nextCursor: z.string().max(1000).nullable() });
export const OperationResultSchema = z.discriminatedUnion('kind', [
  z.object({
    kind: z.literal('exploration'),
    explorationId: id,
    appearanceId: id,
    revision: integer
  }),
  z.object({
    kind: z.literal('research'),
    explorationId: id,
    appearanceId: id,
    answerVersionId: id.nullable(),
    revision: integer,
    outcome: ResearchResultSchema
  }),
  z.object({ kind: z.literal('connection'), connectionId: id }),
  z.object({ kind: z.literal('deletion'), explorationId: id }),
  z.object({ kind: z.literal('purge'), libraryEpoch: epoch }),
  z.object({ kind: z.literal('import'), importId: id, explorationId: id, revision: integer })
]);
export const OperationViewSchema = z.object({
  id,
  state: z.enum(['pending', 'completed', 'failed', 'interrupted']),
  result: OperationResultSchema.nullable(),
  error: z.object({ code: z.string(), message: z.string(), retryable: z.boolean() }).nullable(),
  createdAt: timestamp,
  updatedAt: timestamp
});
export const BeginImportCommandSchema = commandSchema(
  z.object({
    legacyExplorationId: z.string().min(1).max(200),
    contentDigest: z.string().regex(/^[0-9a-fA-F]{64}$/),
    expectedBytes: integer.min(1).max(32 * 1024 * 1024),
    expectedNodes: integer.min(1).max(2000)
  })
);
export const ImportBatchCommandSchema = commandSchema(
  z.object({
    importId: id,
    batchIndex: integer.max(2000),
    records: z
      .array(z.unknown())
      .min(1)
      .max(2000)
      .refine((v) => new TextEncoder().encode(JSON.stringify(v)).byteLength <= 2 * 1024 * 1024)
  })
);
export const FinalizeImportCommandSchema = commandSchema(z.object({ importId: id }));
export const ImportViewSchema = z.object({
  id,
  state: z.enum(['staging', 'completed', 'failed', 'suppressed']),
  mapping: z.record(z.string(), id),
  serverRevision: integer.nullable(),
  error: z.object({ code: z.string(), message: z.string(), retryable: z.boolean() }).nullable()
});

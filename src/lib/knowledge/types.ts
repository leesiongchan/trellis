import type { z } from 'zod';
import type {
  ApiErrorSchema,
  BranchSchema,
  EvidenceRefSchema,
  ExplorationSchema,
  ParagraphSchema,
  ResearchRequestSchema,
  ResearchResultSchema,
  SourceSchema,
  TopicContentSchema,
  TopicNodeSchema,
  ViewSchema
} from './schemas';
export type Source = z.infer<typeof SourceSchema>;
export type EvidenceRef = z.infer<typeof EvidenceRefSchema>;
export type CitedParagraph = z.infer<typeof ParagraphSchema>;
export type BranchSuggestion = z.infer<typeof BranchSchema>;
export type TopicContent = z.infer<typeof TopicContentSchema>;
export type TopicNode = z.infer<typeof TopicNodeSchema>;
export type TopicStatus = TopicNode['status'];
export type Exploration = z.infer<typeof ExplorationSchema>;
export type ViewState = z.infer<typeof ViewSchema>;
export type ApiError = z.infer<typeof ApiErrorSchema>;
export type ResearchRequest = z.infer<typeof ResearchRequestSchema>;
export type ResearchResult = z.infer<typeof ResearchResultSchema>;
export type ResearchClient = (
  input: ResearchRequest,
  signal: AbortSignal
) => Promise<ResearchResult>;
export interface ExplorationRepository {
  list(): Promise<Exploration[]>;
  get(id: string): Promise<Exploration | null>;
  save(value: Exploration, expectedRevision: number): Promise<Exploration>;
  remove(id: string): Promise<void>;
}

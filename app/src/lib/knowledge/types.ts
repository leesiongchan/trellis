import type { z } from 'zod';
import type { ImportedFrom } from './library-types';
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
// Rendering shape projected from acknowledged server snapshots.
export type WorkspaceExploration = Omit<Exploration, 'schemaVersion'> & {
  libraryEpoch?: number;
  viewRestored?: boolean;
  importedFrom?: ImportedFrom;
  nodes: Record<string, TopicNode & { topicId?: string; answerVersionId?: string | null }>;
};
export interface WorkspaceSession {
  readOnly?: boolean;
  exploration: WorkspaceExploration | null;
  clarification: { nodeId: string; question: string } | null;
  notice: string;
  select(id: string): Promise<void>;
  expand(id: string): Promise<void>;
  collapse(id: string): void;
  addQuestion(parentId: string, question: string): Promise<string | undefined>;
  refresh(id: string): Promise<void>;
  clarify(id: string, answer: string): Promise<void>;
  setView(patch: Partial<ViewState>): void;
}

export type SavedExploration = Pick<
  WorkspaceExploration,
  'id' | 'originalPrompt' | 'updatedAt' | 'revision'
> & { topicCount?: number; importedFrom?: ImportedFrom };

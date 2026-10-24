import type {
  ResearchRequest,
  ResearchResult,
  Source,
  TopicContent
} from '#lib/knowledge/types.ts';
export type RetrievedSource = Source & { text: string };
export type DraftResult = ResearchResult;
export type GroundingVerdict = {
  unsupportedParagraphIds: string[];
  unsupportedBranchIndexes: number[];
};
export interface ResearchDependencies {
  search(query: string, signal: AbortSignal): Promise<RetrievedSource[]>;
  generate(
    input: ResearchRequest,
    sources: RetrievedSource[],
    signal: AbortSignal
  ): Promise<DraftResult>;
  verify(
    draft: TopicContent,
    sources: RetrievedSource[],
    signal: AbortSignal
  ): Promise<GroundingVerdict>;
}

import { newId, normalizeQuestion, questionKey } from './normalize';
import { ResearchRequestSchema } from './schemas';
import { ancestorPath } from './tree';
import type { ApiError, Exploration, ResearchClient, ResearchRequest, TopicNode, ViewState } from './types';

const localError = (message: string, code = 'RESEARCH_FAILED'): ApiError => ({ code, message, retryable: true, requestId: 'local' });
const nodeFor = (question: string, parentId: string | null, relationship: string, teaser = ''): TopicNode => ({
  id: newId(), parentId, question, relationship, teaser, status: 'suggested', content: null, childIds: [], lastError: null, isRefreshing: false
});
export class KnowledgeSession {
  exploration = $state.raw<Exploration | null>(null);
  clarification = $state.raw<{ nodeId: string; question: string } | null>(null);
  notice = $state('');
  private epoch = 0;
  private jobs = new Map<string, { promise: Promise<void>; controller: AbortController }>();
  constructor(private options: { research: ResearchClient; onChange: (value: Exploration) => void }) {}
  private update(change: (value: Exploration) => void) {
    if (!this.exploration) return;
    const value = structuredClone(this.exploration); change(value); value.updatedAt = new Date().toISOString();
    this.exploration = value; this.options.onChange(value);
  }
  private reset() { this.epoch++; for (const job of this.jobs.values()) job.controller.abort(); this.jobs.clear(); this.clarification = null; this.notice = ''; }
  load(value: Exploration) { this.reset(); this.exploration = structuredClone(value); }
  dispose() { this.reset(); }
  async start(raw: string) {
    const question = normalizeQuestion(raw);
    ResearchRequestSchema.parse({ explorationId:'new', nodeId:'new', question, clarification:null, ancestors:[] });
    this.reset(); const root = nodeFor(question, null, 'Starting topic'), now = new Date().toISOString();
    this.exploration = { schemaVersion:1, id:newId(), originalPrompt:question, rootId:root.id, nodes:{ [root.id]:root }, revision:0, createdAt:now, updatedAt:now,
      view:{ selectedNodeId:root.id, expandedNodeIds:[root.id], viewport:{ x:0,y:0,zoom:1 }, mobilePane:'map' } };
    this.options.onChange(this.exploration); await this.research(root.id);
  }
  async select(nodeId: string) {
    if (!this.exploration?.nodes[nodeId]) return;
    this.setView({ selectedNodeId:nodeId, mobilePane:'reader' });
    if (!this.exploration.nodes[nodeId].content) await this.research(nodeId);
  }
  async expand(nodeId: string) {
    if (!this.exploration?.nodes[nodeId]) return;
    this.update(v => { v.view.selectedNodeId=nodeId; if (!v.view.expandedNodeIds.includes(nodeId)) v.view.expandedNodeIds.push(nodeId); });
    if (!this.exploration.nodes[nodeId].content) await this.research(nodeId);
  }
  collapse(nodeId: string) { this.update(v => { v.view.expandedNodeIds=v.view.expandedNodeIds.filter(id=>id!==nodeId); }); }
  setView(patch: Partial<ViewState>) { this.update(v => { v.view={...v.view,...patch}; }); }
  async addQuestion(parentId: string, raw: string) {
    const question=normalizeQuestion(raw), value=this.exploration;
    if(!value?.nodes[parentId]) throw new Error('Choose a parent topic first.');
    ResearchRequestSchema.parse({ explorationId:value.id,nodeId:parentId,question,clarification:null,ancestors:[] });
    const key=questionKey(question), ancestors=ancestorPath(value,parentId);
    const sibling=value.nodes[parentId].childIds.find(id=>questionKey(value.nodes[id].question)===key);
    if(sibling) { this.notice='That question already has a branch. Opening it here.'; this.update(v=>{ if(!v.view.expandedNodeIds.includes(parentId))v.view.expandedNodeIds.push(parentId); }); await this.select(sibling); return sibling; }
    if(ancestors.some(n=>questionKey(n.question)===key)) throw new Error('That topic is already on this path. Select it in the breadcrumb.');
    const child=nodeFor(question,parentId,'Your question');
    this.update(v=>{v.nodes[child.id]=child;v.nodes[parentId].childIds.push(child.id);if(!v.view.expandedNodeIds.includes(parentId))v.view.expandedNodeIds.push(parentId);});
    await this.select(child.id); return child.id;
  }
  refresh(nodeId:string) { return this.research(nodeId,true); }
  async clarify(nodeId:string,answer:string) { await this.research(nodeId,true,normalizeQuestion(answer)); }
  private research(nodeId: string, force=false, clarification: string|null=null): Promise<void> {
    const existing=this.jobs.get(nodeId); if(existing)return existing.promise;
    const value=this.exploration, node=value?.nodes[nodeId];
    if(!value || !node || (node.content && !force)) return Promise.resolve();
    const epoch=this.epoch, explorationId=value.id, controller=new AbortController();
    const path=ancestorPath(value,nodeId).slice(0,-1), context=path.length>15?[path[0],...path.slice(-14)]:path;
    const input:ResearchRequest={explorationId,nodeId,question:node.question,clarification,ancestors:context.map(n=>({title:(n.content?.title??n.question).slice(0,160),summary:(n.content?.summary.map(p=>p.text).join(' ')??n.teaser).slice(0,400)}))};
    this.clarification=null;
    this.update(v=>{const n=v.nodes[nodeId];n.lastError=null;if(n.content)n.isRefreshing=true;else n.status='researching';});
    // Schedule provider invocation after registering the shared promise, including synchronous failures.
    const promise=Promise.resolve().then(async()=>{
      try {
        const result=await this.options.research(input,controller.signal);
        if(epoch!==this.epoch || this.exploration?.id!==explorationId)return;
        if(result.kind==='clarification') {
          this.clarification={nodeId,question:result.question};
          this.update(v=>{const n=v.nodes[nodeId];n.status=n.content?'ready':'suggested';n.isRefreshing=false;});return;
        }
        if(result.kind==='insufficient-evidence')throw localError(result.message,'INSUFFICIENT_EVIDENCE');
        this.update(v=>{
          const n=v.nodes[nodeId], ancestorKeys=new Set(ancestorPath(v,nodeId).map(a=>questionKey(a.question)));
          const byQuestion=new Map(n.childIds.map(id=>[questionKey(v.nodes[id].question),id]));
          for(const suggestion of result.content.branches){
            const key=questionKey(suggestion.question);if(ancestorKeys.has(key)||byQuestion.has(key))continue;
            const child=nodeFor(suggestion.question,nodeId,suggestion.relationship,suggestion.reason.text);
            v.nodes[child.id]=child;n.childIds.push(child.id);byQuestion.set(key,child.id);
          }
          n.content=result.content;n.status='ready';n.lastError=null;n.isRefreshing=false;
        });
      } catch(error) {
        if(epoch!==this.epoch || this.exploration?.id!==explorationId)return;
        const candidate=error as Partial<ApiError>;
        const failure= typeof candidate?.message==='string' ? {...localError(candidate.message),...candidate} as ApiError : localError('Research could not finish. Please try again.');
        this.update(v=>{const n=v.nodes[nodeId];n.lastError=failure;n.status=n.content?'ready':'failed';n.isRefreshing=false;});
      } finally { if(epoch===this.epoch)this.jobs.delete(nodeId); }
    });
    this.jobs.set(nodeId,{promise,controller});return promise;
  }
}
export const createSession = (options: { research: ResearchClient; onChange: (value: Exploration) => void }): KnowledgeSession => new KnowledgeSession(options);

# Knowledge Tree Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the approved private knowledge tree: detailed, sourced topic explanations beside an indefinitely expandable, saved tree.

**Architecture:** One SvelteKit app serves the interface and a protected research endpoint. Browser-owned exploration state is stored in IndexedDB; the backend retrieves web evidence, generates structured explanations, and checks their support before returning complete results. Svelte Flow renders the visible portion of a strict tree; content state and viewport state remain separate.

**Tech Stack:** Svelte 5, SvelteKit with adapter-node, TypeScript, `@xyflow/svelte`, Zod, `idb`, the official `openai` Node SDK, Tavily Search over native `fetch`, Vitest, Playwright. Use Node 24+ and pin the available pnpm 11.21.0 in `packageManager`. Resolve compatible stable dependency versions at bootstrap and retain one `pnpm-lock.yaml`.

**Spec:** `docs/superpowers/specs/2026-10-25-knowledge-tree-design.md` (approved 2026-10-25).

## Global Constraints

- “Begin by asking for a topic.”
- “Explain each topic in detail and offer multiple branches into further topics.”
- “Use approach A from the mockups: an expanding tree with the selected topic’s explanation in a separate panel.”
- “Research each opened topic on the web and include visible sources.”
- “Only explicit user actions generate more content. There is no recursive background expansion.”
- “Four to six is a target, not a quota: use fewer when additional suggestions would be redundant or weak.”
- “Collapsing hides descendants without discarding them or canceling unrelated work.”
- “Repeated concepts in different branches remain separate context-specific nodes in the first version.”
- “Persistence is local to the current browser/device.”
- “The existing Tailscale preview uses port 60090 forwarding to the local companion on 60089. Do not overwrite that sharing configuration as part of app planning.”
- “Do not publish to the public internet as an implicit part of this design.”
- Existing `.superpowers/` files are throwaway preview assets. Preserve them and exclude them from version control.
- No app exists yet and the directory is not a Git repository. Initialize version control only when implementation starts; retain the approved design and plan in the initial repository.
- On this Nix system, retry missing tools with `,`; use `rg`, `bat`, `fd`, and `eza` where applicable.

## Review Focus

1. Switching explorations or deleting one while a request is running must not attach results to another tree or resurrect a deleted tree. Owned by Task 2.
2. A failed refresh must leave the previous explanation, sources, and already explored descendants readable. Owned by Tasks 2 and 7.
3. A browser reload during research must restore completed content and offer an explicit retry without automatically starting another paid request. Owned by Task 3.
4. Storage quota failures, corrupted records, and two tabs saving the same exploration must not silently erase work or display a false “saved” state. Owned by Task 3.
5. Unicode questions, ambiguous roots, malformed upstream output, and hostile source text must produce clarification or safe errors, never executable markup or fabricated citations. Owned by Tasks 1, 4, 5, and 7.

## Decisions made for implementation

These select the builder choices delegated by the design. They are proposals for review with this plan.

- Use SvelteKit so the browser and research API share types and one deployment. Use Svelte 5 runes with per-workspace state rather than a server-global reactive store.
- Use Svelte Flow for pan/zoom, connections, and viewport helpers. Implement a small deterministic tree layout rather than adding a graph layout engine.
- Use Tavily Search with page text, then OpenAI structured synthesis and a separate evidence check. Separating retrieved evidence from generated text permits citation checks without arbitrary URL fetching in our server. It requires `TAVILY_API_KEY` and `OPENAI_API_KEY`.
- Set `OPENAI_MODEL=gpt-5.6-terra` as the initial configurable default with reasoning effort `low`; the fetched official model documentation supports Responses and structured output. Recheck availability for the user's API project at execution. Do not silently substitute an unavailable model or claim account access has been verified.
- Return a single complete JSON result initially. A loading state is sufficient; streaming is optional in the design and adds no necessary behavior to the first version.
- Render structured paragraphs with Svelte escaping. No model-generated HTML or Markdown renderer is needed for the initial version.
- Serve the built app on loopback port 3000. Use a separate private Tailscale HTTPS listener on 8443, if free, preserving the preview's TCP listener. Tailscale access and a capability link protect the single-user app without introducing user accounts.
- All generation tests use controlled fixtures. One bounded live smoke check is still required to claim live research works.

## File structure

| Paths to create | Responsibility |
| --- | --- |
| `package.json`, `pnpm-lock.yaml`, `pnpm-workspace.yaml`, `svelte.config.js`, `vite.config.ts`, `tsconfig.json`, `.gitignore`, `.env.example` | Runtime, dependency policy, scripts, configuration placeholders |
| `src/app.html`, `src/app.d.ts`, `src/routes/+layout.svelte`, `src/app.css` | HTML shell, request-local types, base styling |
| `src/lib/knowledge/{types,schemas,normalize,tree}.ts` | Serializable domain model, boundary schemas, duplicate handling, visible tree/layout |
| `src/lib/knowledge/{session.svelte,persistence,client}.ts` | Workspace orchestration, IndexedDB repository, research HTTP client |
| `src/lib/components/{TopicStart,ExplorationList,Workspace,TopicTree,TopicNode,TopicReader,CitationLink,BranchQuestion,WorkspaceControls}.svelte` | Focused UI components |
| `src/lib/server/{config,access,request-guard,log}.ts`, `src/hooks.server.ts` | Private settings, cookie capability access, origin checks, request limits, structured diagnostics |
| `src/lib/server/research/{types,tavily,prompts,openai,grounding,service,handler}.ts` | Retrieval, typed generation, evidence checks, bounded orchestration, testable HTTP handler |
| `src/routes/access/+server.ts`, `src/routes/api/research/+server.ts`, `src/routes/+page.svelte` | Capability entry, research endpoint, app composition |
| `tests/fixtures/knowledge.ts`, `tests/unit/*.test.ts`, `tests/integration/*.test.ts` | Reusable fixtures and meaningful domain/API tests |
| `tests/e2e/*.spec.ts`, `playwright.config.ts` | Browser acceptance scenarios with controlled research responses |
| `scripts/live-smoke.ts`, `README.md`, `docs/runbooks/research.md` | Bounded live check, private setup, diagnosis instructions |

## Shared contracts

Task 1 defines these names and their Zod schemas; subsequent tasks consume them rather than inventing parallel shapes.

- IDs and timestamps are strings; timestamps use ISO 8601. All IDs are generated in application code, never accepted from model output. Browser IDs use `crypto.getRandomValues`, so existing HTTP Tailscale previews are not dependent on secure-context-only `crypto.randomUUID`.
- `TopicStatus = 'suggested' | 'researching' | 'ready' | 'failed'`. Refresh uses separate `isRefreshing` and `lastError` fields while status stays ready.
- `Source = { id, title, url, publisher: string | null, retrievedAt }`.
- `EvidenceRef = { sourceId: string, quote: string }`.
- `CitedParagraph = { id: string, text: string, evidence: EvidenceRef[] }`.
- `BranchSuggestion = { title, question, reason: CitedParagraph, relationship: string }`.
- `TopicContent = { title, summary: CitedParagraph[], sections: { heading: string, paragraphs: CitedParagraph[] }[], analogy: string | null, limitations: string | null, sources: Source[], branches: BranchSuggestion[], researchedAt }`.
- `TopicNode = { id, parentId: string | null, question, relationship, teaser, status, content: TopicContent | null, childIds: string[], lastError: ApiError | null, isRefreshing: boolean }`.
- `ViewState = { selectedNodeId, expandedNodeIds: string[], viewport: { x, y, zoom }, mobilePane: 'map' | 'reader' }`.
- `Exploration = { schemaVersion: 1, id, originalPrompt, rootId, nodes: Record<string, TopicNode>, view: ViewState, revision: number, createdAt, updatedAt }`.
- `ResearchRequest = { explorationId, nodeId, question, clarification: string | null, ancestors: { title, summary }[] }`.
- `ResearchResult = { kind: 'topic', content: TopicContent } | { kind: 'clarification', question: string } | { kind: 'insufficient-evidence', message: string }`.
- `ApiError = { code: string, message: string, retryable: boolean, requestId: string }`; error envelopes are `{ error: ApiError }`.
- `ResearchClient = (input: ResearchRequest, signal: AbortSignal) => Promise<ResearchResult>`.
- `ExplorationRepository`: `list(): Promise<Exploration[]>`, `get(id: string): Promise<Exploration | null>`, `save(value: Exploration, expectedRevision: number): Promise<Exploration>`, `remove(id: string): Promise<void>`. Save compares revisions inside one IndexedDB read-write transaction and increments the revision on success. New unsaved explorations have revision zero; an absent record accepts only expected revision zero. Saved records begin at revision one.
- `ResearchDependencies = { search: (query: string, signal: AbortSignal) => Promise<RetrievedSource[]>, generate: (input: ResearchRequest, sources: RetrievedSource[], signal: AbortSignal) => Promise<DraftResult>, verify: (draft: TopicContent, sources: RetrievedSource[], signal: AbortSignal) => Promise<GroundingVerdict> }`.
- `RetrievedSource = Source & { text: string }`; `DraftResult` has the same result variants as `ResearchResult`. `GroundingVerdict = { unsupportedParagraphIds: string[], unsupportedBranchIndexes: number[] }`.

Boundary limits chosen for this implementation: trimmed questions 1–500 characters; at most 64 KiB request body; ancestor summaries at most 6,000 characters total, retaining the root and nearest relevant ancestors; at most six suggested branches; at most five retrieved sources, each capped at 12,000 characters; 120-second total research timeout; 20-second search timeout; two concurrent server research jobs; ten new research jobs per minute per app instance. The server is a single process. Limits bound a request, not the depth a user can explore across successive requests.

### Task 1: Runnable shell and validated knowledge contracts

**Files:** Create the root configuration/shell files, `src/lib/knowledge/{types,schemas,normalize}.ts`, `tests/fixtures/knowledge.ts`, `tests/unit/contracts.test.ts`, and a minimal `src/routes/+page.svelte`.

**Interfaces:** Consumes the approved spec. Produces all shared domain types/schemas, `normalizeQuestion(value: string): string`, `newId(): string`, and fixture factories `makeExploration(overrides?)`, `makeContent(overrides?)`, `makeRequest(overrides?)` returning the corresponding shared types.

- [ ] **Step 1:** Confirm there are no new user files to overwrite; initialize Git in the project root if it remains absent. Create the minimal SvelteKit manifest/configuration without running a scaffold over the existing documentation. Pin pnpm 11.21.0, add `dev`, `build`, `start`, `check`, `test:unit`, and `test:e2e` scripts. Add `.superpowers/`, `.env*` except `.env.example`, build output, and test artifacts to `.gitignore`.
- [ ] **Step 2:** Install only the chosen dependencies with lifecycle scripts blocked. Inspect any necessary script, allow only required build packages in `pnpm-workspace.yaml`, then verify a frozen install. Use compatible stable versions and record the lockfile; do not leave multiple package-manager lockfiles.
- [ ] **Step 3:** Write failing tests named `preservesUnicodeQuestions`, `rejectsBlankOrOversizedQuestions`, `rejectsUnsafeSourceUrls`, and `roundTripsAnExploration`. Assertions: `normalizeQuestion('  为什么能存储数据？  ') === '为什么能存储数据？'`; blank and 501-character questions fail; `javascript:`, `data:`, credential-bearing and local/private-address source URLs fail; `ExplorationSchema.parse(makeExploration())` retains node IDs and view state. Unicode normalization must not collapse distinct meaningful text.

  Pin boundary assertions as `expect(ResearchRequestSchema.safeParse(makeRequest({ question: ' '.repeat(3) })).success).toBe(false)` and `expect(normalizeQuestion('  为什么能存储数据？  ')).toBe('为什么能存储数据？')`.
- [ ] **Step 4:** Run `pnpm test:unit -- tests/unit/contracts.test.ts`; expect failures for missing domain functions/schemas.
- [ ] **Step 5:** Implement the shared contracts, URL validation, and input normalization in the specified files. Render a functional initial topic form shell with no provider calls yet. Use Svelte escaping and accessible labels.
- [ ] **Step 6:** Run the contract tests and `pnpm check`; expect all tests passing and zero type errors. Verify the shell starts on loopback. Commit as `feat: establish knowledge tree shell and contracts`.

### Task 2: Correct exploration state and request coordination

**Files:** Create `src/lib/knowledge/session.svelte.ts`, `src/lib/knowledge/tree.ts`, and `tests/unit/{session,tree}.test.ts`.

**Interfaces:** Consumes Task 1 contracts and `ResearchClient`. Produces `createSession({ research, onChange }): KnowledgeSession`, where `onChange(value: Exploration): void`; the session exposes `exploration: Exploration | null`, `start(question): Promise<void>`, `load(value): void`, `select(nodeId): Promise<void>`, `expand(nodeId): Promise<void>`, `collapse(nodeId): void`, `addQuestion(parentId, question): Promise<string>`, `refresh(nodeId): Promise<void>`, `clarify(nodeId, answer): Promise<void>`, `setView(patch: Partial<ViewState>): void`, and `dispose(): void`. `visibleTree(value): { nodes: TopicNode[], edges: { id, source, target }[] }`; `layoutTree(value): { id, x, y }[]`.

- [ ] **Step 1:** Write failing tests for `onlyResearchesOpenedNodes`, `deduplicatesDoubleExpansion`, `selectDoesNotExpand`, `collapsePreservesDescendants`, `ignoresPreviousExplorationResults`, `disposePreventsDeletedExplorationResurrection`, `failedRefreshPreservesContentAndChildren`, `reusesExistingChildrenOnRetry`, `suppressesAncestorAndSiblingDuplicates`, and `boundsContextWithoutLimitingDepth`. With deferred mocked responses, resolve B before A and assert each node receives its own result. Assert ready nodes cause zero additional research calls on reopen.

  For a ready child, pin `await session.select(childId); expect(research).not.toHaveBeenCalled(); expect(session.exploration.view.expandedNodeIds).not.toContain(childId)`. For collapse, preserve the content snapshot with `expect(session.exploration.nodes[childId].content).toEqual(previousContent)`.
- [ ] **Step 2:** Run `pnpm test:unit -- tests/unit/session.test.ts tests/unit/tree.test.ts`; expect failures for missing implementation.
- [ ] **Step 3:** Implement session actions using a private per-session request map and exploration generation counter. Capture exploration and node IDs before awaiting; discard results after disposal or loading another exploration. Generate child IDs once per accepted new suggestion and preserve explored children during refresh, matching normalized questions to reuse IDs. Preserve old content on refresh failure. Keep a clarification question in session UI state, preserve `originalPrompt`, and send the answer through `clarify` rather than creating an unrelated root.
- [ ] **Step 4:** Implement an iterative visible-tree traversal and layout in `tree.ts`: fixed 240×150 node cards, 80 horizontal and 24 vertical gap, parent centered over visible children. Test nonoverlap, stable sibling order, hidden descendants, and a synthetic depth of 200 without recursion failure. Do not couple IDs or data storage to coordinates.
- [ ] **Step 5:** Run the targeted tests and `pnpm check`; expect passes, including no extra request on collapse/reopen. Commit as `feat: coordinate lazy topic expansion and stable tree state`.

### Task 3: Durable browser saving and exploration switching

**Files:** Create `src/lib/knowledge/persistence.ts`, `src/lib/components/ExplorationList.svelte`, `tests/unit/persistence.test.ts`, and `tests/e2e/persistence.spec.ts`; add `fake-indexeddb` as a test dependency.

**Interfaces:** Consumes `ExplorationRepository`, Task 1 schemas, and Task 2 session lifecycle. Produces `createExplorationRepository(): ExplorationRepository`, `restoreExploration(value: unknown): Exploration`, and `createAutosave(repository, onStatus): { enqueue(value): void, flush(): Promise<void>, dispose(): void }`. `onStatus(status: 'saving' | 'saved' | 'failed' | 'conflict', message?: string): void`.

- [ ] **Step 1:** Write failing repository tests for atomic revision conflicts, serialized saves completing out of order, storage rejection, malformed/future schema versions, explicit deletion, and interrupted research. Assert restored `researching` nodes become `failed` with a retryable interruption message; completed data remains ready; restoring triggers no research. A refresh interrupted by reload must leave ready content and a refresh error.

  Pin conflicts with `await expect(repository.save(staleValue, staleValue.revision)).rejects.toThrow()` after another transaction has advanced that revision; pin interruption with `expect(restoreExploration(interrupted).nodes[interrupted.rootId].status).toBe('failed')`.
- [ ] **Step 2:** Run `pnpm test:unit -- tests/unit/persistence.test.ts`; expect failure before implementation.
- [ ] **Step 3:** Implement version-1 IndexedDB storage using `idb`, a per-exploration serialized autosave queue with a 300 ms debounce, and transaction-level revision checks. Treat conflicts as visible rather than overwriting another tab. Detect external changes through `BroadcastChannel` when available; reloading from storage is an explicit user action. Do not destroy unreadable records. Flush before switching when possible; if saving fails, keep the active in-memory work and offer retry instead of navigating away silently.
- [ ] **Step 4:** Implement `ExplorationList.svelte` with start-new, resume, and confirmed deletion callbacks; dispose/abort the session and stop its autosave before deletion so late responses cannot restore the removed record.
- [ ] **Step 5:** Write a browser test using a mocked research response: create → expand → pan → reload → resume, asserting content, selection, expansion and viewport restore without a new research call. Add a two-tab conflict case. Run `pnpm test:unit -- tests/unit/persistence.test.ts` and, once the shell integration in Task 6 exists, `pnpm test:e2e -- tests/e2e/persistence.spec.ts`. Commit the independently passing repository work as `feat: save and resume local explorations` and track the integration test until Task 6.

### Task 4: Evidence-backed research service

**Files:** Create `src/lib/server/config.ts`, `src/lib/server/research/{types,tavily,prompts,openai,grounding,service}.ts`, `tests/unit/{retrieval,grounding,research}.test.ts`; modify `.env.example`.

**Interfaces:** Consumes shared research contracts. Produces `searchEvidence(query: string, signal: AbortSignal): Promise<RetrievedSource[]>`, `generateTopic(input: ResearchRequest, sources: RetrievedSource[], signal: AbortSignal): Promise<DraftResult>`, `verifyGrounding(draft: TopicContent, sources: RetrievedSource[], signal: AbortSignal): Promise<GroundingVerdict>`, `validateEvidence(content: TopicContent, sources: RetrievedSource[]): void`, and `researchTopic(input: ResearchRequest, signal: AbortSignal, deps: ResearchDependencies): Promise<ResearchResult>`. `readConfig()` returns private keys, model, app origin/access key and the named limits from this plan, validating only when needed so missing research keys do not prevent saved content from rendering.

- [ ] **Step 1:** Write failing fixture tests: unknown source ID; quote absent from fetched text; invented URL; JavaScript URL; raw content missing; source text saying “ignore previous instructions”; schema-invalid model response; provider refusal; truncated structured output; evidence-check rejection; ordinary and ambiguous Unicode topics. Assert absent usable evidence returns `insufficient-evidence`, not a confident unsourced explanation; ambiguity returns one clarification question. Assert the model cannot introduce tools or fetch destinations.

  Pin unsupported evidence with `expect(() => validateEvidence(contentWithInventedQuote, retrievedSources)).toThrow()` and empty retrieval with `expect((await researchTopic(input, signal, depsWithNoSources)).kind).toBe('insufficient-evidence')`; assert `depsWithNoSources.generate` was not called.
- [ ] **Step 2:** Run `pnpm test:unit -- tests/unit/retrieval.test.ts tests/unit/grounding.test.ts tests/unit/research.test.ts`; expect missing-service failures.
- [ ] **Step 3:** Implement fixed-destination Tavily Search requests with `search_depth: 'advanced'`, `max_results: 5`, `include_raw_content: 'text'`, `include_answer: false`. Validate its response, retain only usable raw page text, assign source IDs on the backend, cap text as specified, and prefer original/authoritative material in synthesis. Query includes the topic, limited ancestor context, and current date when relevant. Do not impose a recent-only date window on historical subjects. Reject unsafe links; never fetch returned URLs directly. If retrieval lacks enough support, report the gap rather than doing unbounded additional searches.
- [ ] **Step 4:** Implement generation with the OpenAI Responses API, `store: false`, no tools, `maxRetries: 0`, low reasoning, at most 8,000 output tokens, and `zodTextFormat` over an object wrapping the result union. Build system instructions that distinguish evidence from instructions, prefer four to six distinct branches only when supported, label analogies, and tailor the explanation to the subject. The model uses server-assigned source IDs, never source URLs. The model-facing schema omits paragraph IDs, sources and timestamps; the generation adapter assigns IDs and attaches authoritative source metadata and the research time before returning `DraftResult`. Parse and validate the result; preserve provider refusals/truncation as safe errors rather than JSON recovery guesses.
- [ ] **Step 5:** In `grounding.ts`, require every factual paragraph and branch connection to cite known evidence with normalized exact quotes from retrieved text. Use a separate structured evidence-check call, at most 2,000 output tokens and no tools, to assess whether those quotes support the claims. Reject unsupported content with `insufficient-evidence`; do not silently remove paragraphs into an incoherent explanation. Do not present this model-assisted check as a guarantee of truth. Short quote lengths should be only what is needed to support the associated claim.
- [ ] **Step 6:** Implement `researchTopic` with a 120-second deadline and injected providers; search has a 20-second child timeout and both model calls share the remaining total deadline. Handle clarification before requiring factual-content support. Return only validated complete content; raw page text stays server-side and is not saved as part of the exploration.
- [ ] **Step 7:** Run targeted tests and `pnpm check`; assert abort propagation, no retries behind the user's back, and no secrets or raw sources in public results. Commit as `feat: research topics with validated source evidence`.

### Task 5: Protected research endpoint and diagnosable failures

**Files:** Create `src/lib/server/{access,request-guard,log}.ts`, `src/lib/server/research/handler.ts`, `src/hooks.server.ts`, `src/routes/access/+server.ts`, `src/routes/api/research/+server.ts`, `src/lib/knowledge/client.ts`, `tests/integration/{access,research-api}.test.ts`; modify `src/app.d.ts`.

**Interfaces:** Consumes `researchTopic` and shared schemas. Produces `requestResearch: ResearchClient`; `authorizeRequest(request: Request, cookieValue: string | undefined): boolean`; and `createResearchHandler(deps: ResearchDependencies): RequestHandler` in `src/lib/server/research/handler.ts`, with the route exporting `POST` using real providers. Requests carry `X-Request-ID`; replies echo it. Success: HTTP 200 with `ResearchResult`. Failures: 400 malformed body, 401 missing access, 403 wrong origin/host, 413 oversized body, 422 invalid input, 429 request/concurrency limit, 503 missing provider setup, 504 timeout, 502 upstream failure, using one `ApiError` envelope.

- [ ] **Step 1:** Write failing integration tests for absent cookie, wrong capability, hostile/missing Origin, hostile Host, non-JSON and over-64-KiB bodies including chunked bodies, whitespace questions, upstream timeout, missing keys, and concurrent requests beyond the limits. Assert simultaneous identical validated requests share one research job. Assert rejected requests make zero provider calls and errors never expose keys, stack traces, prompts, or upstream response bodies.

  Against the running test app, pin anonymous rejection with `expect((await fetch(origin + '/api/research', anonymousRequest)).status).toBe(401)` and wrong-origin rejection with `expect((await fetch(origin + '/api/research', authenticatedWrongOriginRequest)).status).toBe(403)`. Both must leave the research dependency spy untouched.
- [ ] **Step 2:** Run `pnpm test:unit -- tests/integration/access.test.ts tests/integration/research-api.test.ts`; expect route/guard failures.
- [ ] **Step 3:** Implement `/access?key=...` with timing-safe key comparison, a derived signed capability cookie with seven-day expiry, HttpOnly and SameSite=Strict; require Secure outside loopback development. Redirect to `/` with the key removed, `Cache-Control: no-store`, and `Referrer-Policy: no-referrer`. No account/login system. Compare POST Origin and Host to configured `APP_ORIGIN`; preserve framework CSRF protection and do not enable wildcard CORS. Restrict loopback development hosts explicitly.
- [ ] **Step 4:** Implement bounded-body reading, boundary schemas, shared process-wide concurrency/rate limits and deadline handling before calling the service. Atomically insert a shared in-flight Promise into a process-local map keyed by the validated canonical research payload and session capability before awaiting providers. Identical concurrent requests share that job and each receives its own request-correlated response; remove entries on settlement. This only deduplicates in-flight work in the running process and does not promise replay after a restart. Keep Svelte SSR state request-local. Set CSP to allow the app's own scripts/styles and flow transforms without arbitrary remote resources; set frame denial and MIME-sniffing protection. Do not log query strings on the access route.
- [ ] **Step 5:** Implement request-correlated JSON events (`research_started`, `search_completed`, `generation_completed`, `research_completed`, `research_failed`) with entryPoint `research_endpoint`, stage, duration, source count, token usage where available, and safe error code. Never log question text, page content, access cookies, URL keys, or provider credentials. The client maps the uniform error envelope to retryable UI errors while keeping original node/request association.
- [ ] **Step 6:** Run the integration tests and `pnpm check`; induce one mock timeout and inspect emitted JSON for correlation and redaction. Commit as `feat: protect research requests and expose safe failures`.

### Task 6: Desktop tree and detailed explanation workspace

**Files:** Create `src/lib/components/{TopicStart,Workspace,TopicTree,TopicNode,TopicReader,CitationLink,BranchQuestion,WorkspaceControls}.svelte`, `tests/e2e/knowledge-tree.spec.ts`, `playwright.config.ts`; modify `src/routes/+page.svelte`, `src/app.css`, and integrate `ExplorationList.svelte`.

**Interfaces:** Consumes Task 2 session, Task 3 repository/autosave, and Task 5 `requestResearch`. `Workspace` receives `{ session: KnowledgeSession }`. `TopicTree` receives `{ exploration: Exploration, onSelect(id), onExpand(id), onCollapse(id), onViewportChange(viewport) }`. `TopicReader` receives `{ node: TopicNode, onFollow(question), onRefresh(), onClarify(answer) }`. `CitationLink` receives `{ source: Source }`. `TopicNode` implements Svelte Flow's custom-node props and delegates actions; it does not own research or content state.

- [ ] **Step 1:** Write failing Playwright scenarios using `page.route('**/api/research', ...)`: topic submit gives a sourced explanation and first children; selecting a child loads content without expansion; expanding reveals its children; three successive levels retain the root; reopening a ready topic makes no request; adding a custom question creates one branch; equivalent existing sibling selects that sibling with a visible explanation instead of duplicating it.

  With the deterministic microSD fixture, pin `await expect(page.getByRole('button', { name: 'Expand NAND flash', exact: true })).toBeVisible()` and `await expect(page.getByRole('link', { name: 'MicroSD reference', exact: true })).toHaveAttribute('href', fixtureSource.url)`. Count intercepted requests before and after reopening to prove cache reuse.
- [ ] **Step 2:** Run `pnpm test:e2e -- tests/e2e/knowledge-tree.spec.ts`; expect missing-interface behavior failures.
- [ ] **Step 3:** Implement the interface in the visual direction of approved A: warm light background, compact connected cards, distinct selected node, substantial independently scrolling reader, and readable typography. Keep structure and copy faithful to the design; do not use the mockup as production data. Use proper custom node handles and Svelte Flow provider context; disable arbitrary edge creation, node dragging, and delete shortcuts because this is an automatically laid-out exploration tree.
- [ ] **Step 4:** Connect selection/expansion separately to session actions. Use the Task 2 layout, preserve the viewport, keep selected nodes in view after layout, and provide zoom, refocus, collapse and return-to-root controls. A hidden selected descendant remains readable when its ancestor is collapsed; show a “Reveal in tree” action that expands its ancestor path without researching it. Save viewport only on completed pan/zoom gestures, not every animation frame.
- [ ] **Step 5:** Render detailed sections, paragraph-level clickable citations, source list and research date with safe external-link attributes. Display loading, clarification, insufficient evidence, failure/retry and refresh-with-old-content states. Add custom branch input and the saved exploration list. Connect autosave with visible saving/failure/conflict states.
- [ ] **Step 6:** Run `pnpm test:e2e -- tests/e2e/knowledge-tree.spec.ts tests/e2e/persistence.spec.ts`, `pnpm test:unit`, and `pnpm check`; expect all tests through Task 6 to pass. Inspect the actual interface using agent-browser. Commit as `feat: explore a sourced knowledge tree beside its explanation`.

### Task 7: Phone navigation, accessibility and recovery behavior

**Files:** Modify `Workspace.svelte`, `TopicTree.svelte`, `TopicNode.svelte`, `TopicReader.svelte`, `WorkspaceControls.svelte`, and `src/app.css`; create `tests/e2e/{mobile,recovery}.spec.ts`.

**Interfaces:** Consumes existing session view state and error states; no new provider or persistence contracts. Produces a responsive map/reader toggle below 768 px, retaining the same session and viewport.

- [ ] **Step 1:** Write failing browser tests for a 390×844 viewport: selecting a topic opens its reader; map toggle returns to the same node/zoom; branches remain usable by touch; long Unicode titles wrap without clipping actions. Add desktop keyboard tests for selecting, expanding, refocusing and following citations with visible focus.

  Pin the phone transition with `await page.setViewportSize({ width: 390, height: 844 }); await page.getByRole('button', { name: 'Map', exact: true }).click(); await expect(page.getByRole('region', { name: 'Topic map', exact: true })).toBeVisible()`. Pin inert output with `expect(await page.locator('script[data-injected]').count()).toBe(0)` and a failed refresh by asserting the previous paragraph remains visible alongside a retry action.
- [ ] **Step 2:** Write recovery tests: A/B results arrive in reverse order; one request fails while another succeeds; retry keeps existing children; failed refresh preserves old text/sources/descendants; deletion during an in-flight request does not restore the tree; storage failure remains visible; a clarification reply preserves the user's original prompt. Inject model text containing HTML/script into the controlled response and assert it remains inert text, with no unsafe link or script execution.
- [ ] **Step 3:** Run `pnpm test:e2e -- tests/e2e/mobile.spec.ts tests/e2e/recovery.spec.ts`; expect failures for incomplete responsive/recovery behavior.
- [ ] **Step 4:** Implement phone map/reader switching without remounting or destroying the workspace session. Provide labeled touch-friendly controls, visible loading announcements, keyboard expand/collapse controls, adequate contrast, independent reader scrolling, and reduced-motion refocus. Add the explicit save/refresh retry actions the tests exercise; do not add unrelated feature work.
- [ ] **Step 5:** Run the targeted browser tests plus `pnpm check`; inspect desktop and phone screenshots with agent-browser, including a deep tree and a long explanation. Commit as `feat: support phone exploration and resilient navigation`.

### Task 8: Private runtime setup and end-to-end verification

**Files:** Create `scripts/live-smoke.ts`, `README.md`, `docs/runbooks/research.md`; finalize `.env.example` and package scripts.

**Interfaces:** Consumes the built adapter-node app, `/access`, the research API, and configured providers. Produces documented commands for private startup, private sharing, one bounded live research smoke check and stopping only this app's sharing route.

- [ ] **Step 1:** Document setup with `OPENAI_API_KEY`, `TAVILY_API_KEY`, configurable `OPENAI_MODEL`, a generated `APP_ACCESS_KEY`, and `APP_ORIGIN`. Keep actual values in ignored local environment files. Prefer any available credential-setup tool over asking the user to paste secrets into chat. Missing keys leave saved explorations usable and new research gives actionable setup status. Explain that topics/context are sent to research providers and browser saving does not synchronize devices.
- [ ] **Step 2:** Implement `scripts/live-smoke.ts` to authenticate against an already running configured local app, research one root and one child sequentially, and assert nonempty detailed sections, usable sources, valid evidence references, and further branches. Give it a maximum of two research jobs with no automatic repeat. Keep secrets/question bodies out of logs. This is a deliberate live check, not part of normal unit/CI runs.
- [ ] **Step 3:** Run `pnpm install --frozen-lockfile`, `pnpm check`, `pnpm test:unit`, `pnpm test:e2e`, `pnpm build`, and `pnpm audit`. Expect passing checks and no unmitigated reachable high/critical audit findings. Inspect lockfile, script allowlist and ignored-secret handling before committing. Fix failures before proceeding.
- [ ] **Step 4:** Start the production app with `HOST=127.0.0.1 PORT=3000`, loading the ignored environment file using Node's environment-file support. Inspect existing Tailscale Serve configuration. If 8443 is free, add `tailscale serve --bg --https=8443 http://127.0.0.1:3000`; preserve 60090 and all other existing routes. Set `APP_ORIGIN` to that exact HTTPS origin before verifying authenticated POSTs. If the port is occupied, choose another unused HTTPS listener supported by this tailnet instead of replacing it. Do not use Funnel.
- [ ] **Step 5:** Verify the keyed access URL, key-free redirected URL, secure cookie, authenticated research and same-origin rejection through the actual Tailscale HTTPS endpoint. Test anonymous rejection, saved-data reload, source links and branch expansion in desktop and mobile-sized browsers. Do not claim remote-device testing unless performed on such a device; a local browser through the Tailscale URL verifies the endpoint only.
- [ ] **Step 6:** Run the bounded live smoke check once credentials are configured. If they are unavailable, finish all fixture-backed checks and clearly report live research as unverified, with exact configuration steps; do not substitute sample content or claim full live acceptance.
- [ ] **Step 7:** Record the checks and any material limitation in the README/runbook. The runbook answers whether failures are configuration, retrieval, generation, grounding or timeout, using request IDs and safe stage logs. Document stopping only the new route (`tailscale serve --https=8443 off` if that is the chosen listener), and keeping the Mac/server running. Commit as `docs: verify and document private knowledge tree runtime`.

## Plan self-review

- Spec coverage: start/detail/branches and custom questions → Tasks 1, 2, 6; source quality → Task 4; lazy requests, duplicate suppression and out-of-order completion → Task 2; saving/deletion and failures → Tasks 3 and 7; orientation and phone adaptation → Tasks 6 and 7; private keys/network setup → Tasks 5 and 8. Streaming remains intentionally deferred, as allowed by the design.
- Step scan: each task has a named deliverable, specified interfaces, failing behavioral checks, implementation steps, verification and a commit. Bootstrap is included with the first usable domain/shell deliverable. Browser tests introduced before their dependent UI remain explicitly pending until Task 6, not reported as passed.
- Type consistency: `ResearchResult` distinguishes topic/clarification/insufficient evidence; `ApiError` is only the HTTP error envelope. Server-only `RetrievedSource.text` never crosses into saved `TopicContent`. All node actions operate on stable application-generated IDs.
- Review focus: each of the five listed risk classes has explicit test ownership and assertions above.
- Scope: one app, one backend process, browser-local storage, no accounts, public hosting, cross-device sync, curriculum or autonomous tree generation. No runtime code or dependencies were created while writing this plan.

## Documentation consulted

- [SvelteKit adapter-node](https://svelte.dev/docs/kit/adapter-node): Node deployment and explicit host/origin configuration.
- [Svelte Flow custom nodes](https://svelteflow.dev/learn/customization/custom-nodes) and [viewport helpers](https://svelteflow.dev/api-reference/hooks/use-svelte-flow): custom node components, selection and viewport controls.
- [Tavily Search](https://docs.tavily.com/documentation/api-reference/endpoint/search): retrieved page text, fixed API destination, and bounded source results.
- [OpenAI structured outputs](https://developers.openai.com/api/docs/guides/structured-outputs): typed Responses parsing using Zod; malformed/refused responses still need explicit handling.
- [GPT-5.6 Terra](https://developers.openai.com/api/docs/models/gpt-5.6-terra): proposed configurable model and supported Responses/structured output capabilities. Availability in the user's project is unverified.

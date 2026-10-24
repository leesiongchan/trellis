# Knowledge tree app design

Status: approved by the user on 2026-10-25; ready for implementation planning.

## Intent

Turn one topic into an exploration that keeps branching. The user enters something they want to understand, reads a detailed explanation, and opens related topics without losing the path that brought them there.

The reference is Branch Education’s [microSD explanation](https://www.youtube.com/watch?v=9kM3mnxrMMM). Its description and chapter outline move from the familiar object to its internal structure, manufacturing, and related storage technologies. The app should make that movement interactive for any topic. The reference was inspected through its published description and chapter outline; this design does not assume a full video transcript was reviewed.

Success means that expanding a topic consistently reveals useful next questions, and the user can return to an earlier explanation without reconstructing a conversation.

## Confirmed decisions

- Begin by asking for a topic.
- Explain each topic in detail and offer multiple branches into further topics.
- Branches can themselves expand, continuing the exploration.
- Use approach A from the mockups: an expanding tree with the selected topic’s explanation in a separate panel.
- Research each opened topic on the web and include visible sources. The user accepted this proposal after choosing A.

## Proposed defaults

These are design calls made by the assistant, not independently specified by the user. Approval of this document accepts them unless the user requests changes.

- Make a browser app, optimized for desktop exploration and usable on a phone.
- Start with a private, single-user app. Running it on the user’s Mac and reaching it through Tailscale follows the way the mockup is being viewed. Public hosting and account management are outside the first version.
- Save explorations automatically on the current browser/device. Do not promise synchronization between the Mac and phone.
- Let the user supply a custom branch question as well as choose suggested branches.
- Research and generate content on demand, retaining completed explanations for reuse within the exploration.
- Use a hosted AI/search service through the app’s backend. Keep provider credentials on the server. Provider and framework selection belong to the implementation plan.
- Keep the first version focused on explanations and branching; do not require quizzes, learning goals, or onboarding before exploration.

## Goals, non-goals, and anti-goals

Goals:

- Detailed, readable explanations of ordinary and specialist topics.
- Branches that explain why another topic is worth exploring.
- A growing tree that preserves the user’s location and earlier work.
- Sources supporting material claims and connections.
- Returning to a saved exploration and continuing where the user stopped.

Non-goals for the first version:

- Multiplayer editing, shared libraries, accounts, subscriptions, or public publishing.
- Cross-device data synchronization.
- A fixed curriculum, mandatory mastery tracking, quizzes, spaced repetition, or social feeds.
- Generating videos, arbitrary interactive simulations, or complex 3D scenes.
- A global knowledge graph combining every exploration.

Anti-goals:

- A tree that grows automatically until it becomes a cluttered diagram.
- Replacing detailed explanations with short labels or generic summaries.
- Making every new question require a chat conversation.
- Losing the original topic when following a tangent.
- Presenting invented references or unsupported connections as researched facts.

## Approaches considered

### A. Tree with a separate explanation panel — selected

The desktop workspace contains a connected tree and a reading panel. Selecting a topic changes the explanation; expanding it reveals children. This preserves the exploration’s structure while allowing substantial text.

Tradeoff: large trees need navigation, collapsing, and a way to refocus on the selected node. Narrow screens need an adapted presentation.

The assistant recommended this approach because the user emphasized expandable branches. The user selected A after viewing and interacting with both mockups.

### B. Explanations nested inside the branches

Each branch contains its explanation, with children opening underneath it. This is comfortable for continuous reading and small screens.

Tradeoff: a growing document makes it harder to see the whole exploration and compare neighboring branches. The user chose A instead.

The clickable comparison lives in `.superpowers/brainstorm/69882-1792857144/content/expandable-tree-options.html`. It is a throwaway mockup with sample content, not an implementation or a source of researched app content.

## Core experience

### Start an exploration

Show a single topic input and an action to begin. A few optional examples may illustrate the interaction without requiring a predefined topic.

Accept either a subject or a question, such as “microSD cards” or “How can a microSD card store so much data?” Preserve the original wording. If ambiguity would fundamentally change the result, ask one short clarification; otherwise begin with a reasonable interpretation and make that interpretation visible.

Research the root topic, then produce its explanation and four to six promising branch suggestions. Four to six is a target, not a quota: use fewer when additional suggestions would be redundant or weak. Open the root’s first level of branches automatically once available.

### Read a topic

Selecting a node opens its explanation in the reading panel. If it has never been researched, selection initiates research for that node. Do not automatically expand its children just because it was selected.

An explanation should include:

1. A direct answer establishing what the topic is and why it matters.
2. A substantial account of how it works, or the relevant causes, arguments, context, or structure for nontechnical subjects.
3. Concrete examples and, where helpful, an analogy with its limitations made clear.
4. Important qualifications, misconceptions, or disputed interpretations.
5. Questions that lead naturally to the suggested branches.
6. Inline source references and an accessible list of supporting links.

Use a readable overview followed by deeper sections. Detailed content must be available without opening every child. The style should adapt to the subject rather than forcing history, philosophy, and engineering into the same explanation template.

### Expand a branch

Every node has separate select and expand actions. Selecting controls reading; expanding controls the tree.

- A suggested child initially has a title, a short reason to explore it, and its relationship to the parent. Its full explanation is not generated in advance.
- Expanding an unexplored node selects it, researches it, and reveals its suggested children once they are ready.
- If the node was researched by selecting it earlier, expanding reveals its cached children immediately.
- Collapsing hides descendants without discarding them or canceling unrelated work.
- Reopening an explored node uses saved content. A deliberate refresh action can request fresh research while preserving the existing explanation until replacement succeeds.
- Only explicit user actions generate more content. There is no recursive background expansion.

When expanding or selecting a node, keep it in view. Preserve other expanded branches and the user’s overall exploration. Provide a clear action to return to the root or refocus on the selected topic.

### Suggest useful directions

Branch suggestions are questions or recognizable topics, with a short explanation of the connection. Favor a varied mix when relevant:

- Components and mechanisms: “How do memory cells store a bit?”
- Prerequisites: “What is a transistor?”
- Processes: “How is NAND manufactured?”
- Limits and tradeoffs: “Why does flash wear out?”
- Comparisons and applications: “How is a microSD card related to an SSD?”
- Surprising, defensible connections: “What connects data storage to communication?” through error-correcting codes.

These are possible directions, not six mandatory categories. Avoid a repetitive menu or far-fetched associations. Any unexpected connection should state the bridge and have evidence for it.

The user may add their own question beneath the selected topic. It becomes a child with the same research, explanation, persistence, and expansion behavior.

### Stay oriented

Use a horizontal tree with visible connections on desktop. Show compact node previews, distinguish the selected node, and give each expandable node a clear expanded/collapsed state. Provide pan/scroll, zoom, and refocus controls so depth does not make reading unusable. The reading panel must scroll independently of the tree.

Show the path from the root to the selected topic. Repeated concepts in different branches remain separate context-specific nodes in the first version; they can refer to related earlier nodes but do not turn the displayed structure into a graph. Suppress immediate ancestor repetitions and flag equivalent existing siblings before creating duplicate children.

On phones, use an explicit switch between the map and the selected explanation, retaining the same selected node and expansion state. Opening the explanation from a map node must offer a clear way back to that position. Do not shrink both desktop panes into unreadable columns.

### Save and resume

Automatically save each exploration, including its original topic, generated content, sources, custom branches, selection, expanded nodes, and view position. Provide a simple list of saved explorations and a way to start another one.

Persistence is local to the current browser/device. Store provider credentials only on the backend. Show a visible failure state if browser persistence fails; do not imply unsaved content has been saved.

Deleting an exploration is an explicit action with confirmation. Import/export and backup formats may be deferred; browser storage is not a cross-device backup.

## Research and content quality

Search for each newly opened topic. Prefer original, authoritative sources and use current evidence for time-sensitive subjects. Source count should reflect coverage, not a fixed quota. Distinguish the explanation’s synthesis from claims directly supported by a source.

Do not create citations from model memory. References must correspond to URLs returned by the research process, and material claims must be checked against the retrieved content. Unsupported links between subjects should be omitted or clearly presented as an analogy, rather than a factual relationship.

Use relevant ancestor summaries and the user’s question to establish context. Do not send every sibling’s full explanation on each expansion. Keep explanations self-contained enough to revisit later, while acknowledging the parent connection.

Capture the research timestamp and source metadata with the generated result. Refresh is explicit, not a repeated charge each time a user reopens a node.

## System boundaries and state

The browser owns navigation, rendering, and local persistence. A backend owns research and generation, keeping service credentials out of the client. Use a structured result for topic identity, explanation sections, source references, and branch suggestions. The backend validates it before the browser treats it as complete.

Essential records:

- Exploration: ID, original prompt, root node, timestamps, and view state.
- Topic node: stable ID, parent ID, title/question, parent relationship, explanation, source references, suggestions, research timestamp, and status.
- Source: stable reference ID, title, URL, publisher when available, and retrieval timestamp.

Keep content state separate from view state. Selecting or collapsing a node must not erase its content or mark it as unresearched.

Represent suggested, researching, ready, and failed states explicitly. Deduplicate concurrent requests for the same node. Changing selection must not attach a late result to the newly selected node: responses update their original node by ID. A retry must replace the failed attempt without duplicating children.

Intermediate explanation text may stream into the reader. Do not insert partially parsed branch suggestions into the tree. If research fails, preserve completed content and offer a targeted retry. If evidence is insufficient, say so rather than presenting an unsourced result as verified research. Completed explorations remain readable when the backend is unavailable.

## Constraints and remaining builder choices

The workspace currently has no app, framework, dependencies, or Git repository. Only the visual companion and this design exist. The mockup server and its Tailscale forwarding are preview infrastructure, not the product backend.

The builder should choose the framework, tree layout/rendering library, browser storage mechanism, backend runtime, AI/search providers, response contracts, and deployment commands in the implementation plan. Those choices must preserve this interaction and support the private single-user default.

Use ordinary accessible controls: keyboard-selectable topics, explicit expand/collapse actions, visible focus, readable contrast, and reduced-motion behavior. Research results and model output are untrusted content; render them safely, validate links, and prevent retrieved text from directing tool use or exposing credentials. Do not expose paid provider endpoints publicly without access controls and request limits.

The existing Tailscale preview uses port 60090 forwarding to the local companion on 60089. Do not overwrite that sharing configuration as part of app planning. Do not publish to the public internet as an implicit part of this design.

## Acceptance scenarios

1. Start with a topic and receive a detailed, sourced explanation and several relevant branch suggestions.
2. Expand a suggestion to reveal deeper topics; follow at least three successive levels without losing the root or previous branches.
3. Select an already researched topic and see its existing explanation without another research request.
4. Collapse and reopen a subtree without losing generated topics or their explanations.
5. Add a custom question and continue branching from it.
6. Open two branches in succession; each result appears on its own node even if requests finish out of order.
7. Recover from a research failure with retry, preserving other branches and avoiding duplicate children.
8. Reload and resume a saved exploration with its content and navigation state intact in the same browser.
9. Explore on a phone, switch between map and reading, and return to the selected node.
10. Inspect supporting sources and distinguish unavailable evidence from a successfully researched explanation.

## Research informing the design

- [Branch Education: microSD cards](https://www.youtube.com/watch?v=9kM3mnxrMMM): the user’s reference and the source of the sample exploration.
- [LearnFork overview](https://learnfork.com/docs/overview): prior art for branching learning conversations on a canvas. This design centers topic explanations and expandable questions rather than requiring a chat at each node.
- [IHMC: theory of concept maps](https://cmap.ihmc.us/docs/theory-of-concept-maps.php): explicit relationships and connections across subject areas inform the branch reasons. This is design inspiration, not evidence that this app’s interface has been validated for learning outcomes.
- [Explorable Explanations](https://explorabl.es/): inspiration for possible future interactive explanations. Simulations are intentionally outside the initial scope.

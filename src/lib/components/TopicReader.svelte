<script lang="ts">
  import type { TopicNode } from '#lib/knowledge/types.ts';
  import CitationLink from './CitationLink.svelte';
  import BranchQuestion from './BranchQuestion.svelte';
  let {
    node,
    onFollow,
    onRefresh,
    onClarify,
    clarification,
    demo = false
  }: {
    node: TopicNode;
    onFollow: (question: string) => void;
    onRefresh: () => void;
    onClarify: (answer: string) => void;
    clarification: string | null;
    demo?: boolean;
  } = $props();
  let answer = $state('');
  const content = $derived(node.content);
</script>

<aside
  class="reader"
  aria-label="Topic explanation"
  aria-live="polite"
  aria-busy={node.status === 'researching' || node.isRefreshing}
>
  <div class="reader-kicker">
    <span class="eyebrow">{node.relationship || 'EXPLANATION'}</span>{#if content && !demo}<button
        class="icon-button"
        aria-label="Refresh research"
        title="Research this topic again"
        disabled={node.isRefreshing}
        onclick={onRefresh}>↻</button
      >{/if}
  </div>
  <h2>{content?.title ?? node.question}</h2>
  {#if node.status === 'researching'}<div class="research-progress" role="status">
      <span class="loading-ring"></span>
      <div>
        <strong>Following the thread…</strong>
        <p>Finding sources and putting the pieces together.</p>
      </div>
    </div>
    <div class="reader-skeleton">
      <span></span><span></span><span></span><span></span><span></span>
    </div>{/if}
  {#if node.isRefreshing}<p class="inline-status" role="status">
      ↻ Checking fresh sources. Your earlier explanation stays here.
    </p>{/if}
  {#if node.lastError && !demo}<div class="reader-error" role="alert">
      <strong
        >{node.lastError.code === 'SETUP_REQUIRED'
          ? 'Research is not connected yet'
          : node.lastError.code === 'ACCESS_REQUIRED'
            ? 'Open your private access link'
            : 'This thread needs another try'}</strong
      >
      <p>{node.lastError.message}</p>
      {#if node.lastError.retryable}<button class="quiet-button" onclick={onRefresh}
          >Retry research</button
        >{/if}{#if node.lastError.requestId !== 'local'}<small
          >Reference: {node.lastError.requestId}</small
        >{/if}
    </div>{/if}
  {#if clarification && !demo}<form
      class="clarification"
      onsubmit={(event) => {
        event.preventDefault();
        if (answer.trim()) {
          onClarify(answer.trim());
          answer = '';
        }
      }}
    >
      <label for="clarification-answer">{clarification}</label><input
        id="clarification-answer"
        bind:value={answer}
        maxlength="500"
        required
        placeholder="A little more context…"
      /><button class="primary-button" type="submit">Continue exploring ↗</button>
    </form>{/if}
  {#if demo && !content}
    <div class="demo-question">
      <h3>Your question is saved.</h3>
      <p>
        This demo has prewritten explanations for the included topics. Your new questions stay in
        this tree, but are not researched.
      </p>
    </div>
    <BranchQuestion {onFollow} />
  {/if}
  {#if content}
    <div class="reader-prose">
      {#each content.summary as p (p.id)}<p class="summary-paragraph">
          {p.text}{#each [...new Set(p.evidence.map((e) => e.sourceId))] as id}{@const index =
              content.sources.findIndex((s) => s.id === id)}{#if index >= 0}<CitationLink
                source={content.sources[index]}
                index={index + 1}
                compact
              />{/if}{/each}
        </p>{/each}
      {#if content.analogy}<div class="analogy-block">
          <span class="eyebrow">A WAY TO THINK ABOUT IT</span>
          <p>{content.analogy}</p>
          {#if content.limitations}<small>{content.limitations}</small>{/if}
        </div>{/if}
      {#each content.sections as section, sectionIndex (sectionIndex)}<section>
          <h3>{section.heading}</h3>
          {#each section.paragraphs as p (p.id)}<p>
              {p.text}{#each [...new Set(p.evidence.map((e) => e.sourceId))] as id}{@const index =
                  content.sources.findIndex((s) => s.id === id)}{#if index >= 0}<CitationLink
                    source={content.sources[index]}
                    index={index + 1}
                    compact
                  />{/if}{/each}
            </p>{/each}
        </section>{/each}
    </div>
    {#if content.branches.length}<section class="reader-next">
        <div class="section-label">THERE'S MORE TO THIS STORY</div>
        {#each content.branches as branch, i (i)}<button
            class="next-topic"
            onclick={() => onFollow(branch.question)}
            ><span><strong>{branch.title}</strong><small>{branch.reason.text}</small></span><span
              aria-hidden="true">↗</span
            ></button
          >{/each}
      </section>{/if}
    <BranchQuestion {onFollow} />
    <section class="reader-sources">
      <div class="section-label">FOLLOW THE SOURCES <span>{content.sources.length}</span></div>
      {#each content.sources as source, i (source.id)}<CitationLink {source} index={i + 1} />{/each}
      <p class="research-date">
        {#if demo}Prewritten demo · Sources for further reading.{:else}
          Researched {new Date(content.researchedAt).toLocaleDateString(undefined, {
            month: 'long',
            day: 'numeric',
            year: 'numeric'
          })}. Check the original sources for context.
        {/if}
      </p>
    </section>
  {/if}
</aside>

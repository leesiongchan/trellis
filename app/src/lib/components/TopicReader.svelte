<script lang="ts">
  import { css } from '#styled/css';
  import { button } from '#styled/recipes';
  import type { Snippet } from 'svelte';
  import type { TopicNode } from '#lib/knowledge/types.ts';
  import CitationLink from './CitationLink.svelte';
  import BranchQuestion from './BranchQuestion.svelte';
  let {
    node,
    onFollow,
    onRefresh,
    onClarify,
    clarification,
    readOnly = false,
    children
  }: {
    node: TopicNode;
    onFollow: (question: string) => void;
    onRefresh: () => void;
    onClarify: (answer: string) => void;
    clarification: string | null;
    readOnly?: boolean;
    children?: Snippet;
  } = $props();
  let answer = $state('');
  const content = $derived(node.content);
  const styles = {
    label: css({ textStyle: 'label', color: 'accent' }),
    paragraph: css({ textStyle: 'body', color: 'fg', overflowWrap: 'anywhere' }),
    section: css({
      display: 'grid',
      gap: '4',
      pt: '5',
      borderTopWidth: '1px',
      borderColor: 'border',
      minW: '0'
    }),
    input: css({
      minW: '0',
      w: 'full',
      minH: 'control',
      borderWidth: '1px',
      borderColor: 'border.strong',
      rounded: 'control',
      px: '3',
      py: '2',
      bg: 'bg.surface'
    })
  };
</script>

<aside
  class={css({
    h: 'full',
    minH: '0',
    minW: '0',
    overflowY: 'auto',
    bg: 'bg.surface',
    px: 'section',
    py: 'section',
    borderLeftWidth: '1px',
    borderColor: 'border',
    scrollbarWidth: 'thin',
    scrollPaddingBlock: '6',
    containerType: 'inline-size'
  })}
  aria-label="Topic explanation"
  aria-live="polite"
  aria-busy={node.status === 'researching' || node.isRefreshing}
>
  <div class={css({ maxW: 'prose', mx: 'auto', display: 'grid', gap: '6', minW: '0' })}>
    <div
      class={css({
        display: 'flex',
        gap: '4',
        alignItems: 'center',
        justifyContent: 'space-between'
      })}
    >
      <span class={styles.label}>{node.relationship || 'EXPLANATION'}</span>{#if content}<button
          class={button({ visual: 'icon' })}
          aria-label="Refresh research"
          title="Research this topic again"
          disabled={readOnly || node.isRefreshing}
          onclick={onRefresh}>↻</button
        >{/if}
    </div>
    <h2 class={css({ textStyle: 'heading.topic', overflowWrap: 'anywhere', textWrap: 'balance' })}>
      {content?.title ?? node.question}
    </h2>
    {#if node.status === 'researching'}<div
        class={css({
          display: 'flex',
          alignItems: 'start',
          gap: '3',
          p: '4',
          bg: 'accent.subtle',
          rounded: 'control',
          textStyle: 'body.sm'
        })}
        role="status"
      >
        <span class={css({ color: 'accent' })} aria-hidden="true">◌</span>
        <div>
          <strong>Following the thread…</strong>
          <p>Finding sources and putting the pieces together.</p>
        </div>
      </div>
      <div class={css({ display: 'grid', gap: '3' })} aria-hidden="true">
        {#each [1, 2, 3] as line (line)}<span
            class={css({ h: '3', rounded: 'full', bg: 'bg.subtle' })}
          ></span>{/each}
      </div>{/if}
    {#if node.isRefreshing}<p
        class={css({ textStyle: 'body.sm', color: 'fg.muted' })}
        role="status"
      >
        ↻ Checking fresh sources. Your earlier explanation stays here.
      </p>{/if}
    {#if node.lastError}<div
        class={css({
          display: 'grid',
          gap: '3',
          p: '4',
          bg: 'danger.subtle',
          borderLeftWidth: '2px',
          borderColor: 'danger',
          rounded: 'control',
          textStyle: 'body.sm',
          overflowWrap: 'anywhere'
        })}
        role="alert"
      >
        <strong
          >{node.lastError.code === 'SETUP_REQUIRED'
            ? 'Research is not connected yet'
            : node.lastError.code === 'ACCESS_REQUIRED'
              ? 'Open your private access link'
              : 'This thread needs another try'}</strong
        >
        <p>{node.lastError.message}</p>
        {#if node.lastError.retryable}<button
            class={button({ visual: 'quiet' })}
            disabled={readOnly}
            onclick={onRefresh}>Retry research</button
          >{/if}{#if node.lastError.requestId !== 'local'}<small
            >Reference: {node.lastError.requestId}</small
          >{/if}
      </div>{/if}
    {#if clarification}<form
        class={css({
          display: 'grid',
          gap: '3',
          p: '4',
          bg: 'accent.subtle',
          rounded: 'control',
          textStyle: 'body.sm'
        })}
        onsubmit={(event) => {
          event.preventDefault();
          if (!readOnly && answer.trim()) {
            onClarify(answer.trim());
            answer = '';
          }
        }}
      >
        <label for="clarification-answer">{clarification}</label><input
          class={styles.input}
          id="clarification-answer"
          bind:value={answer}
          maxlength="500"
          required
          placeholder="A little more context…"
        /><button class={button({ visual: 'primary' })} type="submit" disabled={readOnly}
          >Continue exploring ↗</button
        >
      </form>{/if}
    {#if content}
      <div class={css({ display: 'grid', gap: '6', minW: '0' })}>
        {#each content.summary as p (p.id)}<p class={styles.paragraph}>
            {p.text}{#each [...new Set(p.evidence.map((e) => e.sourceId))] as id}{@const index =
                content.sources.findIndex((s) => s.id === id)}{#if index >= 0}<CitationLink
                  source={content.sources[index]}
                  index={index + 1}
                  compact
                />{/if}{/each}
          </p>{/each}
        {#if content.analogy}<div
            class={css({
              display: 'grid',
              gap: '3',
              px: '5',
              py: '4',
              bg: 'accent.subtle',
              borderLeftWidth: '2px',
              borderColor: 'accent',
              rounded: 'control',
              overflowWrap: 'anywhere'
            })}
          >
            <span class={styles.label}>A WAY TO THINK ABOUT IT</span>
            <p class={css({ textStyle: 'body', fontFamily: 'heading', fontSize: 'xl' })}>
              {content.analogy}
            </p>
            {#if content.limitations}<small class={css({ textStyle: 'body.sm', color: 'fg.muted' })}
                >{content.limitations}</small
              >{/if}
          </div>{/if}
        {#each content.sections as section, sectionIndex (sectionIndex)}<section
            class={css({ display: 'grid', gap: '4', minW: '0' })}
          >
            <h3 class={css({ textStyle: 'heading.section', overflowWrap: 'anywhere' })}>
              {section.heading}
            </h3>
            {#each section.paragraphs as p (p.id)}<p class={styles.paragraph}>
                {p.text}{#each [...new Set(p.evidence.map((e) => e.sourceId))] as id}{@const index =
                    content.sources.findIndex((s) => s.id === id)}{#if index >= 0}<CitationLink
                      source={content.sources[index]}
                      index={index + 1}
                      compact
                    />{/if}{/each}
              </p>{/each}
          </section>{/each}
      </div>
      {#if content.branches.length}<section class={styles.section}>
          <div class={styles.label}>THERE'S MORE TO THIS STORY</div>
          {#each content.branches as branch, i (i)}<button
              class={css({
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                gap: '4',
                textAlign: 'left',
                w: 'full',
                minH: 'control',
                py: '3',
                px: '2',
                bg: 'transparent',
                borderBottomWidth: '1px',
                borderColor: 'border',
                rounded: 'control',
                _hover: { bg: 'accent.subtle' }
              })}
              disabled={readOnly}
              onclick={() => onFollow(branch.question)}
              ><span class={css({ minW: '0', display: 'grid', gap: '2', overflowWrap: 'anywhere' })}
                ><strong class={css({ textStyle: 'body.sm', fontWeight: 'semibold' })}
                  >{branch.title}</strong
                ><small class={css({ textStyle: 'body.sm', color: 'fg.muted' })}
                  >{branch.reason.text}</small
                ></span
              ><span
                class={css({ color: 'accent', fontSize: 'xl', flexShrink: '0' })}
                aria-hidden="true">↗</span
              ></button
            >{/each}
        </section>{/if}
      <BranchQuestion {onFollow} disabled={readOnly} />
      <section class={styles.section}>
        <div class={styles.label}>FOLLOW THE SOURCES <span>{content.sources.length}</span></div>
        {#each content.sources as source, i (source.id)}<CitationLink
            {source}
            index={i + 1}
          />{/each}
        <p class={css({ textStyle: 'caption', color: 'fg.muted', overflowWrap: 'anywhere' })}>
          Researched {new Date(content.researchedAt).toLocaleDateString(undefined, {
            month: 'long',
            day: 'numeric',
            year: 'numeric'
          })}. Check the original sources for context.
        </p>
      </section>
    {/if}
    {@render children?.()}
  </div>
</aside>

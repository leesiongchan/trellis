<script module lang="ts">
  import type { Node } from '@xyflow/svelte';
  import type { TopicNode as Topic } from '#lib/knowledge/types.ts';
  export type FlowTopicNode=Node<{topic:Topic;expanded:boolean;active:boolean;onSelect:(id:string)=>void;onExpand:(id:string)=>void;onCollapse:(id:string)=>void},'topic'>;
</script>
<script lang="ts">
  import { Handle,Position,type NodeProps } from '@xyflow/svelte';
  let {data}:NodeProps<FlowTopicNode>=$props();
  const topic=$derived(data.topic);
</script>
<div class="topic-node" class:active={data.active} class:root-node={topic.parentId===null} class:busy={topic.status==='researching'}>
  {#if topic.parentId}<Handle type="target" position={Position.Left} isConnectable={false}/>{/if}
  <button class="node-read nodrag nopan" aria-label={'Read '+topic.question} onclick={()=>data.onSelect(topic.id)}><span class="node-relationship">{topic.relationship}</span><strong>{topic.content?.title??topic.question}</strong><span class="node-teaser">{topic.teaser||topic.content?.summary[0]?.text||'Open this question and see where it leads.'}</span></button>
  <button class="node-expand nodrag nopan" aria-label={(data.expanded?'Collapse ':'Expand ')+topic.question} aria-expanded={data.expanded} onclick={()=>data.expanded?data.onCollapse(topic.id):data.onExpand(topic.id)}><span>{data.expanded?'−':topic.status==='researching'?'◌':'+'}</span>{data.expanded?'Collapse branches':topic.status==='researching'?'Researching…':topic.childIds.length?'Explore '+topic.childIds.length+' branches':'Explore branches'}<span class="node-count">{topic.status==='ready'?'↗':'·'}</span></button>
  <Handle type="source" position={Position.Right} isConnectable={false}/>
</div>

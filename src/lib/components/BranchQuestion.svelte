<script lang="ts">
  let { onFollow }: {onFollow:(question:string)=>void}=$props();let question=$state('');
  function submit(event:SubmitEvent){event.preventDefault();if(question.trim()){onFollow(question.trim());question='';}}
</script>
<form class="branch-question" onsubmit={submit}>
  <label for="branch-input">A question of your own?</label>
  <div><input id="branch-input" aria-label="Your branch question" bind:value={question} maxlength="500" placeholder="What else are you wondering?" required/><button type="submit" class="icon-button" aria-label="Add branch" disabled={!question.trim()}>↗</button></div>
</form>

const CHAT_BUBBLE_TAG = 'infring-chat-bubble-render';

const CHAT_BUBBLE_COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-chat-bubble-render', shadow: 'none' }} />
<script lang="ts">
  export let html = '';
  export let plain = '';
  export let typing = '0';

  function asBoolean(value) {
    if (value === true || value === false) return value;
    var text = String(value == null ? '' : value).trim().toLowerCase();
    return text === '1' || text === 'true' || text === 'yes' || text === 'on';
  }

  $: isTyping = asBoolean(typing);
</script>

{#if isTyping}
  <div class="message-bubble-content message-bubble-content-typing">{plain}</div>
{:else}
  <div class="message-bubble-content">{@html html}</div>
{/if}
`;

module.exports = {
  CHAT_BUBBLE_TAG,
  CHAT_BUBBLE_COMPONENT_SOURCE,
};

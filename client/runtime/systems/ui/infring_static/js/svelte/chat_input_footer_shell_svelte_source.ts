const COMPONENT_TAG = 'infring-chat-input-footer-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-chat-input-footer-shell', shadow: 'none' }} />
<script>
  import { onMount, onDestroy } from 'svelte';

  let sending = false;
  let inputText = '';
  let tokenCount = 0;
  let unsubs = [];

  function cp() { return (typeof window !== 'undefined' && window.InfringChatPage) || null; }

  onMount(function() {
    var s = typeof window !== 'undefined' && window.InfringChatStore;
    if (!s) return;
    if (s.sending) unsubs.push(s.sending.subscribe(function(v) { sending = !!v; }));
    if (s.inputText) unsubs.push(s.inputText.subscribe(function(v) { inputText = typeof v === 'string' ? v : ''; }));
    if (s.tokenCount) unsubs.push(s.tokenCount.subscribe(function(v) { tokenCount = Number(v) || 0; }));
  });

  onDestroy(function() {
    for (var i = 0; i < unsubs.length; i++) { if (typeof unsubs[i] === 'function') unsubs[i](); }
  });
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};

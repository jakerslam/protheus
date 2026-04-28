const COMPONENT_TAG = 'infring-sidebar-rail-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-sidebar-rail-shell', shadow: 'none' }} />
<script>
  import { onMount, onDestroy } from 'svelte';

  export let dragbarSurface = 'chat-sidebar';
  export let wall = '';
  export let dragging = false;
  export let parentOwnedMechanics = true;

  let sidebarAgents = [];
  let unsub;

  onMount(function() {
    var s = typeof window !== 'undefined' && window.InfringChatStore;
    if (s && s.sidebarAgents) {
      unsub = s.sidebarAgents.subscribe(function(v) { sidebarAgents = Array.isArray(v) ? v : []; });
    }
  });

  onDestroy(function() { if (typeof unsub === 'function') unsub(); });
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};

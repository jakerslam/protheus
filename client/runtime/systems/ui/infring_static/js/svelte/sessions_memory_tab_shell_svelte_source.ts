const COMPONENT_TAG = 'infring-sessions-memory-tab-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-sessions-memory-tab-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'sessions';
  export let tabId = 'memory';
  export let panelRole = 'sessions-tab';
  export let routeContract = 'sessions:memory';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};

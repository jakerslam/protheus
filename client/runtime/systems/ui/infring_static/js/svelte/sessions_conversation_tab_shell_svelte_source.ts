const COMPONENT_TAG = 'infring-sessions-conversation-tab-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-sessions-conversation-tab-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'sessions';
  export let tabId = 'conversation';
  export let panelRole = 'sessions-tab';
  export let routeContract = 'sessions:conversation';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};

const COMPONENT_TAG = 'infring-logs-live-tab-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-logs-live-tab-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'logs';
  export let tabId = 'live';
  export let panelRole = 'logs-tab';
  export let routeContract = 'logs:live';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};

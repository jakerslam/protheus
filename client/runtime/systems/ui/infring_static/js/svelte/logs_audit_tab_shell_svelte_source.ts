const COMPONENT_TAG = 'infring-logs-audit-tab-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-logs-audit-tab-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'logs';
  export let tabId = 'audit';
  export let panelRole = 'logs-tab';
  export let routeContract = 'logs:audit';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};

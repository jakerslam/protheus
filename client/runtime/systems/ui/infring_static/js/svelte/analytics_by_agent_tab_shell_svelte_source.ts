const COMPONENT_TAG = 'infring-analytics-by-agent-tab-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-analytics-by-agent-tab-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'analytics';
  export let tabId = 'by-agent';
  export let panelRole = 'analytics-tab';
  export let routeContract = 'analytics:by-agent';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};

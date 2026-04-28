const COMPONENT_TAG = 'infring-analytics-summary-tab-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-analytics-summary-tab-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'analytics';
  export let tabId = 'summary';
  export let panelRole = 'analytics-tab';
  export let routeContract = 'analytics:summary';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};

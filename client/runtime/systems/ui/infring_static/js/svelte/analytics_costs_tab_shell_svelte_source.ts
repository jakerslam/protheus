const COMPONENT_TAG = 'infring-analytics-costs-tab-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-analytics-costs-tab-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'analytics';
  export let tabId = 'costs';
  export let panelRole = 'analytics-tab';
  export let routeContract = 'analytics:costs';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};

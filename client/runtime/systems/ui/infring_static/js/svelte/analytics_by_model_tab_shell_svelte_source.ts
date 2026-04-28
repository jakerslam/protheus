const COMPONENT_TAG = 'infring-analytics-by-model-tab-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-analytics-by-model-tab-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'analytics';
  export let tabId = 'by-model';
  export let panelRole = 'analytics-tab';
  export let routeContract = 'analytics:by-model';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};

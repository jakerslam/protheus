const COMPONENT_TAG = 'infring-analytics-page-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-analytics-page-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'analytics';
  export let panelRole = 'page';
  export let routeContract = 'analytics';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};

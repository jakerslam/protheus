const COMPONENT_TAG = 'infring-overview-page-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-overview-page-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'overview';
  export let panelRole = 'page';
  export let routeContract = 'overview';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};

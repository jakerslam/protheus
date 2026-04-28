const COMPONENT_TAG = 'infring-logs-page-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-logs-page-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'logs';
  export let panelRole = 'page';
  export let routeContract = 'logs';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};

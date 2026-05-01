const COMPONENT_TAG = 'infring-comms-page-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-comms-page-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'comms';
  export let panelRole = 'page';
  export let routeContract = 'comms';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};

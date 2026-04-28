const COMPONENT_TAG = 'infring-hands-page-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-hands-page-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'hands';
  export let panelRole = 'page';
  export let routeContract = 'hands';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};

const COMPONENT_TAG = 'infring-eyes-page-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-eyes-page-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'eyes';
  export let panelRole = 'page';
  export let routeContract = 'eyes';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};

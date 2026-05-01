const COMPONENT_TAG = 'infring-runtime-page-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-runtime-page-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'runtime';
  export let panelRole = 'page';
  export let routeContract = 'runtime';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};

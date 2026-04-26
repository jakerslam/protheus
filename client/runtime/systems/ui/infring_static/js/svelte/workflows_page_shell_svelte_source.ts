const COMPONENT_TAG = 'infring-workflows-page-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-workflows-page-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'workflows';
  export let panelRole = 'page';
  export let routeContract = 'workflows';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};

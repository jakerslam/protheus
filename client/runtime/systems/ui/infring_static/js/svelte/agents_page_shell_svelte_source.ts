const COMPONENT_TAG = 'infring-agents-page-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-agents-page-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'agents';
  export let panelRole = 'page';
  export let routeContract = 'agents';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};

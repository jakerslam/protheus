const COMPONENT_TAG = 'infring-sessions-page-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-sessions-page-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'sessions';
  export let panelRole = 'page';
  export let routeContract = 'sessions';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};

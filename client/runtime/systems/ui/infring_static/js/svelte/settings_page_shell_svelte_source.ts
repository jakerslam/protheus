const COMPONENT_TAG = 'infring-settings-page-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-settings-page-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'settings';
  export let panelRole = 'page';
  export let routeContract = 'settings';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};

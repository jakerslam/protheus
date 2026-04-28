const COMPONENT_TAG = 'infring-channels-page-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-channels-page-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'channels';
  export let panelRole = 'page';
  export let routeContract = 'channels';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};

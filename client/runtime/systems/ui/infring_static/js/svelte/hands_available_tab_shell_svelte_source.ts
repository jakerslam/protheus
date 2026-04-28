const COMPONENT_TAG = 'infring-hands-available-tab-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-hands-available-tab-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'hands';
  export let tabId = 'available';
  export let panelRole = 'hands-tab';
  export let routeContract = 'hands:available';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};

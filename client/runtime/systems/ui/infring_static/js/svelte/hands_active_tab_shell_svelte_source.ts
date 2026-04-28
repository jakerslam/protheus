const COMPONENT_TAG = 'infring-hands-active-tab-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-hands-active-tab-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'hands';
  export let tabId = 'active';
  export let panelRole = 'hands-tab';
  export let routeContract = 'hands:active';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};

const COMPONENT_TAG = 'infring-wizard-page-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-wizard-page-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'wizard';
  export let panelRole = 'page';
  export let routeContract = 'wizard';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};

const COMPONENT_TAG = 'infring-scheduler-page-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-scheduler-page-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'scheduler';
  export let panelRole = 'page';
  export let routeContract = 'scheduler';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};

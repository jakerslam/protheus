const COMPONENT_TAG = 'infring-approvals-page-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-approvals-page-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'approvals';
  export let panelRole = 'page';
  export let routeContract = 'approvals';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};

const COMPONENT_TAG = 'infring-workflows-builder-tab-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-workflows-builder-tab-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'workflows';
  export let tabId = 'builder';
  export let panelRole = 'workflow-tab';
  export let routeContract = 'workflows:builder';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};

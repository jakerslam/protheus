const COMPONENT_TAG = 'infring-workflows-list-tab-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-workflows-list-tab-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'workflows';
  export let tabId = 'list';
  export let panelRole = 'workflow-tab';
  export let routeContract = 'workflows:list';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};

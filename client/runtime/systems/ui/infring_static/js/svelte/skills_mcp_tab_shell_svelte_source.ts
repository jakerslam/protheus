const COMPONENT_TAG = 'infring-skills-mcp-tab-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-skills-mcp-tab-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'skills';
  export let tabId = 'mcp';
  export let panelRole = 'skills-tab';
  export let routeContract = 'skills:mcp';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};

const COMPONENT_TAG = 'infring-skills-create-tab-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-skills-create-tab-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'skills';
  export let tabId = 'create';
  export let panelRole = 'skills-tab';
  export let routeContract = 'skills:create';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};

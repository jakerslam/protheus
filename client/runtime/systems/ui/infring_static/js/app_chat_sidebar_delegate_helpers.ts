// Layer ownership: client/runtime/systems/ui (dashboard static UX surface only; no runtime authority).
function infringChatSidebarDelegateMethods() {
  return {
    updateSidebarScrollIndicators: function() {
      return infringUpdateSidebarScrollIndicators(this);
    },
    scheduleSidebarScrollIndicators: function() {
      return infringScheduleSidebarScrollIndicators(this);
    },
    shellAppStoreBridge: function() {
      return infringAppShellStoreBridge();
    },
    notifyShellAppStore: function(reason) {
      return infringNotifyAppShellStore(this, reason);
    },
    getAppStore: function() {
      return infringGetAppStore(this);
    },
    isSystemSidebarThread: function(agent) {
      return infringIsSystemSidebarThread(this, agent);
    },
    isSidebarArchivedAgent: function(agent) {
      return infringIsSidebarArchivedAgent(this, agent);
    },
    isReservedSystemEmoji: function(rawEmoji) {
      return infringIsReservedSystemEmoji(rawEmoji);
    },
    sanitizeSidebarAgentRow: function(agent) {
      return infringSanitizeSidebarAgentRow(this, agent);
    },
    persistChatSidebarTopologyOrder: function() {
      return infringPersistChatSidebarTopologyOrder(this);
    },
    chatSidebarCanReorderTopology: function() {
      return infringChatSidebarCanReorderTopology(this);
    },
    startChatSidebarTopologyDrag: function(agent, ev) {
      return infringStartChatSidebarTopologyDrag(this, agent, ev);
    },
    handleChatSidebarTopologyDragOver: function(agent, ev) {
      return infringHandleChatSidebarTopologyDragOver(this, agent, ev);
    },
    handleChatSidebarTopologyDrop: function(agent, ev) {
      return infringHandleChatSidebarTopologyDrop(this, agent, ev);
    },
    endChatSidebarTopologyDrag: function() {
      return infringEndChatSidebarTopologyDrag(this);
    },
    chatSidebarDragRenderWindow: function(rows) {
      return infringChatSidebarDragRenderWindow(this, rows);
    },
    chatSidebarHasMoreRows: function() {
      return infringChatSidebarHasMoreRows(this);
    },
    showMoreChatSidebarRows: function() {
      return infringShowMoreChatSidebarRows(this);
    },
    toggleAgentChatsSidebar: function() {
      return infringToggleAgentChatsSidebar(this);
    },
    closeAgentChatsSidebar: function() {
      return infringCloseAgentChatsSidebar(this);
    },
    applyBootChatSelection: async function() {
      return infringApplyBootChatSelection(this);
    },
    sidebarAgentSortTs: function(agent) {
      return infringSidebarAgentSortTs(this, agent);
    },
    chatSidebarTopologyKey: function(agent) {
      return infringChatSidebarTopologyKey(this, agent);
    },
    chatSidebarSortComparator: function(a, b) {
      return infringChatSidebarSortComparator(this, a, b);
    },
    syncChatSidebarTopologyOrderFromAgents: function() {
      return infringSyncChatSidebarTopologyOrderFromAgents(this);
    },
    setChatSidebarSortMode: function(mode) {
      return infringSetChatSidebarSortMode(this, mode);
    },
    chatSidebarPreview: function(agent) {
      return infringChatSidebarPreview(this, agent);
    },
    sidebarDisplayEmoji: function(agent) {
      return infringSidebarDisplayEmoji(this, agent);
    },
    archiveAgentFromSidebar: async function(agent) {
      return infringArchiveAgentFromSidebar(this, agent);
    },
    createSidebarAgentChat: async function() {
      return infringCreateSidebarAgentChat(this);
    },
    selectAgentChatFromSidebar: function(agent) {
      return infringSelectAgentChatFromSidebar(this, agent);
    },
    formatChatSidebarTime: function(ts) {
      return infringFormatChatSidebarTime(this, ts);
    }
  };
}

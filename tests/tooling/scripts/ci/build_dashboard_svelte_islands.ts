#!/usr/bin/env node
/* eslint-disable no-console */
// SRS evidence anchor: V10-DASH-004.24
const fs = require('node:fs');
const path = require('node:path');
const esbuild = require('esbuild');
const { compile } = require('svelte/compiler');
const { cleanText, hasFlag, parseBool, readFlag } = require('../../lib/cli.ts');
const { emitStructuredResult } = require('../../lib/result.ts');

const SCRIPT_PATH = 'tests/tooling/scripts/ci/build_dashboard_svelte_islands.ts';
const ISLAND_SPECS = [
  {
    id: 'chat_bubble',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_bubble_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_bubble.bundle.ts',
    fallbackTag: 'infring-chat-bubble-render',
    filename: 'chat_bubble.svelte',
  },
  {
    id: 'chat_stream_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_stream_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_stream_shell.bundle.ts',
    fallbackTag: 'infring-chat-stream-shell',
    filename: 'chat_stream_shell.svelte',
  },
  {
    id: 'sidebar_rail_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/sidebar_rail_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/sidebar_rail_shell.bundle.ts',
    fallbackTag: 'infring-sidebar-rail-shell',
    filename: 'sidebar_rail_shell.svelte',
  },
  {
    id: 'sidebar_agent_list_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/sidebar_agent_list_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/sidebar_agent_list_shell.bundle.ts',
    fallbackTag: 'infring-sidebar-agent-list-shell',
    filename: 'sidebar_agent_list_shell.svelte',
  },
  {
    id: 'popup_window_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/popup_window_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/popup_window_shell.bundle.ts',
    fallbackTag: 'infring-popup-window-shell',
    filename: 'popup_window_shell.svelte',
  },
  {
    id: 'dashboard_popup_overlay_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/dashboard_popup_overlay_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/dashboard_popup_overlay_shell.bundle.ts',
    fallbackTag: 'infring-dashboard-popup-overlay-shell',
    filename: 'dashboard_popup_overlay_shell.svelte',
  },
  {
    id: 'taskbar_menu_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/taskbar_menu_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/taskbar_menu_shell.bundle.ts',
    fallbackTag: 'infring-taskbar-menu-shell',
    filename: 'taskbar_menu_shell.svelte',
  },
  {
    id: 'chat_map_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_map_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_map_shell.bundle.ts',
    fallbackTag: 'infring-chat-map-shell',
    filename: 'chat_map_shell.svelte',
  },
  {
    id: 'agent_details_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/agent_details_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/agent_details_shell.bundle.ts',
    fallbackTag: 'infring-agent-details-shell',
    filename: 'agent_details_shell.svelte',
  },
  {
    id: 'tool_card_stack_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/tool_card_stack_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/tool_card_stack_shell.bundle.ts',
    fallbackTag: 'infring-tool-card-stack-shell',
    filename: 'tool_card_stack_shell.svelte',
  },
  {
    id: 'composer_lane_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/composer_lane_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/composer_lane_shell.bundle.ts',
    fallbackTag: 'infring-composer-lane-shell',
    filename: 'composer_lane_shell.svelte',
  },
  {
    id: 'taskbar_dropdown_cluster_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/taskbar_dropdown_cluster_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/taskbar_dropdown_cluster_shell.bundle.ts',
    fallbackTag: 'infring-taskbar-dropdown-cluster-shell',
    filename: 'taskbar_dropdown_cluster_shell.svelte',
  },
  {
    id: 'taskbar_system_items_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/taskbar_system_items_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/taskbar_system_items_shell.bundle.ts',
    fallbackTag: 'infring-taskbar-system-items-shell',
    filename: 'taskbar_system_items_shell.svelte',
  },
  {
    id: 'bottom_dock_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/bottom_dock_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/bottom_dock_shell.bundle.ts',
    fallbackTag: 'infring-bottom-dock-shell',
    filename: 'bottom_dock_shell.svelte',
  },
  {
    id: 'workspace_panel_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/workspace_panel_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/workspace_panel_shell.bundle.ts',
    fallbackTag: 'infring-workspace-panel-shell',
    filename: 'workspace_panel_shell.svelte',
  },
  {
    id: 'prompt_queue_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/prompt_queue_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/prompt_queue_shell.bundle.ts',
    fallbackTag: 'infring-prompt-queue-shell',
    filename: 'prompt_queue_shell.svelte',
  },
  {
    id: 'prompt_suggestions_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/prompt_suggestions_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/prompt_suggestions_shell.bundle.ts',
    fallbackTag: 'infring-prompt-suggestions-shell',
    filename: 'prompt_suggestions_shell.svelte',
  },
  {
    id: 'context_ring_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/context_ring_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/context_ring_shell.bundle.ts',
    fallbackTag: 'infring-context-ring-shell',
    filename: 'context_ring_shell.svelte',
  },
  {
    id: 'chat_archived_banner_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_archived_banner_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_archived_banner_shell.bundle.ts',
    fallbackTag: 'infring-chat-archived-banner-shell',
    filename: 'chat_archived_banner_shell.svelte',
  },
  {
    id: 'chat_header_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_header_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_header_shell.bundle.ts',
    fallbackTag: 'infring-chat-header-shell',
    filename: 'chat_header_shell.svelte',
  },
  {
    id: 'chat_search_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_search_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_search_shell.bundle.ts',
    fallbackTag: 'infring-chat-search-shell',
    filename: 'chat_search_shell.svelte',
  },
  {
    id: 'chat_input_footer_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_input_footer_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_input_footer_shell.bundle.ts',
    fallbackTag: 'infring-chat-input-footer-shell',
    filename: 'chat_input_footer_shell.svelte',
  },
  {
    id: 'session_switcher_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/session_switcher_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/session_switcher_shell.bundle.ts',
    fallbackTag: 'infring-session-switcher-shell',
    filename: 'session_switcher_shell.svelte',
  },
  {
    id: 'chat_thread_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_thread_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_thread_shell.bundle.ts',
    fallbackTag: 'infring-chat-thread-shell',
    filename: 'chat_thread_shell.svelte',
  },
  {
    id: 'chat_divider_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_divider_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_divider_shell.bundle.ts',
    fallbackTag: 'infring-chat-divider-shell',
    filename: 'chat_divider_shell.svelte',
  },
  {
    id: 'message_meta_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/message_meta_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/message_meta_shell.bundle.ts',
    fallbackTag: 'infring-message-meta-shell',
    filename: 'message_meta_shell.svelte',
  },
  {
    id: 'message_context_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/message_context_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/message_context_shell.bundle.ts',
    fallbackTag: 'infring-message-context-shell',
    filename: 'message_context_shell.svelte',
  },
  {
    id: 'message_progress_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/message_progress_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/message_progress_shell.bundle.ts',
    fallbackTag: 'infring-message-progress-shell',
    filename: 'message_progress_shell.svelte',
  },
  {
    id: 'message_artifact_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/message_artifact_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/message_artifact_shell.bundle.ts',
    fallbackTag: 'infring-message-artifact-shell',
    filename: 'message_artifact_shell.svelte',
  },
  {
    id: 'message_media_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/message_media_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/message_media_shell.bundle.ts',
    fallbackTag: 'infring-message-media-shell',
    filename: 'message_media_shell.svelte',
  },
  {
    id: 'message_terminal_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/message_terminal_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/message_terminal_shell.bundle.ts',
    fallbackTag: 'infring-message-terminal-shell',
    filename: 'message_terminal_shell.svelte',
  },
  {
    id: 'message_placeholder_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/message_placeholder_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/message_placeholder_shell.bundle.ts',
    fallbackTag: 'infring-message-placeholder-shell',
    filename: 'message_placeholder_shell.svelte',
  },
  {
    id: 'messages_surface_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/messages_surface_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/messages_surface_shell.bundle.ts',
    fallbackTag: 'infring-messages-surface-shell',
    filename: 'messages_surface_shell.svelte',
  },
  {
    id: 'chat_empty_state_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_empty_state_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_empty_state_shell.bundle.ts',
    fallbackTag: 'infring-chat-empty-state-shell',
    filename: 'chat_empty_state_shell.svelte',
  },
  {
    id: 'dropzone_overlay_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/dropzone_overlay_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/dropzone_overlay_shell.bundle.ts',
    fallbackTag: 'infring-dropzone-overlay-shell',
    filename: 'dropzone_overlay_shell.svelte',
  },
  {
    id: 'chat_loading_overlay_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_loading_overlay_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_loading_overlay_shell.bundle.ts',
    fallbackTag: 'infring-chat-loading-overlay-shell',
    filename: 'chat_loading_overlay_shell.svelte',
  },
  {
    id: 'chat_map_rail_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_map_rail_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_map_rail_shell.bundle.ts',
    fallbackTag: 'infring-chat-map-rail-shell',
    filename: 'chat_map_rail_shell.svelte',
  },
  {
    id: 'system_thread_placeholder_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/system_thread_placeholder_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/system_thread_placeholder_shell.bundle.ts',
    fallbackTag: 'infring-system-thread-placeholder-shell',
    filename: 'system_thread_placeholder_shell.svelte',
  },
  {
    id: 'chat_map_viewport_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_map_viewport_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_map_viewport_shell.bundle.ts',
    fallbackTag: 'infring-chat-map-viewport-shell',
    filename: 'chat_map_viewport_shell.svelte',
  },
  {
    id: 'chat_loading_content_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_loading_content_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_loading_content_shell.bundle.ts',
    fallbackTag: 'infring-chat-loading-content-shell',
    filename: 'chat_loading_content_shell.svelte',
  },
  {
    id: 'taskbar_hero_menu_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/taskbar_hero_menu_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/taskbar_hero_menu_shell.bundle.ts',
    fallbackTag: 'infring-taskbar-hero-menu-shell',
    filename: 'taskbar_hero_menu_shell.svelte',
  },
  {
    id: 'taskbar_nav_cluster_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/taskbar_nav_cluster_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/taskbar_nav_cluster_shell.bundle.ts',
    fallbackTag: 'infring-taskbar-nav-cluster-shell',
    filename: 'taskbar_nav_cluster_shell.svelte',
  },
  {
    id: 'slash_command_menu_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/slash_command_menu_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/slash_command_menu_shell.bundle.ts',
    fallbackTag: 'infring-slash-command-menu-shell',
    filename: 'slash_command_menu_shell.svelte',
  },
  {
    id: 'model_picker_menu_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/model_picker_menu_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/model_picker_menu_shell.bundle.ts',
    fallbackTag: 'infring-model-picker-menu-shell',
    filename: 'model_picker_menu_shell.svelte',
  },
  {
    id: 'git_tree_picker_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/git_tree_picker_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/git_tree_picker_shell.bundle.ts',
    fallbackTag: 'infring-git-tree-picker-shell',
    filename: 'git_tree_picker_shell.svelte',
  },
  {
    id: 'model_switcher_panel_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/model_switcher_panel_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/model_switcher_panel_shell.bundle.ts',
    fallbackTag: 'infring-model-switcher-panel-shell',
    filename: 'model_switcher_panel_shell.svelte',
  },
  {
    id: 'approvals_page_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/approvals_page_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/approvals_page_shell.bundle.ts',
    fallbackTag: 'infring-approvals-page-shell',
    filename: 'approvals_page_shell.svelte',
  },
  {
    id: 'chat_page_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_page_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_page_shell.bundle.ts',
    fallbackTag: 'infring-chat-page-shell',
    filename: 'chat_page_shell.svelte',
  },
  {
    id: 'agents_page_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/agents_page_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/agents_page_shell.bundle.ts',
    fallbackTag: 'infring-agents-page-shell',
    filename: 'agents_page_shell.svelte',
  },
  {
    id: 'scheduler_page_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/scheduler_page_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/scheduler_page_shell.bundle.ts',
    fallbackTag: 'infring-scheduler-page-shell',
    filename: 'scheduler_page_shell.svelte',
  },
  {
    id: 'scheduler_jobs_tab_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/scheduler_jobs_tab_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/scheduler_jobs_tab_shell.bundle.ts',
    fallbackTag: 'infring-scheduler-jobs-tab-shell',
    filename: 'scheduler_jobs_tab_shell.svelte',
  },
  {
    id: 'scheduler_triggers_tab_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/scheduler_triggers_tab_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/scheduler_triggers_tab_shell.bundle.ts',
    fallbackTag: 'infring-scheduler-triggers-tab-shell',
    filename: 'scheduler_triggers_tab_shell.svelte',
  },
  {
    id: 'scheduler_history_tab_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/scheduler_history_tab_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/scheduler_history_tab_shell.bundle.ts',
    fallbackTag: 'infring-scheduler-history-tab-shell',
    filename: 'scheduler_history_tab_shell.svelte',
  },
  {
    id: 'eyes_page_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/eyes_page_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/eyes_page_shell.bundle.ts',
    fallbackTag: 'infring-eyes-page-shell',
    filename: 'eyes_page_shell.svelte',
  },
  {
    id: 'overview_page_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/overview_page_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/overview_page_shell.bundle.ts',
    fallbackTag: 'infring-overview-page-shell',
    filename: 'overview_page_shell.svelte',
  },
  {
    id: 'workflows_list_tab_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/workflows_list_tab_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/workflows_list_tab_shell.bundle.ts',
    fallbackTag: 'infring-workflows-list-tab-shell',
    filename: 'workflows_list_tab_shell.svelte',
  },
  {
    id: 'workflows_page_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/workflows_page_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/workflows_page_shell.bundle.ts',
    fallbackTag: 'infring-workflows-page-shell',
    filename: 'workflows_page_shell.svelte',
  },
  {
    id: 'workflows_builder_tab_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/workflows_builder_tab_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/workflows_builder_tab_shell.bundle.ts',
    fallbackTag: 'infring-workflows-builder-tab-shell',
    filename: 'workflows_builder_tab_shell.svelte',
  },
  {
    id: 'channels_page_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/channels_page_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/channels_page_shell.bundle.ts',
    fallbackTag: 'infring-channels-page-shell',
    filename: 'channels_page_shell.svelte',
  },
  {
    id: 'skills_page_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/skills_page_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/skills_page_shell.bundle.ts',
    fallbackTag: 'infring-skills-page-shell',
    filename: 'skills_page_shell.svelte',
  },
  {
    id: 'skills_installed_tab_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/skills_installed_tab_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/skills_installed_tab_shell.bundle.ts',
    fallbackTag: 'infring-skills-installed-tab-shell',
    filename: 'skills_installed_tab_shell.svelte',
  },
  {
    id: 'skills_clawhub_tab_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/skills_clawhub_tab_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/skills_clawhub_tab_shell.bundle.ts',
    fallbackTag: 'infring-skills-clawhub-tab-shell',
    filename: 'skills_clawhub_tab_shell.svelte',
  },
  {
    id: 'skills_mcp_tab_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/skills_mcp_tab_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/skills_mcp_tab_shell.bundle.ts',
    fallbackTag: 'infring-skills-mcp-tab-shell',
    filename: 'skills_mcp_tab_shell.svelte',
  },
  {
    id: 'skills_create_tab_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/skills_create_tab_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/skills_create_tab_shell.bundle.ts',
    fallbackTag: 'infring-skills-create-tab-shell',
    filename: 'skills_create_tab_shell.svelte',
  },
  {
    id: 'settings_page_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/settings_page_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/settings_page_shell.bundle.ts',
    fallbackTag: 'infring-settings-page-shell',
    filename: 'settings_page_shell.svelte',
  },
  {
    id: 'settings_providers_tab_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/settings_providers_tab_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/settings_providers_tab_shell.bundle.ts',
    fallbackTag: 'infring-settings-providers-tab-shell',
    filename: 'settings_providers_tab_shell.svelte',
  },
  {
    id: 'settings_models_tab_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/settings_models_tab_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/settings_models_tab_shell.bundle.ts',
    fallbackTag: 'infring-settings-models-tab-shell',
    filename: 'settings_models_tab_shell.svelte',
  },
  {
    id: 'settings_tools_tab_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/settings_tools_tab_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/settings_tools_tab_shell.bundle.ts',
    fallbackTag: 'infring-settings-tools-tab-shell',
    filename: 'settings_tools_tab_shell.svelte',
  },
  {
    id: 'settings_info_tab_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/settings_info_tab_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/settings_info_tab_shell.bundle.ts',
    fallbackTag: 'infring-settings-info-tab-shell',
    filename: 'settings_info_tab_shell.svelte',
  },
  {
    id: 'settings_config_tab_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/settings_config_tab_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/settings_config_tab_shell.bundle.ts',
    fallbackTag: 'infring-settings-config-tab-shell',
    filename: 'settings_config_tab_shell.svelte',
  },
  {
    id: 'settings_security_tab_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/settings_security_tab_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/settings_security_tab_shell.bundle.ts',
    fallbackTag: 'infring-settings-security-tab-shell',
    filename: 'settings_security_tab_shell.svelte',
  },
  {
    id: 'settings_network_tab_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/settings_network_tab_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/settings_network_tab_shell.bundle.ts',
    fallbackTag: 'infring-settings-network-tab-shell',
    filename: 'settings_network_tab_shell.svelte',
  },
  {
    id: 'settings_budget_tab_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/settings_budget_tab_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/settings_budget_tab_shell.bundle.ts',
    fallbackTag: 'infring-settings-budget-tab-shell',
    filename: 'settings_budget_tab_shell.svelte',
  },
  {
    id: 'settings_migration_tab_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/settings_migration_tab_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/settings_migration_tab_shell.bundle.ts',
    fallbackTag: 'infring-settings-migration-tab-shell',
    filename: 'settings_migration_tab_shell.svelte',
  },
  {
    id: 'analytics_page_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/analytics_page_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/analytics_page_shell.bundle.ts',
    fallbackTag: 'infring-analytics-page-shell',
    filename: 'analytics_page_shell.svelte',
  },
  {
    id: 'analytics_summary_tab_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/analytics_summary_tab_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/analytics_summary_tab_shell.bundle.ts',
    fallbackTag: 'infring-analytics-summary-tab-shell',
    filename: 'analytics_summary_tab_shell.svelte',
  },
  {
    id: 'analytics_by_model_tab_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/analytics_by_model_tab_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/analytics_by_model_tab_shell.bundle.ts',
    fallbackTag: 'infring-analytics-by-model-tab-shell',
    filename: 'analytics_by_model_tab_shell.svelte',
  },
  {
    id: 'analytics_by_agent_tab_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/analytics_by_agent_tab_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/analytics_by_agent_tab_shell.bundle.ts',
    fallbackTag: 'infring-analytics-by-agent-tab-shell',
    filename: 'analytics_by_agent_tab_shell.svelte',
  },
  {
    id: 'analytics_costs_tab_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/analytics_costs_tab_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/analytics_costs_tab_shell.bundle.ts',
    fallbackTag: 'infring-analytics-costs-tab-shell',
    filename: 'analytics_costs_tab_shell.svelte',
  },
  {
    id: 'logs_page_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/logs_page_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/logs_page_shell.bundle.ts',
    fallbackTag: 'infring-logs-page-shell',
    filename: 'logs_page_shell.svelte',
  },
  {
    id: 'logs_live_controls_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/logs_live_controls_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/logs_live_controls_shell.bundle.ts',
    fallbackTag: 'infring-logs-live-controls-shell',
    filename: 'logs_live_controls_shell.svelte',
  },
  {
    id: 'logs_audit_controls_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/logs_audit_controls_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/logs_audit_controls_shell.bundle.ts',
    fallbackTag: 'infring-logs-audit-controls-shell',
    filename: 'logs_audit_controls_shell.svelte',
  },
  {
    id: 'logs_live_tab_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/logs_live_tab_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/logs_live_tab_shell.bundle.ts',
    fallbackTag: 'infring-logs-live-tab-shell',
    filename: 'logs_live_tab_shell.svelte',
  },
  {
    id: 'logs_audit_tab_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/logs_audit_tab_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/logs_audit_tab_shell.bundle.ts',
    fallbackTag: 'infring-logs-audit-tab-shell',
    filename: 'logs_audit_tab_shell.svelte',
  },
  {
    id: 'wizard_page_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/wizard_page_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/wizard_page_shell.bundle.ts',
    fallbackTag: 'infring-wizard-page-shell',
    filename: 'wizard_page_shell.svelte',
  },
  {
    id: 'sessions_page_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/sessions_page_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/sessions_page_shell.bundle.ts',
    fallbackTag: 'infring-sessions-page-shell',
    filename: 'sessions_page_shell.svelte',
  },
  {
    id: 'sessions_filter_controls_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/sessions_filter_controls_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/sessions_filter_controls_shell.bundle.ts',
    fallbackTag: 'infring-sessions-filter-controls-shell',
    filename: 'sessions_filter_controls_shell.svelte',
  },
  {
    id: 'sessions_conversation_tab_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/sessions_conversation_tab_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/sessions_conversation_tab_shell.bundle.ts',
    fallbackTag: 'infring-sessions-conversation-tab-shell',
    filename: 'sessions_conversation_tab_shell.svelte',
  },
  {
    id: 'sessions_memory_tab_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/sessions_memory_tab_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/sessions_memory_tab_shell.bundle.ts',
    fallbackTag: 'infring-sessions-memory-tab-shell',
    filename: 'sessions_memory_tab_shell.svelte',
  },
  {
    id: 'comms_page_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/comms_page_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/comms_page_shell.bundle.ts',
    fallbackTag: 'infring-comms-page-shell',
    filename: 'comms_page_shell.svelte',
  },
  {
    id: 'runtime_page_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/runtime_page_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/runtime_page_shell.bundle.ts',
    fallbackTag: 'infring-runtime-page-shell',
    filename: 'runtime_page_shell.svelte',
  },
  {
    id: 'hands_page_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/hands_page_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/hands_page_shell.bundle.ts',
    fallbackTag: 'infring-hands-page-shell',
    filename: 'hands_page_shell.svelte',
  },
  {
    id: 'hands_available_tab_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/hands_available_tab_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/hands_available_tab_shell.bundle.ts',
    fallbackTag: 'infring-hands-available-tab-shell',
    filename: 'hands_available_tab_shell.svelte',
  },
  {
    id: 'hands_active_tab_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/hands_active_tab_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/hands_active_tab_shell.bundle.ts',
    fallbackTag: 'infring-hands-active-tab-shell',
    filename: 'hands_active_tab_shell.svelte',
  },
];

function repoRoot(startDir = __dirname) {
  let dir = path.resolve(startDir);
  while (true) {
    const cargo = path.join(dir, 'Cargo.toml');
    const coreOps = path.join(dir, 'core', 'layer0', 'ops', 'Cargo.toml');
    if (fs.existsSync(cargo) && fs.existsSync(coreOps)) {
      return dir;
    }
    const parent = path.dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  return path.resolve(__dirname, '..', '..', '..', '..');
}

function parseArgs(argv) {
  const out = {
    minify: true,
    out: '',
  };
  const minifyFlag = readFlag(argv, 'minify');
  out.minify = hasFlag(argv, 'minify') || (minifyFlag != null ? parseBool(minifyFlag, true) : true);
  out.out = cleanText(readFlag(argv, 'out') || '', 400);
  return out;
}

function loadIslandSource(root, spec) {
  const sourceModulePath = path.resolve(root, spec.sourcePath);
  const sourceModule = require(sourceModulePath);
  const sourceText = String([
    sourceModule && sourceModule.COMPONENT_SOURCE,
    sourceModule && sourceModule.CHAT_BUBBLE_COMPONENT_SOURCE,
  ].find((value) => typeof value === 'string' && value.trim()) || '').trim();
  const tag = cleanText(
    (sourceModule && sourceModule.COMPONENT_TAG) || (sourceModule && sourceModule.CHAT_BUBBLE_TAG) || spec.fallbackTag,
    120
  ) || spec.fallbackTag;
  if (!sourceText) {
    throw new Error(`dashboard_svelte_source_missing:${spec.sourcePath}`);
  }
  return {
    id: spec.id,
    tag,
    source_text: sourceText,
    source_module: sourceModulePath,
    bundle_path: spec.bundlePath,
    filename: spec.filename || `${spec.id}.svelte`,
  };
}

async function buildDashboardSvelteIslands(options = {}, root = repoRoot(__dirname)) {
  const minify = options && options.minify !== false;
  const builtIslands = [];
  const skippedIslands = [];
  for (const spec of ISLAND_SPECS) {
    const sourceModulePath = path.resolve(root, spec.sourcePath);
    if (!fs.existsSync(sourceModulePath)) {
      skippedIslands.push({
        id: spec.id,
        source_module: path.relative(root, sourceModulePath).replace(/\\/g, '/'),
        reason: 'source_module_missing',
      });
      continue;
    }
    const source = loadIslandSource(root, spec);
    const outFile = path.resolve(root, source.bundle_path);
    fs.mkdirSync(path.dirname(outFile), { recursive: true });
    const compiled = compile(source.source_text, {
      filename: source.filename,
      generate: 'dom',
      dev: false,
      customElement: true,
    });
    await esbuild.build({
      stdin: {
        contents: String(compiled && compiled.js && compiled.js.code ? compiled.js.code : ''),
        loader: 'js',
        sourcefile: `${source.filename}.js`,
        resolveDir: root,
      },
      bundle: true,
      outfile: outFile,
      platform: 'browser',
      format: 'iife',
      target: 'es2020',
      sourcemap: false,
      minify,
      logLevel: 'silent',
      legalComments: 'none',
      banner: {
        js: `/* generated: dashboard svelte island bundle (${source.id}) */`,
      },
    });
    builtIslands.push({
      id: source.id,
      tag: source.tag,
      source_module: path.relative(root, source.source_module).replace(/\\/g, '/'),
      out_file: path.relative(root, outFile).replace(/\\/g, '/'),
      out_bytes: fs.statSync(outFile).size,
    });
  }
  if (builtIslands.length === 0) {
    throw new Error('dashboard_svelte_islands_none_built');
  }

  return {
    ok: true,
    type: 'dashboard_svelte_islands_build',
    islands: builtIslands,
    island_count: builtIslands.length,
    skipped_islands: skippedIslands,
    skipped_count: skippedIslands.length,
    chat_bubble_tag: builtIslands.find((item) => item.id === 'chat_bubble')?.tag || 'infring-chat-bubble-render',
    minify: Boolean(minify),
  };
}

async function run(argv = process.argv.slice(2)) {
  const options = parseArgs(argv);
  try {
    const payload = await buildDashboardSvelteIslands({ minify: options.minify });
    emitStructuredResult(payload, {
      outPath: options.out || undefined,
      strict: false,
      ok: true,
      history: false,
      stdout: false,
    });
    process.stdout.write(`${JSON.stringify(payload)}\n`);
    return 0;
  } catch (error) {
    const payload = {
      ok: false,
      type: 'dashboard_svelte_islands_build_failed',
      error: cleanText(error && error.message ? error.message : String(error), 320),
    };
    emitStructuredResult(payload, {
      outPath: options.out || undefined,
      strict: true,
      ok: false,
      history: false,
      stdout: false,
    });
    process.stderr.write(`${JSON.stringify(payload)}\n`);
    return 1;
  }
}

if (require.main === module) {
  run(process.argv.slice(2)).then((code) => process.exit(code));
}

module.exports = {
  SCRIPT_PATH,
  repoRoot,
  parseArgs,
  loadIslandSource,
  ISLAND_SPECS,
  buildDashboardSvelteIslands,
  run,
};

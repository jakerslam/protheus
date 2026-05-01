// Chat slash command handlers for telemetry, continuity, and worker status.
'use strict';

function infringChatSlashTelemetryMethods() {
  return {
    runSlashAlerts: async function() {
      this.inputText = 'Use the telemetry_continuity_packaging route to report current proactive telemetry alerts with a structured receipt.';
      await this.sendMessage();
    },

    runSlashNextActions: async function() {
      this.inputText = 'Use the telemetry_continuity_packaging route to report predicted next actions with a structured receipt.';
      await this.sendMessage();
    },

    runSlashMemoryHygiene: async function() {
      this.inputText = 'Use the telemetry_continuity_packaging route to report memory hygiene and recommended actions with a structured receipt.';
      await this.sendMessage();
    },

    runSlashContinuity: async function() {
      this.inputText = 'Use the telemetry_continuity_packaging route to report cross-channel continuity, stale sessions, and active agent markers with a structured receipt.';
      await this.sendMessage();
    },

    runSlashOptimizeWorkers: async function() {
      this.inputText = 'Use the telemetry_continuity_packaging route to report worker optimization recommendations with a structured receipt.';
      await this.sendMessage();
    },
  };
}

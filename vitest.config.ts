import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    include: ['tests/vitest/**/*.test.ts'],
    environment: 'node',
    coverage: {
      provider: 'v8',
      enabled: true,
      reporter: ['text-summary', 'json-summary'],
      reportsDirectory: 'coverage/ts',
      include: [
        'systems/conduit/conduit-client.ts',
        'lib/direct_conduit_lane_bridge.js'
      ]
    }
  }
});

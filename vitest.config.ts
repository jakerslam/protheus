import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    include: ['tests/vitest/**/*.test.ts'],
    environment: 'node',
    coverage: {
      // v8 remapping currently crashes on conduit-client.ts; use stable istanbul instrumentation.
      provider: 'istanbul',
      enabled: false,
      reporter: ['text-summary', 'json-summary'],
      reportsDirectory: 'coverage/ts',
      include: [
        'client/runtime/systems/conduit/conduit-client.ts',
        'client/runtime/lib/direct_conduit_lane_bridge.ts'
      ]
    }
  }
});

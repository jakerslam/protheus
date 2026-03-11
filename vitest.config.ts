import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    include: ['client/tests/vitest/**/*.test.ts', 'tests/vitest/**/*.test.ts'],
    environment: 'node',
    coverage: {
      provider: 'v8',
      enabled: true,
      reporter: ['text-summary', 'json-summary'],
      reportsDirectory: 'coverage/ts',
      include: [
        'client/runtime/systems/conduit/conduit-client.ts',
        'client/runtime/lib/direct_conduit_lane_bridge.ts'
      ]
    }
  }
});

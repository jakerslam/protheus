include_parts!(
    "secret_broker_kernel_parts/010-secret-broker-state.rs",
    "secret_broker_kernel_parts/020-parse-ts-ms.rs",
    "secret_broker_kernel_parts/030-read-state.rs",
    "secret_broker_kernel_parts/040-rotation-health-report.rs",
    "secret_broker_kernel_parts/050-run.rs",
);

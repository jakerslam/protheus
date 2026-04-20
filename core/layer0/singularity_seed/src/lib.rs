// Split from lib.rs into focused include parts for maintainability.
include!("lib_parts/010-prelude-and-shared.rs");
include!("lib_parts/020-blob-version-to-normalize-cycle-request.rs");
include!("lib_parts/030-normalize-cycle-request-with-contract-to-loop-blob-path.rs");
include!("lib_parts/040-default-states-to-freeze-seed.rs");
include!("lib_parts/050-guarded-cycle-to-show-seed-state-wasm.rs");
include!("lib_parts/060-mod-tests.rs");

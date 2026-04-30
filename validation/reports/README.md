# Validation reports

This subdomain is part of the physical Validation domain. It should contain controlled-check definitions, metadata, fixtures, schemas, or report destinations that belong to reports.

Migration status: anchor created; existing scattered assets should be moved here by the relevant ASSURANCE-MIGRATE wave.

## Client report archive

- `client_archive/` contains the historical benchmark, proof-pack, coverage, primitive-audit, and runtime snapshot report artifacts relocated from the legacy docs/client report bundle so active report evidence lives inside the Validation physical domain.
- Active consumers should read these reports through `validation/reports/client_archive/**`; new generated reports should prefer focused Validation subdomains when they become current gate truth.

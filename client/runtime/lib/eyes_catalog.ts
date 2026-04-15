import path from 'path';
const catalogStore = require('../systems/adaptive/sensory/eyes/catalog_store.ts');

const {
  defaultCatalog,
  readCatalog,
  ensureCatalog,
  setCatalog,
  mutateCatalog
} = catalogStore as {
  defaultCatalog: Record<string, unknown>;
  readCatalog: (catalogPath: unknown) => Record<string, unknown>;
  ensureCatalog: (catalogPath: unknown) => { ok: boolean; created: boolean; catalog: Record<string, unknown> };
  setCatalog: (catalogPath: unknown, nextCatalog: unknown) => { ok: boolean; catalog: Record<string, unknown> };
  mutateCatalog: (catalogPath: unknown, mutator: (current: Record<string, unknown>) => unknown) => { ok: boolean; catalog: Record<string, unknown> };
};

function canonicalCatalogPath(workspaceDir: unknown): string {
  return path.resolve(String(workspaceDir || ''), 'adaptive', 'sensory', 'eyes', 'catalog.json');
}

function resolveCatalogEnvValue(envValue: unknown): string {
  const explicit = String(envValue || '').trim();
  if (explicit) return explicit;
  const preferred = String(process.env.INFRING_EYES_CATALOG_PATH || '').trim();
  const legacy = String(process.env.PROTHEUS_EYES_CATALOG_PATH || '').trim();
  if (!preferred && legacy) {
    process.env.INFRING_EYES_CATALOG_PATH = legacy;
    return legacy;
  }
  if (preferred && !legacy) {
    process.env.PROTHEUS_EYES_CATALOG_PATH = preferred;
  }
  return preferred;
}

function resolveCatalogPath(workspaceDir: unknown, envValue: unknown): string {
  const canonical = canonicalCatalogPath(workspaceDir);
  const requested = resolveCatalogEnvValue(envValue);
  if (!requested) return canonical;
  const requestedAbs = path.resolve(requested);
  if (requestedAbs !== canonical) {
    throw new Error(`eyes_catalog: catalog path override denied (requested=${requestedAbs})`);
  }
  return canonical;
}

export {
  canonicalCatalogPath,
  resolveCatalogPath,
  defaultCatalog,
  readCatalog,
  ensureCatalog,
  setCatalog,
  mutateCatalog
};

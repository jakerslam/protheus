'use strict';

function toArray(v) {
  return Array.isArray(v) ? v : [];
}

function token(v) {
  return String(v == null ? '' : v)
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9_.:-]+/g, '_')
    .replace(/_+/g, '_')
    .replace(/^_+|_+$/g, '');
}

function mapRows(rows, kind) {
  return toArray(rows).map((row, idx) => {
    const name = String(row && (row.name || row.id || `${kind}_${idx + 1}`) || `${kind}_${idx + 1}`).trim();
    return {
      id: token(name) || `${kind}_${idx + 1}`,
      name,
      source_kind: kind,
      source: row || {}
    };
  });
}

function importPayload(payload) {
  const obj = payload && typeof payload === 'object' ? payload : {};
  const agents = mapRows(obj.agents, 'agent');
  const tasks = mapRows(obj.tasks, 'task');
  const workflows = mapRows(obj.workflows, 'workflow');
  const tools = mapRows(obj.tools, 'tool');

  const sourceItemCount = toArray(obj.agents).length
    + toArray(obj.tasks).length
    + toArray(obj.workflows).length
    + toArray(obj.tools).length;

  const mappedItemCount = agents.length + tasks.length + workflows.length + tools.length;

  return {
    entities: {
      agents,
      tasks,
      workflows,
      tools,
      records: []
    },
    source_item_count: sourceItemCount,
    mapped_item_count: mappedItemCount,
    warnings: []
  };
}

module.exports = {
  engine: 'openfang',
  importPayload
};

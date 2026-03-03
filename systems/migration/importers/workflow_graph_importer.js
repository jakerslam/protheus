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

function importPayload(payload) {
  const obj = payload && typeof payload === 'object' ? payload : {};
  const nodes = toArray(obj.nodes);
  const edges = toArray(obj.edges);

  const workflows = nodes.map((node, idx) => ({
    id: token(node && (node.id || node.name || `node_${idx + 1}`)) || `node_${idx + 1}`,
    name: String(node && (node.name || node.id || `node_${idx + 1}`)),
    edges_out: edges.filter((edge) => String(edge && edge.from || '') === String(node && (node.id || node.name || ''))).length,
    source: node || {}
  }));

  const records = edges.map((edge, idx) => ({
    id: `edge_${idx + 1}`,
    bucket: 'edge',
    source: edge || {}
  }));

  const sourceItemCount = nodes.length + edges.length;
  const mappedItemCount = workflows.length + records.length;

  return {
    entities: {
      agents: [],
      tasks: [],
      workflows,
      tools: [],
      records
    },
    source_item_count: sourceItemCount,
    mapped_item_count: mappedItemCount,
    warnings: []
  };
}

module.exports = {
  engine: 'workflow_graph',
  importPayload
};

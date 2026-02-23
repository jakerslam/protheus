'use strict';

function asString(v) {
  return String(v == null ? '' : v).trim();
}

function asLower(v) {
  return asString(v).toLowerCase();
}

function asStringArrayLower(v) {
  if (!Array.isArray(v)) return [];
  const out = [];
  for (const item of v) {
    const s = asLower(item);
    if (!s) continue;
    if (!out.includes(s)) out.push(s);
  }
  return out;
}

function normalizeCampaigns(strategy) {
  const campaigns = strategy && Array.isArray(strategy.campaigns) ? strategy.campaigns : [];
  return campaigns
    .filter((campaign) => asLower(campaign && campaign.status) === 'active')
    .map((campaign) => {
      const phases = Array.isArray(campaign && campaign.phases) ? campaign.phases : [];
      const activePhases = phases
        .filter((phase) => asLower(phase && phase.status) === 'active')
        .sort((a, b) => {
          const ao = Number(a && a.order || 99);
          const bo = Number(b && b.order || 99);
          if (ao !== bo) return ao - bo;
          const ap = Number(a && a.priority || 0);
          const bp = Number(b && b.priority || 0);
          if (bp !== ap) return bp - ap;
          return asString(a && a.id).localeCompare(asString(b && b.id));
        });
      return {
        id: asLower(campaign && campaign.id),
        name: asString(campaign && campaign.name),
        objective_id: asString(campaign && campaign.objective_id),
        priority: Number(campaign && campaign.priority || 50),
        proposal_types: asStringArrayLower(campaign && campaign.proposal_types),
        source_eyes: asStringArrayLower(campaign && campaign.source_eyes),
        tags: asStringArrayLower(campaign && campaign.tags),
        phases: activePhases.map((phase) => ({
          id: asLower(phase && phase.id),
          name: asString(phase && phase.name),
          objective_id: asString(phase && phase.objective_id),
          order: Number(phase && phase.order || 99),
          priority: Number(phase && phase.priority || 0),
          proposal_types: asStringArrayLower(phase && phase.proposal_types),
          source_eyes: asStringArrayLower(phase && phase.source_eyes),
          tags: asStringArrayLower(phase && phase.tags)
        }))
      };
    })
    .filter((campaign) => campaign.id && campaign.phases.length > 0)
    .sort((a, b) => {
      if (a.priority !== b.priority) return a.priority - b.priority;
      return a.id.localeCompare(b.id);
    });
}

function candidateObjectiveId(candidate) {
  const c = candidate && typeof candidate === 'object' ? candidate : {};
  const proposal = c.proposal && typeof c.proposal === 'object' ? c.proposal : {};
  const meta = proposal.meta && typeof proposal.meta === 'object' ? proposal.meta : {};
  const actionSpec = proposal.action_spec && typeof proposal.action_spec === 'object' ? proposal.action_spec : {};
  const parts = [
    c.objective_binding && c.objective_binding.objective_id,
    c.directive_pulse && c.directive_pulse.objective_id,
    meta.objective_id,
    meta.directive_objective_id,
    actionSpec.objective_id
  ];
  for (const value of parts) {
    const s = asString(value);
    if (s) return s;
  }
  return '';
}

function candidateType(candidate) {
  return asLower(candidate && candidate.proposal && candidate.proposal.type);
}

function candidateSourceEye(candidate) {
  return asLower(candidate && candidate.proposal && candidate.proposal.meta && candidate.proposal.meta.source_eye);
}

function candidateTagSet(candidate) {
  const proposal = candidate && candidate.proposal && typeof candidate.proposal === 'object'
    ? candidate.proposal
    : {};
  const tagsA = asStringArrayLower(proposal.tags);
  const tagsB = asStringArrayLower(proposal.meta && proposal.meta.tags);
  return new Set([...tagsA, ...tagsB]);
}

function hasAnyOverlap(list, setObj) {
  if (!Array.isArray(list) || list.length === 0) return true;
  for (const item of list) {
    if (setObj.has(item)) return true;
  }
  return false;
}

function isFilterMatch(requiredList, value) {
  if (!Array.isArray(requiredList) || requiredList.length === 0) return true;
  return requiredList.includes(value);
}

function scoreMatch(campaign, phase, info) {
  if (!campaign || !phase || !info) return null;

  const objectiveId = asString(info.objective_id);
  const proposalType = asString(info.proposal_type);
  const sourceEye = asString(info.source_eye);

  if (campaign.objective_id && objectiveId !== campaign.objective_id) return null;
  if (phase.objective_id && objectiveId !== phase.objective_id) return null;
  if (!isFilterMatch(campaign.proposal_types, proposalType)) return null;
  if (!isFilterMatch(phase.proposal_types, proposalType)) return null;
  if (!isFilterMatch(campaign.source_eyes, sourceEye)) return null;
  if (!isFilterMatch(phase.source_eyes, sourceEye)) return null;
  if (!hasAnyOverlap(campaign.tags, info.tags)) return null;
  if (!hasAnyOverlap(phase.tags, info.tags)) return null;

  const tagOverlap = Math.max(
    0,
    [...info.tags].filter((tag) => campaign.tags.includes(tag) || phase.tags.includes(tag)).length
  );

  let score = 0;
  score += Math.max(0, 120 - Number(campaign.priority || 50));
  score += Math.max(0, 80 - (Number(phase.order || 99) * 5));
  score += Number(phase.priority || 0);
  if (campaign.objective_id && objectiveId) score += 35;
  if (phase.objective_id && objectiveId) score += 20;
  if (campaign.proposal_types.length > 0) score += 18;
  if (phase.proposal_types.length > 0) score += 14;
  if (campaign.source_eyes.length > 0 || phase.source_eyes.length > 0) score += 10;
  score += Math.min(20, tagOverlap * 4);

  return {
    matched: true,
    score: Number(score.toFixed(3)),
    campaign_id: campaign.id,
    campaign_name: campaign.name || campaign.id,
    campaign_priority: Number(campaign.priority || 50),
    phase_id: phase.id,
    phase_name: phase.name || phase.id,
    phase_order: Number(phase.order || 99),
    phase_priority: Number(phase.priority || 0),
    objective_id: objectiveId || campaign.objective_id || phase.objective_id || null
  };
}

function bestCampaignMatch(candidate, campaigns) {
  if (!Array.isArray(campaigns) || campaigns.length === 0) return null;
  const info = {
    objective_id: candidateObjectiveId(candidate),
    proposal_type: candidateType(candidate),
    source_eye: candidateSourceEye(candidate),
    tags: candidateTagSet(candidate)
  };
  let best = null;
  for (const campaign of campaigns) {
    const phases = Array.isArray(campaign && campaign.phases) ? campaign.phases : [];
    for (const phase of phases) {
      const match = scoreMatch(campaign, phase, info);
      if (!match) continue;
      if (!best || Number(match.score || 0) > Number(best.score || 0)) best = match;
    }
  }
  return best;
}

function annotateCampaignPriority(candidates, strategy) {
  const list = Array.isArray(candidates) ? candidates : [];
  const campaigns = normalizeCampaigns(strategy);
  if (!campaigns.length) {
    for (const candidate of list) {
      candidate.campaign_match = null;
      candidate.campaign_sort_bucket = 0;
      candidate.campaign_sort_score = 0;
    }
    return {
      enabled: false,
      campaign_count: 0,
      matched_count: 0
    };
  }

  let matchedCount = 0;
  const byCampaign = {};
  for (const candidate of list) {
    const match = bestCampaignMatch(candidate, campaigns);
    if (match && match.matched) {
      matchedCount += 1;
      byCampaign[match.campaign_id] = Number(byCampaign[match.campaign_id] || 0) + 1;
      candidate.campaign_match = match;
      candidate.campaign_sort_bucket = 1;
      candidate.campaign_sort_score = Number(match.score || 0);
    } else {
      candidate.campaign_match = null;
      candidate.campaign_sort_bucket = 0;
      candidate.campaign_sort_score = 0;
    }
  }
  return {
    enabled: true,
    campaign_count: campaigns.length,
    matched_count: matchedCount,
    unmatched_count: Math.max(0, list.length - matchedCount),
    matched_by_campaign: byCampaign
  };
}

module.exports = {
  normalizeCampaigns,
  annotateCampaignPriority
};

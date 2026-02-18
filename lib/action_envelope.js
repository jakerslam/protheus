#!/usr/bin/env node
/**
 * action_envelope.js - Action Envelope System
 * 
 * Creates standardized action envelopes for governance checks.
 * Classifies actions into types and risk levels for directive enforcement.
 */

const crypto = require('crypto');

// Action type enum
const ACTION_TYPES = {
  RESEARCH: 'research',
  CODE_CHANGE: 'code_change',
  PUBLISH_PUBLICLY: 'publish_publicly',
  SPEND_MONEY: 'spend_money',
  CHANGE_CREDENTIALS: 'change_credentials',
  DELETE_DATA: 'delete_data',
  OUTBOUND_CONTACT_NEW: 'outbound_contact_new',
  OUTBOUND_CONTACT_EXISTING: 'outbound_contact_existing',
  DEPLOYMENT: 'deployment',
  OTHER: 'other'
};

// Risk levels
const RISK_LEVELS = {
  LOW: 'low',
  MEDIUM: 'medium',
  HIGH: 'high'
};

// High-stakes command patterns
const HIGH_STAKES_PATTERNS = {
  spend_money: [
    /purchase/i,
    /buy/i,
    /subscribe/i,
    /payment/i,
    /\$\d+/,  // Dollar amounts
    /\d+\s*(USD|EUR|GBP)/i
  ],
  publish_publicly: [
    /post\s+to/i,
    /publish/i,
    /tweet/i,
    /moltbook.*create/i,
    /blog/i,
    /medium/i,
    /github.*push/i
  ],
  change_credentials: [
    /password/i,
    /api_key/i,
    /token/i,
    /credential/i,
    /auth/i,
    /secret/i,
    /rotate/i
  ],
  delete_data: [
    /rm\s+-rf/i,
    /delete/i,
    /drop\s+table/i,
    /destroy/i,
    /reset/i,
    /truncate/i
  ],
  outbound_contact_new: [
    /send.*email/i,
    /email.*to/i,
    /message.*new/i,
    /contact.*@/i,
    /reach.out/i
  ],
  deployment: [
    /deploy/i,
    /release/i,
    /production/i,
    /prod/i,
    /go.*live/i
  ]
};

const LOW_RISK_PATTERNS = [
  /read/i,
  /list/i,
  /get/i,
  /fetch/i,
  /search/i,
  /grep/i,
  /cat\s+/i,
  /ls\s+/i,
  /echo/i,
  /test/i,
  /benchmark/i
];

/**
 * Create a standardized action envelope
 */
function createActionEnvelope({
  directive_id = null,
  tier = 2,
  type = ACTION_TYPES.OTHER,
  summary,
  risk = RISK_LEVELS.LOW,
  payload = {},
  tags = [],
  toolName = null,
  commandText = null
}) {
  const actionId = generateActionId();
  
  return {
    action_id: actionId,
    directive_id,
    tier,
    type,
    summary,
    risk,
    payload,
    tags,
    metadata: {
      created_at: new Date().toISOString(),
      tool_name: toolName,
      command_text: commandText,
      requires_approval: false,  // Will be set by resolver
      allowed: true,             // Will be set by resolver
      blocked_reason: null       // Will be set by resolver
    }
  };
}

/**
 * Generate unique action ID
 */
function generateActionId() {
  const timestamp = Date.now().toString(36);
  const random = crypto.randomBytes(4).toString('hex');
  return `act_${timestamp}_${random}`;
}

/**
 * Classify an action based on tool name and command text
 */
function classifyAction({ toolName, commandText, payload = {} }) {
  const text = `${toolName || ''} ${commandText || ''}`.toLowerCase();
  
  // Check for high-stakes patterns
  for (const [type, patterns] of Object.entries(HIGH_STAKES_PATTERNS)) {
    for (const pattern of patterns) {
      if (pattern.test(text)) {
        return {
          type: ACTION_TYPES[type.toUpperCase()] || ACTION_TYPES.OTHER,
          risk: RISK_LEVELS.HIGH,
          confidence: 'medium',
          matched_pattern: pattern.toString()
        };
      }
    }
  }
  
  // Check for low-risk patterns
  for (const pattern of LOW_RISK_PATTERNS) {
    if (pattern.test(text)) {
      return {
        type: ACTION_TYPES.RESEARCH,
        risk: RISK_LEVELS.LOW,
        confidence: 'low',
        matched_pattern: pattern.toString()
      };
    }
  }
  
  // Default classification
  return {
    type: ACTION_TYPES.OTHER,
    risk: RISK_LEVELS.MEDIUM,
    confidence: 'low',
    matched_pattern: null
  };
}

/**
 * Determine if action type requires approval based on T0 invariants
 */
function requiresApprovalByDefault(type) {
  const approvalRequiredTypes = [
    ACTION_TYPES.PUBLISH_PUBLICLY,
    ACTION_TYPES.SPEND_MONEY,
    ACTION_TYPES.CHANGE_CREDENTIALS,
    ACTION_TYPES.DELETE_DATA,
    ACTION_TYPES.OUTBOUND_CONTACT_NEW,
    ACTION_TYPES.DEPLOYMENT
  ];
  
  return approvalRequiredTypes.includes(type);
}

/**
 * Detect irreversible commands in text
 */
function detectIrreversible(commandText) {
  const irreversiblePatterns = [
    /rm\s+-rf/i,
    /rm\s+.*\/\*/i,
    /drop\s+database/i,
    /drop\s+table/i,
    /truncate.*table/i,
    /delete.*where/i,
    /destroy/i,
    /reset\s+--hard/i,
    /git\s+clean\s+-fd/i
  ];
  
  for (const pattern of irreversiblePatterns) {
    if (pattern.test(commandText)) {
      return {
        is_irreversible: true,
        pattern: pattern.toString(),
        severity: 'critical'
      };
    }
  }
  
  return { is_irreversible: false };
}

/**
 * Auto-classify and create envelope in one call
 */
function autoClassifyAndCreate({ toolName, commandText, payload = {}, summary = null }) {
  const classification = classifyAction({ toolName, commandText, payload });
  
  const autoSummary = summary || generateSummary(toolName, commandText, classification.type);
  
  return createActionEnvelope({
    type: classification.type,
    risk: classification.risk,
    summary: autoSummary,
    toolName,
    commandText,
    payload,
    tags: [classification.type, classification.risk]
  });
}

/**
 * Generate human-readable summary
 */
function generateSummary(toolName, commandText, type) {
  if (toolName && commandText) {
    return `${type}: ${toolName} - ${commandText.substring(0, 50)}${commandText.length > 50 ? '...' : ''}`;
  } else if (toolName) {
    return `${type}: ${toolName}`;
  } else if (commandText) {
    return `${type}: ${commandText.substring(0, 60)}${commandText.length > 60 ? '...' : ''}`;
  }
  return `${type}: Unnamed action`;
}

module.exports = {
  ACTION_TYPES,
  RISK_LEVELS,
  createActionEnvelope,
  classifyAction,
  autoClassifyAndCreate,
  requiresApprovalByDefault,
  detectIrreversible,
  generateActionId
};
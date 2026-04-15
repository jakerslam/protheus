// Infring Setup Wizard — First-run guided setup (Provider + Agent + Channel)
'use strict';

/** Escape a string for use inside TOML triple-quoted strings ("""\n...\n"""). */
function wizardTomlMultilineEscape(s) {
  return s.replace(/\\/g, '\\\\').replace(/"""/g, '""\\"');
}

/** Escape a string for use inside a TOML basic (single-line) string ("..."). */
function wizardTomlBasicEscape(s) {
  return s.replace(/\\/g, '\\\\').replace(/"/g, '\\"').replace(/\n/g, '\\n').replace(/\r/g, '\\r').replace(/\t/g, '\\t');
}

function wizardPage() {
  return {
    step: 1,
    totalSteps: 6,
    loading: false,
    error: '',

    // Step 2: Provider setup
    providers: [],
    selectedProvider: '',
    apiKeyInput: '',
    testingProvider: false,
    testResult: null,
    savingKey: false,
    keySaved: false,

    // Step 3: Agent creation
    templates: [
      {
        id: 'assistant',
        name: 'General Assistant',
        description: 'A versatile helper for everyday tasks, answering questions, and providing recommendations.',
        icon: 'GA',
        category: 'General',
        provider: 'deepseek',
        model: 'deepseek-chat',
        profile: 'balanced',
        system_prompt: 'You are a helpful, friendly assistant. Provide clear, accurate, and concise responses. Ask clarifying questions when needed.'
      },
      {
        id: 'coder',
        name: 'Code Helper',
        description: 'A programming-focused agent that writes, reviews, and debugs code across multiple languages.',
        icon: 'CH',
        category: 'Development',
        provider: 'deepseek',
        model: 'deepseek-chat',
        profile: 'precise',
        system_prompt: 'You are an expert programmer. Help users write clean, efficient code. Explain your reasoning. Follow best practices and conventions for the language being used.'
      },
      {
        id: 'researcher',
        name: 'Researcher',
        description: 'An analytical agent that breaks down complex topics, synthesizes information, and provides cited summaries.',
        icon: 'RS',
        category: 'Research',
        provider: 'gemini',
        model: 'gemini-2.5-flash',
        profile: 'balanced',
        system_prompt: 'You are a research analyst. Break down complex topics into clear explanations. Provide structured analysis with key findings. Cite sources when available.'
      },
      {
        id: 'writer',
        name: 'Writer',
        description: 'A creative writing agent that helps with drafting, editing, and improving written content of all kinds.',
        icon: 'WR',
        category: 'Writing',
        provider: 'deepseek',
        model: 'deepseek-chat',
        profile: 'creative',
        system_prompt: 'You are a skilled writer and editor. Help users create polished content. Adapt your tone and style to match the intended audience. Offer constructive suggestions for improvement.'
      },
      {
        id: 'data-analyst',
        name: 'Data Analyst',
        description: 'A data-focused agent that helps analyze datasets, create queries, and interpret statistical results.',
        icon: 'DA',
        category: 'Development',
        provider: 'gemini',
        model: 'gemini-2.5-flash',
        profile: 'precise',
        system_prompt: 'You are a data analysis expert. Help users understand their data, write SQL/Python queries, and interpret results. Present findings clearly with actionable insights.'
      },
      {
        id: 'devops',
        name: 'DevOps Engineer',
        description: 'A systems-focused agent for CI/CD, infrastructure, Docker, and deployment troubleshooting.',
        icon: 'DO',
        category: 'Development',
        provider: 'deepseek',
        model: 'deepseek-chat',
        profile: 'precise',
        system_prompt: 'You are a DevOps engineer. Help with CI/CD pipelines, Docker, Kubernetes, infrastructure as code, and deployment. Prioritize reliability and security.'
      },
      {
        id: 'support',
        name: 'Customer Support',
        description: 'A professional, empathetic agent for handling customer inquiries and resolving issues.',
        icon: 'CS',
        category: 'Business',
        provider: 'groq',
        model: 'llama-3.3-70b-versatile',
        profile: 'balanced',
        system_prompt: 'You are a professional customer support representative. Be empathetic, patient, and solution-oriented. Acknowledge concerns before offering solutions. Escalate complex issues appropriately.'
      },
      {
        id: 'tutor',
        name: 'Tutor',
        description: 'A patient educational agent that explains concepts step-by-step and adapts to the learner\'s level.',
        icon: 'TU',
        category: 'General',
        provider: 'groq',
        model: 'llama-3.3-70b-versatile',
        profile: 'balanced',
        system_prompt: 'You are a patient and encouraging tutor. Explain concepts step by step, starting from fundamentals. Use analogies and examples. Check understanding before moving on. Adapt to the learner\'s pace.'
      },
      {
        id: 'api-designer',
        name: 'API Designer',
        description: 'An agent specialized in RESTful API design, OpenAPI specs, and integration architecture.',
        icon: 'AD',
        category: 'Development',
        provider: 'deepseek',
        model: 'deepseek-chat',
        profile: 'precise',
        system_prompt: 'You are an API design expert. Help users design clean, consistent RESTful APIs following best practices. Cover endpoint naming, request/response schemas, error handling, and versioning.'
      },
      {
        id: 'meeting-notes',
        name: 'Meeting Notes',
        description: 'Summarizes meeting transcripts into structured notes with action items and key decisions.',
        icon: 'MN',
        category: 'Business',
        provider: 'groq',
        model: 'llama-3.3-70b-versatile',
        profile: 'precise',
        system_prompt: 'You are a meeting summarizer. When given a meeting transcript or notes, produce a structured summary with: key decisions, action items (with owners), discussion highlights, and follow-up questions.'
      }
    ],
    selectedTemplate: 0,
    agentName: 'my-assistant',
    creatingAgent: false,
    createdAgent: null,

    // Step 3: Category filtering
    templateCategory: 'All',
    get templateCategories() {
      var cats = { 'All': true };
      this.templates.forEach(function(t) { if (t.category) cats[t.category] = true; });
      return Object.keys(cats);
    },
    get filteredTemplates() {
      var cat = this.templateCategory;
      if (cat === 'All') return this.templates;
      return this.templates.filter(function(t) { return t.category === cat; });
    },

    // Step 3: Profile/tool descriptions
    profileDescriptions: {
      minimal: { label: 'Minimal', desc: 'Read-only file access' },
      coding: { label: 'Coding', desc: 'Files + shell + web fetch' },
      research: { label: 'Research', desc: 'Web search + file read/write' },
      balanced: { label: 'Balanced', desc: 'General-purpose tool set' },
      precise: { label: 'Precise', desc: 'Focused tool set for accuracy' },
      creative: { label: 'Creative', desc: 'Full tools with creative emphasis' },
      full: { label: 'Full', desc: 'All 35+ tools' }
    },
    profileInfo: function(name) { return this.profileDescriptions[name] || { label: name, desc: '' }; },

    // Step 4: Try It chat
    tryItMessages: [],
    tryItInput: '',
    tryItSending: false,
    suggestedMessages: {
      'General': ['What can you help me with?', 'Tell me a fun fact', 'Summarize the latest AI news'],
      'Development': ['Write a Python hello world', 'Explain async/await', 'Review this code snippet'],
      'Research': ['Explain quantum computing simply', 'Compare React vs Vue', 'What are the latest trends in AI?'],
      'Writing': ['Help me write a professional email', 'Improve this paragraph', 'Write a blog intro about AI'],
      'Business': ['Draft a meeting agenda', 'How do I handle a complaint?', 'Create a project status update']
    },
    get currentSuggestions() {
      var tpl = this.templates[this.selectedTemplate];
      var cat = tpl ? tpl.category : 'General';
      return this.suggestedMessages[cat] || this.suggestedMessages['General'];
    },
    async sendTryItMessage(text) {
      if (!text || !text.trim() || !this.createdAgent || this.tryItSending) return;
      text = text.trim();
      this.tryItInput = '';
      this.tryItMessages.push({ role: 'user', text: text });
      this.tryItSending = true;
      try {
        var res = await InfringAPI.post('/api/agents/' + this.createdAgent.id + '/message', { message: text });
        this.tryItMessages.push({ role: 'agent', text: res.response || '(no response)' });
        localStorage.setItem('of-first-msg', 'true');
      } catch(e) {
        this.tryItMessages.push({ role: 'agent', text: 'Error: ' + (e.message || 'Could not reach agent') });
      }
      this.tryItSending = false;
    },

    // Step 5: Channel setup (optional)
    channelType: '',
    channelOptions: [
      {
        name: 'telegram',
        display_name: 'Telegram',
        icon: 'TG',
        description: 'Connect your agent to a Telegram bot for messaging.',
        token_label: 'Bot Token',
        token_placeholder: '123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11',
        token_env: 'TELEGRAM_BOT_TOKEN',
        help: 'Create a bot via @BotFather on Telegram to get your token.'
      },
      {
        name: 'discord',
        display_name: 'Discord',

      if (status === 'installed' || status === 'already_installed') return 'dep-met';
      if (status === 'error' || status === 'timeout') return 'dep-missing';
      return '';
    },

    async recheckDeps() {
      if (!this.setupWizard) return;
      this.setupChecking = true;
      try {
        var data = await InfringAPI.post('/api/hands/' + this.setupWizard.id + '/check-deps', {});
        if (data.requirements && this.setupWizard.requirements) {
          for (var i = 0; i < this.setupWizard.requirements.length; i++) {
            var existing = this.setupWizard.requirements[i];
            for (var j = 0; j < data.requirements.length; j++) {
              if (data.requirements[j].key === existing.key) {
                existing.satisfied = data.requirements[j].satisfied;
                break;
              }
            }
          }
          this.setupWizard.requirements_met = data.requirements_met;
        }
        if (data.requirements_met) {
          this.showToast('All dependencies satisfied!');
        }
      } catch(e) {
        this.showToast('Check failed: ' + (e.message || 'unknown'));
      }
      this.setupChecking = false;
    },

    getInstallCmd(req) {
      if (!req || !req.install) return null;
      var inst = req.install;
      var plat = this.installPlatforms[req.key] || this.detectedPlatform;
      if (plat === 'macos' && inst.macos) return inst.macos;
      if (plat === 'windows' && inst.windows) return inst.windows;
      if (plat === 'linux') {
        return inst.linux_apt || inst.linux_dnf || inst.linux_pacman || inst.pip || null;
      }
      return inst.pip || inst.macos || inst.windows || inst.linux_apt || null;
    },

    getLinuxVariant(req) {
      if (!req || !req.install) return null;
      var inst = req.install;
      var plat = this.installPlatforms[req.key] || this.detectedPlatform;
      if (plat !== 'linux') return null;
      // Return all available Linux variants
      var variants = [];
      if (inst.linux_apt) variants.push({ label: 'apt', cmd: inst.linux_apt });
      if (inst.linux_dnf) variants.push({ label: 'dnf', cmd: inst.linux_dnf });
      if (inst.linux_pacman) variants.push({ label: 'pacman', cmd: inst.linux_pacman });
      if (inst.pip) variants.push({ label: 'pip', cmd: inst.pip });
      return variants.length > 1 ? variants : null;
    },

    copyToClipboard(text) {
      var self = this;
      navigator.clipboard.writeText(text).then(function() {
        self.clipboardMsg = text;
        if (self._clipboardTimer) clearTimeout(self._clipboardTimer);
        self._clipboardTimer = setTimeout(function() { self.clipboardMsg = null; }, 2000);
      });
    },

    get setupReqsMet() {
      if (!this.setupWizard || !this.setupWizard.requirements) return 0;
      var count = 0;
      for (var i = 0; i < this.setupWizard.requirements.length; i++) {
        var req = this.setupWizard.requirements[i];
        if (req.satisfied) { count++; continue; }
        // Count API key reqs as met if user entered a value
        if (req.type === 'ApiKey' && this.apiKeyInputs[req.key] && this.apiKeyInputs[req.key].trim() !== '') count++;
      }
      return count;
    },

    get setupReqsTotal() {
      if (!this.setupWizard || !this.setupWizard.requirements) return 0;
      return this.setupWizard.requirements.length;
    },

    get setupAllReqsMet() {
      if (!this.setupWizard || !this.setupWizard.requirements) return false;
      if (this.setupReqsTotal === 0) return false;
      for (var i = 0; i < this.setupWizard.requirements.length; i++) {
        var req = this.setupWizard.requirements[i];
        if (req.satisfied) continue;
        // API key reqs are satisfied if the user entered a value in the input
        if (req.type === 'ApiKey' && this.apiKeyInputs[req.key] && this.apiKeyInputs[req.key].trim() !== '') continue;
        return false;
      }
      return true;
    },

    getSettingKeyForReq(req) {
      // Find the matching setting key for an API key requirement.
      // Convention: setting key is the lowercase version of the requirement key.
      if (!this.setupWizard || !this.setupWizard.settings) return null;
      var lowerKey = req.key.toLowerCase();
      for (var i = 0; i < this.setupWizard.settings.length; i++) {
        if (this.setupWizard.settings[i].key === lowerKey) return lowerKey;
      }
      // Fallback: try matching by check_value lowercased
      if (req.check_value) {
        var lowerCheck = req.check_value.toLowerCase();
        for (var j = 0; j < this.setupWizard.settings.length; j++) {
          if (this.setupWizard.settings[j].key === lowerCheck) return lowerCheck;
        }
      }
      return null;
    },

    get setupHasReqs() {
      return this.setupReqsTotal > 0;
    },

    get setupHasSettings() {
      return this.setupWizard && this.setupWizard.settings && this.setupWizard.settings.length > 0;
    },

    setupNextStep() {
      // When leaving step 1, sync API key inputs into settings values
      if (this.setupStep === 1) {
        this._syncApiKeysToSettings();
      }
      if (this.setupStep === 1 && this.setupHasSettings) {
        this.setupStep = 2;
      } else if (this.setupStep === 1) {
        this.setupStep = 3;
      } else if (this.setupStep === 2) {
        this.setupStep = 3;
      }
    },

    _syncApiKeysToSettings() {
      if (!this.setupWizard || !this.setupWizard.requirements) return;
      for (var i = 0; i < this.setupWizard.requirements.length; i++) {
        var req = this.setupWizard.requirements[i];
        if (req.type === 'ApiKey' && this.apiKeyInputs[req.key] && this.apiKeyInputs[req.key].trim() !== '') {
          var settingKey = this.getSettingKeyForReq(req);
          if (settingKey) {
            this.settingsValues[settingKey] = this.apiKeyInputs[req.key].trim();
          }
        }
      }
    },

    setupPrevStep() {
      if (this.setupStep === 3 && this.setupHasSettings) {
        this.setupStep = 2;
      } else if (this.setupStep === 3) {
        this.setupStep = this.setupHasReqs ? 1 : 2;
      } else if (this.setupStep === 2 && this.setupHasReqs) {
        this.setupStep = 1;
      }
    },

    closeSetupWizard() {
      this.setupWizard = null;
      this.setupStep = 1;
      this.setupLoading = false;
      this.setupChecking = false;
      this.clipboardMsg = null;
      this.installPlatforms = {};
      this.apiKeyInputs = {};
    },

    async launchHand() {
      if (!this.setupWizard) return;
      var handId = this.setupWizard.id;
      // Sync API key inputs from step 1 into settings values
      if (this.setupWizard.requirements) {
        for (var i = 0; i < this.setupWizard.requirements.length; i++) {
          var req = this.setupWizard.requirements[i];
          if (req.type === 'ApiKey' && this.apiKeyInputs[req.key] && this.apiKeyInputs[req.key].trim() !== '') {
            var settingKey = this.getSettingKeyForReq(req);
            if (settingKey) {
              this.settingsValues[settingKey] = this.apiKeyInputs[req.key].trim();


// Chat voice recording controls.
'use strict';

function infringChatVoiceRecordingMethods() {
  return {
    // Voice: start recording.
    startRecording: async function() {
      if (this.recording) return;
      try {
        var stream = await navigator.mediaDevices.getUserMedia({ audio: true });
        var mimeType = MediaRecorder.isTypeSupported('audio/webm;codecs=opus') ? 'audio/webm;codecs=opus' :
                       MediaRecorder.isTypeSupported('audio/webm') ? 'audio/webm' : 'audio/ogg';
        this._audioChunks = [];
        this._mediaRecorder = new MediaRecorder(stream, { mimeType: mimeType });
        var self = this;
        this._mediaRecorder.ondataavailable = function(e) {
          if (e.data.size > 0) self._audioChunks.push(e.data);
        };
        this._mediaRecorder.onstop = function() {
          stream.getTracks().forEach(function(t) { t.stop(); });
          self._handleRecordingComplete();
        };
        this._mediaRecorder.start(250);
        this.recording = true;
        this.recordingTime = 0;
        this._recordingTimer = setInterval(function() { self.recordingTime++; }, 1000);
      } catch(e) {
        if (typeof InfringToast !== 'undefined') InfringToast.error('Microphone access denied');
      }
    },

    // Voice: stop recording.
    stopRecording: function() {
      if (!this.recording || !this._mediaRecorder) return;
      this._mediaRecorder.stop();
      this.recording = false;
      if (this._recordingTimer) { clearInterval(this._recordingTimer); this._recordingTimer = null; }
    },

    // Voice: format recording time as MM:SS.
    formatRecordingTime: function() {
      var m = Math.floor(this.recordingTime / 60);
      var s = this.recordingTime % 60;
      return (m < 10 ? '0' : '') + m + ':' + (s < 10 ? '0' : '') + s;
    },
  };
}

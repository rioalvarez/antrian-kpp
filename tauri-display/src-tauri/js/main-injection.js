// This script is injected into the webview by Tauri after the page loads.
// It replicates the functionality of the Electron version's injected script.

(function() {
    'use strict';
    console.log('Tauri: Running initialization script...');

    // ============================================
    // A. MARK AS TAURI/ELECTRON APP
    // ============================================
    window.isElectronApp = true; // The web app checks this to alter behavior
    window.isTauriApp = true;

    // Set the base URL for local audio files to use our custom protocol.
    // On Windows: http://local-audio.localhost (Tauri uses http scheme for custom protocols)
    // On macOS/Linux: local-audio://localhost
    // Will be overridden by Rust when local-tts.js is injected.
    if (!window.LOCAL_AUDIO_URL) {
        var isWindows = navigator.platform.indexOf('Win') > -1 || navigator.userAgent.indexOf('Windows') > -1;
        window.LOCAL_AUDIO_URL = isWindows ? 'http://local-audio.localhost' : 'local-audio://localhost';
    }
    if (typeof window.USE_LOCAL_TTS === 'undefined') {
        window.USE_LOCAL_TTS = false;
    }

    // ============================================
    // B. AUTO-ENABLE AUDIO & HIDE BUTTON
    // ============================================
    console.log('Tauri: Auto-enabling audio system (no click required)...');

    // Set flags immediately
    window.audioEnabled = true;
    window.audioInitialized = true;
    if (typeof audioEnabled !== 'undefined') {
        audioEnabled = true;
    }
    if (typeof audioInitialized !== 'undefined') {
        audioInitialized = true;
    }
    localStorage.setItem('display_audio_enabled', 'true');

    // Create and resume AudioContext immediately
    try {
        const AudioContext = window.AudioContext || window.webkitAudioContext;
        if (AudioContext) {
            if (!window.audioContext) {
                window.audioContext = new AudioContext();
            }
            if (window.audioContext.state === 'suspended') {
                window.audioContext.resume().then(() => {
                    console.log('AudioContext resumed successfully:', window.audioContext.state);
                });
            }
        }
    } catch (e) {
        console.error('Failed to create or resume AudioContext:', e);
    }

    // Function to permanently hide the "Enable Audio" button
    function hideAudioButton() {
        const btn = document.getElementById('enable-audio-btn');
        if (btn) {
            btn.style.display = 'none';
            btn.style.visibility = 'hidden';
            btn.remove();
            console.log('Audio button removed');
        }
    }

    // Hide immediately and at different stages of page load
    hideAudioButton();
    document.addEventListener('DOMContentLoaded', hideAudioButton);
    window.addEventListener('load', hideAudioButton);
    setTimeout(hideAudioButton, 500);
    setTimeout(hideAudioButton, 1000);
    setTimeout(hideAudioButton, 2000);

    // Override any functions on the page that might mess with our setup
    window.updateAudioButtonState = function() { /* Do nothing */ };
    window.initAutoAudio = function() {
        console.log('Tauri: initAutoAudio skipped (audio auto-enabled)');
    };

    // Call enableAudioSilent if exists
    if (typeof enableAudioSilent === 'function') {
        try { enableAudioSilent(); } catch(e) {}
    }

    // ============================================
    // C. FIX TTS VOICE - FIND BEST INDONESIAN VOICE
    // This part is for the Web Speech API fallback.
    // It will be overridden if local-tts.js is loaded.
    // ============================================

    let bestIndonesianVoice = null;

    function findBestIndonesianVoice() {
        const voices = speechSynthesis.getVoices();
        console.log('Available voices:', voices.length);

        // Priority list for Indonesian voices (best first)
        const voicePriority = [
            'Google Bahasa Indonesia',
            'Microsoft Gadis Online (Natural)',
            'Microsoft Gadis',
            'Indonesian Female',
            'Indonesian Male',
            'id-ID',
            'id_ID'
        ];

        // Find voices that match Indonesian
        const indonesianVoices = voices.filter(v =>
            v.lang.startsWith('id') ||
            v.name.toLowerCase().includes('indonesia') ||
            v.name.toLowerCase().includes('gadis')
        );

        console.log('Indonesian voices found:', indonesianVoices.map(v => v.name + ' (' + v.lang + ')'));

        // Find best match from priority list
        for (const priority of voicePriority) {
            const match = indonesianVoices.find(v =>
                v.name.includes(priority) || v.lang === priority
            );
            if (match) {
                bestIndonesianVoice = match;
                console.log('Selected voice:', match.name);
                break;
            }
        }

        // Fallback to first Indonesian voice
        if (!bestIndonesianVoice && indonesianVoices.length > 0) {
            bestIndonesianVoice = indonesianVoices[0];
            console.log('Fallback to first Indonesian voice:', bestIndonesianVoice.name);
        }

        return bestIndonesianVoice;
    }

    // Override the announceQueue function to use better voice
    const originalAnnounceQueue = window.announceQueue;

    window.announceQueue = function(queueNumber, counterName, onComplete) {
        // If local TTS is enabled and loaded, use it
        if (window.USE_LOCAL_TTS && typeof window.announceQueueLocal === 'function') {
            window.announceQueueLocal(queueNumber, counterName, onComplete);
            return;
        }

        // Check if sound is enabled
        if (typeof SOUND_ENABLED !== 'undefined' && !SOUND_ENABLED) {
            if (onComplete) onComplete();
            return;
        }

        if (!("speechSynthesis" in window)) {
            if (onComplete) onComplete();
            return;
        }

        // Ensure voices are loaded
        if (!bestIndonesianVoice) {
            findBestIndonesianVoice();
        }

        const formattedNumber = typeof formatQueueNumberForSpeech === 'function'
            ? formatQueueNumberForSpeech(queueNumber)
            : queueNumber;
        const formattedCounter = typeof formatCounterNameForSpeech === 'function'
            ? formatCounterNameForSpeech(counterName)
            : counterName;
        const text = 'Nomor antrian ' + formattedNumber + ', silakan menuju ' + formattedCounter;

        // Cancel any ongoing speech
        speechSynthesis.cancel();

        const utterance = new SpeechSynthesisUtterance(text);
        utterance.lang = 'id-ID';
        utterance.rate = typeof TTS_RATE !== 'undefined' ? TTS_RATE : 0.9;
        utterance.pitch = 1;
        utterance.volume = 1;

        // Use best Indonesian voice if found
        if (bestIndonesianVoice) {
            utterance.voice = bestIndonesianVoice;
        }

        utterance.onend = function() {
            if (onComplete) onComplete();
        };

        utterance.onerror = function(e) {
            console.error('Tauri: Web Speech API TTS Error:', e);
            if (onComplete) onComplete();
        };

        // Small delay to ensure voice is ready
        setTimeout(() => {
            speechSynthesis.speak(utterance);
        }, 50);
    };

    // Load voices (may be async)
    if (speechSynthesis.getVoices().length === 0) {
        speechSynthesis.onvoiceschanged = function() {
            findBestIndonesianVoice();
            console.log('Voices loaded via event');
        };
    } else {
        findBestIndonesianVoice();
    }

    // Force load voices by speaking empty string
    try {
        const dummy = new SpeechSynthesisUtterance('');
        speechSynthesis.speak(dummy);
        speechSynthesis.cancel();
    } catch(e) {}

    // ============================================
    // D. VERIFY AUDIO STATUS (after 1.5 seconds)
    // ============================================
    setTimeout(() => {
        console.log('=== TAURI AUDIO STATUS ===');
        console.log('audioEnabled:', window.audioEnabled);
        console.log('audioInitialized:', window.audioInitialized);
        console.log('AudioContext state:', window.audioContext ? window.audioContext.state : 'N/A');
        console.log('Voices available:', speechSynthesis.getVoices().length);
        console.log('Best Indonesian voice:', bestIndonesianVoice ? bestIndonesianVoice.name : 'None found');
        console.log('Audio button visible:', !!document.getElementById('enable-audio-btn'));
        console.log('USE_LOCAL_TTS:', window.USE_LOCAL_TTS);
        console.log('LOCAL_AUDIO_URL:', window.LOCAL_AUDIO_URL);
        console.log('==========================');
    }, 1500);

    console.log('Tauri: Audio system initialized - NO CLICK REQUIRED');

})();

/**
 * Local TTS Module - Memutar audio dari file lokal
 * Diinject ke halaman display oleh Electron
 */

(function() {
    'use strict';

    console.log('Local TTS Module: Initializing...');

    // Base URL for audio files (will be set by Electron)
    const AUDIO_BASE_URL = window.LOCAL_AUDIO_URL || '/audio';

    // Audio cache
    const audioCache = {};

    // Audio queue
    let audioQueue = [];
    let isPlaying = false;

    // Preload common audio files
    const preloadList = [
        'bell.mp3',
        'nomor_antrian.mp3',
        'silakan_menuju.mp3',
        'loket.mp3',
        'angka_1.mp3',
        'angka_2.mp3',
        'angka_3.mp3',
        'angka_4.mp3',
        'angka_5.mp3',
    ];

    // Preload audio files
    function preloadAudio() {
        preloadList.forEach(file => {
            loadAudio(file);
        });
    }

    // Load audio file into cache
    function loadAudio(filename) {
        if (audioCache[filename]) return audioCache[filename];

        const audio = new Audio(`${AUDIO_BASE_URL}/${filename}`);
        audio.preload = 'auto';
        audioCache[filename] = audio;
        return audio;
    }

    // Play single audio file
    function playAudioFile(filename) {
        return new Promise((resolve, reject) => {
            const audio = loadAudio(filename);

            // Clone audio to allow overlapping plays
            const playableAudio = audio.cloneNode();

            playableAudio.onended = () => resolve();
            playableAudio.onerror = (e) => {
                console.warn(`Audio file not found: ${filename}`);
                resolve(); // Continue even if file missing
            };

            playableAudio.play().catch(err => {
                console.warn(`Failed to play ${filename}:`, err);
                resolve();
            });
        });
    }

    // Convert number to audio file sequence
    function numberToAudioFiles(n) {
        const files = [];

        if (n === 0) {
            files.push('angka_0.mp3');
            return files;
        }

        // Handle thousands
        if (n >= 1000) {
            if (n >= 2000) {
                files.push(...numberToAudioFiles(Math.floor(n / 1000)));
                files.push('ribu.mp3');
            } else {
                files.push('seribu.mp3');
            }
            n = n % 1000;
            if (n === 0) return files;
        }

        // Handle hundreds
        if (n >= 100) {
            if (n >= 200) {
                files.push(`angka_${Math.floor(n / 100)}.mp3`);
                files.push('ratus.mp3');
            } else {
                files.push('seratus.mp3');
            }
            n = n % 100;
            if (n === 0) return files;
        }

        // Handle tens and units
        if (n >= 20) {
            // Use pre-recorded tens (20, 30, etc) or combine
            const tens = Math.floor(n / 10) * 10;
            files.push(`angka_${tens}.mp3`);
            n = n % 10;
            if (n > 0) {
                files.push(`angka_${n}.mp3`);
            }
        } else if (n >= 1) {
            // 1-19 have individual files
            files.push(`angka_${n}.mp3`);
        }

        return files;
    }

    // Parse queue number (e.g., "A001" -> { letter: "A", number: 1 })
    function parseQueueNumber(queueNumber) {
        const match = queueNumber.match(/^([A-Za-z]+)(\d+)$/);
        if (match) {
            return {
                letters: match[1].toUpperCase(),
                number: parseInt(match[2], 10)
            };
        }
        return { letters: '', number: 0 };
    }

    // Parse counter name - handles various formats:
    // "Loket 1" -> { prefix: "Loket", letter: "", number: 1 }
    // "Loket A1" -> { prefix: "Loket", letter: "A", number: 1 }
    // "Loket A 1" -> { prefix: "Loket", letter: "A", number: 1 }
    // "A1" -> { prefix: "", letter: "A", number: 1 }
    // "A 1" -> { prefix: "", letter: "A", number: 1 }
    function parseCounterName(counterName) {
        // Pattern 1: "Loket A1" or "Loket A 1" (prefix + letter + number)
        let match = counterName.match(/^(.+?)\s+([A-Za-z])\s*(\d+)$/);
        if (match) {
            return {
                prefix: match[1].trim(),
                letter: match[2].toUpperCase(),
                number: parseInt(match[3], 10)
            };
        }

        // Pattern 2: "A1" or "A 1" (letter + number only)
        match = counterName.match(/^([A-Za-z])\s*(\d+)$/);
        if (match) {
            return {
                prefix: '',
                letter: match[1].toUpperCase(),
                number: parseInt(match[2], 10)
            };
        }

        // Pattern 3: "Loket 1" (prefix + number only)
        match = counterName.match(/^(.+?)\s*(\d+)$/);
        if (match) {
            return {
                prefix: match[1].trim(),
                letter: '',
                number: parseInt(match[2], 10)
            };
        }

        // No pattern matched
        return { prefix: counterName, letter: '', number: 0 };
    }

    // Build audio sequence for announcement
    function buildAnnouncementSequence(queueNumber, counterName) {
        const files = [];
        const queue = parseQueueNumber(queueNumber);
        const counter = parseCounterName(counterName);

        console.log('Parsed queue:', queue);
        console.log('Parsed counter:', counter);

        // Bell
        files.push('bell.mp3');

        // "Nomor antrian"
        files.push('nomor_antrian.mp3');

        // Queue letters (e.g., "A" from "A001")
        for (const letter of queue.letters) {
            files.push(`huruf_${letter.toLowerCase()}.mp3`);
        }

        // Queue number
        files.push(...numberToAudioFiles(queue.number));

        // "silakan menuju"
        files.push('silakan_menuju.mp3');

        // Counter prefix (e.g., "Loket")
        if (counter.prefix) {
            if (counter.prefix.toLowerCase().includes('loket')) {
                files.push('loket.mp3');
            }
            // Add more prefixes here if needed (e.g., "Counter", "Meja", etc.)
        }

        // Counter letter (e.g., "A" from "Loket A1")
        if (counter.letter) {
            files.push(`huruf_${counter.letter.toLowerCase()}.mp3`);
        }

        // Counter number
        if (counter.number > 0) {
            files.push(...numberToAudioFiles(counter.number));
        }

        return files;
    }

    // Play sequence of audio files
    async function playSequence(files, onComplete) {
        for (const file of files) {
            await playAudioFile(file);
            // Small gap between audio files
            await new Promise(r => setTimeout(r, 100));
        }
        if (onComplete) onComplete();
    }

    // Main announce function - replaces Web Speech API
    window.announceQueueLocal = function(queueNumber, counterName, onComplete) {
        console.log(`Local TTS: Announcing ${queueNumber} -> ${counterName}`);

        const files = buildAnnouncementSequence(queueNumber, counterName);
        console.log('Audio sequence:', files);

        playSequence(files, onComplete);
    };

    // Override the original announceQueue function
    const originalAnnounceQueue = window.announceQueue;

    window.announceQueue = function(queueNumber, counterName, onComplete) {
        // Check if local TTS is enabled
        if (window.USE_LOCAL_TTS) {
            window.announceQueueLocal(queueNumber, counterName, onComplete);
        } else if (originalAnnounceQueue) {
            // Fall back to Web Speech API
            originalAnnounceQueue(queueNumber, counterName, onComplete);
        } else if (onComplete) {
            onComplete();
        }
    };

    // Play bell only
    window.playBellLocal = function() {
        playAudioFile('bell.mp3');
    };

    // Test function
    window.testLocalTTS = function(queueNumber = 'A001', counterName = 'Loket A1') {
        console.log('Testing Local TTS...');
        console.log('Queue:', queueNumber, '-> Counter:', counterName);
        window.USE_LOCAL_TTS = true;
        window.announceQueue(queueNumber, counterName, () => {
            console.log('Test complete');
        });
    };

    // Test various counter formats
    window.testCounterFormats = function() {
        const testCases = [
            { queue: 'A001', counter: 'Loket 1' },      // Loket + angka
            { queue: 'A002', counter: 'Loket A1' },     // Loket + huruf + angka
            { queue: 'B003', counter: 'Loket A 1' },    // Loket + huruf + spasi + angka
            { queue: 'C004', counter: 'A1' },           // Huruf + angka saja
            { queue: 'D005', counter: 'A 1' },          // Huruf + spasi + angka
            { queue: 'E006', counter: 'Loket B2' },     // Loket B2
        ];

        let index = 0;
        function playNext() {
            if (index >= testCases.length) {
                console.log('All tests complete!');
                return;
            }
            const test = testCases[index];
            console.log(`\\n=== Test ${index + 1}: ${test.queue} -> ${test.counter} ===`);
            index++;
            window.testLocalTTS(test.queue, test.counter);
            setTimeout(playNext, 5000); // Wait 5 seconds between tests
        }
        playNext();
    };

    // Initialize
    preloadAudio();

    console.log('Local TTS Module: Ready');
    console.log('Test with: testLocalTTS("A001", "Loket A1")');
    console.log('Test all formats: testCounterFormats()');
})();

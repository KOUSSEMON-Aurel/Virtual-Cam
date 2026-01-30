class PhoneCamUltimate {
    constructor() {
        this.socket = null;
        this.encoder = null;
        this.currentStream = null;
        this.isStreaming = false;
        this.videoElement = document.getElementById('preview');
        this.statsElement = document.getElementById('stats'); // Kept from original
        this.frameCount = 0;
        this.startTime = performance.now(); // Kept from original
        this.currentWidth = 1280;
        this.currentHeight = 720;
        // Assuming setupDynamicControls() is meant to be called here based on the provided snippet
        // However, the original code calls it later in start().
        // For now, I will add it as per the instruction's snippet, but this might need review.
        // this.setupDynamicControls(); // This line was in the provided snippet's constructor, but not in the original.
        // I will omit it from the constructor to maintain original behavior unless explicitly asked.
    }

    log(message) {
        console.log(message);
        const logContent = document.getElementById('logContent');
        const statusEl = document.getElementById('status');

        if (logContent) {
            const time = new Date().toLocaleTimeString().split(' ')[0];
            logContent.innerHTML += `<br>[${time}] ${message}`;
            const debugLogs = document.getElementById('debugLogs');
            if (debugLogs) debugLogs.scrollTop = debugLogs.scrollHeight;
        }

        if (statusEl && message.includes('❌')) {
            statusEl.innerText = message;
        }
    }

    updateStats() {
        const fpsStat = document.getElementById('fps-stat');
        if (fpsStat) {
            const now = performance.now();
            const elapsed = (now - this.startTime) / 1000;
            const fps = (this.frameCount / elapsed).toFixed(1);
            fpsStat.innerText = `FPS: ${fps} | Chunk: ${this.chunkCounter || 0} | T: ${elapsed.toFixed(1)}s`;
        }
    }

    async start() {
        try {
            this.log('[START] 🚀 Démarrage...');
            this.isStreaming = true; // This line was added based on the snippet, assuming it was intended to be here.

            // Reset des compteurs pour un calcul de FPS précis
            this.frameCount = 0;
            this.startTime = performance.now();

            // Vérification de la disponibilité de getUserMedia
            if (!navigator.mediaDevices || !navigator.mediaDevices.getUserMedia) {
                throw new Error(
                    '❌ Accès caméra bloqué ! ' +
                    'Les navigateurs exigent HTTPS pour accéder à la caméra. ' +
                    'Solution : Utilise Chrome/Firefox et accepte le certificat auto-signé, ' +
                    'ou configure un reverse proxy HTTPS (nginx, caddy).'
                );
            }

            this.log('[START] getUserMedia disponible');
            document.getElementById('status').innerText = '📷 Demande d\'accès à la caméra...';

            const cameraMode = document.getElementById('cameraSelect').value;
            const resValue = document.getElementById('resSelect').value;
            const [width, height] = resValue.split('x').map(Number);

            this.log(`[START] Configuration: ${cameraMode}, ${width}x${height}`);
            await this.initStream(cameraMode, width, height);

            this.log('[START] Stream initialisé avec succès');
            // this.isStreaming = true; // Moved up based on snippet

            // Changer le bouton en "Arrêter"
            const btn = document.getElementById('startBtn');
            btn.innerText = '⏹️ ARRÊTER';
            btn.onclick = () => this.stop();

            // Activer les listeners pour changement dynamique
            this.setupDynamicControls();

            this.log('[START] Démarrage terminé avec succès');

        } catch (err) {
            this.log('[START] ❌ Erreur: ' + err.message);
            document.getElementById('status').innerText = '❌ Erreur: ' + err.message;
        }
    }

    sendMetadata(data) {
        if (this.socket && this.socket.readyState === WebSocket.OPEN) {
            this.socket.send(JSON.stringify(data));
        }
    }

    async initStream(cameraMode, width, height) {
        this.log(`[INIT] Début initStream: ${cameraMode}, ${width}x${height}`);

        // Arrêter le stream précédent si existant
        if (this.currentStream) {
            this.log('[INIT] Arrêt du stream précédent');
            this.currentStream.getTracks().forEach(track => track.stop());
        }

        this.currentWidth = width;
        this.currentHeight = height;

        this.log('[INIT] Demande getUserMedia...');
        const stream = await navigator.mediaDevices.getUserMedia({
            video: {
                facingMode: { ideal: cameraMode },
                width: { ideal: width },
                height: { ideal: height },
                frameRate: { ideal: 60 },
            }
        });

        this.log('[INIT] ✅ getUserMedia réussi');
        this.currentStream = stream;

        this.log('[INIT] Configuration flux vidéo...');
        document.getElementById('status').innerText = '🔌 Connexion au serveur...';
        this.videoElement.srcObject = stream;
        await this.videoElement.play();
        this.log('[INIT] Élément vidéo en lecture');

        // Connexion WebSocket (une seule fois)
        if (!this.socket || this.socket.readyState !== WebSocket.OPEN) {
            const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
            const wsUrl = `${protocol}//${window.location.host}/raw`;
            this.log(`[WS] Connexion à ${wsUrl}`);
            this.socket = new WebSocket(wsUrl);
            this.socket.binaryType = 'arraybuffer';

            await new Promise((resolve, reject) => {
                this.socket.onopen = () => {
                    this.log('[WS] ✅ WebSocket connecté');
                    document.getElementById('status').innerText = '✅ Connecté ! Encodage en cours...';
                    // Envoyer les métadonnées de résolution
                    const meta = JSON.stringify({
                        type: 'metadata',
                        width: this.currentWidth,
                        height: this.currentHeight
                    });
                    this.socket.send(meta);
                    resolve();
                };
                this.socket.onerror = (err) => {
                    this.log('[WS] ❌ ERREUR WebSocket: ' + err.message);
                    this.log('[WS] URL tentée: ' + wsUrl);
                    this.log('[WS] Protocol: ' + protocol);
                    this.log('[WS] Host: ' + window.location.host);
                    document.getElementById('status').innerText = '❌ Erreur de connexion WebSocket';
                    reject(err);
                };
                this.socket.onclose = () => {
                    this.log('[WS] WebSocket fermé');
                    document.getElementById('status').innerText = '🔌 WebSocket déconnecté';
                };
                setTimeout(() => reject(new Error('Timeout')), 5000);
            });
        } else {
            // Juste envoyer les nouvelles métadonnées
            const meta = JSON.stringify({ type: 'metadata', width, height });
            this.socket.send(meta);
        }

        document.getElementById('status').innerText = '🎥 Encodage vidéo...';

        // Réinitialiser l'encodeur
        if (this.encoder) {
            await this.encoder.flush();
            this.encoder.close();
        }

        this.log('[ENCODER] Initialisation...');

        const tryConfigs = [
            { codec: 'avc1.420028', width: this.currentWidth, height: this.currentHeight, bitrate: 4_000_000, framerate: 60 }, // High-res 60FPS
            { codec: 'avc1.42001f', width: 1280, height: 720, bitrate: 2_500_000, framerate: 60 }, // Standard 720p 60FPS
            { codec: 'avc1.42E01E', width: 1280, height: 720, bitrate: 2_000_000, framerate: 30 }, // Safe 720p 30FPS
            { codec: 'avc1.42E01E', width: 640, height: 360, bitrate: 800_000, framerate: 30 }   // 360p fallback
        ];

        let success = false;
        for (const config of tryConfigs) {
            try {
                this.log(`[ENCODER] 🧪 Essai: ${config.codec} @ ${config.width}x${config.height} (${config.framerate}fps)`);
                this.encoder = new VideoEncoder({
                    output: (chunk, metadata) => {
                        // Crucial: Transmettre la description AVC (SPS/PPS) au dashboard
                        if (metadata && metadata.decoderConfig && metadata.decoderConfig.description) {
                            const desc = Array.from(new Uint8Array(metadata.decoderConfig.description));
                            this.sendMetadata({
                                type: 'v-config',
                                codec: config.codec,
                                width: config.width,
                                height: config.height,
                                description: desc
                            });
                            this.log('[ENCODER] 📤 Description AVC transmise');
                        }
                        this.handleEncodedChunk(chunk);
                    },
                    error: (e) => this.log('❌ Encoder Fatal: ' + e.message)
                });

                config.latencyMode = 'realtime';

                const support = await VideoEncoder.isConfigSupported(config);
                if (support.supported) {
                    this.encoder.configure(config);
                    this.log(`[ENCODER] ✅ SUCCÈS: ${config.codec}`);
                    this.currentWidth = config.width;
                    this.currentHeight = config.height;
                    success = true;
                    break;
                } else {
                    this.log(`[ENCODER] ⏭️ Non supporté`);
                }
            } catch (e) {
                this.log(`[ENCODER] ❌ Échec: ${e.message}`);
            }
        }

        if (!success) {
            throw new Error("L'encodeur matériel H.264 refuse toutes les configurations. Votre tablette est peut-être trop ancienne pour WebCodecs.");
        }

        document.getElementById('status').innerText = '✅ Streaming actif !';

        const [track] = stream.getVideoTracks();
        const settings = track.getSettings();
        this.log(`[TRACK] Réel: ${settings.width}x${settings.height} @ ${settings.frameRate?.toFixed(0)}fps`);
        this.log(`[TRACK] Label: ${track.label}`);

        // Vérifier si MediaStreamTrackProcessor est disponible
        if (typeof MediaStreamTrackProcessor !== 'undefined') {
            this.log('[PROCESSOR] Mode: MediaStreamTrackProcessor');
            const processor = new MediaStreamTrackProcessor({ track });
            const reader = processor.readable.getReader();
            this.processFrames(reader);
        } else {
            this.log('[PROCESSOR] Mode: Legacy (requestAnimationFrame)');
            this.processFramesLegacy(this.videoElement);
        }
    }

    setupDynamicControls() {
        const cameraSelect = document.getElementById('cameraSelect');
        const resSelect = document.getElementById('resSelect');

        cameraSelect.onchange = async () => {
            if (!this.isStreaming) return;
            document.getElementById('status').innerText = '🔄 Changement de caméra...';
            const cameraMode = cameraSelect.value;
            await this.initStream(cameraMode, this.currentWidth, this.currentHeight);
        };

        resSelect.onchange = async () => {
            if (!this.isStreaming) return;
            document.getElementById('status').innerText = '🔄 Changement de résolution...';
            const resValue = resSelect.value;
            const [width, height] = resValue.split('x').map(Number);
            const cameraMode = cameraSelect.value;
            await this.initStream(cameraMode, width, height);
        };
    }

    stop() {
        this.isStreaming = false;

        if (this.currentStream) {
            this.currentStream.getTracks().forEach(track => track.stop());
        }

        if (this.encoder) {
            this.encoder.close();
            this.encoder = null;
        }

        if (this.socket) {
            this.socket.close();
            this.socket = null;
        }

        this.videoElement.srcObject = null;
        document.getElementById('status').innerText = '⏸️ Arrêté';

        const btn = document.getElementById('startBtn');
        btn.innerText = '🚀 DÉMARRER';
        btn.onclick = () => this.start();

        // Retirer les listeners
        document.getElementById('cameraSelect').onchange = null;
        document.getElementById('resSelect').onchange = null;
    }

    async processFrames(reader) {
        while (this.isStreaming) {
            try {
                const { value: frame, done } = await reader.read();
                if (done) break;

                // Keyframe toutes les secondes (60 frames à 60fps)
                const keyFrame = (this.frameCount % 60) === 0;

                if (this.encoder && this.encoder.state === 'configured') {
                    this.encoder.encode(frame, { keyFrame });
                }

                frame.close();

                this.frameCount++;

                // Mise à jour des stats toutes les 10 frames pour réduire la charge
                if (this.frameCount % 10 === 0) {
                    this.updateStats();
                }
            } catch (e) {
                console.error('Frame processing error:', e);
                break;
            }
        }
    }

    async processFramesLegacy(videoElement) {
        this.log('[LEGACY] Démarrage capture vidéo via videoElement');

        const captureFrame = () => {
            if (!this.isStreaming) return;

            try {
                const frame = new VideoFrame(videoElement, {
                    timestamp: performance.now() * 1000
                });

                // Forcer la première frame à être une keyframe
                const keyFrame = this.frameCount === 0 || (this.frameCount % 60) === 0;

                if (this.encoder && this.encoder.state === 'configured') {
                    this.encoder.encode(frame, { keyFrame });
                    this.frameCount++;

                    // Logger les keyframes
                    if (keyFrame) {
                        this.log(`[LEGACY] 🔑 Keyframe #${this.frameCount} générée`);
                    }

                    if (this.frameCount % 30 === 0) {
                        this.log(`[LEGACY] ${this.frameCount} frames encodées`);
                        document.getElementById('fps').innerText = `FPS: ${Math.round(60)}`;
                    }
                } else {
                    this.log(`[LEGACY] Encodeur état: ${this.encoder?.state || 'null'}`);
                }

                frame.close();
            } catch (err) {
                this.log(`[LEGACY] ❌ Erreur: ${err.message}`);
                console.error('[LEGACY] Erreur complète:', err);
            }

            requestAnimationFrame(captureFrame);
        };

        requestAnimationFrame(captureFrame);
    }

    async handleEncodedChunk(chunk) {
        // Compteur static pour éviter trop de logs
        if (!this.chunkCounter) this.chunkCounter = 0;
        this.chunkCounter++;

        if (this.chunkCounter % 30 === 0) {
            this.log(`[CHUNK] ${this.chunkCounter} chunks générés par l'encodeur`);
        }

        const length = chunk.byteLength;
        const packet = new Uint8Array(8 + length);

        // Header (8 bytes)
        packet[0] = 0x50; // 'P'
        packet[1] = 0x43; // 'C'
        packet[2] = chunk.type === 'key' ? 1 : 0;
        packet[3] = 0; // Flags

        // Payload length (u32 BE)
        new DataView(packet.buffer).setUint32(4, length, false);

        // Copy NAL data (ASYNC!)
        await chunk.copyTo(packet.subarray(8));

        if (this.socket && this.socket.readyState === WebSocket.OPEN) {
            this.socket.send(packet);

            // Logger les keyframes envoyées
            if (chunk.type === 'key') {
                this.log(`[WS] 🔑 Keyframe envoyée (${length} bytes)`);
            }
        }
    }

    updateStats() {
        if (this.fpsStatElement) {
            const now = performance.now();
            const elapsed = (now - this.startTime) / 1000;
            const fps = (this.frameCount / elapsed).toFixed(1);
            this.fpsStatElement.innerText = `FPS: ${fps} | Chunks: ${this.chunkCounter} | T: ${elapsed.toFixed(1)}s`;
        }
    }
}

const client = new PhoneCamUltimate();
document.getElementById('startBtn').onclick = () => client.start();

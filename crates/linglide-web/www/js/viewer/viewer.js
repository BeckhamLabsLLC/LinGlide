/**
 * LinGlide Enhanced Video Viewer
 *
 * WebCodecs H.264 decoder with statistics tracking.
 */

import { StatsTracker } from './stats.js';

/**
 * Video viewer options
 * @typedef {Object} ViewerOptions
 * @property {HTMLCanvasElement} canvas
 * @property {HTMLElement} statusElement
 * @property {HTMLElement} statusTextElement
 * @property {string} serverUrl
 * @property {string} [authToken]
 * @property {() => void} [onConnect]
 * @property {() => void} [onDisconnect]
 * @property {(error: string) => void} [onError]
 * @property {(stats: Object) => void} [onStats]
 */

/**
 * Enhanced video viewer class
 */
export class VideoViewer {
    /**
     * @param {ViewerOptions} options
     */
    constructor(options) {
        this.canvas = options.canvas;
        this.ctx = this.canvas.getContext('2d');
        this.statusElement = options.statusElement;
        this.statusTextElement = options.statusTextElement;
        this.serverUrl = options.serverUrl;
        this.authToken = options.authToken;

        // Callbacks
        this.onConnect = options.onConnect;
        this.onDisconnect = options.onDisconnect;
        this.onError = options.onError;
        this.onStats = options.onStats;

        // WebSocket
        this.ws = null;
        this.reconnectAttempts = 0;
        this.maxReconnectAttempts = 5;
        this.reconnectDelay = 2000;

        // Decoder state
        this.decoder = null;
        this.config = null;
        this.gotKeyframe = false;
        this.frameCount = 0;
        this.skippedFrames = 0;

        // Statistics
        this.stats = new StatsTracker();
        this.statsInterval = null;
    }

    /**
     * Connect to the video WebSocket
     */
    async connect() {
        this.setStatus('Connecting...');

        const protocol = this.serverUrl.startsWith('https') ? 'wss' : 'ws';
        const host = this.serverUrl.replace(/^https?:\/\//, '');
        let url = `${protocol}://${host}/ws/video`;

        if (this.authToken) {
            url += `?token=${encodeURIComponent(this.authToken)}`;
        }

        try {
            this.ws = new WebSocket(url);
            this.ws.binaryType = 'arraybuffer';

            this.ws.onopen = () => this.handleOpen();
            this.ws.onmessage = (event) => this.handleMessage(event);
            this.ws.onclose = () => this.handleClose();
            this.ws.onerror = (error) => this.handleError(error);
        } catch (error) {
            this.setStatus('Connection failed');
            this.onError?.(error.message);
        }
    }

    /**
     * Handle WebSocket open
     */
    handleOpen() {
        console.log('Video WebSocket connected');
        this.setStatus('Waiting for video...');
        this.reconnectAttempts = 0;

        // Start stats reporting
        this.startStatsReporting();
    }

    /**
     * Handle WebSocket message
     * @param {MessageEvent} event
     */
    handleMessage(event) {
        if (typeof event.data === 'string') {
            this.handleControlMessage(JSON.parse(event.data));
        } else {
            this.handleVideoData(new Uint8Array(event.data));
        }
    }

    /**
     * Handle control message
     * @param {Object} msg
     */
    handleControlMessage(msg) {
        switch (msg.type) {
            case 'Init':
                this.config = {
                    width: msg.width,
                    height: msg.height,
                    fps: msg.fps,
                    codec: msg.codec || 'avc1.64002a',
                    codecData: msg.codec_data ? this.base64ToArrayBuffer(msg.codec_data) : null
                };
                this.canvas.width = msg.width;
                this.canvas.height = msg.height;
                console.log('Video config:', this.config);
                this.initDecoder();
                break;

            case 'Ready':
                console.log('Server ready');
                break;

            case 'Ping':
                // Calculate latency and respond
                const serverTime = msg.timestamp;
                const now = Date.now();
                if (serverTime) {
                    this.stats.recordPing(now - serverTime);
                }

                if (this.ws?.readyState === WebSocket.OPEN) {
                    this.ws.send(JSON.stringify({ type: 'Pong', timestamp: msg.timestamp }));
                }
                break;

            case 'Error':
                console.error('Server error:', msg.message);
                this.setStatus(`Error: ${msg.message}`, true);
                this.onError?.(msg.message);
                break;
        }
    }

    /**
     * Convert base64 to ArrayBuffer
     * @param {string} base64
     * @returns {ArrayBuffer}
     */
    base64ToArrayBuffer(base64) {
        const binaryString = atob(base64);
        const bytes = new Uint8Array(binaryString.length);
        for (let i = 0; i < binaryString.length; i++) {
            bytes[i] = binaryString.charCodeAt(i);
        }
        return bytes.buffer;
    }

    /**
     * Initialize the video decoder
     */
    async initDecoder() {
        if (!('VideoDecoder' in window)) {
            this.setStatus('WebCodecs not supported. Use Chrome 94+', true);
            this.onError?.('WebCodecs not supported');
            return;
        }

        try {
            this.decoder = new VideoDecoder({
                output: (frame) => this.handleFrame(frame),
                error: (error) => {
                    console.error('Decoder error:', error);
                }
            });

            const decoderConfig = {
                codec: this.config.codec,
                codedWidth: this.config.width,
                codedHeight: this.config.height,
                optimizeForLatency: true
            };

            await this.decoder.configure(decoderConfig);
            console.log('Decoder initialized');
        } catch (error) {
            console.error('Failed to initialize decoder:', error);
            this.setStatus('Failed to initialize video decoder', true);
            this.onError?.(error.message);
        }
    }

    /**
     * Handle video data
     * @param {Uint8Array} data
     */
    handleVideoData(data) {
        if (!this.decoder || this.decoder.state !== 'configured') {
            return;
        }

        // Track bytes received
        this.stats.recordBytes(data.length);

        // Parse fMP4 and extract NAL units
        const nalUnits = this.parseMP4(data);

        for (const nal of nalUnits) {
            try {
                const chunkType = nal.isKeyframe ? 'key' : 'delta';

                // Wait for first keyframe
                if (!this.gotKeyframe) {
                    if (nal.isKeyframe) {
                        this.gotKeyframe = true;
                        console.log('Got first keyframe');
                    } else {
                        this.skippedFrames++;
                        continue;
                    }
                }

                const chunk = new EncodedVideoChunk({
                    type: chunkType,
                    timestamp: nal.timestamp,
                    data: nal.data
                });

                this.decoder.decode(chunk);
                this.frameCount++;
            } catch (error) {
                console.error('Decode error:', error);
            }
        }
    }

    /**
     * Parse MP4 container
     * @param {Uint8Array} data
     * @returns {Array}
     */
    parseMP4(data) {
        const nalUnits = [];
        let offset = 0;

        while (offset + 8 <= data.length) {
            const size = (data[offset] << 24) | (data[offset + 1] << 16) |
                         (data[offset + 2] << 8) | data[offset + 3];
            const type = String.fromCharCode(data[offset + 4], data[offset + 5],
                                            data[offset + 6], data[offset + 7]);

            if (size < 8 || offset + size > data.length) break;

            if (type === 'mdat') {
                const mdatData = data.slice(offset + 8, offset + size);
                const isKeyframe = this.isKeyframe(mdatData);

                nalUnits.push({
                    data: mdatData,
                    timestamp: performance.now() * 1000,
                    isKeyframe
                });
            }

            offset += size;
        }

        return nalUnits;
    }

    /**
     * Check if NAL contains keyframe
     * @param {Uint8Array} data
     * @returns {boolean}
     */
    isKeyframe(data) {
        for (let i = 0; i < data.length - 4; i++) {
            if (data[i] === 0 && data[i + 1] === 0 && data[i + 2] === 0 && data[i + 3] === 1) {
                const nalType = data[i + 4] & 0x1F;
                if (nalType === 5 || nalType === 7) return true;
            }
            if (data[i] === 0 && data[i + 1] === 0 && data[i + 2] === 1) {
                const nalType = data[i + 3] & 0x1F;
                if (nalType === 5 || nalType === 7) return true;
            }
        }
        return false;
    }

    /**
     * Handle decoded frame
     * @param {VideoFrame} frame
     */
    handleFrame(frame) {
        // Hide status on first frame
        if (this.statusElement && !this.statusElement.classList.contains('hidden')) {
            this.statusElement.classList.add('hidden');
            this.onConnect?.();
        }

        // Record frame for FPS tracking
        this.stats.recordFrame();

        // Draw frame
        this.ctx.drawImage(frame, 0, 0);
        frame.close();
    }

    /**
     * Handle WebSocket close
     */
    handleClose() {
        console.log('Video WebSocket closed');
        this.stopStatsReporting();
        this.onDisconnect?.();

        // Attempt reconnection
        if (this.reconnectAttempts < this.maxReconnectAttempts) {
            this.reconnectAttempts++;
            this.setStatus(`Reconnecting... (${this.reconnectAttempts}/${this.maxReconnectAttempts})`);

            setTimeout(() => {
                this.gotKeyframe = false;
                this.connect();
            }, this.reconnectDelay);
        } else {
            this.setStatus('Connection lost', true);
        }
    }

    /**
     * Handle WebSocket error
     * @param {Event} error
     */
    handleError(error) {
        console.error('Video WebSocket error:', error);
        this.setStatus('Connection error', true);
    }

    /**
     * Set status text
     * @param {string} text
     * @param {boolean} [isError]
     */
    setStatus(text, isError = false) {
        if (this.statusTextElement) {
            this.statusTextElement.textContent = text;
        }
        if (this.statusElement) {
            this.statusElement.classList.remove('hidden');
            this.statusElement.parentElement?.classList.toggle('viewer--error', isError);
        }
    }

    /**
     * Start stats reporting
     */
    startStatsReporting() {
        this.statsInterval = setInterval(() => {
            const stats = this.stats.getStats();
            this.onStats?.(stats);
        }, 1000);
    }

    /**
     * Stop stats reporting
     */
    stopStatsReporting() {
        if (this.statsInterval) {
            clearInterval(this.statsInterval);
            this.statsInterval = null;
        }
    }

    /**
     * Disconnect and clean up
     */
    disconnect() {
        this.stopStatsReporting();

        if (this.ws) {
            this.ws.close();
            this.ws = null;
        }

        if (this.decoder) {
            this.decoder.close();
            this.decoder = null;
        }
    }
}

export default VideoViewer;

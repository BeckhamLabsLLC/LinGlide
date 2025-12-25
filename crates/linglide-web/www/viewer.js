// LinGlide Video Viewer - WebCodecs H.264 decoder

class VideoViewer {
    constructor() {
        this.canvas = document.getElementById('display');
        this.ctx = this.canvas.getContext('2d');
        this.status = document.getElementById('status');
        this.statusText = document.getElementById('status-text');
        this.fullscreenBtn = document.getElementById('fullscreen-btn');

        this.ws = null;
        this.decoder = null;
        this.config = null;
        this.frameQueue = [];
        this.isProcessing = false;

        this.init();
    }

    init() {
        this.fullscreenBtn.addEventListener('click', () => this.toggleFullscreen());
        this.connect();
    }

    connect() {
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const url = `${protocol}//${window.location.host}/ws/video`;

        this.statusText.textContent = 'Connecting...';
        this.ws = new WebSocket(url);
        this.ws.binaryType = 'arraybuffer';

        this.ws.onopen = () => {
            console.log('Video WebSocket connected');
            this.statusText.textContent = 'Waiting for video...';
        };

        this.ws.onmessage = (event) => {
            if (typeof event.data === 'string') {
                this.handleControlMessage(JSON.parse(event.data));
            } else {
                this.handleVideoData(new Uint8Array(event.data));
            }
        };

        this.ws.onclose = () => {
            console.log('Video WebSocket closed');
            this.statusText.textContent = 'Disconnected. Reconnecting...';
            this.status.classList.remove('hidden');
            document.body.classList.add('error');
            setTimeout(() => this.connect(), 2000);
        };

        this.ws.onerror = (error) => {
            console.error('Video WebSocket error:', error);
            this.statusText.textContent = 'Connection error';
            document.body.classList.add('error');
        };
    }

    handleControlMessage(msg) {
        console.log('Control message:', msg);

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
                console.log('Codec:', this.config.codec, 'Has codecData:', !!this.config.codecData);
                this.initDecoder();
                break;

            case 'Ready':
                console.log('Server ready');
                break;

            case 'Ping':
                // Respond with pong
                if (this.ws.readyState === WebSocket.OPEN) {
                    this.ws.send(JSON.stringify({ type: 'Pong', timestamp: msg.timestamp }));
                }
                break;

            case 'Error':
                console.error('Server error:', msg.message);
                this.statusText.textContent = `Error: ${msg.message}`;
                document.body.classList.add('error');
                break;
        }
    }

    base64ToArrayBuffer(base64) {
        const binaryString = atob(base64);
        const bytes = new Uint8Array(binaryString.length);
        for (let i = 0; i < binaryString.length; i++) {
            bytes[i] = binaryString.charCodeAt(i);
        }
        return bytes.buffer;
    }

    async initDecoder() {
        if (!('VideoDecoder' in window)) {
            this.statusText.textContent = 'WebCodecs not supported. Use Chrome 94+';
            document.body.classList.add('error');
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

            // Note: Not using description because our NAL units are in Annex B format
            // The decoder will get SPS/PPS from the keyframes directly

            console.log('Configuring decoder with:', decoderConfig);
            await this.decoder.configure(decoderConfig);

            console.log('Decoder initialized');
        } catch (error) {
            console.error('Failed to initialize decoder:', error);
            this.statusText.textContent = 'Failed to initialize video decoder: ' + error.message;
            document.body.classList.add('error');
        }
    }

    handleVideoData(data) {
        if (!this.decoder || this.decoder.state !== 'configured') {
            console.log('Decoder not ready, state:', this.decoder?.state);
            return;
        }

        // Parse fMP4 and extract NAL units
        const nalUnits = this.parseMP4(data);
        if (nalUnits.length === 0) {
            console.log('No NAL units in segment, size:', data.length);
        }

        for (const nal of nalUnits) {
            try {
                const chunkType = nal.isKeyframe ? 'key' : 'delta';

                // Wait for first keyframe before decoding
                if (!this.gotKeyframe) {
                    if (nal.isKeyframe) {
                        this.gotKeyframe = true;
                        console.log('Got first keyframe, size:', nal.data.length);
                    } else {
                        // Skip P-frames until we get a keyframe
                        this.skippedFrames = (this.skippedFrames || 0) + 1;
                        if (this.skippedFrames <= 5 || this.skippedFrames % 100 === 0) {
                            console.log('Waiting for keyframe, skipped:', this.skippedFrames);
                        }
                        continue;
                    }
                }

                if (nal.isKeyframe || this.frameCount < 5) {
                    console.log('Decoding', chunkType, 'frame, size:', nal.data.length);
                }

                const chunk = new EncodedVideoChunk({
                    type: chunkType,
                    timestamp: nal.timestamp,
                    data: nal.data
                });

                this.decoder.decode(chunk);
                this.frameCount = (this.frameCount || 0) + 1;
            } catch (error) {
                console.error('Decode error:', error);
            }
        }
    }

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
                // Extract video data from mdat
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

    isKeyframe(data) {
        // Check for SPS (type 7) or IDR NAL unit (type 5)
        for (let i = 0; i < data.length - 4; i++) {
            if (data[i] === 0 && data[i + 1] === 0 && data[i + 2] === 0 && data[i + 3] === 1) {
                const nalType = data[i + 4] & 0x1F;
                if (nalType === 5 || nalType === 7) return true;
            }
            // Also check for 3-byte start code
            if (data[i] === 0 && data[i + 1] === 0 && data[i + 2] === 1) {
                const nalType = data[i + 3] & 0x1F;
                if (nalType === 5 || nalType === 7) return true;
            }
        }
        return false;
    }

    handleFrame(frame) {
        // Hide status on first frame
        if (!this.status.classList.contains('hidden')) {
            this.status.classList.add('hidden');
            document.body.classList.remove('error');
            console.log('First frame received:', frame.codedWidth, 'x', frame.codedHeight);
        }

        // Log occasional frame info
        this.renderedFrames = (this.renderedFrames || 0) + 1;
        if (this.renderedFrames <= 3 || this.renderedFrames % 60 === 0) {
            console.log('Frame', this.renderedFrames, ':', frame.codedWidth, 'x', frame.codedHeight, 'format:', frame.format);
        }

        // Draw frame to canvas
        this.ctx.drawImage(frame, 0, 0);
        frame.close();
    }

    toggleFullscreen() {
        if (!document.fullscreenElement) {
            document.documentElement.requestFullscreen().catch(err => {
                console.error('Fullscreen error:', err);
            });
        } else {
            document.exitFullscreen();
        }
    }
}

// Initialize viewer when DOM is ready
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => new VideoViewer());
} else {
    new VideoViewer();
}

// Export for input.js
window.LinGlideViewer = VideoViewer;

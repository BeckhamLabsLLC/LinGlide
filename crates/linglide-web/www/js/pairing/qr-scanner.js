/**
 * LinGlide QR Scanner Component
 *
 * Camera-based QR code scanning using jsQR library.
 * Requires jsQR to be loaded (via CDN or bundled).
 *
 * Enhanced with image preprocessing for better detection on
 * low-brightness laptop screens and varying lighting conditions.
 */

/**
 * QR Scanner component
 */
export class QrScanner {
    /**
     * @param {HTMLElement} container
     * @param {Object} options
     * @param {(data: string) => void} options.onScan
     * @param {() => void} options.onClose
     * @param {(error: string) => void} [options.onError]
     */
    constructor(container, options) {
        this.container = container;
        this.options = options;
        this.video = null;
        this.canvas = null;
        this.ctx = null;
        this.processingCanvas = null;
        this.processingCtx = null;
        this.stream = null;
        this.animationFrame = null;
        this.isScanning = false;
        this.scanAttempt = 0;

        this.render();
    }

    /**
     * Render the scanner UI
     */
    render() {
        this.container.innerHTML = `
            <div class="qr-scanner">
                <div class="qr-scanner__header">
                    <h2 class="qr-scanner__title">Scan QR Code</h2>
                    <button class="qr-scanner__close" aria-label="Close">
                        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M18 6L6 18M6 6l12 12"/>
                        </svg>
                    </button>
                </div>

                <div class="qr-scanner__video-container">
                    <video class="qr-scanner__video" playsinline autoplay muted></video>
                    <canvas class="qr-scanner__canvas" style="display: none;"></canvas>

                    <div class="qr-scanner__overlay">
                        <div class="qr-scanner__frame"></div>
                        <div class="qr-scanner__corners"></div>
                    </div>

                    <div class="qr-scanner__hint">Point camera at QR code</div>
                </div>
            </div>
        `;

        // Store references
        this.closeBtn = this.container.querySelector('.qr-scanner__close');
        this.video = this.container.querySelector('.qr-scanner__video');
        this.canvas = this.container.querySelector('.qr-scanner__canvas');
        this.ctx = this.canvas.getContext('2d', { willReadFrequently: true });
        this.hint = this.container.querySelector('.qr-scanner__hint');

        this.bindEvents();
    }

    /**
     * Bind event listeners
     */
    bindEvents() {
        this.closeBtn.addEventListener('click', () => {
            this.stop();
            this.options.onClose?.();
        });
    }

    /**
     * Start the camera and scanning
     */
    async start() {
        try {
            // Check for camera permission with optimized constraints for QR scanning
            const constraints = {
                video: {
                    facingMode: 'environment', // Prefer back camera
                    width: { ideal: 1920, min: 640 },
                    height: { ideal: 1080, min: 480 },
                    // Request higher exposure for dim screens
                    advanced: [
                        { exposureMode: 'continuous' },
                        { focusMode: 'continuous' }
                    ]
                }
            };

            this.stream = await navigator.mediaDevices.getUserMedia(constraints);
            this.video.srcObject = this.stream;

            await new Promise((resolve, reject) => {
                this.video.onloadedmetadata = resolve;
                this.video.onerror = reject;
            });

            await this.video.play();

            // Set canvas size to match video
            this.canvas.width = this.video.videoWidth;
            this.canvas.height = this.video.videoHeight;

            // Create processing canvas for image enhancement
            this.processingCanvas = document.createElement('canvas');
            this.processingCanvas.width = this.video.videoWidth;
            this.processingCanvas.height = this.video.videoHeight;
            this.processingCtx = this.processingCanvas.getContext('2d', { willReadFrequently: true });

            this.isScanning = true;
            this.scan();
        } catch (error) {
            console.error('Failed to start camera:', error);
            this.hint.textContent = 'Camera access denied';
            this.options.onError?.('Camera access denied. Please allow camera permissions.');
        }
    }

    /**
     * Scan for QR codes with enhanced detection
     */
    scan() {
        if (!this.isScanning) return;

        // Draw current video frame to canvas
        this.ctx.drawImage(this.video, 0, 0, this.canvas.width, this.canvas.height);

        // Get original image data
        const imageData = this.ctx.getImageData(0, 0, this.canvas.width, this.canvas.height);

        // Check if jsQR is available
        if (typeof jsQR !== 'undefined') {
            // Try multiple detection strategies for better results on dim screens
            const code = this.tryMultipleDetectionStrategies(imageData);

            if (code && code.data) {
                // QR code found!
                this.onCodeDetected(code.data);
                return;
            }
        } else {
            console.warn('jsQR library not loaded');
        }

        this.scanAttempt++;

        // Continue scanning
        this.animationFrame = requestAnimationFrame(() => this.scan());
    }

    /**
     * Try multiple detection strategies to find QR code
     * @param {ImageData} originalImageData
     * @returns {Object|null} Detected QR code or null
     */
    tryMultipleDetectionStrategies(originalImageData) {
        // Strategy 1: Try with both normal and inverted (best for screen QR codes)
        let code = jsQR(originalImageData.data, originalImageData.width, originalImageData.height, {
            inversionAttempts: 'attemptBoth'
        });
        if (code) return code;

        // Strategy 2: Enhanced contrast for dim/low-brightness screens
        const enhancedData = this.enhanceContrast(originalImageData, 1.5, 10);
        code = jsQR(enhancedData.data, enhancedData.width, enhancedData.height, {
            inversionAttempts: 'attemptBoth'
        });
        if (code) return code;

        // Strategy 3: Even higher contrast for very dim screens (every 3rd frame to save CPU)
        if (this.scanAttempt % 3 === 0) {
            const highContrastData = this.enhanceContrast(originalImageData, 2.0, 20);
            code = jsQR(highContrastData.data, highContrastData.width, highContrastData.height, {
                inversionAttempts: 'attemptBoth'
            });
            if (code) return code;
        }

        // Strategy 4: Adaptive threshold for challenging lighting (every 5th frame)
        if (this.scanAttempt % 5 === 0) {
            const thresholdData = this.applyAdaptiveThreshold(originalImageData);
            code = jsQR(thresholdData.data, thresholdData.width, thresholdData.height, {
                inversionAttempts: 'attemptBoth'
            });
            if (code) return code;
        }

        return null;
    }

    /**
     * Enhance image contrast for better QR detection
     * @param {ImageData} imageData
     * @param {number} contrast - Contrast multiplier (1.0 = no change)
     * @param {number} brightness - Brightness adjustment (-255 to 255)
     * @returns {ImageData}
     */
    enhanceContrast(imageData, contrast, brightness) {
        const data = new Uint8ClampedArray(imageData.data);
        const factor = (259 * (contrast * 128 + 255)) / (255 * (259 - contrast * 128));

        for (let i = 0; i < data.length; i += 4) {
            // Apply contrast and brightness to RGB channels
            data[i] = this.clamp(factor * (data[i] - 128) + 128 + brightness);     // R
            data[i + 1] = this.clamp(factor * (data[i + 1] - 128) + 128 + brightness); // G
            data[i + 2] = this.clamp(factor * (data[i + 2] - 128) + 128 + brightness); // B
            // Alpha stays the same
        }

        return new ImageData(data, imageData.width, imageData.height);
    }

    /**
     * Apply adaptive threshold for binary QR detection
     * @param {ImageData} imageData
     * @returns {ImageData}
     */
    applyAdaptiveThreshold(imageData) {
        const data = new Uint8ClampedArray(imageData.data);
        const width = imageData.width;
        const height = imageData.height;

        // Convert to grayscale first
        const gray = new Uint8Array(width * height);
        for (let i = 0; i < data.length; i += 4) {
            const idx = i / 4;
            gray[idx] = Math.round(0.299 * data[i] + 0.587 * data[i + 1] + 0.114 * data[i + 2]);
        }

        // Calculate mean for simple threshold
        let sum = 0;
        for (let i = 0; i < gray.length; i++) {
            sum += gray[i];
        }
        const mean = sum / gray.length;
        const threshold = mean * 0.9; // Slightly below mean for better dark detection

        // Apply threshold
        for (let i = 0; i < gray.length; i++) {
            const value = gray[i] > threshold ? 255 : 0;
            const dataIdx = i * 4;
            data[dataIdx] = value;
            data[dataIdx + 1] = value;
            data[dataIdx + 2] = value;
        }

        return new ImageData(data, width, height);
    }

    /**
     * Clamp value to 0-255 range
     * @param {number} value
     * @returns {number}
     */
    clamp(value) {
        return Math.max(0, Math.min(255, Math.round(value)));
    }

    /**
     * Handle detected QR code
     * @param {string} data
     */
    onCodeDetected(data) {
        console.log('QR code detected:', data);

        // Stop scanning
        this.stop();

        // Notify
        this.options.onScan?.(data);
    }

    /**
     * Stop the camera and scanning
     */
    stop() {
        this.isScanning = false;

        if (this.animationFrame) {
            cancelAnimationFrame(this.animationFrame);
            this.animationFrame = null;
        }

        if (this.stream) {
            this.stream.getTracks().forEach(track => track.stop());
            this.stream = null;
        }

        if (this.video) {
            this.video.srcObject = null;
        }
    }

    /**
     * Clean up
     */
    destroy() {
        this.stop();
        this.container.innerHTML = '';
    }
}

export default QrScanner;

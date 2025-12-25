/**
 * LinGlide Statistics Tracker
 *
 * Tracks latency, FPS, and bitrate statistics.
 */

/**
 * Rolling average calculator
 */
class RollingAverage {
    /**
     * @param {number} windowSize - Number of samples to average
     */
    constructor(windowSize = 30) {
        this.windowSize = windowSize;
        this.samples = [];
        this.sum = 0;
    }

    /**
     * Add a sample
     * @param {number} value
     */
    add(value) {
        this.samples.push(value);
        this.sum += value;

        if (this.samples.length > this.windowSize) {
            this.sum -= this.samples.shift();
        }
    }

    /**
     * Get the average
     * @returns {number}
     */
    get average() {
        if (this.samples.length === 0) return 0;
        return this.sum / this.samples.length;
    }

    /**
     * Get the latest value
     * @returns {number}
     */
    get latest() {
        if (this.samples.length === 0) return 0;
        return this.samples[this.samples.length - 1];
    }

    /**
     * Reset the samples
     */
    reset() {
        this.samples = [];
        this.sum = 0;
    }
}

/**
 * Statistics tracker
 */
export class StatsTracker {
    constructor() {
        // Latency tracking (from Ping/Pong)
        this.latency = new RollingAverage(10);

        // FPS tracking
        this.frameTimes = [];
        this.lastFpsCalculation = performance.now();
        this.currentFps = 0;

        // Bitrate tracking
        this.bytesReceived = 0;
        this.lastBitrateCalculation = performance.now();
        this.currentBitrate = 0;

        // Start periodic calculations
        this.calculationInterval = setInterval(() => this.calculate(), 1000);
    }

    /**
     * Record a ping/pong latency measurement
     * @param {number} ms - Round-trip time in milliseconds
     */
    recordPing(ms) {
        this.latency.add(Math.max(0, ms));
    }

    /**
     * Record a decoded frame
     */
    recordFrame() {
        const now = performance.now();
        this.frameTimes.push(now);

        // Keep only last second of frames
        const cutoff = now - 1000;
        while (this.frameTimes.length > 0 && this.frameTimes[0] < cutoff) {
            this.frameTimes.shift();
        }
    }

    /**
     * Record bytes received
     * @param {number} bytes
     */
    recordBytes(bytes) {
        this.bytesReceived += bytes;
    }

    /**
     * Calculate current stats
     */
    calculate() {
        const now = performance.now();

        // Calculate FPS
        const frameCount = this.frameTimes.length;
        this.currentFps = frameCount;

        // Calculate bitrate (bits per second)
        const elapsed = (now - this.lastBitrateCalculation) / 1000;
        if (elapsed > 0) {
            this.currentBitrate = (this.bytesReceived * 8) / elapsed;
            this.bytesReceived = 0;
            this.lastBitrateCalculation = now;
        }
    }

    /**
     * Get current statistics
     * @returns {Object}
     */
    getStats() {
        return {
            // Latency in milliseconds
            latency: Math.round(this.latency.average),
            latencyLatest: Math.round(this.latency.latest),

            // Frames per second
            fps: Math.round(this.currentFps),

            // Bitrate in Mbps
            bitrate: (this.currentBitrate / 1000000).toFixed(2),
            bitrateRaw: this.currentBitrate
        };
    }

    /**
     * Get formatted stats string
     * @returns {string}
     */
    getFormattedStats() {
        const stats = this.getStats();
        return `${stats.latency}ms | ${stats.fps} FPS | ${stats.bitrate} Mbps`;
    }

    /**
     * Reset all statistics
     */
    reset() {
        this.latency.reset();
        this.frameTimes = [];
        this.bytesReceived = 0;
        this.currentFps = 0;
        this.currentBitrate = 0;
    }

    /**
     * Clean up
     */
    destroy() {
        if (this.calculationInterval) {
            clearInterval(this.calculationInterval);
            this.calculationInterval = null;
        }
    }
}

export default StatsTracker;

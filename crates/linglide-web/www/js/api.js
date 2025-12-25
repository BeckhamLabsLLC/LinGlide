/**
 * LinGlide API Client
 *
 * REST API client for server communication.
 */

/**
 * API client for a LinGlide server
 */
export class ApiClient {
    /**
     * @param {string} baseUrl - Server base URL (e.g., https://192.168.1.100:8443)
     */
    constructor(baseUrl) {
        this.baseUrl = baseUrl.replace(/\/$/, ''); // Remove trailing slash
    }

    /**
     * Make an API request
     * @param {string} endpoint
     * @param {RequestInit} options
     * @returns {Promise<any>}
     */
    async request(endpoint, options = {}) {
        const url = `${this.baseUrl}${endpoint}`;

        const defaultOptions = {
            headers: {
                'Content-Type': 'application/json',
            },
        };

        const response = await fetch(url, { ...defaultOptions, ...options });

        if (!response.ok) {
            const errorText = await response.text();
            throw new ApiError(response.status, errorText || response.statusText);
        }

        // Handle empty responses
        const text = await response.text();
        if (!text) return null;

        try {
            return JSON.parse(text);
        } catch {
            return text;
        }
    }

    // ========================================================================
    // Server Info
    // ========================================================================

    /**
     * Get server information
     * @returns {Promise<ServerInfo>}
     */
    async getServerInfo() {
        return this.request('/api/info');
    }

    /**
     * Get discovery information
     * @returns {Promise<DiscoveryInfo>}
     */
    async getDiscoveryInfo() {
        return this.request('/api/discovery');
    }

    // ========================================================================
    // Pairing
    // ========================================================================

    /**
     * Start a new pairing session
     * @returns {Promise<PairingStartResponse>}
     */
    async startPairing() {
        return this.request('/api/pair/start', { method: 'POST' });
    }

    /**
     * Verify a pairing PIN
     * @param {string} sessionId
     * @param {string} pin
     * @param {string} deviceName
     * @param {string} [deviceType]
     * @returns {Promise<PairingVerifyResponse>}
     */
    async verifyPin(sessionId, pin, deviceName, deviceType = 'browser') {
        return this.request('/api/pair/verify', {
            method: 'POST',
            body: JSON.stringify({
                session_id: sessionId,
                pin,
                device_name: deviceName,
                device_type: deviceType
            })
        });
    }

    /**
     * Verify PIN directly without session (for persistent PIN)
     * This is used when users navigate directly to the server URL and enter the PIN.
     * @param {string} pin
     * @param {string} deviceName
     * @param {string} [deviceType]
     * @returns {Promise<PairingVerifyResponse>}
     */
    async verifyPinDirect(pin, deviceName, deviceType = 'browser') {
        return this.request('/api/pair/verify-direct', {
            method: 'POST',
            body: JSON.stringify({
                pin,
                device_name: deviceName,
                device_type: deviceType
            })
        });
    }

    /**
     * Get pairing session status
     * @param {string} sessionId
     * @returns {Promise<PairingStatus>}
     */
    async getPairingStatus(sessionId) {
        return this.request(`/api/pair/status?session_id=${encodeURIComponent(sessionId)}`);
    }

    /**
     * Get QR code image URL for a session
     * @param {string} sessionId
     * @param {number} [size=200]
     * @returns {string}
     */
    getQrCodeUrl(sessionId, size = 200) {
        return `${this.baseUrl}/api/pair/qr?session_id=${encodeURIComponent(sessionId)}&size=${size}`;
    }

    // ========================================================================
    // Devices
    // ========================================================================

    /**
     * List all paired devices
     * @returns {Promise<DeviceInfo[]>}
     */
    async listDevices() {
        return this.request('/api/devices');
    }

    /**
     * Revoke a paired device
     * @param {string} deviceId
     * @returns {Promise<void>}
     */
    async revokeDevice(deviceId) {
        return this.request(`/api/devices/${encodeURIComponent(deviceId)}`, {
            method: 'DELETE'
        });
    }

    // ========================================================================
    // WebSocket URLs
    // ========================================================================

    /**
     * Get video WebSocket URL
     * @param {string} [token] - Auth token
     * @returns {string}
     */
    getVideoWsUrl(token) {
        const protocol = this.baseUrl.startsWith('https') ? 'wss' : 'ws';
        const host = this.baseUrl.replace(/^https?:\/\//, '');
        let url = `${protocol}://${host}/ws/video`;
        if (token) {
            url += `?token=${encodeURIComponent(token)}`;
        }
        return url;
    }

    /**
     * Get input WebSocket URL
     * @param {string} [token] - Auth token
     * @returns {string}
     */
    getInputWsUrl(token) {
        const protocol = this.baseUrl.startsWith('https') ? 'wss' : 'ws';
        const host = this.baseUrl.replace(/^https?:\/\//, '');
        let url = `${protocol}://${host}/ws/input`;
        if (token) {
            url += `?token=${encodeURIComponent(token)}`;
        }
        return url;
    }
}

/**
 * API Error class
 */
export class ApiError extends Error {
    /**
     * @param {number} status
     * @param {string} message
     */
    constructor(status, message) {
        super(message);
        this.name = 'ApiError';
        this.status = status;
    }
}

// ============================================================================
// Type Definitions (for documentation)
// ============================================================================

/**
 * @typedef {Object} ServerInfo
 * @property {string} version
 * @property {number} width
 * @property {number} height
 * @property {number} fps
 * @property {boolean} auth_required
 * @property {number} paired_devices
 * @property {string | null} cert_fingerprint
 */

/**
 * @typedef {Object} DiscoveryInfo
 * @property {string} service_type
 * @property {string} instance_name
 * @property {number} port
 * @property {string | null} fingerprint
 * @property {string[]} addresses
 * @property {string} version
 */

/**
 * @typedef {Object} PairingStartResponse
 * @property {string} session_id
 * @property {string} pin
 * @property {number} expires_in
 */

/**
 * @typedef {Object} PairingVerifyResponse
 * @property {string} device_id
 * @property {string} token
 */

/**
 * @typedef {Object} PairingStatus
 * @property {boolean} valid
 * @property {string | null} pin
 * @property {number} expires_in
 */

/**
 * @typedef {Object} DeviceInfo
 * @property {string} id
 * @property {string} name
 * @property {string} device_type
 * @property {string} created_at
 * @property {string | null} last_seen
 */

export default ApiClient;

/**
 * LinGlide Pairing Controller
 *
 * Manages the device pairing flow including QR scanning, PIN entry,
 * and server discovery.
 */

import { appState, actions, AppView } from '../state.js';
import * as storage from '../storage.js';
import { ApiClient, ApiError } from '../api.js';

/**
 * Pairing controller class
 */
export class PairingController {
    constructor() {
        this.api = null;
        this.pairingSession = null;
        this.qrScanner = null;
    }

    /**
     * Initialize pairing controller
     */
    async init() {
        // Check if already paired
        const credentials = storage.getCredentials();
        const lastServer = storage.getLastServer();

        if (credentials && lastServer) {
            // Try to auto-connect with stored credentials
            appState.set({
                isPaired: true,
                deviceId: credentials.deviceId,
                authToken: credentials.authToken,
                serverUrl: lastServer.url,
                fingerprint: lastServer.fingerprint
            });

            // Validate the token is still valid
            if (await this.validateStoredCredentials(lastServer.url, credentials.authToken)) {
                actions.setView(AppView.CONNECTING);
                return true;
            }

            // Token invalid, clear and show pairing
            storage.clearCredentials();
        }

        // Show pairing landing
        actions.setView(AppView.PAIRING);
        return false;
    }

    /**
     * Validate stored credentials with the server
     * @param {string} serverUrl
     * @param {string} token
     * @returns {Promise<boolean>}
     */
    async validateStoredCredentials(serverUrl, token) {
        try {
            const api = new ApiClient(serverUrl);
            const info = await api.getServerInfo();

            // If auth is required and we have a token, it should work
            if (info.auth_required) {
                // We'll validate the token when connecting to WebSocket
                return true;
            }

            return true;
        } catch (error) {
            console.warn('Failed to validate stored credentials:', error);
            return false;
        }
    }

    /**
     * Start QR code scanning
     */
    async startQrScanning() {
        actions.setView(AppView.QR_SCANNER);
        // QR scanner will be initialized by qr-scanner.js
    }

    /**
     * Handle scanned QR code data
     * @param {string} data - QR code content
     */
    async handleQrScanned(data) {
        try {
            const parsed = this.parseQrData(data);

            if (!parsed) {
                throw new Error('Invalid QR code format');
            }

            // Set server info
            actions.setServer(parsed.url, parsed.fingerprint);

            // If we have PIN, verify directly
            if (parsed.pin && parsed.sessionId) {
                await this.verifyPin(parsed.url, parsed.sessionId, parsed.pin);
            } else {
                // Show PIN entry
                appState.set({
                    serverUrl: parsed.url,
                    fingerprint: parsed.fingerprint
                });
                actions.setView(AppView.PIN_ENTRY);
            }
        } catch (error) {
            console.error('QR scan error:', error);
            actions.setError(error.message || 'Failed to process QR code');
        }
    }

    /**
     * Parse QR code data
     * @param {string} data
     * @returns {{ url: string, pin?: string, sessionId?: string, fingerprint?: string, version?: string } | null}
     */
    parseQrData(data) {
        try {
            // Expected format: linglide://pair?url=...&pin=...&session=...&fp=...&v=...
            if (!data.startsWith('linglide://pair')) {
                return null;
            }

            const url = new URL(data.replace('linglide://', 'https://'));
            const params = url.searchParams;

            const serverUrl = params.get('url');
            if (!serverUrl) return null;

            return {
                url: decodeURIComponent(serverUrl),
                pin: params.get('pin'),
                sessionId: params.get('session'),
                fingerprint: params.get('fp'),
                version: params.get('v')
            };
        } catch (error) {
            console.error('Failed to parse QR data:', error);
            return null;
        }
    }

    /**
     * Show PIN entry view
     */
    showPinEntry() {
        actions.setView(AppView.PIN_ENTRY);
    }

    /**
     * Show manual server entry view
     */
    showServerEntry() {
        actions.setView(AppView.SERVER_ENTRY);
    }

    /**
     * Connect to a server manually
     * @param {string} serverUrl
     */
    async connectToServer(serverUrl) {
        try {
            // Normalize URL
            if (!serverUrl.startsWith('http')) {
                serverUrl = 'https://' + serverUrl;
            }

            const api = new ApiClient(serverUrl);

            // Get server info to validate it's a LinGlide server
            const info = await api.getServerInfo();

            if (info.auth_required) {
                // Need to pair - start pairing session
                const session = await api.startPairing();
                this.pairingSession = {
                    sessionId: session.session_id,
                    pin: session.pin,
                    expiresIn: session.expires_in,
                    startTime: Date.now()
                };

                appState.set({
                    serverUrl,
                    fingerprint: info.cert_fingerprint
                });

                actions.setView(AppView.PIN_ENTRY);
            } else {
                // No auth required, connect directly
                actions.setServer(serverUrl, info.cert_fingerprint);
                storage.setLastServer(serverUrl, info.cert_fingerprint);
                actions.setView(AppView.CONNECTING);
            }
        } catch (error) {
            console.error('Failed to connect to server:', error);
            if (error instanceof ApiError) {
                actions.setError(`Server error: ${error.message}`);
            } else {
                actions.setError('Failed to connect to server. Check the URL and try again.');
            }
        }
    }

    /**
     * Verify a pairing PIN (session-based, for QR code flow)
     * @param {string} serverUrl
     * @param {string} sessionId
     * @param {string} pin
     */
    async verifyPin(serverUrl, sessionId, pin) {
        try {
            const api = new ApiClient(serverUrl);

            // Get device name
            const deviceName = this.getDeviceName();
            const deviceType = this.getDeviceType();

            const result = await api.verifyPin(sessionId, pin, deviceName, deviceType);

            // Store credentials
            storage.setCredentials(result.device_id, result.token);

            // Get fingerprint from server info
            const info = await api.getServerInfo();
            storage.setLastServer(serverUrl, info.cert_fingerprint);

            // Update state
            actions.setPaired(result.device_id, result.token);
            actions.setServer(serverUrl, info.cert_fingerprint);
            actions.setView(AppView.CONNECTING);

            return true;
        } catch (error) {
            console.error('PIN verification failed:', error);
            if (error instanceof ApiError && error.status === 401) {
                throw new Error('Invalid or expired PIN');
            }
            throw error;
        }
    }

    /**
     * Connect to a server and verify PIN directly (for persistent PIN flow)
     * This is used when users navigate directly to the server URL and enter the PIN.
     * @param {string} serverUrl
     * @param {string} pin
     */
    async connectAndVerifyPin(serverUrl, pin) {
        try {
            // Normalize URL
            if (!serverUrl.startsWith('http')) {
                serverUrl = 'https://' + serverUrl;
            }

            const api = new ApiClient(serverUrl);

            // Verify server is reachable
            const info = await api.getServerInfo();

            if (info.auth_required) {
                // Use direct PIN verification (no session needed)
                const deviceName = this.getDeviceName();
                const deviceType = this.getDeviceType();

                const result = await api.verifyPinDirect(pin, deviceName, deviceType);

                // Store credentials
                storage.setCredentials(result.device_id, result.token);
                storage.setLastServer(serverUrl, info.cert_fingerprint);

                // Update state
                actions.setPaired(result.device_id, result.token);
                actions.setServer(serverUrl, info.cert_fingerprint);
                actions.setView(AppView.CONNECTING);

                return true;
            } else {
                // No auth required, connect directly
                actions.setServer(serverUrl, info.cert_fingerprint);
                storage.setLastServer(serverUrl, info.cert_fingerprint);
                actions.setView(AppView.CONNECTING);
                return true;
            }
        } catch (error) {
            console.error('PIN verification failed:', error);
            if (error instanceof ApiError && error.status === 401) {
                throw new Error('Invalid PIN');
            }
            throw error;
        }
    }

    /**
     * Get device name for pairing
     * @returns {string}
     */
    getDeviceName() {
        // Try to get a meaningful device name
        const userAgent = navigator.userAgent;

        if (/iPad/.test(userAgent)) return 'iPad';
        if (/iPhone/.test(userAgent)) return 'iPhone';
        if (/Android.*Mobile/.test(userAgent)) return 'Android Phone';
        if (/Android/.test(userAgent)) return 'Android Tablet';
        if (/Macintosh/.test(userAgent)) return 'Mac';
        if (/Windows/.test(userAgent)) return 'Windows PC';
        if (/Linux/.test(userAgent)) return 'Linux';

        return 'Web Browser';
    }

    /**
     * Get device type for pairing
     * @returns {string}
     */
    getDeviceType() {
        const userAgent = navigator.userAgent;

        if (/iPad|iPhone/.test(userAgent)) return 'ios';
        if (/Android/.test(userAgent)) return 'android';
        return 'browser';
    }

    /**
     * Cancel pairing and return to landing
     */
    cancel() {
        this.pairingSession = null;
        actions.setView(AppView.PAIRING);
    }

    /**
     * Disconnect and unpair
     */
    disconnect() {
        storage.clearCredentials();
        storage.clearServer();
        actions.reset();
        actions.setView(AppView.PAIRING);
    }
}

// Singleton instance
export const pairingController = new PairingController();

export default pairingController;

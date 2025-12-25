/**
 * LinGlide Storage Manager
 *
 * Handles persistent storage of device credentials, server history,
 * and user preferences using localStorage.
 */

const STORAGE_PREFIX = 'linglide_';

const KEYS = {
    DEVICE_ID: 'device_id',
    AUTH_TOKEN: 'auth_token',
    SERVER_URL: 'server_url',
    SERVER_FINGERPRINT: 'server_fingerprint',
    SERVER_HISTORY: 'server_history',
    PREFERENCES: 'preferences'
};

/**
 * Get a value from localStorage
 * @param {string} key
 * @returns {any}
 */
function get(key) {
    try {
        const value = localStorage.getItem(STORAGE_PREFIX + key);
        if (value === null) return null;
        return JSON.parse(value);
    } catch (error) {
        console.warn(`Failed to read ${key} from storage:`, error);
        return null;
    }
}

/**
 * Set a value in localStorage
 * @param {string} key
 * @param {any} value
 */
function set(key, value) {
    try {
        localStorage.setItem(STORAGE_PREFIX + key, JSON.stringify(value));
    } catch (error) {
        console.warn(`Failed to write ${key} to storage:`, error);
    }
}

/**
 * Remove a value from localStorage
 * @param {string} key
 */
function remove(key) {
    try {
        localStorage.removeItem(STORAGE_PREFIX + key);
    } catch (error) {
        console.warn(`Failed to remove ${key} from storage:`, error);
    }
}

// ============================================================================
// Device Credentials
// ============================================================================

/**
 * Get stored device credentials
 * @returns {{ deviceId: string, authToken: string } | null}
 */
export function getCredentials() {
    const deviceId = get(KEYS.DEVICE_ID);
    const authToken = get(KEYS.AUTH_TOKEN);

    if (deviceId && authToken) {
        return { deviceId, authToken };
    }
    return null;
}

/**
 * Store device credentials after successful pairing
 * @param {string} deviceId
 * @param {string} authToken
 */
export function setCredentials(deviceId, authToken) {
    set(KEYS.DEVICE_ID, deviceId);
    set(KEYS.AUTH_TOKEN, authToken);
}

/**
 * Clear stored device credentials
 */
export function clearCredentials() {
    remove(KEYS.DEVICE_ID);
    remove(KEYS.AUTH_TOKEN);
}

// ============================================================================
// Server Connection
// ============================================================================

/**
 * Get last connected server info
 * @returns {{ url: string, fingerprint: string | null } | null}
 */
export function getLastServer() {
    const url = get(KEYS.SERVER_URL);
    if (!url) return null;

    return {
        url,
        fingerprint: get(KEYS.SERVER_FINGERPRINT)
    };
}

/**
 * Store server connection info
 * @param {string} url
 * @param {string | null} fingerprint
 */
export function setLastServer(url, fingerprint = null) {
    set(KEYS.SERVER_URL, url);
    if (fingerprint) {
        set(KEYS.SERVER_FINGERPRINT, fingerprint);
    } else {
        remove(KEYS.SERVER_FINGERPRINT);
    }

    // Also add to history
    addToHistory(url, fingerprint);
}

/**
 * Clear server connection info
 */
export function clearServer() {
    remove(KEYS.SERVER_URL);
    remove(KEYS.SERVER_FINGERPRINT);
}

// ============================================================================
// Server History
// ============================================================================

const MAX_HISTORY = 5;

/**
 * @typedef {Object} ServerHistoryEntry
 * @property {string} url
 * @property {string | null} fingerprint
 * @property {number} lastUsed - Timestamp
 */

/**
 * Get server connection history
 * @returns {ServerHistoryEntry[]}
 */
export function getServerHistory() {
    const history = get(KEYS.SERVER_HISTORY);
    return Array.isArray(history) ? history : [];
}

/**
 * Add a server to history
 * @param {string} url
 * @param {string | null} fingerprint
 */
export function addToHistory(url, fingerprint = null) {
    let history = getServerHistory();

    // Remove existing entry for this URL
    history = history.filter(entry => entry.url !== url);

    // Add new entry at the beginning
    history.unshift({
        url,
        fingerprint,
        lastUsed: Date.now()
    });

    // Limit history size
    if (history.length > MAX_HISTORY) {
        history = history.slice(0, MAX_HISTORY);
    }

    set(KEYS.SERVER_HISTORY, history);
}

/**
 * Remove a server from history
 * @param {string} url
 */
export function removeFromHistory(url) {
    const history = getServerHistory().filter(entry => entry.url !== url);
    set(KEYS.SERVER_HISTORY, history);
}

/**
 * Clear server history
 */
export function clearHistory() {
    set(KEYS.SERVER_HISTORY, []);
}

// ============================================================================
// User Preferences
// ============================================================================

const DEFAULT_PREFERENCES = {
    autoConnect: true,
    showStats: false,
    orientationLock: false,
    quality: 'auto',
    bitrateKbps: 8000
};

/**
 * Get user preferences
 * @returns {typeof DEFAULT_PREFERENCES}
 */
export function getPreferences() {
    const stored = get(KEYS.PREFERENCES);
    return { ...DEFAULT_PREFERENCES, ...stored };
}

/**
 * Update user preferences
 * @param {Partial<typeof DEFAULT_PREFERENCES>} updates
 */
export function updatePreferences(updates) {
    const current = getPreferences();
    set(KEYS.PREFERENCES, { ...current, ...updates });
}

/**
 * Reset preferences to defaults
 */
export function resetPreferences() {
    set(KEYS.PREFERENCES, DEFAULT_PREFERENCES);
}

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Check if this device has been paired before
 * @returns {boolean}
 */
export function isPaired() {
    return getCredentials() !== null;
}

/**
 * Get all stored data for debugging
 * @returns {Object}
 */
export function getAllStoredData() {
    return {
        credentials: getCredentials(),
        lastServer: getLastServer(),
        history: getServerHistory(),
        preferences: getPreferences()
    };
}

/**
 * Clear all stored data
 */
export function clearAll() {
    Object.values(KEYS).forEach(key => remove(key));
}

export default {
    getCredentials,
    setCredentials,
    clearCredentials,
    getLastServer,
    setLastServer,
    clearServer,
    getServerHistory,
    addToHistory,
    removeFromHistory,
    clearHistory,
    getPreferences,
    updatePreferences,
    resetPreferences,
    isPaired,
    getAllStoredData,
    clearAll
};

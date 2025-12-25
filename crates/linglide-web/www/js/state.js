/**
 * LinGlide State Management
 *
 * Simple observable state store for managing application state.
 */

/**
 * Create an observable state store
 * @template T
 * @param {T} initialState - Initial state object
 * @returns {Object} State store with get, set, subscribe methods
 */
export function createStore(initialState) {
    let state = { ...initialState };
    const listeners = new Set();

    return {
        /**
         * Get current state
         * @returns {T}
         */
        get() {
            return { ...state };
        },

        /**
         * Get a specific state property
         * @param {keyof T} key
         * @returns {T[keyof T]}
         */
        getProperty(key) {
            return state[key];
        },

        /**
         * Update state with partial state object
         * @param {Partial<T>} partial
         */
        set(partial) {
            const prevState = state;
            state = { ...state, ...partial };
            this.notify(state, prevState);
        },

        /**
         * Subscribe to state changes
         * @param {(state: T, prevState: T) => void} listener
         * @returns {() => void} Unsubscribe function
         */
        subscribe(listener) {
            listeners.add(listener);
            return () => listeners.delete(listener);
        },

        /**
         * Notify all listeners of state change
         * @param {T} newState
         * @param {T} prevState
         */
        notify(newState, prevState) {
            listeners.forEach(listener => {
                try {
                    listener(newState, prevState);
                } catch (error) {
                    console.error('State listener error:', error);
                }
            });
        },

        /**
         * Reset state to initial values
         */
        reset() {
            const prevState = state;
            state = { ...initialState };
            this.notify(state, prevState);
        }
    };
}

// Application state types/views
export const AppView = {
    LOADING: 'loading',
    PAIRING: 'pairing',
    QR_SCANNER: 'qr_scanner',
    PIN_ENTRY: 'pin_entry',
    SERVER_ENTRY: 'server_entry',
    CONNECTING: 'connecting',
    VIEWER: 'viewer',
    ERROR: 'error'
};

// Connection quality levels
export const ConnectionQuality = {
    GOOD: 'good',       // < 50ms latency
    FAIR: 'fair',       // 50-100ms latency
    POOR: 'poor'        // > 100ms latency
};

// Initial application state
const initialAppState = {
    // Current view
    view: AppView.LOADING,

    // Pairing state
    isPaired: false,
    deviceId: null,
    authToken: null,

    // Server connection
    serverUrl: null,
    fingerprint: null,
    isConnected: false,
    isReconnecting: false,

    // Video/display
    displayWidth: 0,
    displayHeight: 0,
    fps: 0,

    // Statistics
    latency: 0,
    currentFps: 0,
    bitrate: 0,
    connectionQuality: ConnectionQuality.GOOD,

    // UI state
    showStatusBar: false,
    showSettings: false,
    orientationLock: false,
    isFullscreen: false,

    // Error state
    error: null
};

// Global application state store
export const appState = createStore(initialAppState);

// State selectors (convenience getters)
export const selectors = {
    isAuthenticated: () => {
        const state = appState.get();
        return state.isPaired && state.authToken !== null;
    },

    canConnect: () => {
        const state = appState.get();
        return state.serverUrl !== null;
    },

    isViewing: () => {
        return appState.getProperty('view') === AppView.VIEWER;
    },

    getConnectionInfo: () => {
        const state = appState.get();
        return {
            url: state.serverUrl,
            token: state.authToken,
            fingerprint: state.fingerprint
        };
    },

    getStats: () => {
        const state = appState.get();
        return {
            latency: state.latency,
            fps: state.currentFps,
            bitrate: state.bitrate,
            quality: state.connectionQuality
        };
    }
};

// State actions (convenience setters)
export const actions = {
    setView(view) {
        appState.set({ view, error: null });
    },

    setError(error) {
        appState.set({
            view: AppView.ERROR,
            error: typeof error === 'string' ? error : error.message
        });
    },

    setPaired(deviceId, authToken) {
        appState.set({
            isPaired: true,
            deviceId,
            authToken
        });
    },

    setServer(url, fingerprint = null) {
        appState.set({
            serverUrl: url,
            fingerprint
        });
    },

    setConnected(isConnected, displayInfo = null) {
        const updates = { isConnected, isReconnecting: false };
        if (displayInfo) {
            updates.displayWidth = displayInfo.width;
            updates.displayHeight = displayInfo.height;
            updates.fps = displayInfo.fps;
        }
        appState.set(updates);
    },

    setReconnecting(isReconnecting) {
        appState.set({ isReconnecting });
    },

    updateStats(stats) {
        const quality =
            stats.latency < 50 ? ConnectionQuality.GOOD :
            stats.latency < 100 ? ConnectionQuality.FAIR :
            ConnectionQuality.POOR;

        appState.set({
            latency: stats.latency ?? appState.getProperty('latency'),
            currentFps: stats.fps ?? appState.getProperty('currentFps'),
            bitrate: stats.bitrate ?? appState.getProperty('bitrate'),
            connectionQuality: quality
        });
    },

    toggleSettings() {
        appState.set({ showSettings: !appState.getProperty('showSettings') });
    },

    setFullscreen(isFullscreen) {
        appState.set({ isFullscreen });
    },

    disconnect() {
        appState.set({
            isConnected: false,
            view: AppView.PAIRING
        });
    },

    reset() {
        appState.reset();
    }
};

export default appState;

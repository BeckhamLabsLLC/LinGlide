/**
 * LinGlide Main Application Controller
 *
 * Orchestrates the application flow, routing between views,
 * and managing global state.
 */

import { appState, actions, AppView, selectors } from './state.js';
import * as storage from './storage.js';
import { pairingController } from './pairing/pairing-controller.js';
import { PinEntry } from './pairing/pin-entry.js';
import { QrScanner } from './pairing/qr-scanner.js';

/**
 * Main application class
 */
class LinGlideApp {
    constructor() {
        this.container = null;
        this.currentView = null;
        this.viewComponents = {};

        // Video/input modules (loaded dynamically)
        this.viewer = null;
        this.inputHandler = null;
    }

    /**
     * Initialize the application
     */
    async init() {
        console.log('LinGlide initializing...');

        // Get main container
        this.container = document.getElementById('app') || document.body;

        // Subscribe to state changes
        appState.subscribe((state, prevState) => {
            if (state.view !== prevState.view) {
                this.renderView(state.view);
            }
        });

        // Initialize pairing controller
        const autoConnecting = await pairingController.init();

        if (autoConnecting) {
            // Auto-connecting with stored credentials
            this.startViewer();
        }
    }

    /**
     * Render the current view
     * @param {string} view
     */
    renderView(view) {
        console.log('Rendering view:', view);

        // Clean up previous view
        this.cleanupCurrentView();

        switch (view) {
            case AppView.LOADING:
                this.renderLoading();
                break;

            case AppView.PAIRING:
                this.renderPairingLanding();
                break;

            case AppView.QR_SCANNER:
                this.renderQrScanner();
                break;

            case AppView.PIN_ENTRY:
                this.renderPinEntry();
                break;

            case AppView.SERVER_ENTRY:
                this.renderServerEntry();
                break;

            case AppView.CONNECTING:
                this.renderConnecting();
                this.startViewer();
                break;

            case AppView.VIEWER:
                // Viewer is already started
                break;

            case AppView.ERROR:
                this.renderError(appState.getProperty('error'));
                break;

            default:
                console.warn('Unknown view:', view);
        }

        this.currentView = view;
    }

    /**
     * Clean up the current view
     */
    cleanupCurrentView() {
        // Destroy view-specific components
        Object.values(this.viewComponents).forEach(component => {
            if (component && typeof component.destroy === 'function') {
                component.destroy();
            }
        });
        this.viewComponents = {};
    }

    /**
     * Render loading view
     */
    renderLoading() {
        this.container.innerHTML = `
            <div class="pairing-landing">
                <div class="loader"></div>
                <p style="margin-top: 16px; color: var(--color-text-secondary);">Loading...</p>
            </div>
        `;
    }

    /**
     * Render pairing landing page
     */
    renderPairingLanding() {
        this.container.innerHTML = `
            <div class="pairing-landing">
                <svg class="pairing-landing__logo" viewBox="0 0 80 80" fill="none">
                    <rect width="80" height="80" rx="16" fill="var(--color-accent)" opacity="0.2"/>
                    <rect x="16" y="16" width="48" height="48" rx="8" stroke="var(--color-accent)" stroke-width="2"/>
                    <circle cx="40" cy="40" r="12" fill="var(--color-accent)"/>
                </svg>

                <h1 class="pairing-landing__title">LinGlide</h1>
                <p class="pairing-landing__subtitle">Connect to your Linux desktop to use this device as an extended display</p>

                <div class="pairing-landing__buttons">
                    <button class="pair-btn pair-btn--primary" id="btn-scan-qr">
                        <svg class="pair-btn__icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <rect x="3" y="3" width="7" height="7"/>
                            <rect x="14" y="3" width="7" height="7"/>
                            <rect x="3" y="14" width="7" height="7"/>
                            <rect x="14" y="14" width="7" height="7"/>
                        </svg>
                        Scan QR Code
                    </button>

                    <button class="pair-btn pair-btn--secondary" id="btn-enter-pin">
                        <svg class="pair-btn__icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <circle cx="12" cy="12" r="3"/>
                            <path d="M12 2v4m0 12v4M2 12h4m12 0h4"/>
                        </svg>
                        Enter PIN
                    </button>
                </div>

                ${this.renderServerHistory()}
            </div>
        `;

        // Bind events
        document.getElementById('btn-scan-qr')?.addEventListener('click', () => {
            pairingController.startQrScanning();
        });

        document.getElementById('btn-enter-pin')?.addEventListener('click', () => {
            pairingController.showPinEntry();
        });

        // Bind history items
        this.container.querySelectorAll('.server-entry__recent-item').forEach(item => {
            item.addEventListener('click', () => {
                const url = item.dataset.url;
                if (url) {
                    pairingController.connectToServer(url);
                }
            });
        });
    }

    /**
     * Render server history section
     * @returns {string}
     */
    renderServerHistory() {
        const history = storage.getServerHistory();
        if (history.length === 0) return '';

        return `
            <div class="server-entry__recent" style="margin-top: 32px; width: 100%; max-width: 300px;">
                <p class="server-entry__recent-title">Recent Servers</p>
                <ul class="server-entry__recent-list">
                    ${history.slice(0, 3).map(entry => `
                        <li class="server-entry__recent-item" data-url="${entry.url}">
                            ${new URL(entry.url).host}
                        </li>
                    `).join('')}
                </ul>
            </div>
        `;
    }

    /**
     * Render QR scanner view
     */
    renderQrScanner() {
        const scanner = new QrScanner(this.container, {
            onScan: (data) => {
                pairingController.handleQrScanned(data);
            },
            onClose: () => {
                pairingController.cancel();
            },
            onError: (error) => {
                actions.setError(error);
            }
        });

        scanner.start();
        this.viewComponents.qrScanner = scanner;
    }

    /**
     * Render PIN entry view
     */
    renderPinEntry() {
        const state = appState.get();

        const pinEntry = new PinEntry(this.container, {
            onSubmit: async (pin, serverUrl) => {
                // Use direct PIN verification (no session required)
                await pairingController.connectAndVerifyPin(serverUrl, pin);
            },
            onCancel: () => {
                pairingController.cancel();
            }
        });

        // Pre-fill server URL from state or auto-detect from browser location
        const serverUrl = state.serverUrl || this.detectServerUrl();
        if (serverUrl) {
            pinEntry.setServerUrl(serverUrl);
        }

        pinEntry.focus();
        this.viewComponents.pinEntry = pinEntry;
    }

    /**
     * Render server entry view
     */
    renderServerEntry() {
        // Auto-detect server URL from current browser location
        const detectedUrl = this.detectServerUrl();

        this.container.innerHTML = `
            <div class="server-entry">
                <div class="pin-entry__header">
                    <button class="pin-entry__back" id="btn-back" aria-label="Go back">
                        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M19 12H5M12 19l-7-7 7-7"/>
                        </svg>
                    </button>
                </div>

                <div class="server-entry__content">
                    <h1 class="server-entry__title">Enter Server URL</h1>

                    <input
                        type="url"
                        id="server-url"
                        class="server-entry__input"
                        placeholder="https://192.168.1.100:8443"
                        inputmode="url"
                        value="${detectedUrl}"
                    />

                    <button class="pair-btn pair-btn--primary" id="btn-connect">
                        Connect
                    </button>

                    ${this.renderServerHistory()}
                </div>
            </div>
        `;

        // Bind events
        document.getElementById('btn-back')?.addEventListener('click', () => {
            pairingController.cancel();
        });

        document.getElementById('btn-connect')?.addEventListener('click', () => {
            const url = document.getElementById('server-url')?.value?.trim();
            if (url) {
                pairingController.connectToServer(url);
            }
        });

        // Bind history items
        this.container.querySelectorAll('.server-entry__recent-item').forEach(item => {
            item.addEventListener('click', () => {
                const url = item.dataset.url;
                if (url) {
                    document.getElementById('server-url').value = url;
                }
            });
        });
    }

    /**
     * Auto-detect the server URL from the current browser location
     * @returns {string} Detected URL or empty string
     */
    detectServerUrl() {
        const currentUrl = window.location.origin;

        // Check if this looks like a LinGlide server URL (not localhost dev server on common ports)
        const isLikelyLinGlideServer = currentUrl.startsWith('https://') &&
            !currentUrl.includes('localhost:3000') &&
            !currentUrl.includes('localhost:5173') &&
            !currentUrl.includes('127.0.0.1:3000') &&
            !currentUrl.includes('127.0.0.1:5173');

        return isLikelyLinGlideServer ? currentUrl : '';
    }

    /**
     * Render connecting view
     */
    renderConnecting() {
        this.container.innerHTML = `
            <div class="pairing-landing">
                <div class="loader"></div>
                <p style="margin-top: 16px; color: var(--color-text-secondary);">Connecting to server...</p>
            </div>
        `;
    }

    /**
     * Render error view
     * @param {string} error
     */
    renderError(error) {
        this.container.innerHTML = `
            <div class="pairing-landing">
                <svg class="pairing-success__icon" viewBox="0 0 24 24" fill="none" stroke="var(--color-error)" stroke-width="2">
                    <circle cx="12" cy="12" r="10"/>
                    <path d="M15 9l-6 6M9 9l6 6"/>
                </svg>

                <h1 class="pairing-landing__title" style="color: var(--color-error);">Error</h1>
                <p class="pairing-landing__subtitle">${error || 'An unexpected error occurred'}</p>

                <div class="pairing-landing__buttons">
                    <button class="pair-btn pair-btn--secondary" id="btn-retry">
                        Try Again
                    </button>
                </div>
            </div>
        `;

        document.getElementById('btn-retry')?.addEventListener('click', () => {
            actions.setView(AppView.PAIRING);
        });
    }

    /**
     * Start the video viewer
     */
    async startViewer() {
        // Dynamically import viewer module
        const { VideoViewer } = await import('./viewer/viewer.js');

        // Set up viewer container
        this.container.innerHTML = `
            <div class="viewer" id="viewer-container">
                <canvas id="display" class="viewer__canvas"></canvas>
                <div id="status" class="viewer__status">
                    <div class="loader"></div>
                    <p id="status-text" class="viewer__status-text">Connecting...</p>
                </div>
            </div>
        `;

        // Get connection info
        const { url, token } = selectors.getConnectionInfo();

        // Initialize viewer
        this.viewer = new VideoViewer({
            canvas: document.getElementById('display'),
            statusElement: document.getElementById('status'),
            statusTextElement: document.getElementById('status-text'),
            serverUrl: url,
            authToken: token,
            onConnect: () => {
                actions.setConnected(true);
                actions.setView(AppView.VIEWER);
            },
            onDisconnect: () => {
                actions.setConnected(false);
                actions.setReconnecting(true);
            },
            onError: (error) => {
                actions.setError(error);
            },
            onStats: (stats) => {
                actions.updateStats(stats);
            }
        });

        await this.viewer.connect();

        // Also start input handler
        const { InputHandler } = await import('./viewer/input.js');
        this.inputHandler = new InputHandler({
            canvas: document.getElementById('display'),
            serverUrl: url,
            authToken: token
        });
    }
}

// Initialize app when DOM is ready
const app = new LinGlideApp();

if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => app.init());
} else {
    app.init();
}

export default app;

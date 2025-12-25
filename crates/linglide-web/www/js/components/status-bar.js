/**
 * LinGlide Status Bar Component
 *
 * Collapsible top bar showing connection stats.
 */

import { appState, ConnectionQuality } from '../state.js';

/**
 * Status bar component
 */
export class StatusBar {
    /**
     * @param {HTMLElement} container
     */
    constructor(container) {
        this.container = container;
        this.element = null;
        this.hideTimeout = null;
        this.isVisible = false;

        this.render();
        this.bindEvents();

        // Subscribe to state changes
        this.unsubscribe = appState.subscribe((state) => {
            this.update(state);
        });
    }

    /**
     * Render the status bar
     */
    render() {
        this.element = document.createElement('div');
        this.element.className = 'status-bar';
        this.element.innerHTML = `
            <div class="status-bar__content">
                <div class="status-bar__stats">
                    <div class="status-bar__stat">
                        <span class="status-bar__stat-value" id="stat-latency">--</span>
                        <span class="status-bar__stat-unit">ms</span>
                    </div>
                    <div class="status-bar__stat">
                        <span class="status-bar__stat-value" id="stat-fps">--</span>
                        <span class="status-bar__stat-unit">FPS</span>
                    </div>
                    <div class="status-bar__stat">
                        <span class="status-bar__stat-value" id="stat-bitrate">--</span>
                        <span class="status-bar__stat-unit">Mbps</span>
                    </div>
                </div>

                <div class="status-bar__quality" id="quality-indicator">
                    <svg class="status-bar__quality-icon status-bar__quality-icon--good" viewBox="0 0 24 24" fill="currentColor">
                        <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 15l-5-5 1.41-1.41L10 14.17l7.59-7.59L19 8l-9 9z"/>
                    </svg>
                </div>
            </div>
        `;

        this.container.appendChild(this.element);

        // Store references
        this.latencyEl = this.element.querySelector('#stat-latency');
        this.fpsEl = this.element.querySelector('#stat-fps');
        this.bitrateEl = this.element.querySelector('#stat-bitrate');
        this.qualityEl = this.element.querySelector('#quality-indicator');
    }

    /**
     * Bind event listeners
     */
    bindEvents() {
        // Show on tap/click
        this.element.addEventListener('click', () => {
            this.toggle();
        });

        // Show on touch/mouse activity in viewer
        document.addEventListener('touchstart', () => this.showTemporarily(), { passive: true });
        document.addEventListener('mousemove', () => this.showTemporarily());
    }

    /**
     * Update stats display
     * @param {Object} state
     */
    update(state) {
        if (this.latencyEl) {
            this.latencyEl.textContent = state.latency || '--';
        }
        if (this.fpsEl) {
            this.fpsEl.textContent = state.currentFps || '--';
        }
        if (this.bitrateEl) {
            this.bitrateEl.textContent = state.bitrate || '--';
        }

        // Update quality indicator
        if (this.qualityEl) {
            const icon = this.qualityEl.querySelector('svg');
            icon.className = 'status-bar__quality-icon';

            switch (state.connectionQuality) {
                case ConnectionQuality.GOOD:
                    icon.classList.add('status-bar__quality-icon--good');
                    break;
                case ConnectionQuality.FAIR:
                    icon.classList.add('status-bar__quality-icon--fair');
                    break;
                case ConnectionQuality.POOR:
                    icon.classList.add('status-bar__quality-icon--poor');
                    break;
            }
        }
    }

    /**
     * Show the status bar
     */
    show() {
        this.isVisible = true;
        this.element.classList.add('status-bar--visible');
    }

    /**
     * Hide the status bar
     */
    hide() {
        this.isVisible = false;
        this.element.classList.remove('status-bar--visible');
    }

    /**
     * Toggle visibility
     */
    toggle() {
        if (this.isVisible) {
            this.hide();
        } else {
            this.show();
        }
    }

    /**
     * Show temporarily then auto-hide
     * @param {number} duration - Duration in ms before hiding
     */
    showTemporarily(duration = 3000) {
        // Clear existing timeout
        if (this.hideTimeout) {
            clearTimeout(this.hideTimeout);
        }

        this.show();

        this.hideTimeout = setTimeout(() => {
            this.hide();
        }, duration);
    }

    /**
     * Clean up
     */
    destroy() {
        if (this.unsubscribe) {
            this.unsubscribe();
        }
        if (this.hideTimeout) {
            clearTimeout(this.hideTimeout);
        }
        if (this.element && this.element.parentNode) {
            this.element.parentNode.removeChild(this.element);
        }
    }
}

export default StatusBar;

/**
 * LinGlide Settings Panel Component
 *
 * Slide-up settings panel for viewer controls.
 */

import { appState, actions } from '../state.js';
import * as storage from '../storage.js';
import { AboutPanel } from './about-panel.js';

/**
 * Settings panel component
 */
export class SettingsPanel {
    /**
     * @param {HTMLElement} container
     * @param {Object} options
     * @param {() => void} [options.onDisconnect]
     */
    constructor(container, options = {}) {
        this.container = container;
        this.options = options;
        this.overlay = null;
        this.panel = null;
        this.isVisible = false;

        this.preferences = storage.getPreferences();
        this.aboutPanel = null;

        this.render();
        this.bindEvents();
    }

    /**
     * Render the settings panel
     */
    render() {
        // Create overlay
        this.overlay = document.createElement('div');
        this.overlay.className = 'settings-overlay';
        this.container.appendChild(this.overlay);

        // Create panel
        this.panel = document.createElement('div');
        this.panel.className = 'settings-panel';
        this.panel.innerHTML = `
            <div class="settings-panel__header">
                <h2 class="settings-panel__title">Settings</h2>
                <button class="settings-panel__close" aria-label="Close settings">
                    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M18 6L6 18M6 6l12 12"/>
                    </svg>
                </button>
            </div>

            <div class="settings-panel__content">
                <!-- Quality slider -->
                <div class="settings-item">
                    <div class="settings-item__label">
                        <span class="settings-item__title">Video Quality</span>
                        <span class="settings-item__description">Higher quality uses more bandwidth</span>
                    </div>
                    <div class="slider">
                        <input type="range" class="slider__input" id="quality-slider"
                               min="1000" max="20000" step="1000" value="${this.preferences.bitrateKbps}">
                        <span class="slider__value" id="quality-value">${this.preferences.bitrateKbps / 1000} Mbps</span>
                    </div>
                </div>

                <!-- Show stats toggle -->
                <div class="settings-item">
                    <div class="settings-item__label">
                        <span class="settings-item__title">Show Statistics</span>
                        <span class="settings-item__description">Display latency, FPS, and bitrate</span>
                    </div>
                    <div class="toggle ${this.preferences.showStats ? 'toggle--active' : ''}" id="toggle-stats">
                        <div class="toggle__handle"></div>
                    </div>
                </div>

                <!-- Orientation lock toggle -->
                <div class="settings-item">
                    <div class="settings-item__label">
                        <span class="settings-item__title">Lock Orientation</span>
                        <span class="settings-item__description">Prevent screen rotation</span>
                    </div>
                    <div class="toggle ${this.preferences.orientationLock ? 'toggle--active' : ''}" id="toggle-orientation">
                        <div class="toggle__handle"></div>
                    </div>
                </div>

                <!-- Fullscreen button -->
                <div class="settings-item">
                    <div class="settings-item__label">
                        <span class="settings-item__title">Fullscreen</span>
                        <span class="settings-item__description">Enter fullscreen mode</span>
                    </div>
                    <button class="pair-btn pair-btn--secondary" id="btn-fullscreen" style="padding: 8px 16px; min-height: auto;">
                        Enter
                    </button>
                </div>

                <!-- About button -->
                <div class="settings-item settings-item--clickable" id="btn-about">
                    <div class="settings-item__label">
                        <span class="settings-item__title">About LinGlide</span>
                        <span class="settings-item__description">Version, credits, and links</span>
                    </div>
                    <svg class="settings-item__chevron" viewBox="0 0 24 24"
                         fill="none" stroke="currentColor" stroke-width="2">
                        <polyline points="9 18 15 12 9 6"/>
                    </svg>
                </div>

                <!-- Disconnect button -->
                <button class="settings-disconnect" id="btn-disconnect">
                    Disconnect
                </button>
            </div>
        `;
        this.container.appendChild(this.panel);

        // Create FAB
        this.fab = document.createElement('div');
        this.fab.className = 'fab';
        this.fab.innerHTML = `
            <button class="fab__button" aria-label="Open settings">
                <svg class="fab__icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <circle cx="12" cy="12" r="3"/>
                    <path d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 010 2.83 2 2 0 01-2.83 0l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-2 2 2 2 0 01-2-2v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83 0 2 2 0 010-2.83l.06-.06a1.65 1.65 0 00.33-1.82 1.65 1.65 0 00-1.51-1H3a2 2 0 01-2-2 2 2 0 012-2h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 010-2.83 2 2 0 012.83 0l.06.06a1.65 1.65 0 001.82.33H9a1.65 1.65 0 001-1.51V3a2 2 0 012-2 2 2 0 012 2v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 0 2 2 0 010 2.83l-.06.06a1.65 1.65 0 00-.33 1.82V9a1.65 1.65 0 001.51 1H21a2 2 0 012 2 2 2 0 01-2 2h-.09a1.65 1.65 0 00-1.51 1z"/>
                </svg>
            </button>
        `;
        this.container.appendChild(this.fab);
    }

    /**
     * Bind event listeners
     */
    bindEvents() {
        // FAB click
        this.fab.addEventListener('click', () => this.show());

        // Overlay click
        this.overlay.addEventListener('click', () => this.hide());

        // Close button
        this.panel.querySelector('.settings-panel__close').addEventListener('click', () => this.hide());

        // Quality slider
        const qualitySlider = this.panel.querySelector('#quality-slider');
        const qualityValue = this.panel.querySelector('#quality-value');
        qualitySlider.addEventListener('input', (e) => {
            const value = parseInt(e.target.value);
            qualityValue.textContent = `${value / 1000} Mbps`;
        });
        qualitySlider.addEventListener('change', (e) => {
            const value = parseInt(e.target.value);
            this.preferences.bitrateKbps = value;
            storage.updatePreferences({ bitrateKbps: value });
            // TODO: Notify server of bitrate change
        });

        // Toggle switches
        this.panel.querySelector('#toggle-stats').addEventListener('click', (e) => {
            const toggle = e.currentTarget;
            toggle.classList.toggle('toggle--active');
            const enabled = toggle.classList.contains('toggle--active');
            this.preferences.showStats = enabled;
            storage.updatePreferences({ showStats: enabled });
            appState.set({ showStatusBar: enabled });
        });

        this.panel.querySelector('#toggle-orientation').addEventListener('click', (e) => {
            const toggle = e.currentTarget;
            toggle.classList.toggle('toggle--active');
            const enabled = toggle.classList.contains('toggle--active');
            this.preferences.orientationLock = enabled;
            storage.updatePreferences({ orientationLock: enabled });
            this.setOrientationLock(enabled);
        });

        // Fullscreen button
        this.panel.querySelector('#btn-fullscreen').addEventListener('click', () => {
            this.toggleFullscreen();
            this.hide();
        });

        // Disconnect button
        this.panel.querySelector('#btn-disconnect').addEventListener('click', () => {
            this.hide();
            this.options.onDisconnect?.();
        });

        // About button
        this.panel.querySelector('#btn-about').addEventListener('click', () => {
            this.hide();
            this.showAbout();
        });
    }

    /**
     * Show about panel
     */
    showAbout() {
        if (!this.aboutPanel) {
            this.aboutPanel = new AboutPanel(this.container, {
                onClose: () => {
                    this.aboutPanel.destroy();
                    this.aboutPanel = null;
                }
            });
        }
        this.aboutPanel.show();
    }

    /**
     * Show the settings panel
     */
    show() {
        this.isVisible = true;
        this.overlay.classList.add('settings-overlay--visible');
        this.panel.classList.add('settings-panel--visible');
        this.fab.classList.add('hidden');
    }

    /**
     * Hide the settings panel
     */
    hide() {
        this.isVisible = false;
        this.overlay.classList.remove('settings-overlay--visible');
        this.panel.classList.remove('settings-panel--visible');
        this.fab.classList.remove('hidden');
    }

    /**
     * Toggle fullscreen mode
     */
    async toggleFullscreen() {
        try {
            if (!document.fullscreenElement) {
                await document.documentElement.requestFullscreen();
                actions.setFullscreen(true);
            } else {
                await document.exitFullscreen();
                actions.setFullscreen(false);
            }
        } catch (error) {
            console.error('Fullscreen error:', error);
        }
    }

    /**
     * Set orientation lock
     * @param {boolean} lock
     */
    async setOrientationLock(lock) {
        if (!screen.orientation?.lock) {
            console.warn('Orientation lock not supported');
            return;
        }

        try {
            if (lock) {
                await screen.orientation.lock('landscape');
            } else {
                screen.orientation.unlock();
            }
        } catch (error) {
            console.error('Orientation lock error:', error);
        }
    }

    /**
     * Clean up
     */
    destroy() {
        if (this.overlay?.parentNode) {
            this.overlay.parentNode.removeChild(this.overlay);
        }
        if (this.panel?.parentNode) {
            this.panel.parentNode.removeChild(this.panel);
        }
        if (this.fab?.parentNode) {
            this.fab.parentNode.removeChild(this.fab);
        }
    }
}

export default SettingsPanel;

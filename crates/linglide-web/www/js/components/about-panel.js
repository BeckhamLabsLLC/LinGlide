/**
 * LinGlide About Panel Component
 * Displays app info, version, credits, and links
 */

/**
 * About panel component
 */
export class AboutPanel {
    /**
     * @param {HTMLElement} container
     * @param {Object} options
     * @param {() => void} [options.onClose]
     */
    constructor(container, options = {}) {
        this.container = container;
        this.options = options;
        this.overlay = null;
        this.panel = null;
        this.isVisible = false;

        this.appInfo = {
            name: 'LinGlide',
            version: '0.1.0',
            description: 'High-performance Linux screen sharing for mobile devices. Use your phone or tablet as an extended display with touch control.',
            license: 'MIT License',
            githubUrl: 'https://github.com/BeckhamLabs/linglide',
            issuesUrl: 'https://github.com/BeckhamLabs/linglide/issues',
            websiteUrl: 'https://beckhamlabs.com'
        };

        this.render();
        this.bindEvents();
    }

    /**
     * Render the about panel
     */
    render() {
        // Create overlay
        this.overlay = document.createElement('div');
        this.overlay.className = 'about-overlay';
        this.container.appendChild(this.overlay);

        // Create panel
        this.panel = document.createElement('div');
        this.panel.className = 'about-panel';
        this.panel.innerHTML = `
            <div class="about-panel__header">
                <button class="about-panel__back" aria-label="Go back">
                    <svg width="24" height="24" viewBox="0 0 24 24" fill="none"
                         stroke="currentColor" stroke-width="2">
                        <path d="M19 12H5M12 19l-7-7 7-7"/>
                    </svg>
                </button>
                <h2 class="about-panel__title">About</h2>
                <div style="width: 44px;"></div>
            </div>

            <div class="about-panel__content">
                <!-- Logo Section -->
                <div class="about-panel__logo-section">
                    ${this.renderLogo()}
                    <h1 class="about-panel__app-name">${this.appInfo.name}</h1>
                    <span class="about-panel__version">Version ${this.appInfo.version}</span>
                </div>

                <!-- Description -->
                <p class="about-panel__description">${this.appInfo.description}</p>

                <!-- Links Section -->
                <div class="about-panel__links">
                    <a href="${this.appInfo.githubUrl}" target="_blank"
                       rel="noopener noreferrer" class="about-panel__link">
                        ${this.renderGithubIcon()}
                        <span>View on GitHub</span>
                        ${this.renderExternalIcon()}
                    </a>
                    <a href="${this.appInfo.issuesUrl}" target="_blank"
                       rel="noopener noreferrer" class="about-panel__link about-panel__link--secondary">
                        ${this.renderIssueIcon()}
                        <span>Report Issue</span>
                        ${this.renderExternalIcon()}
                    </a>
                </div>

                <!-- Credits Section -->
                <div class="about-panel__credits">
                    <div class="about-panel__credit-item">
                        <span class="about-panel__credit-label">Developed by</span>
                        <a href="${this.appInfo.websiteUrl}" target="_blank"
                           rel="noopener noreferrer" class="about-panel__credit-value about-panel__credit-value--link">
                            BeckhamLabs
                        </a>
                    </div>
                    <div class="about-panel__credit-item">
                        <span class="about-panel__credit-label">License</span>
                        <span class="about-panel__credit-value">${this.appInfo.license}</span>
                    </div>
                    <div class="about-panel__credit-item">
                        <span class="about-panel__credit-label">Copyright</span>
                        <span class="about-panel__credit-value">2024-2025 BeckhamLabs</span>
                    </div>
                </div>

                <!-- Footer -->
                <div class="about-panel__footer">
                    <p>Made with care for the Linux community</p>
                </div>
            </div>
        `;
        this.container.appendChild(this.panel);
    }

    /**
     * Render the LinGlide logo SVG
     */
    renderLogo() {
        return `
            <svg class="about-panel__logo" viewBox="0 0 100 100" fill="none">
                <defs>
                    <linearGradient id="logo-gradient" x1="0%" y1="0%" x2="100%" y2="100%">
                        <stop offset="0%" stop-color="var(--color-accent)"/>
                        <stop offset="100%" stop-color="#6bb3ff"/>
                    </linearGradient>
                </defs>
                <!-- Outer rounded rectangle (device frame) -->
                <rect x="10" y="15" width="80" height="55" rx="8"
                      stroke="url(#logo-gradient)" stroke-width="3" fill="none"/>
                <!-- Inner screen -->
                <rect x="16" y="21" width="68" height="43" rx="4"
                      fill="url(#logo-gradient)" opacity="0.2"/>
                <!-- Connection lines (representing screen sharing) -->
                <path d="M50 70 L50 85" stroke="url(#logo-gradient)" stroke-width="3"
                      stroke-linecap="round"/>
                <path d="M35 85 L65 85" stroke="url(#logo-gradient)" stroke-width="3"
                      stroke-linecap="round"/>
                <!-- Glide/wave accent -->
                <path d="M25 42 Q37 32, 50 42 T75 42" stroke="var(--color-accent)"
                      stroke-width="2.5" fill="none" stroke-linecap="round"/>
            </svg>
        `;
    }

    /**
     * Render GitHub icon
     */
    renderGithubIcon() {
        return `
            <svg class="about-panel__link-icon" viewBox="0 0 24 24" fill="currentColor">
                <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205
                         11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555
                         -3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02
                         -.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305
                         3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0
                         -1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315
                         3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23
                         3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0
                         4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015
                         2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63
                         -5.37-12-12-12z"/>
            </svg>
        `;
    }

    /**
     * Render issue/bug icon
     */
    renderIssueIcon() {
        return `
            <svg class="about-panel__link-icon" viewBox="0 0 24 24" fill="none"
                 stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10"/>
                <path d="M12 8v4M12 16h.01"/>
            </svg>
        `;
    }

    /**
     * Render external link icon
     */
    renderExternalIcon() {
        return `
            <svg class="about-panel__external-icon" viewBox="0 0 24 24"
                 fill="none" stroke="currentColor" stroke-width="2">
                <path d="M18 13v6a2 2 0 01-2 2H5a2 2 0 01-2-2V8a2 2 0 012-2h6"/>
                <polyline points="15 3 21 3 21 9"/>
                <line x1="10" y1="14" x2="21" y2="3"/>
            </svg>
        `;
    }

    /**
     * Bind event listeners
     */
    bindEvents() {
        this.overlay.addEventListener('click', () => this.hide());
        this.panel.querySelector('.about-panel__back').addEventListener('click', () => this.hide());

        // Handle escape key
        this.handleEscape = (e) => {
            if (e.key === 'Escape' && this.isVisible) {
                this.hide();
            }
        };
        document.addEventListener('keydown', this.handleEscape);
    }

    /**
     * Show the about panel
     */
    show() {
        this.isVisible = true;
        this.overlay.classList.add('about-overlay--visible');
        this.panel.classList.add('about-panel--visible');

        // Prevent body scroll
        document.body.style.overflow = 'hidden';
    }

    /**
     * Hide the about panel
     */
    hide() {
        this.isVisible = false;
        this.overlay.classList.remove('about-overlay--visible');
        this.panel.classList.remove('about-panel--visible');

        // Restore body scroll
        document.body.style.overflow = '';

        this.options.onClose?.();
    }

    /**
     * Clean up and remove from DOM
     */
    destroy() {
        document.removeEventListener('keydown', this.handleEscape);
        this.overlay?.remove();
        this.panel?.remove();
    }
}

export default AboutPanel;

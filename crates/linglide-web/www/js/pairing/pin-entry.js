/**
 * LinGlide PIN Entry Component
 *
 * Handles the 6-digit PIN input for device pairing.
 * Auto-detects the server URL from the current browser location.
 */

/**
 * PIN Entry component
 */
export class PinEntry {
    /**
     * @param {HTMLElement} container
     * @param {Object} options
     * @param {(pin: string, serverUrl: string) => Promise<void>} options.onSubmit
     * @param {() => void} options.onCancel
     */
    constructor(container, options) {
        this.container = container;
        this.options = options;
        this.digits = ['', '', '', '', '', ''];
        this.activeIndex = 0;
        this.isSubmitting = false;
        this.error = null;

        this.render();
        this.bindEvents();
        this.autoDetectServerUrl();
    }

    /**
     * Auto-detect and pre-fill the server URL from the current browser location
     */
    autoDetectServerUrl() {
        // If we're already on a LinGlide server, use its URL
        const currentUrl = window.location.origin;

        // Check if this looks like a LinGlide server URL (not localhost dev server on common ports)
        const isLikelyLinGlideServer = currentUrl.startsWith('https://') &&
            !currentUrl.includes('localhost:3000') &&
            !currentUrl.includes('localhost:5173') &&
            !currentUrl.includes('127.0.0.1:3000') &&
            !currentUrl.includes('127.0.0.1:5173');

        if (isLikelyLinGlideServer && this.hostInput) {
            this.hostInput.value = currentUrl;
        }
    }

    /**
     * Render the PIN entry UI
     */
    render() {
        this.container.innerHTML = `
            <div class="pin-entry">
                <div class="pin-entry__header">
                    <button class="pin-entry__back" aria-label="Go back">
                        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M19 12H5M12 19l-7-7 7-7"/>
                        </svg>
                    </button>
                </div>

                <div class="pin-entry__content">
                    <h1 class="pin-entry__title">Enter PIN</h1>
                    <p class="pin-entry__subtitle">Enter the 6-digit PIN shown on your desktop</p>

                    <div class="pin-entry__host">
                        <label class="pin-entry__host-label">Server URL</label>
                        <input type="text" class="pin-entry__host-input" placeholder="https://192.168.1.100:8443" />
                    </div>

                    <div class="pin-entry__digits" role="group" aria-label="PIN input">
                        ${this.digits.map((_, i) => `
                            <div class="pin-digit ${i === 0 ? 'pin-digit--active' : ''}" data-index="${i}">
                                ${this.digits[i]}
                            </div>
                        `).join('')}
                    </div>

                    <!-- Hidden input for keyboard -->
                    <input
                        type="text"
                        class="pin-entry__input"
                        inputmode="numeric"
                        pattern="[0-9]*"
                        maxlength="6"
                        autocomplete="one-time-code"
                        aria-label="PIN"
                    />

                    <div class="pin-entry__status">
                        <span class="pin-entry__status-text"></span>
                    </div>
                </div>
            </div>
        `;

        // Store references
        this.backBtn = this.container.querySelector('.pin-entry__back');
        this.hostInput = this.container.querySelector('.pin-entry__host-input');
        this.hiddenInput = this.container.querySelector('.pin-entry__input');
        this.digitBoxes = this.container.querySelectorAll('.pin-digit');
        this.statusText = this.container.querySelector('.pin-entry__status-text');
    }

    /**
     * Bind event listeners
     */
    bindEvents() {
        // Back button
        this.backBtn.addEventListener('click', () => {
            this.options.onCancel?.();
        });

        // Focus hidden input when clicking digit boxes
        this.digitBoxes.forEach((box, index) => {
            box.addEventListener('click', () => {
                this.activeIndex = index;
                this.updateDigitStyles();
                this.hiddenInput.focus();
            });
        });

        // Handle input
        this.hiddenInput.addEventListener('input', (e) => {
            const value = e.target.value.replace(/\D/g, '').slice(0, 6);
            this.setDigits(value);
        });

        // Handle keydown for backspace
        this.hiddenInput.addEventListener('keydown', (e) => {
            if (e.key === 'Backspace' && this.hiddenInput.value === '') {
                this.clearLastDigit();
            }
        });

        // Auto-focus hidden input
        this.container.addEventListener('click', () => {
            if (!this.isSubmitting) {
                this.hiddenInput.focus();
            }
        });
    }

    /**
     * Set digits from input value
     * @param {string} value
     */
    setDigits(value) {
        const chars = value.split('');

        this.digits = chars.concat(Array(6 - chars.length).fill(''));
        this.activeIndex = Math.min(chars.length, 5);

        this.updateDigitDisplay();
        this.updateDigitStyles();

        // Auto-submit when 6 digits entered
        if (chars.length === 6) {
            this.submit();
        }
    }

    /**
     * Clear the last entered digit
     */
    clearLastDigit() {
        const lastFilledIndex = this.digits.findLastIndex(d => d !== '');
        if (lastFilledIndex >= 0) {
            this.digits[lastFilledIndex] = '';
            this.activeIndex = lastFilledIndex;
            this.updateDigitDisplay();
            this.updateDigitStyles();
        }
    }

    /**
     * Update digit box display
     */
    updateDigitDisplay() {
        this.digitBoxes.forEach((box, i) => {
            box.textContent = this.digits[i];
        });
        this.hiddenInput.value = this.digits.join('');
    }

    /**
     * Update digit box styles
     */
    updateDigitStyles() {
        this.digitBoxes.forEach((box, i) => {
            box.classList.remove('pin-digit--active', 'pin-digit--filled', 'pin-digit--error');

            if (this.error) {
                box.classList.add('pin-digit--error');
            } else if (this.digits[i] !== '') {
                box.classList.add('pin-digit--filled');
            } else if (i === this.activeIndex) {
                box.classList.add('pin-digit--active');
            }
        });
    }

    /**
     * Submit the PIN
     */
    async submit() {
        const pin = this.digits.join('');
        if (pin.length !== 6) return;

        const serverUrl = this.hostInput.value.trim();
        if (!serverUrl) {
            this.setError('Please enter the server URL');
            return;
        }

        this.isSubmitting = true;
        this.setStatus('Verifying...', false);
        this.hiddenInput.disabled = true;

        try {
            await this.options.onSubmit?.(pin, serverUrl);
        } catch (error) {
            this.setError(error.message || 'Invalid PIN');
            this.reset();
        }

        this.isSubmitting = false;
        this.hiddenInput.disabled = false;
    }

    /**
     * Set status message
     * @param {string} message
     * @param {boolean} isError
     */
    setStatus(message, isError = false) {
        this.statusText.textContent = message;
        this.statusText.className = 'pin-entry__status-text';
        if (isError) {
            this.statusText.classList.add('pin-entry__status-text--error');
        }
    }

    /**
     * Set error state
     * @param {string} message
     */
    setError(message) {
        this.error = message;
        this.setStatus(message, true);
        this.updateDigitStyles();

        // Clear error after animation
        setTimeout(() => {
            this.error = null;
            this.updateDigitStyles();
        }, 500);
    }

    /**
     * Reset the PIN entry
     */
    reset() {
        this.digits = ['', '', '', '', '', ''];
        this.activeIndex = 0;
        this.hiddenInput.value = '';
        this.updateDigitDisplay();
        this.updateDigitStyles();
        this.hiddenInput.focus();
    }

    /**
     * Set the server URL (takes precedence over auto-detection)
     * @param {string} url
     */
    setServerUrl(url) {
        if (url && this.hostInput) {
            this.hostInput.value = url;
        }
    }

    /**
     * Focus the input
     */
    focus() {
        this.hiddenInput.focus();
    }

    /**
     * Clean up
     */
    destroy() {
        this.container.innerHTML = '';
    }
}

export default PinEntry;

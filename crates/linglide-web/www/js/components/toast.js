/**
 * LinGlide Toast Notification Component
 *
 * Simple toast notifications for user feedback.
 */

/**
 * Toast types
 */
export const ToastType = {
    SUCCESS: 'success',
    ERROR: 'error',
    INFO: 'info'
};

/**
 * Toast manager (singleton)
 */
class ToastManager {
    constructor() {
        this.container = null;
        this.toasts = [];
        this.init();
    }

    /**
     * Initialize the toast container
     */
    init() {
        if (this.container) return;

        this.container = document.createElement('div');
        this.container.className = 'toast-container';
        document.body.appendChild(this.container);
    }

    /**
     * Show a toast notification
     * @param {string} message
     * @param {string} [type=ToastType.INFO]
     * @param {number} [duration=3000]
     * @returns {HTMLElement}
     */
    show(message, type = ToastType.INFO, duration = 3000) {
        const toast = document.createElement('div');
        toast.className = `toast toast--${type}`;

        const icon = this.getIcon(type);
        toast.innerHTML = `
            ${icon}
            <span class="toast__message">${message}</span>
        `;

        this.container.appendChild(toast);
        this.toasts.push(toast);

        // Auto-dismiss
        if (duration > 0) {
            setTimeout(() => this.dismiss(toast), duration);
        }

        return toast;
    }

    /**
     * Get icon SVG for toast type
     * @param {string} type
     * @returns {string}
     */
    getIcon(type) {
        switch (type) {
            case ToastType.SUCCESS:
                return `
                    <svg class="toast__icon" viewBox="0 0 24 24" fill="none" stroke="var(--color-success)" stroke-width="2">
                        <path d="M22 11.08V12a10 10 0 11-5.93-9.14"/>
                        <path d="M22 4L12 14.01l-3-3"/>
                    </svg>
                `;
            case ToastType.ERROR:
                return `
                    <svg class="toast__icon" viewBox="0 0 24 24" fill="none" stroke="var(--color-error)" stroke-width="2">
                        <circle cx="12" cy="12" r="10"/>
                        <path d="M15 9l-6 6M9 9l6 6"/>
                    </svg>
                `;
            case ToastType.INFO:
            default:
                return `
                    <svg class="toast__icon" viewBox="0 0 24 24" fill="none" stroke="var(--color-accent)" stroke-width="2">
                        <circle cx="12" cy="12" r="10"/>
                        <path d="M12 16v-4M12 8h.01"/>
                    </svg>
                `;
        }
    }

    /**
     * Dismiss a toast
     * @param {HTMLElement} toast
     */
    dismiss(toast) {
        if (!toast || !toast.parentNode) return;

        toast.classList.add('toast--exiting');

        setTimeout(() => {
            if (toast.parentNode) {
                toast.parentNode.removeChild(toast);
            }
            const index = this.toasts.indexOf(toast);
            if (index > -1) {
                this.toasts.splice(index, 1);
            }
        }, 250);
    }

    /**
     * Dismiss all toasts
     */
    dismissAll() {
        [...this.toasts].forEach(toast => this.dismiss(toast));
    }

    /**
     * Convenience methods
     */
    success(message, duration) {
        return this.show(message, ToastType.SUCCESS, duration);
    }

    error(message, duration) {
        return this.show(message, ToastType.ERROR, duration);
    }

    info(message, duration) {
        return this.show(message, ToastType.INFO, duration);
    }
}

// Singleton instance
export const toast = new ToastManager();

export default toast;

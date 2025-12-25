/**
 * LinGlide Input Handler
 *
 * Handles touch, mouse, and stylus input for the video viewer.
 */

/**
 * Input handler options
 * @typedef {Object} InputOptions
 * @property {HTMLCanvasElement} canvas
 * @property {string} serverUrl
 * @property {string} [authToken]
 */

/**
 * Input handler class
 */
export class InputHandler {
    /**
     * @param {InputOptions} options
     */
    constructor(options) {
        this.canvas = options.canvas;
        this.serverUrl = options.serverUrl;
        this.authToken = options.authToken;

        this.ws = null;
        this.activeTouches = new Map();
        this.lastMousePosition = { x: 0, y: 0 };

        this.connect();
        this.bindEvents();
    }

    /**
     * Connect to input WebSocket
     */
    connect() {
        const protocol = this.serverUrl.startsWith('https') ? 'wss' : 'ws';
        const host = this.serverUrl.replace(/^https?:\/\//, '');
        let url = `${protocol}://${host}/ws/input`;

        if (this.authToken) {
            url += `?token=${encodeURIComponent(this.authToken)}`;
        }

        this.ws = new WebSocket(url);

        this.ws.onopen = () => {
            console.log('Input WebSocket connected');
        };

        this.ws.onclose = () => {
            console.log('Input WebSocket closed');
            // Reconnect after delay
            setTimeout(() => this.connect(), 2000);
        };

        this.ws.onerror = (error) => {
            console.error('Input WebSocket error:', error);
        };
    }

    /**
     * Bind input event listeners
     */
    bindEvents() {
        // Touch events
        this.canvas.addEventListener('touchstart', (e) => this.handleTouchStart(e), { passive: false });
        this.canvas.addEventListener('touchmove', (e) => this.handleTouchMove(e), { passive: false });
        this.canvas.addEventListener('touchend', (e) => this.handleTouchEnd(e), { passive: false });
        this.canvas.addEventListener('touchcancel', (e) => this.handleTouchCancel(e), { passive: false });

        // Mouse events
        this.canvas.addEventListener('mousedown', (e) => this.handleMouseDown(e));
        this.canvas.addEventListener('mousemove', (e) => this.handleMouseMove(e));
        this.canvas.addEventListener('mouseup', (e) => this.handleMouseUp(e));
        this.canvas.addEventListener('wheel', (e) => this.handleWheel(e), { passive: false });

        // Context menu (right-click)
        this.canvas.addEventListener('contextmenu', (e) => e.preventDefault());

        // Pointer events for stylus detection
        if ('PointerEvent' in window) {
            this.canvas.addEventListener('pointerdown', (e) => this.handlePointerDown(e));
            this.canvas.addEventListener('pointermove', (e) => this.handlePointerMove(e));
            this.canvas.addEventListener('pointerup', (e) => this.handlePointerUp(e));
        }
    }

    /**
     * Get normalized coordinates (0.0 to 1.0)
     * @param {number} clientX
     * @param {number} clientY
     * @returns {{ x: number, y: number }}
     */
    normalizeCoords(clientX, clientY) {
        const rect = this.canvas.getBoundingClientRect();

        // Calculate position relative to canvas display size
        const displayX = clientX - rect.left;
        const displayY = clientY - rect.top;

        // Normalize to 0.0-1.0 range (server expects normalized coords)
        return {
            x: Math.max(0, Math.min(1, displayX / rect.width)),
            y: Math.max(0, Math.min(1, displayY / rect.height))
        };
    }

    /**
     * Send input event
     * @param {Object} event
     */
    send(event) {
        if (this.ws?.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify(event));
        }
    }

    // ========================================================================
    // Touch Events
    // ========================================================================

    handleTouchStart(e) {
        e.preventDefault();

        for (const touch of e.changedTouches) {
            const coords = this.normalizeCoords(touch.clientX, touch.clientY);
            this.activeTouches.set(touch.identifier, coords);

            this.send({
                type: 'TouchStart',
                id: touch.identifier,
                x: coords.x,
                y: coords.y
            });
        }
    }

    handleTouchMove(e) {
        e.preventDefault();

        for (const touch of e.changedTouches) {
            const coords = this.normalizeCoords(touch.clientX, touch.clientY);
            this.activeTouches.set(touch.identifier, coords);

            this.send({
                type: 'TouchMove',
                id: touch.identifier,
                x: coords.x,
                y: coords.y
            });
        }
    }

    handleTouchEnd(e) {
        e.preventDefault();

        for (const touch of e.changedTouches) {
            this.activeTouches.delete(touch.identifier);

            this.send({
                type: 'TouchEnd',
                id: touch.identifier
            });
        }
    }

    handleTouchCancel(e) {
        e.preventDefault();

        for (const touch of e.changedTouches) {
            this.activeTouches.delete(touch.identifier);

            this.send({
                type: 'TouchCancel',
                id: touch.identifier
            });
        }
    }

    // ========================================================================
    // Mouse Events
    // ========================================================================

    handleMouseDown(e) {
        const coords = this.normalizeCoords(e.clientX, e.clientY);
        this.lastMousePosition = coords;

        this.send({
            type: 'MouseDown',
            button: e.button,
            x: coords.x,
            y: coords.y
        });
    }

    handleMouseMove(e) {
        const coords = this.normalizeCoords(e.clientX, e.clientY);
        this.lastMousePosition = coords;

        // Only send if button is pressed (for performance)
        if (e.buttons > 0) {
            this.send({
                type: 'MouseMove',
                x: coords.x,
                y: coords.y
            });
        }
    }

    handleMouseUp(e) {
        const coords = this.normalizeCoords(e.clientX, e.clientY);

        this.send({
            type: 'MouseUp',
            button: e.button,
            x: coords.x,
            y: coords.y
        });
    }

    handleWheel(e) {
        e.preventDefault();

        this.send({
            type: 'Scroll',
            dx: Math.round(e.deltaX),
            dy: Math.round(e.deltaY)
        });
    }

    // ========================================================================
    // Pointer/Stylus Events
    // ========================================================================

    handlePointerDown(e) {
        // Only handle pen input here
        if (e.pointerType !== 'pen') return;

        const coords = this.normalizeCoords(e.clientX, e.clientY);

        this.send({
            type: 'PenDown',
            x: coords.x,
            y: coords.y,
            pressure: e.pressure,
            tilt_x: e.tiltX || 0,
            tilt_y: e.tiltY || 0,
            button: e.button
        });
    }

    handlePointerMove(e) {
        if (e.pointerType !== 'pen') return;

        const coords = this.normalizeCoords(e.clientX, e.clientY);

        if (e.pressure > 0) {
            this.send({
                type: 'PenMove',
                x: coords.x,
                y: coords.y,
                pressure: e.pressure,
                tilt_x: e.tiltX || 0,
                tilt_y: e.tiltY || 0
            });
        } else {
            this.send({
                type: 'PenHover',
                x: coords.x,
                y: coords.y,
                pressure: 0,
                tilt_x: e.tiltX || 0,
                tilt_y: e.tiltY || 0
            });
        }
    }

    handlePointerUp(e) {
        if (e.pointerType !== 'pen') return;

        const coords = this.normalizeCoords(e.clientX, e.clientY);

        this.send({
            type: 'PenUp',
            x: coords.x,
            y: coords.y
        });
    }

    /**
     * Disconnect and clean up
     */
    disconnect() {
        if (this.ws) {
            this.ws.close();
            this.ws = null;
        }
    }
}

export default InputHandler;

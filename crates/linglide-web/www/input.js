// LinGlide Input Handler - Touch, mouse, and stylus event capture

class InputHandler {
    constructor() {
        console.log('InputHandler: initializing...');
        this.canvas = document.getElementById('display');
        if (!this.canvas) {
            console.error('InputHandler: canvas not found!');
            return;
        }
        console.log('InputHandler: canvas found', this.canvas);
        this.ws = null;
        this.activeTouches = new Map();
        this.penActive = false;
        this.penDown = false;

        this.init();
    }

    init() {
        this.connect();
        this.setupEventListeners();
        console.log('InputHandler: initialized, event listeners attached');
    }

    connect() {
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const url = `${protocol}//${window.location.host}/ws/input`;
        console.log('InputHandler: connecting to', url);

        this.ws = new WebSocket(url);

        this.ws.onopen = () => {
            console.log('InputHandler: WebSocket connected');
        };

        this.ws.onclose = (e) => {
            console.log('InputHandler: WebSocket closed', e.code, e.reason);
            setTimeout(() => this.connect(), 2000);
        };

        this.ws.onerror = (error) => {
            console.error('InputHandler: WebSocket error:', error);
        };
    }

    send(event) {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            console.log('InputHandler: sending', event.type);
            this.ws.send(JSON.stringify(event));
        } else {
            console.log('InputHandler: cannot send, ws not open', this.ws?.readyState);
        }
    }

    // Normalize coordinates to 0-1 range relative to canvas
    normalizeCoords(clientX, clientY) {
        const rect = this.canvas.getBoundingClientRect();
        const x = (clientX - rect.left) / rect.width;
        const y = (clientY - rect.top) / rect.height;
        return {
            x: Math.max(0, Math.min(1, x)),
            y: Math.max(0, Math.min(1, y))
        };
    }

    // Determine pen button type from pointer event
    getPenButton(e) {
        // Check for eraser (button 5 or eraser pointerType)
        if (e.button === 5 || (e.pointerType === 'pen' && e.buttons & 32)) {
            return 'Eraser';
        }
        // Barrel button 2 (tertiary)
        if (e.buttons & 4) {
            return 'Tertiary';
        }
        // Barrel button 1 (secondary) - typically right-click equivalent
        if (e.buttons & 2) {
            return 'Secondary';
        }
        // Primary (tip)
        return 'Primary';
    }

    setupEventListeners() {
        // Use Pointer Events API for unified touch/pen/mouse handling
        // This provides pressure, tilt, and pointer type information
        if (window.PointerEvent) {
            this.canvas.addEventListener('pointerdown', (e) => this.handlePointerDown(e), { passive: false });
            this.canvas.addEventListener('pointermove', (e) => this.handlePointerMove(e), { passive: false });
            this.canvas.addEventListener('pointerup', (e) => this.handlePointerUp(e), { passive: false });
            this.canvas.addEventListener('pointercancel', (e) => this.handlePointerCancel(e), { passive: false });
            this.canvas.addEventListener('pointerenter', (e) => this.handlePointerEnter(e), { passive: false });
            this.canvas.addEventListener('pointerleave', (e) => this.handlePointerLeave(e), { passive: false });

            // Capture pointer for smoother tracking
            this.canvas.addEventListener('gotpointercapture', () => {});
            this.canvas.addEventListener('lostpointercapture', () => {});
        } else {
            // Fallback to legacy touch/mouse events
            this.canvas.addEventListener('touchstart', (e) => this.handleTouchStart(e), { passive: false });
            this.canvas.addEventListener('touchmove', (e) => this.handleTouchMove(e), { passive: false });
            this.canvas.addEventListener('touchend', (e) => this.handleTouchEnd(e), { passive: false });
            this.canvas.addEventListener('touchcancel', (e) => this.handleTouchCancel(e), { passive: false });

            this.canvas.addEventListener('mousedown', (e) => this.handleMouseDown(e));
            this.canvas.addEventListener('mousemove', (e) => this.handleMouseMove(e));
            this.canvas.addEventListener('mouseup', (e) => this.handleMouseUp(e));
        }

        // Wheel events (works with both APIs)
        this.canvas.addEventListener('wheel', (e) => this.handleWheel(e), { passive: false });

        // Prevent context menu
        this.canvas.addEventListener('contextmenu', (e) => e.preventDefault());

        // Prevent default touch behaviors
        document.addEventListener('gesturestart', (e) => e.preventDefault());
        document.addEventListener('gesturechange', (e) => e.preventDefault());
        document.addEventListener('gestureend', (e) => e.preventDefault());
    }

    // Pointer Events API handlers (unified touch/pen/mouse)
    handlePointerDown(e) {
        e.preventDefault();

        // Capture pointer for this element
        try {
            this.canvas.setPointerCapture(e.pointerId);
        } catch (err) {
            // Ignore capture errors
        }

        const { x, y } = this.normalizeCoords(e.clientX, e.clientY);

        if (e.pointerType === 'pen') {
            this.penActive = true;
            this.penDown = true;
            this.send({
                type: 'PenDown',
                x,
                y,
                pressure: e.pressure || 0.5,
                tilt_x: e.tiltX || 0,
                tilt_y: e.tiltY || 0,
                button: this.getPenButton(e)
            });
        } else if (e.pointerType === 'touch') {
            this.activeTouches.set(e.pointerId, { x, y });
            this.send({
                type: 'TouchStart',
                id: e.pointerId % 10,  // Limit to 10 touch points
                x,
                y
            });
        } else {
            // Mouse
            this.send({
                type: 'MouseDown',
                button: e.button,
                x,
                y
            });
        }
    }

    handlePointerMove(e) {
        e.preventDefault();
        const { x, y } = this.normalizeCoords(e.clientX, e.clientY);

        if (e.pointerType === 'pen') {
            if (this.penDown) {
                // Pen drawing (touching surface)
                this.send({
                    type: 'PenMove',
                    x,
                    y,
                    pressure: e.pressure || 0.5,
                    tilt_x: e.tiltX || 0,
                    tilt_y: e.tiltY || 0
                });
            } else if (this.penActive) {
                // Pen hovering
                this.send({
                    type: 'PenHover',
                    x,
                    y,
                    pressure: 0,
                    tilt_x: e.tiltX || 0,
                    tilt_y: e.tiltY || 0
                });
            }
        } else if (e.pointerType === 'touch') {
            if (this.activeTouches.has(e.pointerId)) {
                this.activeTouches.set(e.pointerId, { x, y });
                this.send({
                    type: 'TouchMove',
                    id: e.pointerId % 10,
                    x,
                    y
                });
            }
        } else {
            // Mouse - only send if button pressed (drag)
            if (e.buttons !== 0) {
                this.send({
                    type: 'MouseMove',
                    x,
                    y
                });
            }
        }
    }

    handlePointerUp(e) {
        e.preventDefault();

        // Release pointer capture
        try {
            this.canvas.releasePointerCapture(e.pointerId);
        } catch (err) {
            // Ignore release errors
        }

        const { x, y } = this.normalizeCoords(e.clientX, e.clientY);

        if (e.pointerType === 'pen') {
            if (this.penDown) {
                this.penDown = false;
                this.send({
                    type: 'PenUp',
                    x,
                    y
                });
            }
        } else if (e.pointerType === 'touch') {
            this.activeTouches.delete(e.pointerId);
            this.send({
                type: 'TouchEnd',
                id: e.pointerId % 10
            });
        } else {
            // Mouse
            this.send({
                type: 'MouseUp',
                button: e.button,
                x,
                y
            });
        }
    }

    handlePointerCancel(e) {
        e.preventDefault();

        if (e.pointerType === 'pen') {
            if (this.penDown) {
                this.penDown = false;
                const { x, y } = this.normalizeCoords(e.clientX, e.clientY);
                this.send({
                    type: 'PenUp',
                    x,
                    y
                });
            }
        } else if (e.pointerType === 'touch') {
            this.activeTouches.delete(e.pointerId);
            this.send({
                type: 'TouchCancel',
                id: e.pointerId % 10
            });
        }
    }

    handlePointerEnter(e) {
        if (e.pointerType === 'pen') {
            this.penActive = true;
            // Start hover tracking
            const { x, y } = this.normalizeCoords(e.clientX, e.clientY);
            this.send({
                type: 'PenHover',
                x,
                y,
                pressure: 0,
                tilt_x: e.tiltX || 0,
                tilt_y: e.tiltY || 0
            });
        }
    }

    handlePointerLeave(e) {
        if (e.pointerType === 'pen') {
            this.penActive = false;
            // Pen left the canvas - will be handled by server
        }
    }

    handleTouchStart(e) {
        e.preventDefault();

        for (const touch of e.changedTouches) {
            const { x, y } = this.normalizeCoords(touch.clientX, touch.clientY);
            this.activeTouches.set(touch.identifier, { x, y });

            this.send({
                type: 'TouchStart',
                id: touch.identifier,
                x,
                y
            });
        }
    }

    handleTouchMove(e) {
        e.preventDefault();

        for (const touch of e.changedTouches) {
            const { x, y } = this.normalizeCoords(touch.clientX, touch.clientY);
            this.activeTouches.set(touch.identifier, { x, y });

            this.send({
                type: 'TouchMove',
                id: touch.identifier,
                x,
                y
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

    handleMouseDown(e) {
        const { x, y } = this.normalizeCoords(e.clientX, e.clientY);

        this.send({
            type: 'MouseDown',
            button: e.button,
            x,
            y
        });
    }

    handleMouseMove(e) {
        // Only send if a button is pressed (drag)
        if (e.buttons === 0) return;

        const { x, y } = this.normalizeCoords(e.clientX, e.clientY);

        this.send({
            type: 'MouseMove',
            x,
            y
        });
    }

    handleMouseUp(e) {
        const { x, y } = this.normalizeCoords(e.clientX, e.clientY);

        this.send({
            type: 'MouseUp',
            button: e.button,
            x,
            y
        });
    }

    handleWheel(e) {
        e.preventDefault();

        this.send({
            type: 'Scroll',
            dx: e.deltaX,
            dy: e.deltaY
        });
    }
}

// Initialize input handler when DOM is ready
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => new InputHandler());
} else {
    new InputHandler();
}

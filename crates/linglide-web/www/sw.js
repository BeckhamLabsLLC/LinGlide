/**
 * LinGlide Service Worker
 *
 * Provides offline caching for static assets and the pairing UI.
 */

const CACHE_NAME = 'linglide-v2';

// Assets to cache for offline use
const STATIC_ASSETS = [
    '/',
    '/index.html',
    '/css/base.css',
    '/css/pairing.css',
    '/css/viewer.css',
    '/css/controls.css',
    '/js/app.js',
    '/js/state.js',
    '/js/storage.js',
    '/js/api.js',
    '/js/pairing/pairing-controller.js',
    '/js/pairing/pin-entry.js',
    '/js/pairing/qr-scanner.js',
    '/js/viewer/viewer.js',
    '/js/viewer/stats.js',
    '/js/viewer/input.js',
    '/js/components/status-bar.js',
    '/js/components/settings-panel.js',
    '/js/components/about-panel.js',
    '/js/components/toast.js',
    '/manifest.json'
];

// Install event - cache static assets
self.addEventListener('install', (event) => {
    console.log('[SW] Installing service worker');

    event.waitUntil(
        caches.open(CACHE_NAME).then((cache) => {
            console.log('[SW] Caching static assets');
            return cache.addAll(STATIC_ASSETS);
        }).then(() => {
            // Activate immediately
            return self.skipWaiting();
        })
    );
});

// Activate event - clean up old caches
self.addEventListener('activate', (event) => {
    console.log('[SW] Activating service worker');

    event.waitUntil(
        caches.keys().then((cacheNames) => {
            return Promise.all(
                cacheNames
                    .filter((name) => name !== CACHE_NAME)
                    .map((name) => {
                        console.log('[SW] Deleting old cache:', name);
                        return caches.delete(name);
                    })
            );
        }).then(() => {
            // Take control of all clients immediately
            return self.clients.claim();
        })
    );
});

// Fetch event - serve from cache, fallback to network
self.addEventListener('fetch', (event) => {
    const url = new URL(event.request.url);

    // Skip non-GET requests
    if (event.request.method !== 'GET') {
        return;
    }

    // Skip API requests - these should always go to network
    if (url.pathname.startsWith('/api/') || url.pathname.startsWith('/ws/')) {
        return;
    }

    // Skip cross-origin requests
    if (url.origin !== self.location.origin) {
        return;
    }

    event.respondWith(
        caches.match(event.request).then((cachedResponse) => {
            if (cachedResponse) {
                // Return cached response
                return cachedResponse;
            }

            // Fetch from network
            return fetch(event.request).then((networkResponse) => {
                // Don't cache non-successful responses
                if (!networkResponse || networkResponse.status !== 200) {
                    return networkResponse;
                }

                // Clone and cache the response
                const responseToCache = networkResponse.clone();
                caches.open(CACHE_NAME).then((cache) => {
                    cache.put(event.request, responseToCache);
                });

                return networkResponse;
            }).catch(() => {
                // Network failed, try to return a cached fallback
                return caches.match('/').then((fallback) => {
                    return fallback || new Response('Offline', {
                        status: 503,
                        statusText: 'Service Unavailable'
                    });
                });
            });
        })
    );
});

// Handle messages from the main thread
self.addEventListener('message', (event) => {
    if (event.data && event.data.type === 'SKIP_WAITING') {
        self.skipWaiting();
    }
});

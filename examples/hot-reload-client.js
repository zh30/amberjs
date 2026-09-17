/**
 * Amber Hot Reload Client
 *
 * This script provides browser-side hot reload functionality.
 * It connects to the Amber WebSocket server and reloads the page
 * when file changes are detected.
 *
 * Usage:
 *   1. Start amberjs in watch mode: amber run app.js --watch
 *   2. Include this script in your HTML page
 *   3. The page will automatically reload when files change
 *
 * Example HTML:
 *   <script src="hot-reload-client.js"></script>
 *   <script>
 *     new AmberHotReload({
 *       port: 9999,
 *       onReload: (event) => console.log('Reloading:', event),
 *       onError: (error) => console.error('Hot reload error:', error)
 *     });
 *   </script>
 */

class AmberHotReload {
  /**
   * Create a new hot reload client
   * @param {Object} options - Configuration options
   * @param {number} [options.port=9999] - WebSocket server port
   * @param {string} [options.host='location.hostname'] - WebSocket server host
   * @param {Function} [options.onReload] - Callback when reload event received
   * @param {Function} [options.onError] - Callback on connection error
   * @param {Function} [options.onConnect] - Callback on successful connection
   * @param {boolean] [options.autoReload=true] - Auto reload page on change
   * @param {boolean] [options.showNotifications=true] - Show browser notifications
   */
  constructor(options = {}) {
    this.port = options.port || 9999;
    this.host = options.host || (typeof location !== 'undefined' ? location.hostname : 'localhost');
    this.onReload = options.onReload || (() => {});
    this.onError = options.onError || ((err) => console.error('[amberjs] Hot reload error:', err));
    this.onConnect = options.onConnect || (() => {});
    this.autoReload = options.autoReload !== false;
    this.showNotifications = options.showNotifications !== false;
    this.socket = null;
    this.reconnectAttempts = 0;
    this.maxReconnectAttempts = 10;
    this.reconnectDelay = 1000;

    this.connect();
  }

  /**
   * Connect to the WebSocket server
   */
  connect() {
    const wsUrl = `ws://${this.host}:${this.port}`;

    console.log(`[amberjs] Connecting to hot reload server: ${wsUrl}`);

    try {
      this.socket = new WebSocket(wsUrl);

      this.socket.onopen = () => {
        console.log('[amberjs] Hot reload connected');
        this.reconnectAttempts = 0;

        if (this.showNotifications && typeof Notification !== 'undefined' && Notification.permission === 'granted') {
          this.showNotification('Hot Reload Connected', 'Ready to watch for changes');
        }

        if (this.onConnect) {
          this.onConnect({ host: this.host, port: this.port });
        }
      };

      this.socket.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);
          this.handleMessage(data);
        } catch (err) {
          console.error('[amberjs] Failed to parse hot reload message:', err);
        }
      };

      this.socket.onclose = (event) => {
        console.log('[amberjs] Hot reload disconnected:', event.code, event.reason);
        this.scheduleReconnect();
      };

      this.socket.onerror = (error) => {
        console.error('[amberjs] Hot reload WebSocket error:', error);
        if (this.onError) {
          this.onError(error);
        }
      };
    } catch (err) {
      console.error('[amberjs] Failed to create WebSocket:', err);
      this.scheduleReconnect();
    }
  }

  /**
   * Handle incoming WebSocket messages
   * @param {Object} data - Parsed message data
   */
  handleMessage(data) {
    switch (data.event_type) {
      case 'reload':
        console.log('[amberjs] File changed:', data.file_path);

        if (this.onReload) {
          this.onReload(data);
        }

        if (this.autoReload) {
          this.reloadPage(data);
        }
        break;

      case 'error':
        console.error('[amberjs] Hot reload error:', data.message);
        if (this.onError) {
          this.onError(new Error(data.message));
        }
        break;

      case 'status':
        console.log('[amberjs] Hot reload status:', data.message);
        break;

      default:
        console.log('[amberjs] Unknown hot reload event:', data);
    }
  }

  /**
   * Reload the page
   * @param {Object} event - The reload event data
   */
  reloadPage(event) {
    console.log('[amberjs] Reloading page...', event.file_path);

    if (this.showNotifications && typeof Notification !== 'undefined' && Notification.permission === 'granted') {
      this.showNotification('Reloading', `File changed: ${event.file_path || 'unknown'}`);
    }

    // Small delay to allow notification to show
    setTimeout(() => {
      window.location.reload();
    }, 100);
  }

  /**
   * Schedule a reconnection attempt
   */
  scheduleReconnect() {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.log('[amberjs] Max reconnect attempts reached, giving up');
      return;
    }

    this.reconnectAttempts++;
    const delay = this.reconnectDelay * Math.pow(1.5, this.reconnectAttempts - 1);

    console.log(`[amberjs] Reconnecting in ${Math.round(delay)}ms (attempt ${this.reconnectAttempts}/${this.maxReconnectAttempts})`);

    setTimeout(() => {
      this.connect();
    }, delay);
  }

  /**
   * Show a browser notification
   * @param {string} title - Notification title
   * @param {string} body - Notification body text
   */
  showNotification(title, body) {
    if (typeof Notification !== 'undefined' && Notification.permission === 'granted') {
      new Notification(title, { body, icon: '/favicon.ico' });
    }
  }

  /**
   * Request notification permission
   * @returns {Promise<boolean>} - Whether permission was granted
   */
  async requestNotificationPermission() {
    if (typeof Notification !== 'undefined' && Notification.permission === 'default') {
      const permission = await Notification.requestPermission();
      return permission === 'granted';
    }
    return Notification.permission === 'granted';
  }

  /**
   * Disconnect from the WebSocket server
   */
  disconnect() {
    if (this.socket) {
      this.socket.close();
      this.socket = null;
    }
  }
}

// Export for module usage
if (typeof module !== 'undefined' && module.exports) {
  module.exports = AmberHotReload;
}

// Auto-initialize if running in browser and autoInit option is set
if (typeof window !== 'undefined') {
  window.AmberHotReload = AmberHotReload;

  // Auto-initialize with data attributes from script tag
  const script = document.currentScript;
  if (script && script.dataset.autoInit !== 'false') {
    const config = {
      port: parseInt(script.dataset.port || '9999', 10),
      host: script.dataset.host || undefined,
      autoReload: script.dataset.autoReload !== 'false',
      showNotifications: script.dataset.showNotifications !== 'false'
    };

    // Request notification permission
    if (typeof Notification !== 'undefined' && Notification.permission === 'default') {
      Notification.requestPermission();
    }

    window.hotReload = new AmberHotReload(config);
  }
}

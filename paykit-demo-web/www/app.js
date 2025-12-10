// Paykit Web Demo Application

import init, {
    Identity,
    BrowserStorage,
    DirectoryClient,
    WasmPaymentRequest,
    WasmRequestStorage,
    WasmSubscriptionAgreementStorage,
    WasmSubscription,
    WasmContact,
    WasmContactStorage,
    WasmPaymentMethodConfig,
    WasmPaymentMethodStorage,
    WasmReceiptStorage,
    WasmPaymentCoordinator,
    WasmDashboard,
    WasmAutoPayRule,
    WasmAutoPayRuleStorage,
    WasmPeerSpendingLimit,
    WasmPeerSpendingLimitStorage,
    format_timestamp,
    is_valid_pubkey,
    version
} from './pkg/paykit_demo_web.js';

// Global state
let storage;
let currentIdentity = null;
let requestStorage;
let subscriptionStorage;
let contactStorage;
let paymentMethodStorage;
let receiptStorage;
let paymentCoordinator;
let dashboard;
let autopayRuleStorage;
let peerLimitStorage;


// Initialize the WASM module
async function initializeApp() {
    try {
        await init();
        console.log('Paykit WASM initialized');
        
        // Verify required classes are available
        if (typeof WasmAutoPayRuleStorage === 'undefined') {
            console.error('WasmAutoPayRuleStorage is not available');
        }
        if (typeof WasmPeerSpendingLimitStorage === 'undefined') {
            console.error('WasmPeerSpendingLimitStorage is not available');
        }
        
        // Initialize storage
        storage = new BrowserStorage();
        requestStorage = new WasmRequestStorage(null);
        subscriptionStorage = new WasmSubscriptionAgreementStorage();
        contactStorage = new WasmContactStorage();
        paymentMethodStorage = new WasmPaymentMethodStorage();
        receiptStorage = new WasmReceiptStorage();
        paymentCoordinator = new WasmPaymentCoordinator();
        dashboard = new WasmDashboard();
        
        // Initialize auto-pay and spending limits storage
        try {
            if (typeof WasmAutoPayRuleStorage !== 'undefined') {
                autopayRuleStorage = new WasmAutoPayRuleStorage();
            } else {
                console.warn('WasmAutoPayRuleStorage not available - auto-pay features disabled');
            }
            if (typeof WasmPeerSpendingLimitStorage !== 'undefined') {
                peerLimitStorage = new WasmPeerSpendingLimitStorage();
            } else {
                console.warn('WasmPeerSpendingLimitStorage not available - spending limits disabled');
            }
        } catch (err) {
            console.error('Failed to initialize auto-pay/limit storage:', err);
            // Continue anyway - these features will show errors if used
        }
        
        // Load current identity if it exists
        await loadCurrentIdentity();
        
        // Update UI
        updateIdentityDisplay();
        updateIdentityList();
        await updateRequestsList();
        await updateSubscriptionsList();
        
        // Update auto-pay and limits (may fail if storage not initialized)
        try {
            await updateAutoPayRulesList();
            await updatePeerLimitsList();
        } catch (err) {
            console.warn('Failed to update auto-pay/limits lists:', err);
        }
        
        await updateContactsList();
        await updatePaymentMethodsList();
        await updateReceiptsList();
        await updateDashboard();
        
        // Set version
        document.getElementById('wasm-version').textContent = version();
        
        showNotification('Paykit initialized successfully', 'success');
    } catch (error) {
        console.error('Initialization error:', error);
        handleError(error, 'Initialization');
        // Show error in UI
        const errorMsg = error.message || String(error);
        showNotification('Initialization failed: ' + errorMsg, 'error');
    }
}

// Identity Management
async function loadCurrentIdentity() {
    try {
        const currentName = storage.getCurrentIdentity();
        if (currentName) {
            currentIdentity = storage.loadIdentity(currentName);
            console.log('Loaded identity:', currentName);
        }
    } catch (error) {
        console.log('No current identity');
    }
}

function updateIdentityDisplay() {
    const displayEl = document.getElementById('current-identity');
    
    if (currentIdentity) {
        const nickname = currentIdentity.nickname();
        displayEl.innerHTML = `
            ${nickname ? `<div class="label">Nickname</div><div class="value">${nickname}</div>` : ''}
            <div class="label">Public Key</div>
            <div class="value">${currentIdentity.publicKey()}</div>
            <div class="label">Pubky URI</div>
            <div class="value">${currentIdentity.pubkyUri()}</div>
        `;
        document.getElementById('export-btn').disabled = false;
        document.getElementById('initiate-payment-btn').disabled = false;
    } else {
        displayEl.innerHTML = '<p class="empty-state">No identity selected. Create or load one below.</p>';
        document.getElementById('export-btn').disabled = true;
        const paymentBtn = document.getElementById('initiate-payment-btn');
        if (paymentBtn) paymentBtn.disabled = true;
    }
}

async function updateIdentityList() {
    const listEl = document.getElementById('identity-list');
    
    try {
        const names = storage.listIdentities();
        
        if (names.length === 0) {
            listEl.innerHTML = '<p class="empty-state">No saved identities</p>';
            return;
        }
        
        const currentName = storage.getCurrentIdentity();
        
        listEl.innerHTML = names.map(name => {
            try {
                const identity = storage.loadIdentity(name);
                const isActive = name === currentName;
                
                return `
                    <div class="identity-item ${isActive ? 'active' : ''}">
                        <div class="identity-item-info">
                            <div class="identity-item-name">${name}</div>
                            <div class="identity-item-key">${identity.publicKey().substring(0, 40)}...</div>
                        </div>
                        <div class="identity-item-actions">
                            ${!isActive ? `<button class="btn-sm" onclick="loadIdentity('${name}')">Load</button>` : '<span style="color: var(--success)">✓ Active</span>'}
                            <button class="btn-sm" onclick="deleteIdentity('${name}')" style="background: var(--error)">Delete</button>
                        </div>
                    </div>
                `;
            } catch (e) {
                return '';
            }
        }).join('');
    } catch (error) {
        listEl.innerHTML = '<p class="empty-state">Error loading identities</p>';
    }
}

// Identity Actions
function generateIdentity() {
    try {
        const nickname = document.getElementById('nickname-input').value.trim();
        
        let identity;
        if (nickname) {
            identity = Identity.withNickname(nickname);
        } else {
            identity = new Identity();
        }
        
        // Save it
        const name = nickname || `identity_${Date.now()}`;
        storage.saveIdentity(name, identity);
        storage.setCurrentIdentity(name);
        
        currentIdentity = identity;
        
        updateIdentityDisplay();
        updateIdentityList();
        
        document.getElementById('nickname-input').value = '';
        
        showNotification(`Identity "${name}" created successfully`, 'success');
    } catch (error) {
        handleError(error, 'Identity generation');
    }
}

function exportIdentity() {
    if (!currentIdentity) {
        showNotification('No identity to export', 'error');
        return;
    }
    
    try {
        const json = currentIdentity.toJSON();
        
        // Copy to clipboard
        navigator.clipboard.writeText(json).then(() => {
            showNotification('Identity JSON copied to clipboard', 'success');
        }).catch(() => {
            // Fallback: show in textarea
            document.getElementById('import-textarea').value = json;
            showNotification('Identity JSON displayed below', 'info');
        });
    } catch (error) {
        handleError(error, 'Identity export');
    }
}

function importIdentity() {
    try {
        const json = document.getElementById('import-textarea').value.trim();
        if (!json) {
            showNotification('Please paste identity JSON', 'error');
            return;
        }
        
        const identity = Identity.fromJSON(json);
        const nickname = identity.nickname();
        const name = nickname || `imported_${Date.now()}`;
        
        storage.saveIdentity(name, identity);
        storage.setCurrentIdentity(name);
        
        currentIdentity = identity;
        
        updateIdentityDisplay();
        updateIdentityList();
        
        document.getElementById('import-textarea').value = '';
        
        showNotification(`Identity "${name}" imported successfully`, 'success');
    } catch (error) {
        console.error('Failed to import identity:', error);
        showNotification('Failed to import identity: ' + error.message, 'error');
    }
}

window.loadIdentity = function(name) {
    try {
        currentIdentity = storage.loadIdentity(name);
        storage.setCurrentIdentity(name);
        
        updateIdentityDisplay();
        updateIdentityList();
        
        showNotification(`Switched to identity "${name}"`, 'success');
    } catch (error) {
        showNotification('Failed to load identity: ' + error.message, 'error');
    }
};

window.deleteIdentity = function(name) {
    if (!confirm(`Delete identity "${name}"?`)) {
        return;
    }
    
    try {
        storage.deleteIdentity(name);
        
        const currentName = storage.getCurrentIdentity();
        if (currentName === name) {
            currentIdentity = null;
            updateIdentityDisplay();
        }
        
        updateIdentityList();
        
        showNotification(`Identity "${name}" deleted`, 'success');
    } catch (error) {
        showNotification('Failed to delete identity: ' + error.message, 'error');
    }
};

// Directory Operations
async function queryDirectory() {
    const uriInput = document.getElementById('pubky-uri-input').value.trim();
    const homeserverInput = document.getElementById('homeserver-input').value.trim();
    const resultsEl = document.getElementById('query-results');
    
    if (!uriInput) {
        showNotification('Please enter a Pubky URI', 'error');
        return;
    }
    
    resultsEl.innerHTML = '<p class="empty-state">Querying...</p>';
    
    try {
        // Extract public key from URI
        const publicKey = uriInput.replace('pubky://', '');
        
        const client = new DirectoryClient(homeserverInput);
        const results = await client.queryMethods(publicKey);
        
        if (results && typeof results === 'object') {
            const entries = Object.entries(results);
            
            if (entries.length === 0) {
                resultsEl.innerHTML = '<p class="empty-state">No payment methods found</p>';
            } else {
                resultsEl.innerHTML = entries.map(([method, endpoint]) => `
                    <div class="result-item">
                        <div class="result-item-method">${method}</div>
                        <div class="result-item-endpoint">${endpoint}</div>
                    </div>
                `).join('');
            }
            
            showNotification(`Found ${entries.length} payment method(s)`, 'success');
        } else {
            resultsEl.innerHTML = '<p class="empty-state">No payment methods found</p>';
        }
    } catch (error) {
        console.error('Query failed:', error);
        resultsEl.innerHTML = '<p class="empty-state">Query failed. See console for details.</p>';
        showNotification('Query failed: ' + error.message, 'error');
    }
}

window.tryExample = function(uri) {
    document.getElementById('pubky-uri-input').value = uri;
    switchTab('directory');
    setTimeout(() => {
        document.getElementById('pubky-uri-input').focus();
    }, 100);
};

// Payment Operations with Real-time Status Updates
let paymentStatusInterval = null;

/**
 * Parse a noise:// endpoint string to extract WebSocket URL and server key
 * Format: noise://host:port@pubkey_hex
 * Returns: { wsUrl: string, serverKeyHex: string, host: string, port: number }
 */
function parseNoiseEndpoint(endpoint) {
    if (!endpoint.startsWith('noise://')) {
        throw new Error('Endpoint must start with "noise://"');
    }
    
    const withoutPrefix = endpoint.substring(8); // Remove "noise://"
    const atIndex = withoutPrefix.indexOf('@');
    
    if (atIndex === -1) {
        throw new Error('Invalid Noise endpoint format. Expected: noise://host:port@pubkey_hex');
    }
    
    const hostPort = withoutPrefix.substring(0, atIndex);
    const serverKeyHex = withoutPrefix.substring(atIndex + 1);
    
    // Validate server key is 64 hex characters (32 bytes)
    if (!/^[0-9a-fA-F]{64}$/.test(serverKeyHex)) {
        throw new Error('Server key must be 64 hex characters (32 bytes)');
    }
    
    // Parse host and port
    const colonIndex = hostPort.lastIndexOf(':');
    if (colonIndex === -1) {
        throw new Error('Invalid host:port format');
    }
    
    const host = hostPort.substring(0, colonIndex);
    const port = parseInt(hostPort.substring(colonIndex + 1), 10);
    
    if (isNaN(port) || port < 1 || port > 65535) {
        throw new Error('Invalid port number');
    }
    
    // Convert to WebSocket URL
    // Use wss:// for non-localhost, ws:// for localhost
    const protocol = (host === 'localhost' || host === '127.0.0.1' || host.startsWith('192.168.') || host.startsWith('10.') || host.startsWith('172.')) 
        ? 'ws' 
        : 'wss';
    const wsUrl = `${protocol}://${host}:${port}`;
    
    return {
        wsUrl,
        serverKeyHex,
        host,
        port
    };
}

/**
 * Extract public key from recipient URI or contact name
 * Supports: pubky://... or raw public key or contact name
 * Returns: public key string
 */
async function extractPayeePubkey(recipientUri) {
    // If it's a pubky:// URI, extract the key
    if (recipientUri.startsWith('pubky://')) {
        return recipientUri.substring(8); // Remove "pubky://"
    }
    
    // If it looks like a raw public key (long string), return as-is
    if (recipientUri.length > 40 && is_valid_pubkey(recipientUri)) {
        return recipientUri;
    }
    
    // Try to resolve as contact name
    try {
        const contacts = await contactStorage.list_contacts();
        for (const contact of contacts) {
            if (contact.name === recipientUri) {
                return contact.public_key;
            }
        }
    } catch (error) {
        console.warn('Failed to search contacts:', error);
    }
    
    // If not found as contact, assume it's a public key
    if (is_valid_pubkey(recipientUri)) {
        return recipientUri;
    }
    
    throw new Error(`Could not resolve recipient: ${recipientUri}. Please provide a pubky:// URI, public key, or contact name.`);
}

/**
 * Discover recipient's payment endpoint via directory
 * Returns: noise:// endpoint string or null if not found
 */
async function discoverRecipientEndpoint(recipientUri, homeserver) {
    try {
        // Extract payee public key
        const payeePubkey = await extractPayeePubkey(recipientUri);
        
        // Query directory for payment methods
        const client = new DirectoryClient(homeserver);
        const methods = await client.queryMethods(payeePubkey);
        
        if (!methods || typeof methods !== 'object') {
            return null;
        }
        
        // Look for noise:// endpoints
        const entries = Object.entries(methods);
        for (const [methodId, endpoint] of entries) {
            if (typeof endpoint === 'string' && endpoint.startsWith('noise://')) {
                return endpoint;
            }
        }
        
        return null;
    } catch (error) {
        console.error('Failed to discover endpoint:', error);
        throw new Error(`Failed to discover payment endpoint: ${error.message}`);
    }
}

function updatePaymentStatus(status, message, details = {}) {
    const statusEl = document.getElementById('payment-status');
    if (!statusEl) return;
    
    const statusIcons = {
        'preparing': '⏳',
        'connecting': '🔌',
        'negotiating': '🤝',
        'processing': '⚙️',
        'success': '✅',
        'error': '❌',
        'cancelled': '🚫'
    };
    
    const icon = statusIcons[status] || '⏳';
    
    statusEl.innerHTML = `
        <div class="status-item">
            <span class="status-label">Status</span>
            <span class="status-value ${status}">${icon} ${message}</span>
        </div>
        ${details.payer ? `<div class="status-item">
            <span class="status-label">Payer</span>
            <span class="status-value">${details.payer.substring(0, 20)}...</span>
        </div>` : ''}
        ${details.payee ? `<div class="status-item">
            <span class="status-label">Payee</span>
            <span class="status-value">${details.payee.substring(0, 30)}...</span>
        </div>` : ''}
        ${details.amount ? `<div class="status-item">
            <span class="status-label">Amount</span>
            <span class="status-value">${details.amount} ${details.currency || 'SAT'}</span>
        </div>` : ''}
        ${details.method ? `<div class="status-item">
            <span class="status-label">Method</span>
            <span class="status-value">${details.method}</span>
        </div>` : ''}
        ${details.progress ? `<div class="status-item">
            <span class="status-label">Progress</span>
            <div class="progress-bar" style="width: ${details.progress}%"></div>
        </div>` : ''}
        ${details.error ? `<div class="status-item">
            <span class="status-label">Error</span>
            <span class="status-value error-detail">${details.error}</span>
        </div>` : ''}
    `;
    
    // Announce status change for screen readers
    statusEl.setAttribute('aria-live', 'polite');
    statusEl.setAttribute('role', 'status');
}

async function initiatePayment() {
    const recipient = document.getElementById('recipient-input').value.trim();
    const amount = document.getElementById('amount-input').value;
    const currency = document.getElementById('currency-input').value.trim() || 'SAT';
    const method = document.getElementById('method-select').value;
    const homeserver = document.getElementById('homeserver-input')?.value || 'https://demo.httprelay.io';
    
    // Clear any existing interval
    if (paymentStatusInterval) {
        clearInterval(paymentStatusInterval);
        paymentStatusInterval = null;
    }
    
    // Validate inputs
    if (!currentIdentity) {
        showNotification('Please create or load an identity first', 'error');
        updatePaymentStatus('error', 'No identity loaded', {});
        return;
    }
    
    if (!recipient) {
        showNotification('Please enter a recipient URI', 'error');
        updatePaymentStatus('error', 'Recipient required', {});
        return;
    }
    
    if (!amount || parseFloat(amount) <= 0) {
        showNotification('Please enter a valid amount', 'error');
        updatePaymentStatus('error', 'Invalid amount', {});
        return;
    }
    
    // Disable payment button during processing
    const paymentBtn = document.getElementById('initiate-payment-btn');
    const originalDisabled = paymentBtn.disabled;
    paymentBtn.disabled = true;
    
    try {
        // Step 1: Extract payee public key
        updatePaymentStatus('preparing', 'Resolving recipient...', {
            payer: currentIdentity.publicKey(),
            payee: recipient,
            amount: amount,
            currency: currency,
            method: method,
            progress: 10
        });
        
        let payeePubkey;
        try {
            payeePubkey = await extractPayeePubkey(recipient);
        } catch (error) {
            throw new Error(`Failed to resolve recipient: ${error.message}`);
        }
        
        // Step 2: Discover payment endpoint
        updatePaymentStatus('preparing', 'Discovering payment endpoint...', {
            payer: currentIdentity.publicKey(),
            payee: payeePubkey,
            amount: amount,
            currency: currency,
            method: method,
            progress: 20
        });
        
        let noiseEndpoint;
        try {
            noiseEndpoint = await discoverRecipientEndpoint(recipient, homeserver);
        } catch (error) {
            throw new Error(`Failed to discover endpoint: ${error.message}`);
        }
        
        if (!noiseEndpoint) {
            // Check if any endpoint was found (but not noise://)
            try {
                const payeePubkeyForCheck = await extractPayeePubkey(recipient);
                const client = new DirectoryClient(homeserver);
                const methods = await client.queryMethods(payeePubkeyForCheck);
                
                if (methods && typeof methods === 'object' && Object.keys(methods).length > 0) {
                    const endpoints = Object.values(methods).filter(e => typeof e === 'string');
                    updatePaymentStatus('error', 'No interactive endpoint found', {
                        payer: currentIdentity.publicKey(),
                        payee: payeePubkey,
                        amount: amount,
                        currency: currency,
                        method: method,
                        progress: 100,
                        error: `Found ${endpoints.length} endpoint(s) but none are noise://. For interactive payments, recipient must publish a noise:// endpoint.`
                    });
                    showNotification('Endpoint found but not interactive (noise://). For interactive payments, recipient must publish a noise:// endpoint.', 'warning');
                    return;
                }
            } catch (e) {
                // Ignore errors in fallback check
            }
            
            updatePaymentStatus('error', 'No payment endpoint found', {
                payer: currentIdentity.publicKey(),
                payee: payeePubkey,
                amount: amount,
                currency: currency,
                method: method,
                progress: 100,
                error: 'Recipient has not published any payment endpoints. They need to publish a noise:// endpoint for interactive payments.'
            });
            showNotification('No payment endpoint found for recipient. They need to publish a noise:// endpoint.', 'error');
            return;
        }
        
        // Step 3: Parse noise endpoint
        updatePaymentStatus('preparing', 'Parsing endpoint...', {
            payer: currentIdentity.publicKey(),
            payee: payeePubkey,
            amount: amount,
            currency: currency,
            method: method,
            progress: 30
        });
        
        let endpointData;
        try {
            endpointData = parseNoiseEndpoint(noiseEndpoint);
        } catch (error) {
            throw new Error(`Invalid endpoint format: ${error.message}`);
        }
        
        // Step 4: Connect and perform handshake
        updatePaymentStatus('connecting', 'Connecting to recipient...', {
            payer: currentIdentity.publicKey(),
            payee: payeePubkey,
            amount: amount,
            currency: currency,
            method: method,
            progress: 40
        });
        
        // Step 5: Send payment request
        updatePaymentStatus('negotiating', 'Performing Noise handshake...', {
            payer: currentIdentity.publicKey(),
            payee: payeePubkey,
            amount: amount,
            currency: currency,
            method: method,
            progress: 50
        });
        
        // Get identity JSON
        const payerIdentityJson = currentIdentity.toJSON();
        
        // Step 6: Initiate payment via WasmPaymentCoordinator
        updatePaymentStatus('processing', 'Sending payment request...', {
            payer: currentIdentity.publicKey(),
            payee: payeePubkey,
            amount: amount,
            currency: currency,
            method: method,
            progress: 60
        });
        
        let receiptJson;
        try {
            receiptJson = await paymentCoordinator.initiate_payment(
                payerIdentityJson,
                endpointData.wsUrl,
                payeePubkey,
                endpointData.serverKeyHex,
                amount,
                currency,
                method
            );
        } catch (error) {
            // Parse error message
            let errorMsg = error.message || String(error);
            
            // Provide helpful error messages
            if (errorMsg.includes('Connection failed') || errorMsg.includes('WebSocket')) {
                errorMsg = `Connection failed: ${errorMsg}. Make sure the recipient's WebSocket server is running and accessible.`;
            } else if (errorMsg.includes('handshake') || errorMsg.includes('Noise')) {
                errorMsg = `Noise protocol error: ${errorMsg}. Check that server key is correct.`;
            } else if (errorMsg.includes('Payment error')) {
                // Error from payee
                errorMsg = `Payment rejected: ${errorMsg}`;
            }
            
            updatePaymentStatus('error', 'Payment failed', {
                payer: currentIdentity.publicKey(),
                payee: payeePubkey,
                amount: amount,
                currency: currency,
                method: method,
                progress: 100,
                error: errorMsg
            });
            showNotification(errorMsg, 'error');
            return;
        }
        
        // Step 7: Parse and store receipt
        updatePaymentStatus('processing', 'Receiving confirmation...', {
            payer: currentIdentity.publicKey(),
            payee: payeePubkey,
            amount: amount,
            currency: currency,
            method: method,
            progress: 80
        });
        
        // Receipt is already stored by WasmPaymentCoordinator, just refresh the list
        await updateReceiptsList();
        
        // Step 8: Success
        updatePaymentStatus('success', 'Payment completed successfully', {
            payer: currentIdentity.publicKey(),
            payee: payeePubkey,
            amount: amount,
            currency: currency,
            method: method,
            progress: 100
        });
        
        showNotification('Payment completed successfully!', 'success');
        
        // Update dashboard
        await updateDashboard();
        
    } catch (error) {
        console.error('Payment error:', error);
        updatePaymentStatus('error', 'Payment failed', {
            payer: currentIdentity?.publicKey() || 'unknown',
            payee: recipient,
            amount: amount,
            currency: currency,
            method: method,
            progress: 100,
            error: error.message || String(error)
        });
        showNotification(`Payment failed: ${error.message || String(error)}`, 'error');
    } finally {
        // Re-enable payment button
        paymentBtn.disabled = originalDisabled;
    }
}

// Payment form validation
function validatePaymentForm() {
    const recipientInput = document.getElementById('recipient-input');
    const amountInput = document.getElementById('amount-input');
    const currencyInput = document.getElementById('currency-input');
    const paymentBtn = document.getElementById('initiate-payment-btn');
    
    if (!recipientInput || !amountInput || !currencyInput || !paymentBtn) {
        return;
    }
    
    const recipient = recipientInput.value.trim();
    const amount = amountInput.value.trim();
    const currency = currencyInput.value.trim();
    
    let isValid = true;
    let errors = [];
    
    // Validate recipient
    if (!recipient) {
        isValid = false;
        recipientInput.classList.add('error');
    } else {
        recipientInput.classList.remove('error');
    }
    
    // Validate amount
    const amountNum = parseFloat(amount);
    if (!amount || isNaN(amountNum) || amountNum <= 0) {
        isValid = false;
        amountInput.classList.add('error');
        if (amount && isNaN(amountNum)) {
            errors.push('Amount must be a number');
        } else if (amount && amountNum <= 0) {
            errors.push('Amount must be greater than 0');
        }
    } else {
        amountInput.classList.remove('error');
    }
    
    // Validate currency
    if (!currency || currency.length < 2 || currency.length > 10) {
        isValid = false;
        currencyInput.classList.add('error');
        if (currency && (currency.length < 2 || currency.length > 10)) {
            errors.push('Currency must be 2-10 characters');
        }
    } else {
        currencyInput.classList.remove('error');
    }
    
    // Update button state
    paymentBtn.disabled = !isValid || !currentIdentity;
    
    // Show validation errors (optional - can be enhanced with error display)
    if (errors.length > 0 && document.activeElement === amountInput) {
        // Only show errors when user is actively editing
    }
    
    return isValid;
}

// UI Management
function switchTab(tabName) {
    try {
        console.log('Switching to tab:', tabName);
        
        // Update tab buttons with ARIA attributes
        document.querySelectorAll('.tab').forEach(btn => {
            const isActive = btn.dataset.tab === tabName;
            btn.classList.toggle('active', isActive);
            btn.setAttribute('aria-selected', isActive ? 'true' : 'false');
        });
        
        // Update tab content with ARIA attributes
        document.querySelectorAll('.tab-content').forEach(content => {
            const isActive = content.id === `${tabName}-tab`;
            if (isActive) {
                content.classList.add('active');
                content.hidden = false;
            } else {
                content.classList.remove('active');
                content.hidden = true;
            }
        });
        
        // Focus management for accessibility
        const activeTab = document.querySelector(`.tab[data-tab="${tabName}"]`);
        if (activeTab) {
            activeTab.focus();
        }
        
        console.log('Tab switched successfully');
    } catch (error) {
        console.error('Error switching tab:', error);
    }
}

function showNotification(message, type = 'info', duration = 3000) {
    const notifEl = document.getElementById('notification');
    notifEl.textContent = message;
    notifEl.className = `notification ${type}`;
    
    // Accessibility
    notifEl.setAttribute('role', 'alert');
    notifEl.setAttribute('aria-live', type === 'error' ? 'assertive' : 'polite');
    
    // Trigger reflow to restart animation
    notifEl.offsetHeight;
    
    notifEl.classList.add('show');
    
    // Auto-hide after duration
    setTimeout(() => {
        notifEl.classList.remove('show');
    }, duration);
    
    // For errors, allow manual dismiss
    if (type === 'error') {
        notifEl.style.cursor = 'pointer';
        notifEl.title = 'Click to dismiss';
        const originalOnClick = notifEl.onclick;
        notifEl.onclick = () => {
            notifEl.classList.remove('show');
            notifEl.onclick = originalOnClick;
        };
    }
}

// Enhanced error handling with user-friendly messages
function handleError(error, context = 'operation') {
    console.error(`Error in ${context}:`, error);
    
    let userMessage = 'An unexpected error occurred';
    
    if (error.message) {
        const msg = error.message.toLowerCase();
        
        // Network errors
        if (msg.includes('network') || msg.includes('fetch') || msg.includes('websocket')) {
            userMessage = 'Network error: Please check your connection and try again';
        }
        // Storage errors
        else if (msg.includes('quota') || msg.includes('storage')) {
            userMessage = 'Storage limit reached: Please clear some data and try again';
        }
        // Validation errors
        else if (msg.includes('invalid') || msg.includes('validation')) {
            userMessage = `Invalid input: ${error.message}`;
        }
        // Permission errors
        else if (msg.includes('permission') || msg.includes('denied')) {
            userMessage = 'Permission denied: Please check browser settings';
        }
        // Use original message if it's user-friendly
        else if (error.message.length < 100) {
            userMessage = error.message;
        }
    }
    
    showNotification(`${context}: ${userMessage}`, 'error', 5000);
    return userMessage;
}

// Subscription Management
async function createPaymentRequest() {
    if (!currentIdentity) {
        showNotification('Please create or load an identity first', 'error');
        return;
    }

    const recipientInput = document.getElementById('request-recipient-input');
    const amountInput = document.getElementById('request-amount-input');
    const currencyInput = document.getElementById('request-currency-input');
    const descriptionInput = document.getElementById('request-description-input');
    const expiresInput = document.getElementById('request-expires-input');

    const recipient = recipientInput.value.trim();
    const amount = amountInput.value.trim();
    const currency = currencyInput.value.trim() || 'SAT';
    const description = descriptionInput.value.trim();
    const expiresHours = parseInt(expiresInput.value) || 24;

    if (!recipient || !amount) {
        showNotification('Recipient and amount are required', 'error');
        return;
    }

    if (!is_valid_pubkey(recipient)) {
        showNotification('Invalid recipient public key', 'error');
        return;
    }

    try {
        const fromKey = currentIdentity.publicKey();
        const expiresAt = Math.floor(Date.now() / 1000) + (expiresHours * 3600);

        let request = new WasmPaymentRequest(fromKey, recipient, amount, currency, 'lightning');
        
        if (description) {
            request = request.with_description(description);
        }
        
        request = request.with_expiration(expiresAt);

        await requestStorage.save_request(request);

        showNotification(`Payment request created: ${request.request_id}`, 'success');

        // Clear form
        recipientInput.value = '';
        amountInput.value = '';
        descriptionInput.value = '';

        // Refresh list
        await updateRequestsList();
    } catch (error) {
        console.error('Failed to create request:', error);
        showNotification('Failed to create request: ' + error.message, 'error');
    }
}

async function updateRequestsList() {
    const listEl = document.getElementById('requests-list');
    
    if (!listEl) return; // Element might not exist yet
    
    try {
        const requests = await requestStorage.list_requests();
        
        if (requests.length === 0) {
            listEl.innerHTML = '<p class="empty-state">No payment requests yet. Create one above.</p>';
            return;
        }

        listEl.innerHTML = requests.map(req => `
            <div class="request-item ${req.is_expired ? 'expired' : ''}">
                <div class="request-header">
                    <div class="request-id">Request: ${req.request_id.substring(0, 12)}...</div>
                    <div class="request-amount">${req.amount} ${req.currency}</div>
                </div>
                <div class="request-details">
                    <div class="request-field">
                        <span class="label">From:</span>
                        <span class="value">${req.from.substring(0, 20)}...</span>
                    </div>
                    <div class="request-field">
                        <span class="label">To:</span>
                        <span class="value">${req.to.substring(0, 20)}...</span>
                    </div>
                    ${req.description ? `<div class="request-field">
                        <span class="label">Description:</span>
                        <span class="value">${req.description}</span>
                    </div>` : ''}
                    <div class="request-field">
                        <span class="label">Created:</span>
                        <span class="value">${format_timestamp(req.created_at)}</span>
                    </div>
                    ${req.expires_at ? `<div class="request-field">
                        <span class="label">Expires:</span>
                        <span class="value">${format_timestamp(req.expires_at)} ${req.is_expired ? '⚠️ EXPIRED' : ''}</span>
                    </div>` : ''}
                </div>
                <div class="request-actions">
                    <button class="btn-sm" onclick="copyRequestId('${req.request_id}')">Copy ID</button>
                    <button class="btn-sm" style="background: var(--error)" onclick="deleteRequest('${req.request_id}')">Delete</button>
                </div>
            </div>
        `).join('');
    } catch (error) {
        console.error('Failed to load requests:', error);
        listEl.innerHTML = '<p class="error-state">Failed to load requests</p>';
    }
}

async function deleteRequest(requestId) {
    if (!confirm('Delete this payment request?')) return;
    
    try {
        await requestStorage.delete_request(requestId);
        showNotification('Request deleted', 'success');
        await updateRequestsList();
    } catch (error) {
        console.error('Failed to delete request:', error);
        showNotification('Failed to delete request: ' + error.message, 'error');
    }
}

async function clearAllRequests() {
    if (!confirm('Delete ALL payment requests? This cannot be undone.')) return;
    
    try {
        await requestStorage.clear_all();
        showNotification('All requests cleared', 'success');
        await updateRequestsList();
    } catch (error) {
        console.error('Failed to clear requests:', error);
        showNotification('Failed to clear requests: ' + error.message, 'error');
    }
}

function copyRequestId(requestId) {
    navigator.clipboard.writeText(requestId).then(() => {
        showNotification('Request ID copied to clipboard', 'success');
    }).catch(err => {
        console.error('Failed to copy:', err);
        showNotification('Failed to copy to clipboard', 'error');
    });
}

// Make functions global for HTML onclick handlers
window.deleteRequest = deleteRequest;
window.copyRequestId = copyRequestId;

// ============================================================
// Subscription Management
// ============================================================

async function createSubscription() {
    if (!currentIdentity) {
        showNotification('Please create or load an identity first', 'error');
        return;
    }

    const providerInput = document.getElementById('sub-provider-input');
    const amountInput = document.getElementById('sub-amount-input');
    const currencyInput = document.getElementById('sub-currency-input');
    const frequencySelect = document.getElementById('sub-frequency-select');
    const customIntervalInput = document.getElementById('sub-custom-interval');
    const descriptionInput = document.getElementById('sub-description-input');

    const provider = providerInput.value.trim();
    const amount = amountInput.value.trim();
    const currency = currencyInput.value.trim() || 'SAT';
    let frequency = frequencySelect.value;
    const description = descriptionInput.value.trim();

    if (!provider || !amount || !description) {
        showNotification('Provider, amount, and description are required', 'error');
        return;
    }

    if (!is_valid_pubkey(provider)) {
        showNotification('Invalid provider public key', 'error');
        return;
    }

    // Handle custom frequency
    if (frequency === 'custom') {
        const customInterval = customIntervalInput.value.trim();
        if (!customInterval) {
            showNotification('Custom interval is required', 'error');
            return;
        }
        frequency = `custom:${customInterval}`;
    }

    try {
        const subscriberKey = currentIdentity.publicKey();

        const subscription = new WasmSubscription(
            subscriberKey,
            provider,
            amount,
            currency,
            frequency,
            description
        );

        await subscriptionStorage.save_subscription(subscription);

        showNotification(`Subscription created: ${subscription.subscription_id}`, 'success');

        // Clear form
        providerInput.value = '';
        amountInput.value = '';
        descriptionInput.value = '';
        customIntervalInput.value = '';

        // Refresh list
        await updateSubscriptionsList();
    } catch (error) {
        console.error('Failed to create subscription:', error);
        showNotification('Failed to create subscription: ' + error.message, 'error');
    }
}

async function updateSubscriptionsList() {
    const listEl = document.getElementById('subscriptions-list');
    
    if (!listEl) return; // Element might not exist yet
    
    try {
        const subscriptions = await subscriptionStorage.list_all_subscriptions();
        
        if (subscriptions.length === 0) {
            listEl.innerHTML = '<p class="empty-state">No subscriptions yet. Create one above.</p>';
            return;
        }

        listEl.innerHTML = subscriptions.map(sub => {
            const isActive = sub.is_active || false;
            const isExpired = sub.is_expired || false;
            const statusClass = isExpired ? 'expired' : '';
            const statusBadge = isExpired ? '⚠️ EXPIRED' : isActive ? '✓ ACTIVE' : '⏸️ INACTIVE';
            const statusColor = isExpired ? 'var(--error)' : isActive ? 'var(--success)' : 'var(--warning)';

            return `
            <div class="request-item ${statusClass}">
                <div class="request-header">
                    <div class="request-id">Subscription: ${sub.subscription_id.substring(0, 12)}...</div>
                    <div class="request-amount">${sub.amount} ${sub.currency}</div>
                </div>
                <div class="request-details">
                    <div class="request-field">
                        <span class="label">Status:</span>
                        <span class="value" style="color: ${statusColor}">${statusBadge}</span>
                    </div>
                    <div class="request-field">
                        <span class="label">Subscriber:</span>
                        <span class="value">${sub.subscriber.substring(0, 20)}...</span>
                    </div>
                    <div class="request-field">
                        <span class="label">Provider:</span>
                        <span class="value">${sub.provider.substring(0, 20)}...</span>
                    </div>
                    <div class="request-field">
                        <span class="label">Frequency:</span>
                        <span class="value">${formatFrequency(sub.frequency)}</span>
                    </div>
                    ${sub.starts_at ? `<div class="request-field">
                        <span class="label">Starts:</span>
                        <span class="value">${format_timestamp(sub.starts_at)}</span>
                    </div>` : ''}
                    ${sub.ends_at ? `<div class="request-field">
                        <span class="label">Ends:</span>
                        <span class="value">${format_timestamp(sub.ends_at)}</span>
                    </div>` : ''}
                </div>
                <div class="request-actions">
                    <button class="btn-sm" onclick="copySubscriptionId('${sub.subscription_id}')">Copy ID</button>
                    <button class="btn-sm" style="background: var(--error)" onclick="deleteSubscription('${sub.subscription_id}')">Delete</button>
                </div>
            </div>
        `;
        }).join('');
        
        // Update auto-pay subscription selector
        await updateAutopaySubscriptionSelector();
    } catch (error) {
        console.error('Failed to load subscriptions:', error);
        listEl.innerHTML = '<p class="error-state">Failed to load subscriptions</p>';
    }
}

function formatFrequency(frequency) {
    if (!frequency) return 'Unknown';
    
    if (frequency === 'Daily') return 'Daily';
    if (frequency === 'Weekly') return 'Weekly';
    
    // Handle Monthly format
    if (frequency.startsWith('Monthly')) {
        const match = frequency.match(/Monthly\s*\{\s*day_of_month:\s*(\d+)\s*\}/);
        if (match) {
            return `Monthly (${match[1]}${getDaySuffix(match[1])} of month)`;
        }
        return 'Monthly';
    }
    
    // Handle Yearly format
    if (frequency.startsWith('Yearly')) {
        const match = frequency.match(/Yearly\s*\{\s*month:\s*(\d+),\s*day:\s*(\d+)\s*\}/);
        if (match) {
            const months = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
            const monthName = months[parseInt(match[1]) - 1] || match[1];
            return `Yearly (${monthName} ${match[2]}${getDaySuffix(match[2])})`;
        }
        return 'Yearly';
    }
    
    // Handle Custom format
    if (frequency.startsWith('Custom')) {
        const match = frequency.match(/Custom\s*\{\s*interval_seconds:\s*(\d+)\s*\}/);
        if (match) {
            const seconds = parseInt(match[1]);
            if (seconds < 60) return `Every ${seconds}s`;
            if (seconds < 3600) return `Every ${Math.floor(seconds / 60)}m`;
            if (seconds < 86400) return `Every ${Math.floor(seconds / 3600)}h`;
            return `Every ${Math.floor(seconds / 86400)}d`;
        }
        return 'Custom';
    }
    
    return frequency;
}

function getDaySuffix(day) {
    const d = parseInt(day);
    if (d >= 11 && d <= 13) return 'th';
    switch (d % 10) {
        case 1: return 'st';
        case 2: return 'nd';
        case 3: return 'rd';
        default: return 'th';
    }
}

async function deleteSubscription(subscriptionId) {
    if (!confirm('Delete this subscription?')) return;
    
    try {
        await subscriptionStorage.delete_subscription(subscriptionId);
        await subscriptionStorage.delete_signed_subscription(subscriptionId);
        showNotification('Subscription deleted', 'success');
        await updateSubscriptionsList();
    } catch (error) {
        console.error('Failed to delete subscription:', error);
        showNotification('Failed to delete subscription: ' + error.message, 'error');
    }
}

async function clearAllSubscriptions() {
    if (!confirm('Delete ALL subscriptions? This cannot be undone.')) return;
    
    try {
        await subscriptionStorage.clear_all();
        showNotification('All subscriptions cleared', 'success');
        await updateSubscriptionsList();
    } catch (error) {
        console.error('Failed to clear subscriptions:', error);
        showNotification('Failed to clear subscriptions: ' + error.message, 'error');
    }
}

function copySubscriptionId(subscriptionId) {
    navigator.clipboard.writeText(subscriptionId).then(() => {
        showNotification('Subscription ID copied to clipboard', 'success');
    }).catch(err => {
        console.error('Failed to copy:', err);
        showNotification('Failed to copy to clipboard', 'error');
    });
}

// Make subscription functions global for HTML onclick handlers
window.deleteSubscription = deleteSubscription;
window.copySubscriptionId = copySubscriptionId;

// ============================================================
// Auto-Pay Management
// ============================================================

async function enableAutoPay(subscriptionId, maxAmount, requireConfirmation) {
    try {
        if (!subscriptionId || !maxAmount) {
            showNotification('Subscription ID and max amount are required', 'error');
            return;
        }

        if (!autopayRuleStorage) {
            showNotification('Auto-pay storage not initialized', 'error');
            return;
        }

        // Try to get signed subscription first, then unsigned
        let sub = null;
        let peerPubkey = null;
        
        const signedSub = await subscriptionStorage.get_signed_subscription(subscriptionId);
        if (signedSub) {
            sub = signedSub.subscription();
            peerPubkey = sub.provider();
        } else {
            // Try unsigned subscription
            const unsignedSub = await subscriptionStorage.get_subscription(subscriptionId);
            if (unsignedSub) {
                peerPubkey = unsignedSub.provider();
            }
        }
        
        if (!peerPubkey) {
            showNotification('Subscription not found. Please create a subscription first.', 'error');
            return;
        }
        
        // Convert period to seconds (default monthly = 30 days)
        const periodSeconds = 30 * 24 * 60 * 60; // 30 days in seconds

        const rule = new WasmAutoPayRule(
            subscriptionId,
            peerPubkey,
            parseInt(maxAmount),
            periodSeconds,
            requireConfirmation || false
        );

        await autopayRuleStorage.save_autopay_rule(rule);
        showNotification('Auto-pay enabled', 'success');
        await updateAutoPayRulesList();
    } catch (error) {
        console.error('Failed to enable auto-pay:', error);
        showNotification('Failed to enable auto-pay: ' + (error.message || String(error)), 'error');
    }
}

async function disableAutoPay(subscriptionId) {
    try {
        const rule = await autopayRuleStorage.get_autopay_rule(subscriptionId);
        if (rule) {
            rule.disable();
            await autopayRuleStorage.save_autopay_rule(rule);
            showNotification('Auto-pay disabled', 'success');
            await updateAutoPayRulesList();
        } else {
            showNotification('Auto-pay rule not found', 'error');
        }
    } catch (error) {
        console.error('Failed to disable auto-pay:', error);
        showNotification('Failed to disable auto-pay: ' + error.message, 'error');
    }
}

async function getAutoPayStatus(subscriptionId) {
    try {
        const rule = await autopayRuleStorage.get_autopay_rule(subscriptionId);
        return rule;
    } catch (error) {
        console.error('Failed to get auto-pay status:', error);
        return null;
    }
}

async function listAutoPayRules() {
    try {
        if (!autopayRuleStorage) {
            console.warn('Auto-pay storage not initialized');
            return [];
        }
        return await autopayRuleStorage.list_autopay_rules();
    } catch (error) {
        console.error('Failed to list auto-pay rules:', error);
        return [];
    }
}

async function updateAutoPayRulesList() {
    const listEl = document.getElementById('autopay-rules-list');
    if (!listEl) {
        console.log('autopay-rules-list element not found');
        return;
    }
    
    try {
        const rules = await listAutoPayRules();

        if (rules.length === 0) {
            listEl.innerHTML = '<p class="empty-state">No auto-pay rules configured</p>';
            return;
        }

        listEl.innerHTML = rules.map(rule => {
            const enabled = rule.enabled ? 'Enabled' : 'Disabled';
            const enabledClass = rule.enabled ? 'active' : 'inactive';
            const confirmRequired = rule.require_confirmation ? 'Yes' : 'No';
            const confirmClass = rule.require_confirmation ? 'warning' : 'success';
            const maxAmount = Number(rule.max_amount);
            const subscriptionId = String(rule.subscription_id);
            const peerPubkey = String(rule.peer_pubkey);
            
            return `
                <div class="autopay-rule-item ${enabledClass}">
                    <div class="rule-header">
                        <div class="rule-id">Subscription: ${subscriptionId.substring(0, 12)}...</div>
                        <div class="rule-status ${enabledClass}">${enabled}</div>
                    </div>
                    <div class="rule-details">
                        <div class="rule-detail">
                            <span class="label">Peer:</span>
                            <span class="value">${peerPubkey.substring(0, 20)}...</span>
                        </div>
                        <div class="rule-detail">
                            <span class="label">Max Amount:</span>
                            <span class="value">${maxAmount} sats</span>
                        </div>
                        <div class="rule-detail">
                            <span class="label">Confirmation Required:</span>
                            <span class="value ${confirmClass}">${confirmRequired}</span>
                        </div>
                    </div>
                    <div class="rule-actions">
                        <button class="btn btn-small" onclick="toggleAutoPay('${subscriptionId}')">
                            ${rule.enabled ? 'Disable' : 'Enable'}
                        </button>
                        <button class="btn btn-small btn-danger" onclick="deleteAutoPayRule('${subscriptionId}')">
                            Delete
                        </button>
                    </div>
                </div>
            `;
        }).join('');
    } catch (error) {
        console.error('Failed to update auto-pay rules list:', error);
    }
}

async function toggleAutoPay(subscriptionId) {
    try {
        const rule = await autopayRuleStorage.get_autopay_rule(subscriptionId);
        if (rule) {
            if (rule.enabled()) {
                await disableAutoPay(subscriptionId);
            } else {
                rule.enable();
                await autopayRuleStorage.save_autopay_rule(rule);
                showNotification('Auto-pay enabled', 'success');
                await updateAutoPayRulesList();
            }
        }
    } catch (error) {
        console.error('Failed to toggle auto-pay:', error);
        showNotification('Failed to toggle auto-pay: ' + error.message, 'error');
    }
}

async function deleteAutoPayRule(subscriptionId) {
    if (!confirm('Delete this auto-pay rule?')) return;
    
    try {
        await autopayRuleStorage.delete_autopay_rule(subscriptionId);
        showNotification('Auto-pay rule deleted', 'success');
        await updateAutoPayRulesList();
    } catch (error) {
        console.error('Failed to delete auto-pay rule:', error);
        showNotification('Failed to delete auto-pay rule: ' + error.message, 'error');
    }
}

// ============================================================
// Spending Limits Management
// ============================================================

function periodToSeconds(period) {
    switch (period) {
        case 'daily':
            return 24 * 60 * 60;
        case 'weekly':
            return 7 * 24 * 60 * 60;
        case 'monthly':
            return 30 * 24 * 60 * 60;
        default:
            return 30 * 24 * 60 * 60; // Default to monthly
    }
}

async function setPeerLimit(peerPubkey, limit, period) {
    try {
        if (!peerPubkey || !limit || !period) {
            showNotification('Peer pubkey, limit, and period are required', 'error');
            return;
        }

        if (!peerLimitStorage) {
            showNotification('Peer limit storage not initialized', 'error');
            return;
        }

        // Validate pubkey
        const cleanPubkey = peerPubkey.replace('pubky://', '');
        if (!is_valid_pubkey(cleanPubkey)) {
            showNotification('Invalid peer pubkey', 'error');
            return;
        }

        const periodSeconds = periodToSeconds(period);
        const limitAmount = parseInt(limit);

        if (limitAmount <= 0) {
            showNotification('Limit must be positive', 'error');
            return;
        }

        const spendingLimit = new WasmPeerSpendingLimit(
            cleanPubkey,
            limitAmount,
            periodSeconds
        );

        await peerLimitStorage.save_peer_limit(spendingLimit);
        showNotification('Spending limit set', 'success');
        await updatePeerLimitsList();
    } catch (error) {
        console.error('Failed to set peer limit:', error);
        showNotification('Failed to set peer limit: ' + (error.message || String(error)), 'error');
    }
}

async function getPeerLimit(peerPubkey) {
    try {
        const cleanPubkey = peerPubkey.replace('pubky://', '');
        return await peerLimitStorage.get_peer_limit(cleanPubkey);
    } catch (error) {
        console.error('Failed to get peer limit:', error);
        return null;
    }
}

async function listPeerLimits() {
    try {
        if (!peerLimitStorage) {
            console.warn('Peer limit storage not initialized');
            return [];
        }
        return await peerLimitStorage.list_peer_limits();
    } catch (error) {
        console.error('Failed to list peer limits:', error);
        return [];
    }
}

async function updatePeerLimitsList() {
    const listEl = document.getElementById('peer-limits-list');
    if (!listEl) {
        console.log('peer-limits-list element not found');
        return;
    }
    
    try {
        const limits = await listPeerLimits();

        if (limits.length === 0) {
            listEl.innerHTML = '<p class="empty-state">No spending limits configured</p>';
            return;
        }

        listEl.innerHTML = limits.map(limit => {
            const currentSpent = Number(limit.current_spent);
            const totalLimit = Number(limit.total_limit);
            const periodSeconds = Number(limit.period_seconds);
            const periodStart = Number(limit.period_start);
            const peerPubkey = String(limit.peer_pubkey);
            
            const percentage = (currentSpent / totalLimit) * 100;
            const periodName = periodSeconds === 24 * 60 * 60 ? 'Daily' :
                              periodSeconds === 7 * 24 * 60 * 60 ? 'Weekly' :
                              periodSeconds === 30 * 24 * 60 * 60 ? 'Monthly' : 'Custom';
            
            // Calculate days until reset
            const now = Math.floor(Date.now() / 1000);
            const elapsed = now - periodStart;
            const remaining = Math.max(0, periodSeconds - elapsed);
            const daysRemaining = Math.floor(remaining / (24 * 60 * 60));
            
            // Escape single quotes for onclick attributes
            const escapedPubkey = peerPubkey.replace(/'/g, "\\'");
            
            return `
                <div class="spending-limit-item">
                    <div class="limit-header">
                        <div class="limit-peer">${peerPubkey.substring(0, 20)}...</div>
                        <div class="limit-period">${periodName}</div>
                    </div>
                    <div class="limit-progress">
                        <div class="progress-bar">
                            <div class="progress-fill" style="width: ${Math.min(percentage, 100)}%"></div>
                        </div>
                        <div class="limit-amounts">
                            <span>${currentSpent} / ${totalLimit} sats</span>
                            <span class="remaining">${Number(limit.remaining_limit)} remaining</span>
                        </div>
                    </div>
                    <div class="limit-details">
                        <div class="limit-detail">
                            <span class="label">Resets in:</span>
                            <span class="value">${daysRemaining} days</span>
                        </div>
                    </div>
                    <div class="limit-actions">
                        <button class="btn btn-small" onclick="resetPeerLimit('${escapedPubkey}')">
                            Reset
                        </button>
                        <button class="btn btn-small btn-danger" onclick="deletePeerLimit('${escapedPubkey}')">
                            Delete
                        </button>
                    </div>
                </div>
            `;
        }).join('');
    } catch (error) {
        console.error('Failed to update peer limits list:', error);
    }
}

async function resetPeerLimit(peerPubkey) {
    try {
        const limit = await peerLimitStorage.get_peer_limit(peerPubkey.replace('pubky://', ''));
        if (limit) {
            limit.reset();
            await peerLimitStorage.save_peer_limit(limit);
            showNotification('Spending limit reset', 'success');
            await updatePeerLimitsList();
        } else {
            showNotification('Spending limit not found', 'error');
        }
    } catch (error) {
        console.error('Failed to reset peer limit:', error);
        showNotification('Failed to reset peer limit: ' + error.message, 'error');
    }
}

async function deletePeerLimit(peerPubkey) {
    if (!confirm('Delete this spending limit?')) return;
    
    try {
        await peerLimitStorage.delete_peer_limit(peerPubkey.replace('pubky://', ''));
        showNotification('Spending limit deleted', 'success');
        await updatePeerLimitsList();
    } catch (error) {
        console.error('Failed to delete peer limit:', error);
        showNotification('Failed to delete peer limit: ' + error.message, 'error');
    }
}

// Update subscription selector for auto-pay
async function updateAutopaySubscriptionSelector() {
    const select = document.getElementById('autopay-subscription-select');
    if (!select) return;
    
    try {
        const subscriptions = await subscriptionStorage.list_active_subscriptions();
        const currentValue = select.value;
        
        // Clear existing options except the first one
        select.innerHTML = '<option value="">Select a subscription...</option>';
        
        subscriptions.forEach(sub => {
            const option = document.createElement('option');
            option.value = sub.subscription_id;
            option.textContent = `${sub.subscription_id.substring(0, 12)}... - ${sub.amount} ${sub.currency} (${sub.frequency})`;
            select.appendChild(option);
        });
        
        // Restore selection if it still exists
        if (currentValue) {
            select.value = currentValue;
        }
    } catch (error) {
        console.error('Failed to update subscription selector:', error);
    }
}

// Make functions global for HTML onclick handlers
window.toggleAutoPay = toggleAutoPay;
window.deleteAutoPayRule = deleteAutoPayRule;
window.resetPeerLimit = resetPeerLimit;
window.deletePeerLimit = deletePeerLimit;

// ============================================================
// Contact Management
// ============================================================

async function addContact() {
    const nameInput = document.getElementById('contact-name-input');
    const pubkeyInput = document.getElementById('contact-pubkey-input');
    const notesInput = document.getElementById('contact-notes-input');

    const name = nameInput.value.trim();
    let pubkey = pubkeyInput.value.trim();
    const notes = notesInput.value.trim();

    if (!name || !pubkey) {
        showNotification('Name and Pubky URI are required', 'error');
        return;
    }

    // Strip pubky:// prefix if present
    pubkey = pubkey.replace('pubky://', '');

    // Validate pubkey
    if (!is_valid_pubkey(pubkey)) {
        showNotification('Invalid Pubky URI or public key', 'error');
        return;
    }

    try {
        // Create contact
        let contact = new WasmContact(pubkey, name);
        if (notes) {
            contact = contact.with_notes(notes);
        }

        // Save contact
        await contactStorage.save_contact(contact);

        showNotification(`Contact "${name}" added successfully`, 'success');

        // Clear form
        nameInput.value = '';
        pubkeyInput.value = '';
        notesInput.value = '';

        // Refresh list
        await updateContactsList();
    } catch (error) {
        console.error('Failed to add contact:', error);
        showNotification('Failed to add contact: ' + error.message, 'error');
    }
}

async function updateContactsList(searchQuery = '') {
    const listEl = document.getElementById('contacts-list');
    
    if (!listEl) return;
    
    try {
        let contacts;
        
        if (searchQuery) {
            contacts = await contactStorage.search_contacts(searchQuery);
        } else {
            contacts = await contactStorage.list_contacts();
        }
        
        if (contacts.length === 0) {
            if (searchQuery) {
                listEl.innerHTML = '<p class="empty-state">No contacts found matching "' + searchQuery + '"</p>';
            } else {
                listEl.innerHTML = '<p class="empty-state">No contacts yet. Add one above or import from follows.</p>';
            }
            return;
        }

        listEl.innerHTML = contacts.map(contact => {
            const name = contact.name;
            const pubkey = contact.public_key;
            const notes = contact.notes || '';
            const paymentCount = contact.payment_history?.length || 0;
            
            // Generate avatar initials
            const initials = name.split(' ').map(n => n[0]).join('').substring(0, 2).toUpperCase();
            
            // Generate avatar color based on name
            const colorHash = name.split('').reduce((acc, char) => acc + char.charCodeAt(0), 0);
            const colors = ['#3b82f6', '#10b981', '#f59e0b', '#ef4444', '#8b5cf6', '#ec4899'];
            const avatarColor = colors[colorHash % colors.length];
            
            return `
                <div class="contact-card" onclick="viewContact('${pubkey}')">
                    <div class="contact-avatar" style="background: ${avatarColor}">${initials}</div>
                    <div class="contact-info">
                        <div class="contact-name">${name}</div>
                        <div class="contact-pubkey">${pubkey.substring(0, 20)}...</div>
                        ${notes ? `<div class="contact-notes-preview">${notes.substring(0, 50)}${notes.length > 50 ? '...' : ''}</div>` : ''}
                    </div>
                    <div class="contact-stats">
                        <div class="contact-stat">
                            <span class="stat-value">${paymentCount}</span>
                            <span class="stat-label">payments</span>
                        </div>
                    </div>
                </div>
            `;
        }).join('');
    } catch (error) {
        console.error('Failed to load contacts:', error);
        listEl.innerHTML = '<p class="error-state">Failed to load contacts</p>';
    }
}

async function viewContact(pubkey) {
    try {
        const contact = await contactStorage.get_contact(pubkey);
        
        if (!contact) {
            showNotification('Contact not found', 'error');
            return;
        }

        const modal = document.getElementById('contact-modal');
        const content = document.getElementById('contact-modal-content');
        
        const name = contact.name;
        const uri = contact.pubky_uri;
        const notes = contact.notes || 'No notes';
        const addedAt = format_timestamp(contact.added_at);
        const paymentHistory = contact.payment_history || [];
        
        content.innerHTML = `
            <div class="contact-detail-section">
                <div class="contact-detail-label">Name</div>
                <div class="contact-detail-value">${name}</div>
            </div>
            <div class="contact-detail-section">
                <div class="contact-detail-label">Pubky URI</div>
                <div class="contact-detail-value contact-uri">${uri}</div>
            </div>
            <div class="contact-detail-section">
                <div class="contact-detail-label">Public Key</div>
                <div class="contact-detail-value contact-pubkey-full">${pubkey}</div>
            </div>
            <div class="contact-detail-section">
                <div class="contact-detail-label">Notes</div>
                <div class="contact-detail-value">${notes}</div>
            </div>
            <div class="contact-detail-section">
                <div class="contact-detail-label">Added</div>
                <div class="contact-detail-value">${addedAt}</div>
            </div>
            <div class="contact-detail-section">
                <div class="contact-detail-label">Payment History</div>
                <div class="contact-detail-value">
                    ${paymentHistory.length > 0 
                        ? `<ul class="payment-history-list">${paymentHistory.map(id => `<li>${id}</li>`).join('')}</ul>`
                        : 'No payments yet'}
                </div>
            </div>
            <div class="contact-actions">
                <button class="btn btn-secondary" onclick="discoverContactMethods('${pubkey}')">Discover Methods</button>
                <button class="btn btn-secondary" onclick="editContact('${pubkey}')">Edit Notes</button>
                <button class="btn btn-secondary" style="background: var(--error)" onclick="deleteContact('${pubkey}')">Delete Contact</button>
            </div>
        `;
        
        modal.style.display = 'block';
    } catch (error) {
        console.error('Failed to view contact:', error);
        showNotification('Failed to load contact details: ' + error.message, 'error');
    }
}

async function discoverContactMethods(pubkey) {
    showNotification('Discovering payment methods...', 'info');
    
    try {
        const homeserver = document.getElementById('homeserver-input')?.value || 'https://demo.httprelay.io';
        const client = new DirectoryClient(homeserver);
        const results = await client.queryMethods(pubkey);
        
        if (results && typeof results === 'object') {
            const entries = Object.entries(results);
            
            if (entries.length === 0) {
                showNotification('No payment methods found for this contact', 'info');
            } else {
                const methodsList = entries.map(([method, endpoint]) => 
                    `${method}: ${endpoint}`
                ).join('\n');
                showNotification(`Found ${entries.length} method(s):\n${methodsList}`, 'success');
            }
        } else {
            showNotification('No payment methods found', 'info');
        }
    } catch (error) {
        console.error('Failed to discover methods:', error);
        showNotification('Failed to discover methods: ' + error.message, 'error');
    }
}

async function editContact(pubkey) {
    const newNotes = prompt('Enter new notes for this contact:');
    
    if (newNotes === null) return; // User cancelled
    
    try {
        const contact = await contactStorage.get_contact(pubkey);
        
        if (!contact) {
            showNotification('Contact not found', 'error');
            return;
        }

        // Create updated contact
        let updatedContact = new WasmContact(pubkey, contact.name);
        if (newNotes) {
            updatedContact = updatedContact.with_notes(newNotes);
        }
        
        await contactStorage.save_contact(updatedContact);
        
        showNotification('Contact updated successfully', 'success');
        
        // Close modal and refresh
        closeContactModal();
        await updateContactsList();
    } catch (error) {
        console.error('Failed to edit contact:', error);
        showNotification('Failed to update contact: ' + error.message, 'error');
    }
}

async function deleteContact(pubkey) {
    const contact = await contactStorage.get_contact(pubkey);
    
    if (!contact) {
        showNotification('Contact not found', 'error');
        return;
    }

    if (!confirm(`Delete contact "${contact.name}"? This cannot be undone.`)) {
        return;
    }
    
    try {
        await contactStorage.delete_contact(pubkey);
        showNotification('Contact deleted successfully', 'success');
        
        // Close modal and refresh
        closeContactModal();
        await updateContactsList();
    } catch (error) {
        console.error('Failed to delete contact:', error);
        showNotification('Failed to delete contact: ' + error.message, 'error');
    }
}

function closeContactModal() {
    const modal = document.getElementById('contact-modal');
    modal.style.display = 'none';
}

async function searchContacts() {
    const searchInput = document.getElementById('contact-search-input');
    const query = searchInput.value.trim();
    await updateContactsList(query);
}

async function importFromFollows() {
    showNotification('Import from Follows feature coming soon!', 'info');
    // This will be implemented in Phase 7
}

// Make contact functions global for HTML onclick handlers
window.viewContact = viewContact;
window.discoverContactMethods = discoverContactMethods;
window.editContact = editContact;
window.deleteContact = deleteContact;

// ===========================
// Payment Methods Management
// ===========================

async function updatePaymentMethodsList() {
    const listEl = document.getElementById('methods-list');
    
    if (!listEl) return;
    
    try {
        const methods = await paymentMethodStorage.list_methods();
        
        if (methods.length === 0) {
            listEl.innerHTML = '<p class="empty-state">No payment methods configured. Add one below.</p>';
            return;
        }
        
        listEl.innerHTML = methods.map((method, index) => `
            <div class="method-card" data-method-id="${method.method_id}" data-priority="${method.priority}">
                <div class="method-header">
                    <div class="method-title">
                        <span class="method-icon">${getMethodIcon(method.method_id)}</span>
                        <span class="method-name">${method.method_id}</span>
                        ${method.is_preferred ? '<span class="preferred-badge">⭐ Preferred</span>' : ''}
                        ${method.is_public ? '<span class="public-badge">🌐 Public</span>' : '<span class="private-badge">🔒 Private</span>'}
                    </div>
                    <div class="method-actions">
                        <button class="btn-sm" onclick="moveMethodUp('${method.method_id}', ${method.priority})">↑</button>
                        <button class="btn-sm" onclick="moveMethodDown('${method.method_id}', ${method.priority})">↓</button>
                        <button class="btn-sm" onclick="togglePreferred('${method.method_id}', ${method.is_preferred})" style="background: var(--warning)">
                            ${method.is_preferred ? 'Unset ⭐' : 'Set ⭐'}
                        </button>
                        <button class="btn-sm" onclick="deletePaymentMethod('${method.method_id}')" style="background: var(--error)">Delete</button>
                    </div>
                </div>
                <div class="method-body">
                    <div class="method-endpoint">${truncateEndpoint(method.endpoint)}</div>
                    <div class="method-meta">Priority: ${method.priority}</div>
                </div>
            </div>
        `).join('');
    } catch (error) {
        console.error('Failed to load payment methods:', error);
        listEl.innerHTML = '<p class="empty-state error">Error loading payment methods</p>';
        showNotification('Failed to load payment methods: ' + error.message, 'error');
    }
}

function getMethodIcon(methodId) {
    const icons = {
        'lightning': '⚡',
        'onchain': '₿',
        'custom': '💳'
    };
    return icons[methodId] || '💰';
}

function truncateEndpoint(endpoint) {
    if (endpoint.length > 60) {
        return endpoint.substring(0, 30) + '...' + endpoint.substring(endpoint.length - 27);
    }
    return endpoint;
}

async function addPaymentMethod() {
    const methodSelect = document.getElementById('method-id-input');
    const customMethodInput = document.getElementById('custom-method-id-input');
    const endpointInput = document.getElementById('method-endpoint-input');
    const publicCheckbox = document.getElementById('method-public-checkbox');
    const preferredCheckbox = document.getElementById('method-preferred-checkbox');
    const priorityInput = document.getElementById('method-priority-input');
    
    let methodId = methodSelect.value.trim();
    
    if (methodId === 'custom') {
        methodId = customMethodInput.value.trim();
    }
    
    const endpoint = endpointInput.value.trim();
    const isPublic = publicCheckbox.checked;
    const isPreferred = preferredCheckbox.checked;
    const priority = parseInt(priorityInput.value) || 1;
    
    if (!methodId) {
        showNotification('Please select or enter a method type', 'error');
        return;
    }
    
    if (!endpoint) {
        showNotification('Please enter an endpoint', 'error');
        return;
    }
    
    try {
        const method = new WasmPaymentMethodConfig(methodId, endpoint, isPublic, isPreferred, priority);
        await paymentMethodStorage.save_method(method);
        
        // Clear form
        methodSelect.value = '';
        customMethodInput.value = '';
        endpointInput.value = '';
        publicCheckbox.checked = true;
        preferredCheckbox.checked = false;
        priorityInput.value = '1';
        document.getElementById('custom-method-id-group').style.display = 'none';
        
        await updatePaymentMethodsList();
        showNotification(`Payment method "${methodId}" added successfully`, 'success');
    } catch (error) {
        console.error('Failed to add payment method:', error);
        showNotification('Failed to add payment method: ' + error.message, 'error');
    }
}

async function deletePaymentMethod(methodId) {
    if (!confirm(`Delete payment method "${methodId}"? This cannot be undone.`)) {
        return;
    }
    
    try {
        await paymentMethodStorage.delete_method(methodId);
        await updatePaymentMethodsList();
        showNotification(`Payment method "${methodId}" deleted`, 'success');
    } catch (error) {
        console.error('Failed to delete payment method:', error);
        showNotification('Failed to delete payment method: ' + error.message, 'error');
    }
}

async function togglePreferred(methodId, currentValue) {
    try {
        await paymentMethodStorage.set_preferred(methodId, !currentValue);
        await updatePaymentMethodsList();
        showNotification(`Updated preferred status for "${methodId}"`, 'success');
    } catch (error) {
        console.error('Failed to update preferred status:', error);
        showNotification('Failed to update preferred status: ' + error.message, 'error');
    }
}

async function moveMethodUp(methodId, currentPriority) {
    if (currentPriority <= 1) {
        showNotification('Already at highest priority', 'info');
        return;
    }
    
    try {
        await paymentMethodStorage.update_priority(methodId, currentPriority - 1);
        await updatePaymentMethodsList();
        showNotification(`Moved "${methodId}" up`, 'success');
    } catch (error) {
        console.error('Failed to update priority:', error);
        showNotification('Failed to update priority: ' + error.message, 'error');
    }
}

async function moveMethodDown(methodId, currentPriority) {
    try {
        await paymentMethodStorage.update_priority(methodId, currentPriority + 1);
        await updatePaymentMethodsList();
        showNotification(`Moved "${methodId}" down`, 'success');
    } catch (error) {
        console.error('Failed to update priority:', error);
        showNotification('Failed to update priority: ' + error.message, 'error');
    }
}

async function mockPublishMethods() {
    try {
        const result = await paymentMethodStorage.mock_publish();
        showNotification(result, 'warning');
    } catch (error) {
        console.error('Failed to mock publish:', error);
        showNotification('Failed to mock publish: ' + error.message, 'error');
    }
}

// Make payment method functions global for HTML onclick handlers
window.deletePaymentMethod = deletePaymentMethod;
window.togglePreferred = togglePreferred;
window.moveMethodUp = moveMethodUp;
window.moveMethodDown = moveMethodDown;

// ===========================
// Receipt Management
// ===========================

async function updateReceiptsList(filtered = null) {
    const listEl = document.getElementById('receipts-list');
    
    if (!listEl) return;
    
    try {
        const receipts = filtered || await receiptStorage.list_receipts();
        
        if (receipts.length === 0) {
            listEl.innerHTML = '<p class="empty-state">No receipts yet. Complete a payment to see receipts here.</p>';
            return;
        }
        
        // Parse and sort receipts by timestamp (newest first)
        const parsedReceipts = receipts
            .map(r => {
                try {
                    return JSON.parse(r);
                } catch {
                    return null;
                }
            })
            .filter(r => r !== null)
            .sort((a, b) => b.timestamp - a.timestamp);
        
        listEl.innerHTML = parsedReceipts.map(receipt => {
            const direction = currentIdentity && receipt.payer === currentIdentity.publicKey() ? 'sent' : 'received';
            const directionIcon = direction === 'sent' ? '📤' : '📥';
            const directionClass = direction === 'sent' ? 'sent' : 'received';
            const contact_pubkey = direction === 'sent' ? receipt.payee : receipt.payer;
            const timestamp = new Date(receipt.timestamp * 1000).toLocaleString();
            
            return `
                <div class="receipt-card ${directionClass}">
                    <div class="receipt-header">
                        <div class="receipt-direction">
                            <span class="direction-icon">${directionIcon}</span>
                            <span class="direction-label">${direction.toUpperCase()}</span>
                        </div>
                        <div class="receipt-amount">
                            ${receipt.amount} ${receipt.currency}
                        </div>
                    </div>
                    <div class="receipt-body">
                        <div class="receipt-field">
                            <span class="receipt-label">Method:</span>
                            <span class="receipt-value">${getMethodIcon(receipt.method)} ${receipt.method}</span>
                        </div>
                        <div class="receipt-field">
                            <span class="receipt-label">${direction === 'sent' ? 'To:' : 'From:'}</span>
                            <span class="receipt-value receipt-pubkey">${truncateKey(contact_pubkey)}</span>
                        </div>
                        <div class="receipt-field">
                            <span class="receipt-label">Time:</span>
                            <span class="receipt-value">${timestamp}</span>
                        </div>
                        <div class="receipt-field">
                            <span class="receipt-label">Receipt ID:</span>
                            <span class="receipt-value receipt-id">${receipt.receipt_id}</span>
                        </div>
                    </div>
                    <div class="receipt-actions">
                        <button class="btn-sm" onclick="viewReceiptDetails('${receipt.receipt_id}')">View Details</button>
                        <button class="btn-sm" onclick="deleteReceipt('${receipt.receipt_id}')" style="background: var(--error)">Delete</button>
                    </div>
                </div>
            `;
        }).join('');
        
        // Update statistics
        await updateReceiptStatistics();
        
    } catch (error) {
        console.error('Failed to load receipts:', error);
        listEl.innerHTML = '<p class="empty-state error">Error loading receipts</p>';
        showNotification('Failed to load receipts: ' + error.message, 'error');
    }
}

function truncateKey(key) {
    if (key.length > 20) {
        return key.substring(0, 10) + '...' + key.substring(key.length - 10);
    }
    return key;
}

async function updateReceiptStatistics() {
    if (!currentIdentity) return;
    
    try {
        const stats = await receiptStorage.get_statistics(currentIdentity.publicKey());
        
        document.getElementById('total-receipts').textContent = stats.total || 0;
        document.getElementById('sent-receipts').textContent = stats.sent || 0;
        document.getElementById('received-receipts').textContent = stats.received || 0;
    } catch (error) {
        console.error('Failed to update statistics:', error);
    }
}

async function applyReceiptFilters() {
    if (!currentIdentity) {
        showNotification('Please select an identity first', 'error');
        return;
    }
    
    const direction = document.getElementById('filter-direction').value;
    const method = document.getElementById('filter-method').value;
    const contact = document.getElementById('filter-contact').value;
    
    try {
        let receipts;
        
        if (direction !== 'all') {
            receipts = await receiptStorage.filter_by_direction(direction, currentIdentity.publicKey());
        } else if (method !== 'all') {
            receipts = await receiptStorage.filter_by_method(method);
        } else if (contact !== 'all') {
            receipts = await receiptStorage.filter_by_contact(contact, currentIdentity.publicKey());
        } else {
            receipts = null; // Show all
        }
        
        await updateReceiptsList(receipts);
        showNotification('Filters applied', 'success');
    } catch (error) {
        console.error('Failed to apply filters:', error);
        showNotification('Failed to apply filters: ' + error.message, 'error');
    }
}

async function resetReceiptFilters() {
    document.getElementById('filter-direction').value = 'all';
    document.getElementById('filter-method').value = 'all';
    document.getElementById('filter-contact').value = 'all';
    await updateReceiptsList();
    showNotification('Filters reset', 'success');
}

async function exportReceipts() {
    try {
        const json = await receiptStorage.export_as_json();
        
        // Create download link
        const blob = new Blob([json], { type: 'application/json' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `paykit-receipts-${Date.now()}.json`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
        
        showNotification('Receipts exported successfully', 'success');
    } catch (error) {
        console.error('Failed to export receipts:', error);
        showNotification('Failed to export receipts: ' + error.message, 'error');
    }
}

async function clearAllReceipts() {
    if (!confirm('Delete all receipts? This cannot be undone.')) {
        return;
    }
    
    try {
        await receiptStorage.clear_all();
        await updateReceiptsList();
        showNotification('All receipts deleted', 'success');
    } catch (error) {
        console.error('Failed to clear receipts:', error);
        showNotification('Failed to clear receipts: ' + error.message, 'error');
    }
}

async function deleteReceipt(receiptId) {
    if (!confirm('Delete this receipt?')) {
        return;
    }
    
    try {
        await receiptStorage.delete_receipt(receiptId);
        await updateReceiptsList();
        showNotification('Receipt deleted', 'success');
    } catch (error) {
        console.error('Failed to delete receipt:', error);
        showNotification('Failed to delete receipt: ' + error.message, 'error');
    }
}

async function viewReceiptDetails(receiptId) {
    try {
        const receiptJson = await receiptStorage.get_receipt(receiptId);
        if (!receiptJson) {
            showNotification('Receipt not found', 'error');
            return;
        }
        
        const receipt = JSON.parse(receiptJson);
        
        // Create a modal or alert with details
        const details = `
Receipt Details:
━━━━━━━━━━━━━━━━
Receipt ID: ${receipt.receipt_id}
Payer: ${receipt.payer}
Payee: ${receipt.payee}
Amount: ${receipt.amount} ${receipt.currency}
Method: ${receipt.method}
Timestamp: ${new Date(receipt.timestamp * 1000).toLocaleString()}
        `;
        
        alert(details);
    } catch (error) {
        console.error('Failed to view receipt:', error);
        showNotification('Failed to view receipt: ' + error.message, 'error');
    }
}

async function populateContactFilter() {
    try {
        const contacts = await contactStorage.list_contacts();
        const filterSelect = document.getElementById('filter-contact');
        
        if (!filterSelect) return;
        
        // Clear existing options except "All Contacts"
        filterSelect.innerHTML = '<option value="all">All Contacts</option>';
        
        // Add contact options
        contacts.forEach(contact => {
            const option = document.createElement('option');
            option.value = contact.public_key;
            option.textContent = contact.name;
            filterSelect.appendChild(option);
        });
    } catch (error) {
        console.error('Failed to populate contact filter:', error);
    }
}

// Make receipt functions global for HTML onclick handlers
window.viewReceiptDetails = viewReceiptDetails;
window.deleteReceipt = deleteReceipt;

// ===========================
// Dashboard
// ===========================

async function updateDashboard() {
    if (!currentIdentity) {
        // Show setup prompt
        document.getElementById('setup-identity').classList.remove('completed');
        updateSetupProgress();
        return;
    }
    
    try {
        // Get overview statistics
        const stats = await dashboard.get_overview_stats(currentIdentity.publicKey());
        
        // Update stat cards
        document.getElementById('dash-contacts').textContent = stats.contacts || 0;
        document.getElementById('dash-methods').textContent = stats.payment_methods || 0;
        document.getElementById('dash-receipts').textContent = stats.total_receipts || 0;
        document.getElementById('dash-subscriptions').textContent = stats.total_subscriptions || 0;
        
        // Update setup checklist
        const checklist = await dashboard.get_setup_checklist();
        updateSetupItem('setup-identity', true);
        updateSetupItem('setup-contacts', checklist.has_contacts);
        updateSetupItem('setup-methods', checklist.has_payment_methods);
        updateSetupItem('setup-preferred', checklist.has_preferred_method);
        updateSetupProgress();
        
        // Update recent activity
        await updateRecentActivity();
        
    } catch (error) {
        console.error('Failed to update dashboard:', error);
    }
}

function updateSetupItem(itemId, isComplete) {
    const item = document.getElementById(itemId);
    if (!item) return;
    
    const icon = item.querySelector('.setup-icon');
    if (isComplete) {
        item.classList.add('completed');
        icon.textContent = '✅';
    } else {
        item.classList.remove('completed');
        icon.textContent = '⏳';
    }
}

function updateSetupProgress() {
    const items = [
        document.getElementById('setup-identity'),
        document.getElementById('setup-contacts'),
        document.getElementById('setup-methods'),
        document.getElementById('setup-preferred')
    ];
    
    const completed = items.filter(item => item && item.classList.contains('completed')).length;
    const percentage = (completed / items.length) * 100;
    
    const fill = document.getElementById('setup-progress-fill');
    const text = document.getElementById('setup-progress-text');
    
    if (fill) fill.style.width = `${percentage}%`;
    if (text) text.textContent = `${Math.round(percentage)}% Complete`;
}

async function updateRecentActivity() {
    const activityEl = document.getElementById('dashboard-activity');
    if (!activityEl || !currentIdentity) return;
    
    try {
        const activities = await dashboard.get_recent_activity(currentIdentity.publicKey(), 10);
        
        if (activities.length === 0) {
            activityEl.innerHTML = '<p class="empty-state">No recent activity. Complete a payment to see activity here.</p>';
            return;
        }
        
        activityEl.innerHTML = activities.map(activity => {
            const direction = activity.direction === 'sent' ? '📤 Sent' : '📥 Received';
            const directionClass = activity.direction === 'sent' ? 'sent' : 'received';
            const timestamp = new Date(activity.timestamp * 1000).toLocaleString();
            
            return `
                <div class="activity-item ${directionClass}">
                    <div class="activity-icon">${direction}</div>
                    <div class="activity-content">
                        <div class="activity-amount">${activity.amount} ${activity.currency}</div>
                        <div class="activity-time">${timestamp}</div>
                    </div>
                </div>
            `;
        }).join('');
    } catch (error) {
        console.error('Failed to update recent activity:', error);
        activityEl.innerHTML = '<p class="empty-state">Error loading activity</p>';
    }
}

// Event Listeners
document.addEventListener('DOMContentLoaded', () => {
    console.log('DOMContentLoaded - Setting up event listeners');
    
    // Tab navigation with keyboard support
    const tabs = document.querySelectorAll('.tab');
    console.log(`Found ${tabs.length} tabs`);
    tabs.forEach(btn => {
        if (!btn.dataset.tab) {
            console.warn('Tab button missing data-tab attribute:', btn);
            return;
        }
        btn.addEventListener('click', (e) => {
            e.preventDefault();
            console.log('Tab clicked:', btn.dataset.tab);
            switchTab(btn.dataset.tab);
        });
        
        // Keyboard navigation (Arrow keys)
        btn.addEventListener('keydown', (e) => {
            const tabs = Array.from(document.querySelectorAll('.tab'));
            const currentIndex = tabs.indexOf(btn);
            
            if (e.key === 'ArrowRight' || e.key === 'ArrowDown') {
                e.preventDefault();
                const nextIndex = (currentIndex + 1) % tabs.length;
                tabs[nextIndex].focus();
                switchTab(tabs[nextIndex].dataset.tab);
            } else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') {
                e.preventDefault();
                const prevIndex = (currentIndex - 1 + tabs.length) % tabs.length;
                tabs[prevIndex].focus();
                switchTab(tabs[prevIndex].dataset.tab);
            } else if (e.key === 'Home') {
                e.preventDefault();
                tabs[0].focus();
                switchTab(tabs[0].dataset.tab);
            } else if (e.key === 'End') {
                e.preventDefault();
                tabs[tabs.length - 1].focus();
                switchTab(tabs[tabs.length - 1].dataset.tab);
            }
        });
    });
    
    // Identity actions
    document.getElementById('generate-btn').addEventListener('click', generateIdentity);
    document.getElementById('export-btn').addEventListener('click', exportIdentity);
    document.getElementById('import-btn').addEventListener('click', importIdentity);
    
    // Directory actions
    document.getElementById('query-btn').addEventListener('click', queryDirectory);
    
    // Contact actions
    document.getElementById('add-contact-btn').addEventListener('click', addContact);
    document.getElementById('refresh-contacts-btn').addEventListener('click', () => updateContactsList());
    document.getElementById('import-follows-btn').addEventListener('click', importFromFollows);
    document.getElementById('contact-search-input').addEventListener('input', searchContacts);
    document.getElementById('close-contact-modal').addEventListener('click', closeContactModal);
    
    // Payment Methods actions
    document.getElementById('add-method-btn').addEventListener('click', addPaymentMethod);
    document.getElementById('refresh-methods-btn').addEventListener('click', updatePaymentMethodsList);
    document.getElementById('mock-publish-btn').addEventListener('click', mockPublishMethods);
    
    // Handle method type selection to show/hide custom ID input
    document.getElementById('method-id-input').addEventListener('change', (e) => {
        const customGroup = document.getElementById('custom-method-id-group');
        if (e.target.value === 'custom') {
            customGroup.style.display = 'block';
        } else {
            customGroup.style.display = 'none';
        }
    });
    
    // Receipt actions
    document.getElementById('refresh-receipts-btn').addEventListener('click', () => updateReceiptsList());
    document.getElementById('export-receipts-btn').addEventListener('click', exportReceipts);
    document.getElementById('clear-receipts-btn').addEventListener('click', clearAllReceipts);
    document.getElementById('apply-filters-btn').addEventListener('click', applyReceiptFilters);
    document.getElementById('reset-filters-btn').addEventListener('click', resetReceiptFilters);
    
    // Populate contact filter on tab switch
    document.querySelectorAll('.tab').forEach(btn => {
        const originalListener = btn.onclick;
        btn.addEventListener('click', async () => {
            if (btn.dataset.tab === 'receipts') {
                await populateContactFilter();
            }
        });
    });
    
    // Close modal when clicking outside
    window.addEventListener('click', (event) => {
        const modal = document.getElementById('contact-modal');
        if (event.target === modal) {
            closeContactModal();
        }
    });
    
    // Subscription actions
    document.getElementById('create-request-btn').addEventListener('click', createPaymentRequest);
    document.getElementById('refresh-requests-btn').addEventListener('click', updateRequestsList);
    document.getElementById('clear-requests-btn').addEventListener('click', clearAllRequests);
    
    document.getElementById('create-subscription-btn').addEventListener('click', createSubscription);
    document.getElementById('refresh-subscriptions-btn').addEventListener('click', updateSubscriptionsList);
    document.getElementById('clear-subscriptions-btn').addEventListener('click', clearAllSubscriptions);
    
    // Handle frequency select change to show/hide custom interval input
    document.getElementById('sub-frequency-select').addEventListener('change', (e) => {
        const customGroup = document.getElementById('custom-frequency-group');
        if (e.target.value === 'custom') {
            customGroup.style.display = 'block';
        } else {
            customGroup.style.display = 'none';
        }
    });
    
    // Auto-pay actions
    const enableAutopayBtn = document.getElementById('enable-autopay-btn');
    if (enableAutopayBtn) {
        enableAutopayBtn.addEventListener('click', async () => {
            const subscriptionId = document.getElementById('autopay-subscription-select')?.value;
            const maxAmount = document.getElementById('autopay-max-amount-input')?.value;
            const requireConfirmation = document.getElementById('autopay-require-confirmation')?.checked || false;
            
            if (!subscriptionId || !maxAmount) {
                showNotification('Subscription and max amount are required', 'error');
                return;
            }
            
            await enableAutoPay(subscriptionId, maxAmount, requireConfirmation);
            
            // Clear form
            const select = document.getElementById('autopay-subscription-select');
            const amountInput = document.getElementById('autopay-max-amount-input');
            const confirmCheckbox = document.getElementById('autopay-require-confirmation');
            if (select) select.value = '';
            if (amountInput) amountInput.value = '';
            if (confirmCheckbox) confirmCheckbox.checked = false;
        });
    }
    
    const refreshAutopayBtn = document.getElementById('refresh-autopay-btn');
    if (refreshAutopayBtn) {
        refreshAutopayBtn.addEventListener('click', updateAutoPayRulesList);
    }
    
    // Spending limits actions
    const setLimitBtn = document.getElementById('set-limit-btn');
    if (setLimitBtn) {
        setLimitBtn.addEventListener('click', async () => {
            const peerPubkey = document.getElementById('limit-peer-input')?.value.trim();
            const limit = document.getElementById('limit-amount-input')?.value;
            const period = document.getElementById('limit-period-select')?.value;
            
            if (!peerPubkey || !limit) {
                showNotification('Peer pubkey and limit are required', 'error');
                return;
            }
            
            await setPeerLimit(peerPubkey, limit, period);
            
            // Clear form
            const peerInput = document.getElementById('limit-peer-input');
            const limitInput = document.getElementById('limit-amount-input');
            const periodSelect = document.getElementById('limit-period-select');
            if (peerInput) peerInput.value = '';
            if (limitInput) limitInput.value = '';
            if (periodSelect) periodSelect.value = 'monthly';
        });
    }
    
    const refreshLimitsBtn = document.getElementById('refresh-limits-btn');
    if (refreshLimitsBtn) {
        refreshLimitsBtn.addEventListener('click', updatePeerLimitsList);
    }
    
    // Payment actions
    document.getElementById('initiate-payment-btn').addEventListener('click', initiatePayment);
    
    // Payment form validation
    const recipientInput = document.getElementById('recipient-input');
    const amountInput = document.getElementById('amount-input');
    const currencyInput = document.getElementById('currency-input');
    
    if (recipientInput) {
        recipientInput.addEventListener('input', validatePaymentForm);
        recipientInput.addEventListener('blur', validatePaymentForm);
    }
    
    if (amountInput) {
        amountInput.addEventListener('input', validatePaymentForm);
        amountInput.addEventListener('blur', validatePaymentForm);
    }
    
    if (currencyInput) {
        currencyInput.addEventListener('input', validatePaymentForm);
    }
    
    // Initial validation
    validatePaymentForm();
    
    // Initialize app (don't block on errors)
    initializeApp().catch(err => {
        console.error('Failed to initialize app:', err);
        showNotification('App initialization had errors - some features may not work', 'error');
    });
    
    // Ensure tab navigation works even if initialization fails
    console.log('Event listeners attached, tab navigation should work');
});


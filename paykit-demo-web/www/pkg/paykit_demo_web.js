let wasm;

function isLikeNone(x) {
    return x === undefined || x === null;
}

function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return  `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        } else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        } else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for(let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches && builtInMatches.length > 1) {
        className = builtInMatches[1];
    } else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        } catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}

let WASM_VECTOR_LEN = 0;

let cachedUint8ArrayMemory0 = null;

function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    }
}

function passStringToWasm0(arg, malloc, realloc) {

    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }

    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

let cachedDataViewMemory0 = null;

function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });

cachedTextDecoder.decode();

const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return decodeText(ptr, len);
}

function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_externrefs.set(idx, obj);
    return idx;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        const idx = addToExternrefTable0(e);
        wasm.__wbindgen_exn_store(idx);
    }
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(state => state.dtor(state.a, state.b));

function makeMutClosure(arg0, arg1, dtor, f) {
    const state = { a: arg0, b: arg1, cnt: 1, dtor };
    const real = (...args) => {

        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        } finally {
            state.a = a;
            real._wbg_cb_unref();
        }
    };
    real._wbg_cb_unref = () => {
        if (--state.cnt === 0) {
            state.dtor(state.a, state.b);
            state.a = 0;
            CLOSURE_DTORS.unregister(state);
        }
    };
    CLOSURE_DTORS.register(real, state, state);
    return real;
}

function getArrayJsValueFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    const mem = getDataViewMemory0();
    const result = [];
    for (let i = ptr; i < ptr + 4 * len; i += 4) {
        result.push(wasm.__wbindgen_externrefs.get(mem.getUint32(i, true)));
    }
    wasm.__externref_drop_slice(ptr, len);
    return result;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

function _assertClass(instance, klass) {
    if (!(instance instanceof klass)) {
        throw new Error(`expected instance of ${klass.name}`);
    }
}
/**
 * @param {string} pubkey
 * @returns {boolean}
 */
export function is_valid_pubkey(pubkey) {
    const ptr0 = passStringToWasm0(pubkey, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.is_valid_pubkey(ptr0, len0);
    return ret !== 0;
}

/**
 * Utility functions for subscriptions
 * @param {bigint} timestamp
 * @returns {string}
 */
export function format_timestamp(timestamp) {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.format_timestamp(timestamp);
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * Parse a Noise endpoint string and return WebSocket URL and server key
 *
 * Format: noise://host:port@pubkey_hex
 * Returns JSON: { ws_url: string, server_key_hex: string, host: string, port: number }
 * @param {string} endpoint
 * @returns {any}
 */
export function parse_noise_endpoint_wasm(endpoint) {
    const ptr0 = passStringToWasm0(endpoint, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.parse_noise_endpoint_wasm(ptr0, len0);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Extract public key from pubky:// URI or raw public key
 *
 * Returns public key string
 * @param {string} uri
 * @returns {string}
 */
export function extract_pubkey_from_uri_wasm(uri) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(uri, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.extract_pubkey_from_uri_wasm(ptr0, len0);
        var ptr2 = ret[0];
        var len2 = ret[1];
        if (ret[3]) {
            ptr2 = 0; len2 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * Initialize the WASM module
 *
 * This should be called once when the module is loaded.
 * It sets up panic hooks for better error messages in the browser console.
 */
export function init() {
    wasm.init();
}

/**
 * Get the version of the Paykit WASM module
 * @returns {string}
 */
export function version() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.version();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

function wasm_bindgen__convert__closures_____invoke__h75da7eae032c0859(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures_____invoke__h75da7eae032c0859(arg0, arg1, arg2);
}

function wasm_bindgen__convert__closures_____invoke__h7460171fa07d4e7b(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures_____invoke__h7460171fa07d4e7b(arg0, arg1, arg2);
}

function wasm_bindgen__convert__closures_____invoke__hec0e381372c60b88(arg0, arg1) {
    wasm.wasm_bindgen__convert__closures_____invoke__hec0e381372c60b88(arg0, arg1);
}

function wasm_bindgen__convert__closures_____invoke__h8a0305fb7488cc73(arg0, arg1, arg2, arg3) {
    wasm.wasm_bindgen__convert__closures_____invoke__h8a0305fb7488cc73(arg0, arg1, arg2, arg3);
}

const __wbindgen_enum_BinaryType = ["blob", "arraybuffer"];

const BrowserStorageFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_browserstorage_free(ptr >>> 0, 1));
/**
 * Storage manager for browser localStorage
 */
export class BrowserStorage {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        BrowserStorageFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_browserstorage_free(ptr, 0);
    }
    /**
     * Load an identity from localStorage
     * @param {string} name
     * @returns {Identity}
     */
    loadIdentity(name) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.browserstorage_loadIdentity(this.__wbg_ptr, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return Identity.__wrap(ret[0]);
    }
    /**
     * Save an identity to localStorage
     * @param {string} name
     * @param {Identity} identity
     */
    saveIdentity(name, identity) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertClass(identity, Identity);
        const ret = wasm.browserstorage_saveIdentity(this.__wbg_ptr, ptr0, len0, identity.__wbg_ptr);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Delete an identity from localStorage
     * @param {string} name
     */
    deleteIdentity(name) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.browserstorage_deleteIdentity(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * List all saved identity names
     * @returns {any[]}
     */
    listIdentities() {
        const ret = wasm.browserstorage_listIdentities(this.__wbg_ptr);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        var v1 = getArrayJsValueFromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * Get the current active identity name
     * @returns {string | undefined}
     */
    getCurrentIdentity() {
        const ret = wasm.browserstorage_getCurrentIdentity(this.__wbg_ptr);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * Set the current active identity
     * @param {string} name
     */
    setCurrentIdentity(name) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.browserstorage_setCurrentIdentity(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Create a new browser storage manager
     */
    constructor() {
        const ret = wasm.browserstorage_new();
        this.__wbg_ptr = ret >>> 0;
        BrowserStorageFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Clear all Paykit data from localStorage
     */
    clearAll() {
        const ret = wasm.browserstorage_clearAll(this.__wbg_ptr);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
}
if (Symbol.dispose) BrowserStorage.prototype[Symbol.dispose] = BrowserStorage.prototype.free;

const DirectoryClientFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_directoryclient_free(ptr >>> 0, 1));
/**
 * Directory client for querying payment methods
 */
export class DirectoryClient {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        DirectoryClientFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_directoryclient_free(ptr, 0);
    }
    /**
     * Query payment methods for a public key
     * @param {string} public_key
     * @returns {Promise<any>}
     */
    queryMethods(public_key) {
        const ptr0 = passStringToWasm0(public_key, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.directoryclient_queryMethods(this.__wbg_ptr, ptr0, len0);
        return ret;
    }
    /**
     * Publish payment methods (placeholder - requires authentication)
     * @param {any} _methods
     * @returns {Promise<void>}
     */
    publishMethods(_methods) {
        const ret = wasm.directoryclient_publishMethods(this.__wbg_ptr, _methods);
        return ret;
    }
    /**
     * Create a new directory client
     * @param {string} homeserver
     */
    constructor(homeserver) {
        const ptr0 = passStringToWasm0(homeserver, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.directoryclient_new(ptr0, len0);
        this.__wbg_ptr = ret >>> 0;
        DirectoryClientFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
}
if (Symbol.dispose) DirectoryClient.prototype[Symbol.dispose] = DirectoryClient.prototype.free;

const IdentityFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_identity_free(ptr >>> 0, 1));
/**
 * JavaScript-facing identity wrapper
 */
export class Identity {

    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(Identity.prototype);
        obj.__wbg_ptr = ptr;
        IdentityFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        IdentityFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_identity_free(ptr, 0);
    }
    /**
     * Get the public key as a hex string
     * @returns {string}
     */
    publicKey() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.identity_publicKey(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Create an identity with a nickname
     * @param {string} nickname
     * @returns {Identity}
     */
    static withNickname(nickname) {
        const ptr0 = passStringToWasm0(nickname, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.identity_withNickname(ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return Identity.__wrap(ret[0]);
    }
    /**
     * Get Ed25519 public key (for Noise identity)
     * Returns hex-encoded public key
     * @returns {string}
     */
    ed25519PublicKeyHex() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.identity_ed25519PublicKeyHex(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get Ed25519 secret key (for Noise key derivation)
     * Returns hex-encoded secret key
     * @returns {string}
     */
    ed25519SecretKeyHex() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.identity_ed25519SecretKeyHex(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Generate a new random identity
     */
    constructor() {
        const ret = wasm.identity_new();
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        this.__wbg_ptr = ret[0] >>> 0;
        IdentityFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Export identity to JSON string
     * @returns {string}
     */
    toJSON() {
        let deferred2_0;
        let deferred2_1;
        try {
            const ret = wasm.identity_toJSON(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Get the nickname (if set)
     * @returns {string | undefined}
     */
    nickname() {
        const ret = wasm.identity_nickname(this.__wbg_ptr);
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * Import identity from JSON string
     * @param {string} json
     * @returns {Identity}
     */
    static fromJSON(json) {
        const ptr0 = passStringToWasm0(json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.identity_fromJSON(ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return Identity.__wrap(ret[0]);
    }
    /**
     * Get the Pubky URI
     * @returns {string}
     */
    pubkyUri() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.identity_pubkyUri(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
}
if (Symbol.dispose) Identity.prototype[Symbol.dispose] = Identity.prototype.free;

const WasmAutoPayRuleFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmautopayrule_free(ptr >>> 0, 1));
/**
 * WASM-friendly auto-pay rule
 */
export class WasmAutoPayRule {

    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(WasmAutoPayRule.prototype);
        obj.__wbg_ptr = ptr;
        WasmAutoPayRuleFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmAutoPayRuleFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmautopayrule_free(ptr, 0);
    }
    /**
     * Get the maximum amount
     * @returns {bigint}
     */
    get max_amount() {
        const ret = wasm.wasmautopayrule_max_amount(this.__wbg_ptr);
        return ret;
    }
    /**
     * Get the peer public key
     * @returns {string}
     */
    get peer_pubkey() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmautopayrule_peer_pubkey(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get the period in seconds
     * @returns {bigint}
     */
    get period_seconds() {
        const ret = wasm.wasmautopayrule_period_seconds(this.__wbg_ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
     * Get the subscription ID
     * @returns {string}
     */
    get subscription_id() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmautopayrule_subscription_id(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Check if manual confirmation is required
     * @returns {boolean}
     */
    get require_confirmation() {
        const ret = wasm.wasmautopayrule_require_confirmation(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Set whether manual confirmation is required
     * @param {boolean} required
     */
    set_require_confirmation(required) {
        wasm.wasmautopayrule_set_require_confirmation(this.__wbg_ptr, required);
    }
    /**
     * Get the rule ID
     * @returns {string}
     */
    get id() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmautopayrule_id(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Create a new auto-pay rule
     * @param {string} subscription_id
     * @param {string} peer_pubkey
     * @param {bigint} max_amount
     * @param {bigint} period_seconds
     * @param {boolean} require_confirmation
     */
    constructor(subscription_id, peer_pubkey, max_amount, period_seconds, require_confirmation) {
        const ptr0 = passStringToWasm0(subscription_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(peer_pubkey, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmautopayrule_new(ptr0, len0, ptr1, len1, max_amount, period_seconds, require_confirmation);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        this.__wbg_ptr = ret[0] >>> 0;
        WasmAutoPayRuleFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Enable the rule
     */
    enable() {
        wasm.wasmautopayrule_enable(this.__wbg_ptr);
    }
    /**
     * Disable the rule
     */
    disable() {
        wasm.wasmautopayrule_disable(this.__wbg_ptr);
    }
    /**
     * Check if the rule is enabled
     * @returns {boolean}
     */
    get enabled() {
        const ret = wasm.wasmautopayrule_enabled(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Convert to JSON for storage
     * @returns {string}
     */
    to_json() {
        let deferred2_0;
        let deferred2_1;
        try {
            const ret = wasm.wasmautopayrule_to_json(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Create from JSON
     * @param {string} json
     * @returns {WasmAutoPayRule}
     */
    static from_json(json) {
        const ptr0 = passStringToWasm0(json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmautopayrule_from_json(ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return WasmAutoPayRule.__wrap(ret[0]);
    }
}
if (Symbol.dispose) WasmAutoPayRule.prototype[Symbol.dispose] = WasmAutoPayRule.prototype.free;

const WasmAutoPayRuleStorageFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmautopayrulestorage_free(ptr >>> 0, 1));
/**
 * Storage for auto-pay rules in browser localStorage
 */
export class WasmAutoPayRuleStorage {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmAutoPayRuleStorageFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmautopayrulestorage_free(ptr, 0);
    }
    /**
     * Get an auto-pay rule by subscription ID
     * @param {string} subscription_id
     * @returns {Promise<WasmAutoPayRule | undefined>}
     */
    get_autopay_rule(subscription_id) {
        const ptr0 = passStringToWasm0(subscription_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmautopayrulestorage_get_autopay_rule(this.__wbg_ptr, ptr0, len0);
        return ret;
    }
    /**
     * Save an auto-pay rule
     * @param {WasmAutoPayRule} rule
     * @returns {Promise<void>}
     */
    save_autopay_rule(rule) {
        _assertClass(rule, WasmAutoPayRule);
        const ret = wasm.wasmautopayrulestorage_save_autopay_rule(this.__wbg_ptr, rule.__wbg_ptr);
        return ret;
    }
    /**
     * List all auto-pay rules
     * @returns {Promise<any[]>}
     */
    list_autopay_rules() {
        const ret = wasm.wasmautopayrulestorage_list_autopay_rules(this.__wbg_ptr);
        return ret;
    }
    /**
     * Delete an auto-pay rule
     * @param {string} subscription_id
     * @returns {Promise<void>}
     */
    delete_autopay_rule(subscription_id) {
        const ptr0 = passStringToWasm0(subscription_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmautopayrulestorage_delete_autopay_rule(this.__wbg_ptr, ptr0, len0);
        return ret;
    }
    /**
     * Create new storage manager
     */
    constructor() {
        const ret = wasm.wasmautopayrulestorage_new();
        this.__wbg_ptr = ret >>> 0;
        WasmAutoPayRuleStorageFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Clear all auto-pay rules
     * @returns {Promise<void>}
     */
    clear_all() {
        const ret = wasm.wasmautopayrulestorage_clear_all(this.__wbg_ptr);
        return ret;
    }
}
if (Symbol.dispose) WasmAutoPayRuleStorage.prototype[Symbol.dispose] = WasmAutoPayRuleStorage.prototype.free;

const WasmContactFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmcontact_free(ptr >>> 0, 1));
/**
 * A contact in the address book
 *
 * Represents a peer you may send payments to, with optional metadata
 * and payment history tracking.
 *
 * # Examples
 *
 * ```
 * use paykit_demo_web::WasmContact;
 *
 * let contact = WasmContact::new(
 *     "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo".to_string(),
 *     "Bob's Coffee Shop".to_string()
 * ).unwrap();
 * ```
 */
export class WasmContact {

    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(WasmContact.prototype);
        obj.__wbg_ptr = ptr;
        WasmContactFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmContactFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmcontact_free(ptr, 0);
    }
    /**
     * Get the contact's public key
     * @returns {string}
     */
    get public_key() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmcontact_public_key(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Add notes to the contact
     *
     * # Examples
     *
     * ```
     * use paykit_demo_web::WasmContact;
     *
     * let contact = WasmContact::new(
     *     "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo".to_string(),
     *     "Alice".to_string()
     * ).unwrap().with_notes("Met at Bitcoin conference".to_string());
     * ```
     * @param {string} notes
     * @returns {WasmContact}
     */
    with_notes(notes) {
        const ptr = this.__destroy_into_raw();
        const ptr0 = passStringToWasm0(notes, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmcontact_with_notes(ptr, ptr0, len0);
        return WasmContact.__wrap(ret);
    }
    /**
     * Get the contact's payment history (receipt IDs)
     * @returns {any[]}
     */
    get payment_history() {
        const ret = wasm.wasmcontact_payment_history(this.__wbg_ptr);
        var v1 = getArrayJsValueFromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * Create a new contact
     *
     * # Arguments
     *
     * * `public_key` - The contact's z32-encoded public key
     * * `name` - Human-readable name for the contact
     *
     * # Errors
     *
     * Returns an error if the public key is invalid.
     *
     * # Examples
     *
     * ```
     * use paykit_demo_web::WasmContact;
     *
     * let contact = WasmContact::new(
     *     "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo".to_string(),
     *     "Alice".to_string()
     * ).unwrap();
     * ```
     * @param {string} public_key
     * @param {string} name
     */
    constructor(public_key, name) {
        const ptr0 = passStringToWasm0(public_key, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmcontact_new(ptr0, len0, ptr1, len1);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        this.__wbg_ptr = ret[0] >>> 0;
        WasmContactFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Get the contact's name
     * @returns {string}
     */
    get name() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmcontact_name(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get the contact's notes
     * @returns {string | undefined}
     */
    get notes() {
        const ret = wasm.wasmcontact_notes(this.__wbg_ptr);
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * Convert contact to JSON string
     * @returns {string}
     */
    to_json() {
        let deferred2_0;
        let deferred2_1;
        try {
            const ret = wasm.wasmcontact_to_json(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Get the timestamp when contact was added
     * @returns {bigint}
     */
    get added_at() {
        const ret = wasm.wasmcontact_added_at(this.__wbg_ptr);
        return ret;
    }
    /**
     * Create contact from JSON string
     * @param {string} json
     * @returns {WasmContact}
     */
    static from_json(json) {
        const ptr0 = passStringToWasm0(json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmcontact_from_json(ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return WasmContact.__wrap(ret[0]);
    }
    /**
     * Get the Pubky URI for this contact
     *
     * # Examples
     *
     * ```
     * use paykit_demo_web::WasmContact;
     *
     * let contact = WasmContact::new(
     *     "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo".to_string(),
     *     "Alice".to_string()
     * ).unwrap();
     *
     * assert_eq!(contact.pubky_uri(), "pubky://8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo");
     * ```
     * @returns {string}
     */
    pubky_uri() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmcontact_pubky_uri(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
}
if (Symbol.dispose) WasmContact.prototype[Symbol.dispose] = WasmContact.prototype.free;

const WasmContactStorageFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmcontactstorage_free(ptr >>> 0, 1));
/**
 * Storage manager for contacts in browser localStorage
 *
 * Provides CRUD operations for managing contacts with localStorage persistence.
 *
 * # Examples
 *
 * ```
 * use paykit_demo_web::{WasmContact, WasmContactStorage};
 * use wasm_bindgen_test::*;
 *
 * wasm_bindgen_test_configure!(run_in_browser);
 *
 * #[wasm_bindgen_test]
 * async fn example_storage() {
 *     let storage = WasmContactStorage::new();
 *     let contact = WasmContact::new(
 *         "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo".to_string(),
 *         "Alice".to_string()
 *     ).unwrap();
 *
 *     storage.save_contact(&contact).await.unwrap();
 *     let contacts = storage.list_contacts().await.unwrap();
 *     assert_eq!(contacts.len(), 1);
 * }
 * ```
 */
export class WasmContactStorage {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmContactStorageFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmcontactstorage_free(ptr, 0);
    }
    /**
     * Get a contact by public key
     *
     * Returns `None` if the contact doesn't exist.
     *
     * # Examples
     *
     * ```
     * use paykit_demo_web::{WasmContact, WasmContactStorage};
     * use wasm_bindgen_test::*;
     *
     * wasm_bindgen_test_configure!(run_in_browser);
     *
     * #[wasm_bindgen_test]
     * async fn get_example() {
     *     let storage = WasmContactStorage::new();
     *     let pubkey = "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo";
     *     let contact = storage.get_contact(pubkey).await.unwrap();
     *     // contact is None if not found
     * }
     * ```
     * @param {string} public_key
     * @returns {Promise<WasmContact | undefined>}
     */
    get_contact(public_key) {
        const ptr0 = passStringToWasm0(public_key, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmcontactstorage_get_contact(this.__wbg_ptr, ptr0, len0);
        return ret;
    }
    /**
     * Save a contact to localStorage
     *
     * If a contact with the same public key exists, it will be overwritten.
     *
     * # Examples
     *
     * ```
     * use paykit_demo_web::{WasmContact, WasmContactStorage};
     * use wasm_bindgen_test::*;
     *
     * wasm_bindgen_test_configure!(run_in_browser);
     *
     * #[wasm_bindgen_test]
     * async fn save_example() {
     *     let storage = WasmContactStorage::new();
     *     let contact = WasmContact::new(
     *         "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo".to_string(),
     *         "Alice".to_string()
     *     ).unwrap();
     *     storage.save_contact(&contact).await.unwrap();
     * }
     * ```
     * @param {WasmContact} contact
     * @returns {Promise<void>}
     */
    save_contact(contact) {
        _assertClass(contact, WasmContact);
        const ret = wasm.wasmcontactstorage_save_contact(this.__wbg_ptr, contact.__wbg_ptr);
        return ret;
    }
    /**
     * List all contacts, sorted alphabetically by name
     *
     * # Examples
     *
     * ```
     * use paykit_demo_web::WasmContactStorage;
     * use wasm_bindgen_test::*;
     *
     * wasm_bindgen_test_configure!(run_in_browser);
     *
     * #[wasm_bindgen_test]
     * async fn list_example() {
     *     let storage = WasmContactStorage::new();
     *     let contacts = storage.list_contacts().await.unwrap();
     *     // Returns empty vector if no contacts
     * }
     * ```
     * @returns {Promise<any[]>}
     */
    list_contacts() {
        const ret = wasm.wasmcontactstorage_list_contacts(this.__wbg_ptr);
        return ret;
    }
    /**
     * Delete a contact by public key
     *
     * # Examples
     *
     * ```
     * use paykit_demo_web::WasmContactStorage;
     * use wasm_bindgen_test::*;
     *
     * wasm_bindgen_test_configure!(run_in_browser);
     *
     * #[wasm_bindgen_test]
     * async fn delete_example() {
     *     let storage = WasmContactStorage::new();
     *     let pubkey = "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo";
     *     storage.delete_contact(pubkey).await.unwrap();
     * }
     * ```
     * @param {string} public_key
     * @returns {Promise<void>}
     */
    delete_contact(public_key) {
        const ptr0 = passStringToWasm0(public_key, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmcontactstorage_delete_contact(this.__wbg_ptr, ptr0, len0);
        return ret;
    }
    /**
     * Search contacts by name (case-insensitive partial match)
     *
     * Returns all contacts whose name contains the search query.
     *
     * # Examples
     *
     * ```
     * use paykit_demo_web::WasmContactStorage;
     * use wasm_bindgen_test::*;
     *
     * wasm_bindgen_test_configure!(run_in_browser);
     *
     * #[wasm_bindgen_test]
     * async fn search_example() {
     *     let storage = WasmContactStorage::new();
     *     let results = storage.search_contacts("alice").await.unwrap();
     *     // Returns contacts with "alice" in their name
     * }
     * ```
     * @param {string} query
     * @returns {Promise<any[]>}
     */
    search_contacts(query) {
        const ptr0 = passStringToWasm0(query, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmcontactstorage_search_contacts(this.__wbg_ptr, ptr0, len0);
        return ret;
    }
    /**
     * Update payment history for a contact
     *
     * Adds a receipt ID to the contact's payment history.
     *
     * # Examples
     *
     * ```
     * use paykit_demo_web::WasmContactStorage;
     * use wasm_bindgen_test::*;
     *
     * wasm_bindgen_test_configure!(run_in_browser);
     *
     * #[wasm_bindgen_test]
     * async fn update_history_example() {
     *     let storage = WasmContactStorage::new();
     *     let pubkey = "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo";
     *     storage.update_payment_history(pubkey, "receipt_123").await.unwrap();
     * }
     * ```
     * @param {string} public_key
     * @param {string} receipt_id
     * @returns {Promise<void>}
     */
    update_payment_history(public_key, receipt_id) {
        const ptr0 = passStringToWasm0(public_key, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(receipt_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmcontactstorage_update_payment_history(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        return ret;
    }
    /**
     * Create a new contact storage manager
     *
     * # Examples
     *
     * ```
     * use paykit_demo_web::WasmContactStorage;
     *
     * let storage = WasmContactStorage::new();
     * ```
     */
    constructor() {
        const ret = wasm.wasmcontactstorage_new();
        this.__wbg_ptr = ret >>> 0;
        WasmContactStorageFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
}
if (Symbol.dispose) WasmContactStorage.prototype[Symbol.dispose] = WasmContactStorage.prototype.free;

const WasmDashboardFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmdashboard_free(ptr >>> 0, 1));
/**
 * Dashboard statistics aggregator
 *
 * Collects statistics from all Paykit features and provides
 * a unified overview for the dashboard UI.
 *
 * # Examples
 *
 * ```
 * use paykit_demo_web::WasmDashboard;
 *
 * let dashboard = WasmDashboard::new();
 * let stats = dashboard.get_overview_stats("my_pubkey").await?;
 * ```
 */
export class WasmDashboard {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmDashboardFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmdashboard_free(ptr, 0);
    }
    /**
     * Check if setup is complete
     *
     * Returns true if the user has:
     * - At least one contact
     * - At least one payment method configured
     *
     * # Examples
     *
     * ```
     * use paykit_demo_web::WasmDashboard;
     *
     * let dashboard = WasmDashboard::new();
     * let is_ready = dashboard.is_setup_complete().await?;
     * ```
     * @returns {Promise<boolean>}
     */
    is_setup_complete() {
        const ret = wasm.wasmdashboard_is_setup_complete(this.__wbg_ptr);
        return ret;
    }
    /**
     * Get comprehensive overview statistics
     *
     * Returns an object with statistics from all features:
     * - contacts: Number of saved contacts
     * - payment_methods: Number of configured methods
     * - preferred_methods: Number of preferred methods
     * - total_receipts: Total receipts
     * - sent_receipts: Sent payments
     * - received_receipts: Received payments
     * - total_subscriptions: Total subscriptions
     * - active_subscriptions: Currently active subscriptions
     *
     * # Arguments
     *
     * * `current_pubkey` - Current user's public key for receipt direction
     *
     * # Examples
     *
     * ```
     * use paykit_demo_web::WasmDashboard;
     *
     * let dashboard = WasmDashboard::new();
     * let stats = dashboard.get_overview_stats("my_pubkey").await?;
     * ```
     * @param {string} current_pubkey
     * @returns {Promise<any>}
     */
    get_overview_stats(current_pubkey) {
        const ptr0 = passStringToWasm0(current_pubkey, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdashboard_get_overview_stats(this.__wbg_ptr, ptr0, len0);
        return ret;
    }
    /**
     * Get recent activity summary
     *
     * Returns an array of recent activity items from receipts and subscriptions.
     * Each item includes: type, timestamp, description.
     *
     * # Arguments
     *
     * * `current_pubkey` - Current user's public key
     * * `limit` - Maximum number of items to return
     *
     * # Examples
     *
     * ```
     * use paykit_demo_web::WasmDashboard;
     *
     * let dashboard = WasmDashboard::new();
     * let activity = dashboard.get_recent_activity("my_pubkey", 10).await?;
     * ```
     * @param {string} current_pubkey
     * @param {number} limit
     * @returns {Promise<any[]>}
     */
    get_recent_activity(current_pubkey, limit) {
        const ptr0 = passStringToWasm0(current_pubkey, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdashboard_get_recent_activity(this.__wbg_ptr, ptr0, len0, limit);
        return ret;
    }
    /**
     * Get setup checklist
     *
     * Returns an object with boolean flags for each setup step:
     * - has_identity: Whether identity is set (checked by caller)
     * - has_contacts: Whether user has any contacts
     * - has_payment_methods: Whether user has configured methods
     * - has_preferred_method: Whether user has a preferred method
     *
     * # Examples
     *
     * ```
     * use paykit_demo_web::WasmDashboard;
     *
     * let dashboard = WasmDashboard::new();
     * let checklist = dashboard.get_setup_checklist().await?;
     * ```
     * @returns {Promise<any>}
     */
    get_setup_checklist() {
        const ret = wasm.wasmdashboard_get_setup_checklist(this.__wbg_ptr);
        return ret;
    }
    /**
     * Create a new dashboard aggregator
     *
     * # Examples
     *
     * ```
     * use paykit_demo_web::WasmDashboard;
     *
     * let dashboard = WasmDashboard::new();
     * ```
     */
    constructor() {
        const ret = wasm.wasmdashboard_new();
        this.__wbg_ptr = ret >>> 0;
        WasmDashboardFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
}
if (Symbol.dispose) WasmDashboard.prototype[Symbol.dispose] = WasmDashboard.prototype.free;

const WasmPaymentClientFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmpaymentclient_free(ptr >>> 0, 1));
/**
 * WASM-exposed client for initiating payments over WebSocket
 */
export class WasmPaymentClient {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmPaymentClientFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmpaymentclient_free(ptr, 0);
    }
    /**
     * Create a new payment client
     */
    constructor() {
        const ret = wasm.wasmpaymentclient_new();
        this.__wbg_ptr = ret >>> 0;
        WasmPaymentClientFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Connect to a payee and initiate a payment
     * Returns a promise that resolves with the receipt
     * @param {string} _ws_url
     * @param {string} _payee_pubkey
     * @param {string} _amount
     * @param {string} _currency
     * @param {string} _method
     * @returns {Promise<any>}
     */
    pay(_ws_url, _payee_pubkey, _amount, _currency, _method) {
        const ptr0 = passStringToWasm0(_ws_url, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(_payee_pubkey, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(_amount, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ptr3 = passStringToWasm0(_currency, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len3 = WASM_VECTOR_LEN;
        const ptr4 = passStringToWasm0(_method, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len4 = WASM_VECTOR_LEN;
        const ret = wasm.wasmpaymentclient_pay(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4);
        return ret;
    }
}
if (Symbol.dispose) WasmPaymentClient.prototype[Symbol.dispose] = WasmPaymentClient.prototype.free;

const WasmPaymentCoordinatorFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmpaymentcoordinator_free(ptr >>> 0, 1));
/**
 * Payment coordinator for initiating payments
 */
export class WasmPaymentCoordinator {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmPaymentCoordinatorFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmpaymentcoordinator_free(ptr, 0);
    }
    /**
     * Get stored receipts
     * @returns {Promise<any[]>}
     */
    get_receipts() {
        const ret = wasm.wasmpaymentcoordinator_get_receipts(this.__wbg_ptr);
        return ret;
    }
    /**
     * Initiate a payment to a payee
     *
     * This performs the full payment flow:
     * 1. Connect to payee's WebSocket endpoint
     * 2. Perform Noise handshake
     * 3. Send payment request
     * 4. Receive receipt confirmation
     * 5. Store receipt
     *
     * Returns receipt JSON on success
     * @param {string} payer_identity_json
     * @param {string} ws_url
     * @param {string} payee_pubkey
     * @param {string} server_static_key_hex
     * @param {string} amount
     * @param {string} currency
     * @param {string} method
     * @returns {Promise<string>}
     */
    initiate_payment(payer_identity_json, ws_url, payee_pubkey, server_static_key_hex, amount, currency, method) {
        const ptr0 = passStringToWasm0(payer_identity_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(ws_url, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(payee_pubkey, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ptr3 = passStringToWasm0(server_static_key_hex, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len3 = WASM_VECTOR_LEN;
        const ptr4 = passStringToWasm0(amount, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len4 = WASM_VECTOR_LEN;
        const ptr5 = passStringToWasm0(currency, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len5 = WASM_VECTOR_LEN;
        const ptr6 = passStringToWasm0(method, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len6 = WASM_VECTOR_LEN;
        const ret = wasm.wasmpaymentcoordinator_initiate_payment(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4, ptr5, len5, ptr6, len6);
        return ret;
    }
    /**
     * Create new payment coordinator
     */
    constructor() {
        const ret = wasm.wasmpaymentcoordinator_new();
        this.__wbg_ptr = ret >>> 0;
        WasmPaymentCoordinatorFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
}
if (Symbol.dispose) WasmPaymentCoordinator.prototype[Symbol.dispose] = WasmPaymentCoordinator.prototype.free;

const WasmPaymentMethodConfigFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmpaymentmethodconfig_free(ptr >>> 0, 1));
/**
 * A payment method configuration
 *
 * Represents a configured payment method with endpoint, visibility,
 * and preference settings.
 *
 * # Examples
 *
 * ```
 * use paykit_demo_web::WasmPaymentMethodConfig;
 *
 * let method = WasmPaymentMethodConfig::new(
 *     "lightning".to_string(),
 *     "lnurl1dp68gurn8ghj7um5v93kketj9ehx2amn9uh8wetvdskkkmn0wahz7mrww4excup0...".to_string(),
 *     true,  // is_public
 *     true,  // is_preferred
 *     1      // priority (1 = highest)
 * ).unwrap();
 * ```
 */
export class WasmPaymentMethodConfig {

    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(WasmPaymentMethodConfig.prototype);
        obj.__wbg_ptr = ptr;
        WasmPaymentMethodConfigFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmPaymentMethodConfigFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmpaymentmethodconfig_free(ptr, 0);
    }
    /**
     * Get the preferred status
     * @returns {boolean}
     */
    get is_preferred() {
        const ret = wasm.wasmpaymentmethodconfig_is_preferred(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Create a new payment method configuration
     *
     * # Arguments
     *
     * * `method_id` - Unique identifier (e.g., "lightning", "onchain", "custom")
     * * `endpoint` - Payment endpoint (e.g., LNURL, Bitcoin address, etc.)
     * * `is_public` - Whether to publish this method publicly
     * * `is_preferred` - Whether this is a preferred method
     * * `priority` - Priority order (1 = highest priority)
     *
     * # Errors
     *
     * Returns an error if method_id or endpoint is empty.
     *
     * # Examples
     *
     * ```
     * use paykit_demo_web::WasmPaymentMethodConfig;
     *
     * let method = WasmPaymentMethodConfig::new(
     *     "lightning".to_string(),
     *     "lnurl1234...".to_string(),
     *     true,
     *     true,
     *     1
     * ).unwrap();
     * ```
     * @param {string} method_id
     * @param {string} endpoint
     * @param {boolean} is_public
     * @param {boolean} is_preferred
     * @param {number} priority
     */
    constructor(method_id, endpoint, is_public, is_preferred, priority) {
        const ptr0 = passStringToWasm0(method_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(endpoint, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmpaymentmethodconfig_new(ptr0, len0, ptr1, len1, is_public, is_preferred, priority);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        this.__wbg_ptr = ret[0] >>> 0;
        WasmPaymentMethodConfigFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Convert method to JSON string
     * @returns {string}
     */
    to_json() {
        let deferred2_0;
        let deferred2_1;
        try {
            const ret = wasm.wasmpaymentmethodconfig_to_json(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Get the endpoint
     * @returns {string}
     */
    get endpoint() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmpaymentmethodconfig_endpoint(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get the priority
     * @returns {number}
     */
    get priority() {
        const ret = wasm.wasmpaymentmethodconfig_priority(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Create method from JSON string
     * @param {string} json
     * @returns {WasmPaymentMethodConfig}
     */
    static from_json(json) {
        const ptr0 = passStringToWasm0(json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmpaymentmethodconfig_from_json(ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return WasmPaymentMethodConfig.__wrap(ret[0]);
    }
    /**
     * Get the public visibility status
     * @returns {boolean}
     */
    get is_public() {
        const ret = wasm.wasmpaymentmethodconfig_is_public(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Get the method ID
     * @returns {string}
     */
    get method_id() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmpaymentmethodconfig_method_id(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
}
if (Symbol.dispose) WasmPaymentMethodConfig.prototype[Symbol.dispose] = WasmPaymentMethodConfig.prototype.free;

const WasmPaymentMethodStorageFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmpaymentmethodstorage_free(ptr >>> 0, 1));
/**
 * Storage manager for payment methods in browser localStorage
 *
 * Provides CRUD operations for managing payment method configurations
 * with localStorage persistence.
 *
 * # Examples
 *
 * ```
 * use paykit_demo_web::{WasmPaymentMethodConfig, WasmPaymentMethodStorage};
 * use wasm_bindgen_test::*;
 *
 * wasm_bindgen_test_configure!(run_in_browser);
 *
 * #[wasm_bindgen_test]
 * async fn example_storage() {
 *     let storage = WasmPaymentMethodStorage::new();
 *     let method = WasmPaymentMethodConfig::new(
 *         "lightning".to_string(),
 *         "lnurl1234...".to_string(),
 *         true,
 *         true,
 *         1
 *     ).unwrap();
 *
 *     storage.save_method(&method).await.unwrap();
 *     let methods = storage.list_methods().await.unwrap();
 *     assert!(methods.len() >= 1);
 * }
 * ```
 */
export class WasmPaymentMethodStorage {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmPaymentMethodStorageFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmpaymentmethodstorage_free(ptr, 0);
    }
    /**
     * Get a payment method by method_id
     *
     * Returns `None` if the method doesn't exist.
     *
     * # Examples
     *
     * ```
     * use paykit_demo_web::WasmPaymentMethodStorage;
     * use wasm_bindgen_test::*;
     *
     * wasm_bindgen_test_configure!(run_in_browser);
     *
     * #[wasm_bindgen_test]
     * async fn get_example() {
     *     let storage = WasmPaymentMethodStorage::new();
     *     let method = storage.get_method("lightning").await.unwrap();
     *     // method is None if not found
     * }
     * ```
     * @param {string} method_id
     * @returns {Promise<WasmPaymentMethodConfig | undefined>}
     */
    get_method(method_id) {
        const ptr0 = passStringToWasm0(method_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmpaymentmethodstorage_get_method(this.__wbg_ptr, ptr0, len0);
        return ret;
    }
    /**
     * Save a payment method to localStorage
     *
     * If a method with the same method_id exists, it will be overwritten.
     *
     * # Examples
     *
     * ```
     * use paykit_demo_web::{WasmPaymentMethodConfig, WasmPaymentMethodStorage};
     * use wasm_bindgen_test::*;
     *
     * wasm_bindgen_test_configure!(run_in_browser);
     *
     * #[wasm_bindgen_test]
     * async fn save_example() {
     *     let storage = WasmPaymentMethodStorage::new();
     *     let method = WasmPaymentMethodConfig::new(
     *         "lightning".to_string(),
     *         "lnurl1234...".to_string(),
     *         true,
     *         true,
     *         1
     *     ).unwrap();
     *     storage.save_method(&method).await.unwrap();
     * }
     * ```
     * @param {WasmPaymentMethodConfig} method
     * @returns {Promise<void>}
     */
    save_method(method) {
        _assertClass(method, WasmPaymentMethodConfig);
        const ret = wasm.wasmpaymentmethodstorage_save_method(this.__wbg_ptr, method.__wbg_ptr);
        return ret;
    }
    /**
     * List all payment methods, sorted by priority (lowest number = highest priority)
     *
     * # Examples
     *
     * ```
     * use paykit_demo_web::WasmPaymentMethodStorage;
     * use wasm_bindgen_test::*;
     *
     * wasm_bindgen_test_configure!(run_in_browser);
     *
     * #[wasm_bindgen_test]
     * async fn list_example() {
     *     let storage = WasmPaymentMethodStorage::new();
     *     let methods = storage.list_methods().await.unwrap();
     *     // Returns empty vector if no methods
     * }
     * ```
     * @returns {Promise<any[]>}
     */
    list_methods() {
        const ret = wasm.wasmpaymentmethodstorage_list_methods(this.__wbg_ptr);
        return ret;
    }
    /**
     * Mock publish methods to Pubky homeserver
     *
     * **⚠️ WARNING: This is a MOCK implementation for demo purposes only.**
     *
     * This function simulates publishing by saving a special marker to localStorage.
     * It does NOT actually publish methods to a real Pubky homeserver.
     *
     * For production use, integrate with Pubky's authenticated PUT operations
     * to publish methods to the directory.
     *
     * # Examples
     *
     * ```
     * use paykit_demo_web::WasmPaymentMethodStorage;
     * use wasm_bindgen_test::*;
     *
     * wasm_bindgen_test_configure!(run_in_browser);
     *
     * #[wasm_bindgen_test]
     * async fn mock_publish_example() {
     *     let storage = WasmPaymentMethodStorage::new();
     *     storage.mock_publish().await.unwrap();
     * }
     * ```
     * @returns {Promise<string>}
     */
    mock_publish() {
        const ret = wasm.wasmpaymentmethodstorage_mock_publish(this.__wbg_ptr);
        return ret;
    }
    /**
     * Delete a payment method by method_id
     *
     * # Examples
     *
     * ```
     * use paykit_demo_web::WasmPaymentMethodStorage;
     * use wasm_bindgen_test::*;
     *
     * wasm_bindgen_test_configure!(run_in_browser);
     *
     * #[wasm_bindgen_test]
     * async fn delete_example() {
     *     let storage = WasmPaymentMethodStorage::new();
     *     storage.delete_method("lightning").await.unwrap();
     * }
     * ```
     * @param {string} method_id
     * @returns {Promise<void>}
     */
    delete_method(method_id) {
        const ptr0 = passStringToWasm0(method_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmpaymentmethodstorage_delete_method(this.__wbg_ptr, ptr0, len0);
        return ret;
    }
    /**
     * Set or update the preferred status of a payment method
     *
     * # Examples
     *
     * ```
     * use paykit_demo_web::WasmPaymentMethodStorage;
     * use wasm_bindgen_test::*;
     *
     * wasm_bindgen_test_configure!(run_in_browser);
     *
     * #[wasm_bindgen_test]
     * async fn set_preferred_example() {
     *     let storage = WasmPaymentMethodStorage::new();
     *     storage.set_preferred("lightning", true).await.unwrap();
     * }
     * ```
     * @param {string} method_id
     * @param {boolean} preferred
     * @returns {Promise<void>}
     */
    set_preferred(method_id, preferred) {
        const ptr0 = passStringToWasm0(method_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmpaymentmethodstorage_set_preferred(this.__wbg_ptr, ptr0, len0, preferred);
        return ret;
    }
    /**
     * Update the priority of a payment method
     *
     * Lower numbers = higher priority (1 is highest)
     *
     * # Examples
     *
     * ```
     * use paykit_demo_web::WasmPaymentMethodStorage;
     * use wasm_bindgen_test::*;
     *
     * wasm_bindgen_test_configure!(run_in_browser);
     *
     * #[wasm_bindgen_test]
     * async fn update_priority_example() {
     *     let storage = WasmPaymentMethodStorage::new();
     *     storage.update_priority("lightning", 1).await.unwrap();
     * }
     * ```
     * @param {string} method_id
     * @param {number} priority
     * @returns {Promise<void>}
     */
    update_priority(method_id, priority) {
        const ptr0 = passStringToWasm0(method_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmpaymentmethodstorage_update_priority(this.__wbg_ptr, ptr0, len0, priority);
        return ret;
    }
    /**
     * Get all preferred payment methods, sorted by priority
     *
     * # Examples
     *
     * ```
     * use paykit_demo_web::WasmPaymentMethodStorage;
     * use wasm_bindgen_test::*;
     *
     * wasm_bindgen_test_configure!(run_in_browser);
     *
     * #[wasm_bindgen_test]
     * async fn get_preferred_example() {
     *     let storage = WasmPaymentMethodStorage::new();
     *     let preferred = storage.get_preferred_methods().await.unwrap();
     * }
     * ```
     * @returns {Promise<any[]>}
     */
    get_preferred_methods() {
        const ret = wasm.wasmpaymentmethodstorage_get_preferred_methods(this.__wbg_ptr);
        return ret;
    }
    /**
     * Create a new payment method storage manager
     *
     * # Examples
     *
     * ```
     * use paykit_demo_web::WasmPaymentMethodStorage;
     *
     * let storage = WasmPaymentMethodStorage::new();
     * ```
     */
    constructor() {
        const ret = wasm.wasmpaymentmethodstorage_new();
        this.__wbg_ptr = ret >>> 0;
        WasmPaymentMethodStorageFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
}
if (Symbol.dispose) WasmPaymentMethodStorage.prototype[Symbol.dispose] = WasmPaymentMethodStorage.prototype.free;

const WasmPaymentReceiverFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmpaymentreceiver_free(ptr >>> 0, 1));
/**
 * Payment receiver for accepting payments
 */
export class WasmPaymentReceiver {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmPaymentReceiverFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmpaymentreceiver_free(ptr, 0);
    }
    /**
     * Get stored receipts
     * @returns {Promise<any[]>}
     */
    get_receipts() {
        const ret = wasm.wasmpaymentreceiver_get_receipts(this.__wbg_ptr);
        return ret;
    }
    /**
     * Accept a payment request
     *
     * Note: In browser, this typically requires a WebSocket relay server
     * since browsers cannot directly accept incoming connections.
     * @param {string} request_json
     * @returns {Promise<string>}
     */
    accept_payment(request_json) {
        const ptr0 = passStringToWasm0(request_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmpaymentreceiver_accept_payment(this.__wbg_ptr, ptr0, len0);
        return ret;
    }
    /**
     * Create new payment receiver
     */
    constructor() {
        const ret = wasm.wasmpaymentcoordinator_new();
        this.__wbg_ptr = ret >>> 0;
        WasmPaymentReceiverFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
}
if (Symbol.dispose) WasmPaymentReceiver.prototype[Symbol.dispose] = WasmPaymentReceiver.prototype.free;

const WasmPaymentRequestFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmpaymentrequest_free(ptr >>> 0, 1));
/**
 * JavaScript-friendly payment request
 */
export class WasmPaymentRequest {

    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(WasmPaymentRequest.prototype);
        obj.__wbg_ptr = ptr;
        WasmPaymentRequestFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmPaymentRequestFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmpaymentrequest_free(ptr, 0);
    }
    /**
     * Get created timestamp
     * @returns {bigint}
     */
    get created_at() {
        const ret = wasm.wasmpaymentrequest_created_at(this.__wbg_ptr);
        return ret;
    }
    /**
     * Get expiration timestamp
     * @returns {bigint | undefined}
     */
    get expires_at() {
        const ret = wasm.wasmpaymentrequest_expires_at(this.__wbg_ptr);
        return ret[0] === 0 ? undefined : ret[1];
    }
    /**
     * Check if expired
     * @returns {boolean}
     */
    is_expired() {
        const ret = wasm.wasmpaymentrequest_is_expired(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Get request ID
     * @returns {string}
     */
    get request_id() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmpaymentrequest_request_id(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get description
     * @returns {string | undefined}
     */
    get description() {
        const ret = wasm.wasmpaymentrequest_description(this.__wbg_ptr);
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * Add expiration time (Unix timestamp)
     * @param {bigint} expires_at
     * @returns {WasmPaymentRequest}
     */
    with_expiration(expires_at) {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.wasmpaymentrequest_with_expiration(ptr, expires_at);
        return WasmPaymentRequest.__wrap(ret);
    }
    /**
     * Add description to the request
     * @param {string} description
     * @returns {WasmPaymentRequest}
     */
    with_description(description) {
        const ptr = this.__destroy_into_raw();
        const ptr0 = passStringToWasm0(description, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmpaymentrequest_with_description(ptr, ptr0, len0);
        return WasmPaymentRequest.__wrap(ret);
    }
    /**
     * Get to public key
     * @returns {string}
     */
    get to() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmpaymentrequest_to(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Create a new payment request
     * @param {string} from_pubkey
     * @param {string} to_pubkey
     * @param {string} amount
     * @param {string} currency
     * @param {string} method
     */
    constructor(from_pubkey, to_pubkey, amount, currency, method) {
        const ptr0 = passStringToWasm0(from_pubkey, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(to_pubkey, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(amount, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ptr3 = passStringToWasm0(currency, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len3 = WASM_VECTOR_LEN;
        const ptr4 = passStringToWasm0(method, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len4 = WASM_VECTOR_LEN;
        const ret = wasm.wasmpaymentrequest_new(ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        this.__wbg_ptr = ret[0] >>> 0;
        WasmPaymentRequestFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Get from public key
     * @returns {string}
     */
    get from() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmpaymentrequest_from(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get amount
     * @returns {string}
     */
    get amount() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmpaymentrequest_amount(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Convert to JSON
     * @returns {string}
     */
    to_json() {
        let deferred2_0;
        let deferred2_1;
        try {
            const ret = wasm.wasmpaymentrequest_to_json(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Get currency
     * @returns {string}
     */
    get currency() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmpaymentrequest_currency(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Create from JSON
     * @param {string} json
     * @returns {WasmPaymentRequest}
     */
    static from_json(json) {
        const ptr0 = passStringToWasm0(json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmpaymentrequest_from_json(ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return WasmPaymentRequest.__wrap(ret[0]);
    }
}
if (Symbol.dispose) WasmPaymentRequest.prototype[Symbol.dispose] = WasmPaymentRequest.prototype.free;

const WasmPaymentServerFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmpaymentserver_free(ptr >>> 0, 1));
/**
 * WASM-exposed server for receiving payments over WebSocket
 */
export class WasmPaymentServer {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmPaymentServerFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmpaymentserver_free(ptr, 0);
    }
    /**
     * Create a new payment server
     */
    constructor() {
        const ret = wasm.wasmpaymentclient_new();
        this.__wbg_ptr = ret >>> 0;
        WasmPaymentServerFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Start listening for payment requests
     * Note: In browser, this requires a WebSocket relay server
     * @param {number} _port
     * @returns {Promise<void>}
     */
    listen(_port) {
        const ret = wasm.wasmpaymentserver_listen(this.__wbg_ptr, _port);
        return ret;
    }
}
if (Symbol.dispose) WasmPaymentServer.prototype[Symbol.dispose] = WasmPaymentServer.prototype.free;

const WasmPeerSpendingLimitFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmpeerspendinglimit_free(ptr >>> 0, 1));
/**
 * WASM-friendly peer spending limit
 */
export class WasmPeerSpendingLimit {

    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(WasmPeerSpendingLimit.prototype);
        obj.__wbg_ptr = ptr;
        WasmPeerSpendingLimitFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmPeerSpendingLimitFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmpeerspendinglimit_free(ptr, 0);
    }
    /**
     * Get the peer public key
     * @returns {string}
     */
    get peer_pubkey() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmpeerspendinglimit_peer_pubkey(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get the total limit
     * @returns {bigint}
     */
    get total_limit() {
        const ret = wasm.wasmautopayrule_max_amount(this.__wbg_ptr);
        return ret;
    }
    /**
     * Get the period start timestamp
     * @returns {bigint}
     */
    get period_start() {
        const ret = wasm.wasmpeerspendinglimit_period_start(this.__wbg_ptr);
        return ret;
    }
    /**
     * Get the current spent amount
     * @returns {bigint}
     */
    get current_spent() {
        const ret = wasm.wasmautopayrule_period_seconds(this.__wbg_ptr);
        return ret;
    }
    /**
     * Get the period in seconds
     * @returns {bigint}
     */
    get period_seconds() {
        const ret = wasm.wasmpeerspendinglimit_period_seconds(this.__wbg_ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
     * Record a payment
     * @param {bigint} amount
     */
    record_payment(amount) {
        const ret = wasm.wasmpeerspendinglimit_record_payment(this.__wbg_ptr, amount);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Get the remaining limit
     * @returns {bigint}
     */
    get remaining_limit() {
        const ret = wasm.wasmpeerspendinglimit_remaining_limit(this.__wbg_ptr);
        return ret;
    }
    /**
     * Create a new peer spending limit
     * @param {string} peer_pubkey
     * @param {bigint} total_limit
     * @param {bigint} period_seconds
     */
    constructor(peer_pubkey, total_limit, period_seconds) {
        const ptr0 = passStringToWasm0(peer_pubkey, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmpeerspendinglimit_new(ptr0, len0, total_limit, period_seconds);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        this.__wbg_ptr = ret[0] >>> 0;
        WasmPeerSpendingLimitFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Reset the spending counter
     */
    reset() {
        wasm.wasmpeerspendinglimit_reset(this.__wbg_ptr);
    }
    /**
     * Convert to JSON for storage
     * @returns {string}
     */
    to_json() {
        let deferred2_0;
        let deferred2_1;
        try {
            const ret = wasm.wasmpeerspendinglimit_to_json(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Check if a payment amount is allowed
     * @param {bigint} amount
     * @returns {boolean}
     */
    can_spend(amount) {
        const ret = wasm.wasmpeerspendinglimit_can_spend(this.__wbg_ptr, amount);
        return ret !== 0;
    }
    /**
     * Create from JSON
     * @param {string} json
     * @returns {WasmPeerSpendingLimit}
     */
    static from_json(json) {
        const ptr0 = passStringToWasm0(json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmpeerspendinglimit_from_json(ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return WasmPeerSpendingLimit.__wrap(ret[0]);
    }
}
if (Symbol.dispose) WasmPeerSpendingLimit.prototype[Symbol.dispose] = WasmPeerSpendingLimit.prototype.free;

const WasmPeerSpendingLimitStorageFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmpeerspendinglimitstorage_free(ptr >>> 0, 1));
/**
 * Storage for peer spending limits in browser localStorage
 */
export class WasmPeerSpendingLimitStorage {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmPeerSpendingLimitStorageFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmpeerspendinglimitstorage_free(ptr, 0);
    }
    /**
     * Get a peer spending limit by peer pubkey
     * @param {string} peer_pubkey
     * @returns {Promise<WasmPeerSpendingLimit | undefined>}
     */
    get_peer_limit(peer_pubkey) {
        const ptr0 = passStringToWasm0(peer_pubkey, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmpeerspendinglimitstorage_get_peer_limit(this.__wbg_ptr, ptr0, len0);
        return ret;
    }
    /**
     * Save a peer spending limit
     * @param {WasmPeerSpendingLimit} limit
     * @returns {Promise<void>}
     */
    save_peer_limit(limit) {
        _assertClass(limit, WasmPeerSpendingLimit);
        const ret = wasm.wasmpeerspendinglimitstorage_save_peer_limit(this.__wbg_ptr, limit.__wbg_ptr);
        return ret;
    }
    /**
     * List all peer spending limits
     * @returns {Promise<any[]>}
     */
    list_peer_limits() {
        const ret = wasm.wasmpeerspendinglimitstorage_list_peer_limits(this.__wbg_ptr);
        return ret;
    }
    /**
     * Delete a peer spending limit
     * @param {string} peer_pubkey
     * @returns {Promise<void>}
     */
    delete_peer_limit(peer_pubkey) {
        const ptr0 = passStringToWasm0(peer_pubkey, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmpeerspendinglimitstorage_delete_peer_limit(this.__wbg_ptr, ptr0, len0);
        return ret;
    }
    /**
     * Create new storage manager
     */
    constructor() {
        const ret = wasm.wasmpeerspendinglimitstorage_new();
        this.__wbg_ptr = ret >>> 0;
        WasmPeerSpendingLimitStorageFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Clear all peer spending limits
     * @returns {Promise<void>}
     */
    clear_all() {
        const ret = wasm.wasmpeerspendinglimitstorage_clear_all(this.__wbg_ptr);
        return ret;
    }
}
if (Symbol.dispose) WasmPeerSpendingLimitStorage.prototype[Symbol.dispose] = WasmPeerSpendingLimitStorage.prototype.free;

const WasmReceiptStorageFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmreceiptstorage_free(ptr >>> 0, 1));
/**
 * Receipt storage in browser localStorage
 */
export class WasmReceiptStorage {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmReceiptStorageFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmreceiptstorage_free(ptr, 0);
    }
    /**
     * Get a receipt by ID
     * @param {string} receipt_id
     * @returns {Promise<string | undefined>}
     */
    get_receipt(receipt_id) {
        const ptr0 = passStringToWasm0(receipt_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmreceiptstorage_get_receipt(this.__wbg_ptr, ptr0, len0);
        return ret;
    }
    /**
     * Save a receipt
     * @param {string} receipt_id
     * @param {string} receipt_json
     * @returns {Promise<void>}
     */
    save_receipt(receipt_id, receipt_json) {
        const ptr0 = passStringToWasm0(receipt_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(receipt_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmreceiptstorage_save_receipt(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        return ret;
    }
    /**
     * List all receipts
     * @returns {Promise<any[]>}
     */
    list_receipts() {
        const ret = wasm.wasmreceiptstorage_list_receipts(this.__wbg_ptr);
        return ret;
    }
    /**
     * Delete a receipt
     * @param {string} receipt_id
     * @returns {Promise<void>}
     */
    delete_receipt(receipt_id) {
        const ptr0 = passStringToWasm0(receipt_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmreceiptstorage_delete_receipt(this.__wbg_ptr, ptr0, len0);
        return ret;
    }
    /**
     * Export receipts as JSON array
     *
     * # Returns
     *
     * A JSON string containing array of all receipts
     *
     * # Examples
     *
     * ```
     * let storage = WasmReceiptStorage::new();
     * let json = storage.export_as_json().await?;
     * // Download or process json
     * ```
     * @returns {Promise<string>}
     */
    export_as_json() {
        const ret = wasm.wasmreceiptstorage_export_as_json(this.__wbg_ptr);
        return ret;
    }
    /**
     * Get receipt statistics
     *
     * Returns an object with:
     * - total: Total number of receipts
     * - sent: Number of sent payments
     * - received: Number of received payments
     *
     * # Arguments
     *
     * * `current_pubkey` - Current user's public key
     *
     * # Examples
     *
     * ```
     * let storage = WasmReceiptStorage::new();
     * let stats = storage.get_statistics("my_pubkey").await?;
     * ```
     * @param {string} current_pubkey
     * @returns {Promise<any>}
     */
    get_statistics(current_pubkey) {
        const ptr0 = passStringToWasm0(current_pubkey, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmreceiptstorage_get_statistics(this.__wbg_ptr, ptr0, len0);
        return ret;
    }
    /**
     * Filter receipts by method
     *
     * # Arguments
     *
     * * `method` - Payment method ID (e.g., "lightning", "onchain")
     *
     * # Examples
     *
     * ```
     * let storage = WasmReceiptStorage::new();
     * let lightning_receipts = storage.filter_by_method("lightning").await?;
     * ```
     * @param {string} method
     * @returns {Promise<any[]>}
     */
    filter_by_method(method) {
        const ptr0 = passStringToWasm0(method, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmreceiptstorage_filter_by_method(this.__wbg_ptr, ptr0, len0);
        return ret;
    }
    /**
     * Filter receipts by contact public key
     *
     * # Arguments
     *
     * * `contact_pubkey` - Public key of the contact
     * * `current_pubkey` - Current user's public key
     *
     * # Examples
     *
     * ```
     * let storage = WasmReceiptStorage::new();
     * let alice_receipts = storage.filter_by_contact("8pin...", "my_pubkey").await?;
     * ```
     * @param {string} contact_pubkey
     * @param {string} current_pubkey
     * @returns {Promise<any[]>}
     */
    filter_by_contact(contact_pubkey, current_pubkey) {
        const ptr0 = passStringToWasm0(contact_pubkey, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(current_pubkey, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmreceiptstorage_filter_by_contact(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        return ret;
    }
    /**
     * Filter receipts by direction (sent/received)
     *
     * # Arguments
     *
     * * `direction` - "sent" or "received"
     * * `current_pubkey` - Current user's public key to determine direction
     *
     * # Examples
     *
     * ```
     * let storage = WasmReceiptStorage::new();
     * let sent = storage.filter_by_direction("sent", "8pin...").await?;
     * ```
     * @param {string} direction
     * @param {string} current_pubkey
     * @returns {Promise<any[]>}
     */
    filter_by_direction(direction, current_pubkey) {
        const ptr0 = passStringToWasm0(direction, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(current_pubkey, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmreceiptstorage_filter_by_direction(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        return ret;
    }
    /**
     * Create new receipt storage
     */
    constructor() {
        const ret = wasm.wasmpaymentcoordinator_new();
        this.__wbg_ptr = ret >>> 0;
        WasmReceiptStorageFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Clear all receipts
     *
     * # Examples
     *
     * ```
     * let storage = WasmReceiptStorage::new();
     * storage.clear_all().await?;
     * ```
     * @returns {Promise<void>}
     */
    clear_all() {
        const ret = wasm.wasmreceiptstorage_clear_all(this.__wbg_ptr);
        return ret;
    }
}
if (Symbol.dispose) WasmReceiptStorage.prototype[Symbol.dispose] = WasmReceiptStorage.prototype.free;

const WasmRequestStorageFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmrequeststorage_free(ptr >>> 0, 1));
/**
 * Request-only storage manager for browser (simplified wrapper)
 * For full subscription storage, use WasmSubscriptionAgreementStorage
 */
export class WasmRequestStorage {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmRequestStorageFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmrequeststorage_free(ptr, 0);
    }
    /**
     * Get a payment request by ID
     * @param {string} request_id
     * @returns {Promise<WasmPaymentRequest | undefined>}
     */
    get_request(request_id) {
        const ptr0 = passStringToWasm0(request_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmrequeststorage_get_request(this.__wbg_ptr, ptr0, len0);
        return ret;
    }
    /**
     * Save a payment request to browser localStorage
     * @param {WasmPaymentRequest} request
     * @returns {Promise<void>}
     */
    save_request(request) {
        _assertClass(request, WasmPaymentRequest);
        const ret = wasm.wasmrequeststorage_save_request(this.__wbg_ptr, request.__wbg_ptr);
        return ret;
    }
    /**
     * List all payment requests
     * @returns {Promise<any[]>}
     */
    list_requests() {
        const ret = wasm.wasmrequeststorage_list_requests(this.__wbg_ptr);
        return ret;
    }
    /**
     * Delete a payment request
     * @param {string} request_id
     * @returns {Promise<void>}
     */
    delete_request(request_id) {
        const ptr0 = passStringToWasm0(request_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmrequeststorage_delete_request(this.__wbg_ptr, ptr0, len0);
        return ret;
    }
    /**
     * Create new storage manager
     * @param {string | null} [storage_key]
     */
    constructor(storage_key) {
        var ptr0 = isLikeNone(storage_key) ? 0 : passStringToWasm0(storage_key, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmrequeststorage_new(ptr0, len0);
        this.__wbg_ptr = ret >>> 0;
        WasmRequestStorageFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Clear all payment requests
     * @returns {Promise<void>}
     */
    clear_all() {
        const ret = wasm.wasmrequeststorage_clear_all(this.__wbg_ptr);
        return ret;
    }
}
if (Symbol.dispose) WasmRequestStorage.prototype[Symbol.dispose] = WasmRequestStorage.prototype.free;

const WasmSignedSubscriptionFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmsignedsubscription_free(ptr >>> 0, 1));
/**
 * JavaScript-friendly signed subscription
 */
export class WasmSignedSubscription {

    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(WasmSignedSubscription.prototype);
        obj.__wbg_ptr = ptr;
        WasmSignedSubscriptionFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmSignedSubscriptionFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmsignedsubscription_free(ptr, 0);
    }
    /**
     * Check if expired
     * @returns {boolean}
     */
    is_expired() {
        const ret = wasm.wasmsignedsubscription_is_expired(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Get subscription details
     * @returns {WasmSubscription}
     */
    subscription() {
        const ret = wasm.wasmsignedsubscription_subscription(this.__wbg_ptr);
        return WasmSubscription.__wrap(ret);
    }
    /**
     * Check if signatures are valid
     * @returns {boolean}
     */
    verify_signatures() {
        const ret = wasm.wasmsignedsubscription_verify_signatures(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] !== 0;
    }
    /**
     * Check if active
     * @returns {boolean}
     */
    is_active() {
        const ret = wasm.wasmsignedsubscription_is_active(this.__wbg_ptr);
        return ret !== 0;
    }
}
if (Symbol.dispose) WasmSignedSubscription.prototype[Symbol.dispose] = WasmSignedSubscription.prototype.free;

const WasmSubscriptionFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmsubscription_free(ptr >>> 0, 1));
/**
 * JavaScript-friendly subscription
 */
export class WasmSubscription {

    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(WasmSubscription.prototype);
        obj.__wbg_ptr = ptr;
        WasmSubscriptionFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmSubscriptionFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmsubscription_free(ptr, 0);
    }
    /**
     * Get created timestamp
     * @returns {bigint}
     */
    get created_at() {
        const ret = wasm.wasmsubscription_created_at(this.__wbg_ptr);
        return ret;
    }
    /**
     * Check if expired
     * @returns {boolean}
     */
    is_expired() {
        const ret = wasm.wasmsubscription_is_expired(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Get subscriber public key
     * @returns {string}
     */
    get subscriber() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmsubscription_subscriber(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get description
     * @returns {string}
     */
    get description() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmsubscription_description(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get subscription ID
     * @returns {string}
     */
    get subscription_id() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmsubscription_subscription_id(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Create a new subscription
     * @param {string} subscriber_pubkey
     * @param {string} provider_pubkey
     * @param {string} amount
     * @param {string} currency
     * @param {string} frequency
     * @param {string} description
     */
    constructor(subscriber_pubkey, provider_pubkey, amount, currency, frequency, description) {
        const ptr0 = passStringToWasm0(subscriber_pubkey, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(provider_pubkey, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(amount, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ptr3 = passStringToWasm0(currency, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len3 = WASM_VECTOR_LEN;
        const ptr4 = passStringToWasm0(frequency, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len4 = WASM_VECTOR_LEN;
        const ptr5 = passStringToWasm0(description, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len5 = WASM_VECTOR_LEN;
        const ret = wasm.wasmsubscription_new(ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4, ptr5, len5);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        this.__wbg_ptr = ret[0] >>> 0;
        WasmSubscriptionFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Get amount
     * @returns {string}
     */
    get amount() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmsubscription_amount(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get ends timestamp (or null)
     * @returns {bigint | undefined}
     */
    get ends_at() {
        const ret = wasm.wasmsubscription_ends_at(this.__wbg_ptr);
        return ret[0] === 0 ? undefined : ret[1];
    }
    /**
     * Get currency
     * @returns {string}
     */
    get currency() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmsubscription_currency(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get provider public key
     * @returns {string}
     */
    get provider() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmsubscription_provider(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Validate subscription
     */
    validate() {
        const ret = wasm.wasmsubscription_validate(this.__wbg_ptr);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Get frequency
     * @returns {string}
     */
    get frequency() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmsubscription_frequency(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Check if active
     * @returns {boolean}
     */
    is_active() {
        const ret = wasm.wasmsubscription_is_active(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Get starts timestamp
     * @returns {bigint}
     */
    get starts_at() {
        const ret = wasm.wasmsubscription_starts_at(this.__wbg_ptr);
        return ret;
    }
}
if (Symbol.dispose) WasmSubscription.prototype[Symbol.dispose] = WasmSubscription.prototype.free;

const WasmSubscriptionAgreementStorageFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmsubscriptionagreementstorage_free(ptr >>> 0, 1));
/**
 * Storage for subscription agreements (WASM)
 *
 * Full implementation using browser localStorage
 */
export class WasmSubscriptionAgreementStorage {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmSubscriptionAgreementStorageFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmsubscriptionagreementstorage_free(ptr, 0);
    }
    /**
     * Get a subscription by ID
     * @param {string} id
     * @returns {Promise<WasmSubscription | undefined>}
     */
    get_subscription(id) {
        const ptr0 = passStringToWasm0(id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmsubscriptionagreementstorage_get_subscription(this.__wbg_ptr, ptr0, len0);
        return ret;
    }
    /**
     * Save a subscription
     * @param {WasmSubscription} subscription
     * @returns {Promise<void>}
     */
    save_subscription(subscription) {
        _assertClass(subscription, WasmSubscription);
        const ret = wasm.wasmsubscriptionagreementstorage_save_subscription(this.__wbg_ptr, subscription.__wbg_ptr);
        return ret;
    }
    /**
     * Delete a subscription by ID
     * @param {string} id
     * @returns {Promise<void>}
     */
    delete_subscription(id) {
        const ptr0 = passStringToWasm0(id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmsubscriptionagreementstorage_delete_subscription(this.__wbg_ptr, ptr0, len0);
        return ret;
    }
    /**
     * List all subscriptions (including inactive)
     * @returns {Promise<any[]>}
     */
    list_all_subscriptions() {
        const ret = wasm.wasmsubscriptionagreementstorage_list_all_subscriptions(this.__wbg_ptr);
        return ret;
    }
    /**
     * Get a signed subscription by ID
     * @param {string} id
     * @returns {Promise<WasmSignedSubscription | undefined>}
     */
    get_signed_subscription(id) {
        const ptr0 = passStringToWasm0(id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmsubscriptionagreementstorage_get_signed_subscription(this.__wbg_ptr, ptr0, len0);
        return ret;
    }
    /**
     * Save a signed subscription
     * @param {WasmSignedSubscription} signed
     * @returns {Promise<void>}
     */
    save_signed_subscription(signed) {
        _assertClass(signed, WasmSignedSubscription);
        const ret = wasm.wasmsubscriptionagreementstorage_save_signed_subscription(this.__wbg_ptr, signed.__wbg_ptr);
        return ret;
    }
    /**
     * List active subscriptions
     * @returns {Promise<any[]>}
     */
    list_active_subscriptions() {
        const ret = wasm.wasmsubscriptionagreementstorage_list_active_subscriptions(this.__wbg_ptr);
        return ret;
    }
    /**
     * Delete a signed subscription by ID
     * @param {string} id
     * @returns {Promise<void>}
     */
    delete_signed_subscription(id) {
        const ptr0 = passStringToWasm0(id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmsubscriptionagreementstorage_delete_signed_subscription(this.__wbg_ptr, ptr0, len0);
        return ret;
    }
    /**
     * Create new storage (uses browser localStorage)
     */
    constructor() {
        const ret = wasm.wasmsubscriptionagreementstorage_new();
        this.__wbg_ptr = ret >>> 0;
        WasmSubscriptionAgreementStorageFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Clear all subscriptions
     * @returns {Promise<void>}
     */
    clear_all() {
        const ret = wasm.wasmsubscriptionagreementstorage_clear_all(this.__wbg_ptr);
        return ret;
    }
}
if (Symbol.dispose) WasmSubscriptionAgreementStorage.prototype[Symbol.dispose] = WasmSubscriptionAgreementStorage.prototype.free;

const EXPECTED_RESPONSE_TYPES = new Set(['basic', 'cors', 'default']);

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);

            } catch (e) {
                const validResponse = module.ok && EXPECTED_RESPONSE_TYPES.has(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else {
                    throw e;
                }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);

    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };

        } else {
            return instance;
        }
    }
}

function __wbg_get_imports() {
    const imports = {};
    imports.wbg = {};
    imports.wbg.__wbg___wbindgen_boolean_get_6d5a1ee65bab5f68 = function(arg0) {
        const v = arg0;
        const ret = typeof(v) === 'boolean' ? v : undefined;
        return isLikeNone(ret) ? 0xFFFFFF : ret ? 1 : 0;
    };
    imports.wbg.__wbg___wbindgen_debug_string_df47ffb5e35e6763 = function(arg0, arg1) {
        const ret = debugString(arg1);
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg___wbindgen_is_function_ee8a6c5833c90377 = function(arg0) {
        const ret = typeof(arg0) === 'function';
        return ret;
    };
    imports.wbg.__wbg___wbindgen_is_object_c818261d21f283a4 = function(arg0) {
        const val = arg0;
        const ret = typeof(val) === 'object' && val !== null;
        return ret;
    };
    imports.wbg.__wbg___wbindgen_is_string_fbb76cb2940daafd = function(arg0) {
        const ret = typeof(arg0) === 'string';
        return ret;
    };
    imports.wbg.__wbg___wbindgen_is_undefined_2d472862bd29a478 = function(arg0) {
        const ret = arg0 === undefined;
        return ret;
    };
    imports.wbg.__wbg___wbindgen_number_get_a20bf9b85341449d = function(arg0, arg1) {
        const obj = arg1;
        const ret = typeof(obj) === 'number' ? obj : undefined;
        getDataViewMemory0().setFloat64(arg0 + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
    };
    imports.wbg.__wbg___wbindgen_string_get_e4f06c90489ad01b = function(arg0, arg1) {
        const obj = arg1;
        const ret = typeof(obj) === 'string' ? obj : undefined;
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg___wbindgen_throw_b855445ff6a94295 = function(arg0, arg1) {
        throw new Error(getStringFromWasm0(arg0, arg1));
    };
    imports.wbg.__wbg__wbg_cb_unref_2454a539ea5790d9 = function(arg0) {
        arg0._wbg_cb_unref();
    };
    imports.wbg.__wbg_buffer_ccc4520b36d3ccf4 = function(arg0) {
        const ret = arg0.buffer;
        return ret;
    };
    imports.wbg.__wbg_call_525440f72fbfc0ea = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = arg0.call(arg1, arg2);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_call_e762c39fa8ea36bf = function() { return handleError(function (arg0, arg1) {
        const ret = arg0.call(arg1);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_code_20d453b11b200026 = function(arg0) {
        const ret = arg0.code;
        return ret;
    };
    imports.wbg.__wbg_crypto_574e78ad8b13b65f = function(arg0) {
        const ret = arg0.crypto;
        return ret;
    };
    imports.wbg.__wbg_data_ee4306d069f24f2d = function(arg0) {
        const ret = arg0.data;
        return ret;
    };
    imports.wbg.__wbg_error_5f6e662ebe6fb57a = function(arg0, arg1) {
        console.error(getStringFromWasm0(arg0, arg1));
    };
    imports.wbg.__wbg_fetch_cf02cfa16eaaaae8 = function(arg0, arg1, arg2) {
        const ret = arg0.fetch(getStringFromWasm0(arg1, arg2));
        return ret;
    };
    imports.wbg.__wbg_getItem_89f57d6acc51a876 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        const ret = arg1.getItem(getStringFromWasm0(arg2, arg3));
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    }, arguments) };
    imports.wbg.__wbg_getRandomValues_38a1ff1ea09f6cc7 = function() { return handleError(function (arg0, arg1) {
        globalThis.crypto.getRandomValues(getArrayU8FromWasm0(arg0, arg1));
    }, arguments) };
    imports.wbg.__wbg_getRandomValues_b8f5dbd5f3995a9e = function() { return handleError(function (arg0, arg1) {
        arg0.getRandomValues(arg1);
    }, arguments) };
    imports.wbg.__wbg_getTime_14776bfb48a1bff9 = function(arg0) {
        const ret = arg0.getTime();
        return ret;
    };
    imports.wbg.__wbg_get_efcb449f58ec27c2 = function() { return handleError(function (arg0, arg1) {
        const ret = Reflect.get(arg0, arg1);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_instanceof_ArrayBuffer_70beb1189ca63b38 = function(arg0) {
        let result;
        try {
            result = arg0 instanceof ArrayBuffer;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_Response_f4f3e87e07f3135c = function(arg0) {
        let result;
        try {
            result = arg0 instanceof Response;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_Window_4846dbb3de56c84c = function(arg0) {
        let result;
        try {
            result = arg0 instanceof Window;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_json_5d2ba74e315ef6e6 = function() { return handleError(function (arg0) {
        const ret = arg0.json();
        return ret;
    }, arguments) };
    imports.wbg.__wbg_key_38d01a092280ffc6 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = arg1.key(arg2 >>> 0);
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    }, arguments) };
    imports.wbg.__wbg_length_69bca3cb64fc8748 = function(arg0) {
        const ret = arg0.length;
        return ret;
    };
    imports.wbg.__wbg_length_7534a213da0a65cd = function() { return handleError(function (arg0) {
        const ret = arg0.length;
        return ret;
    }, arguments) };
    imports.wbg.__wbg_localStorage_3034501cd2b3da3f = function() { return handleError(function (arg0) {
        const ret = arg0.localStorage;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    }, arguments) };
    imports.wbg.__wbg_log_487361ca397c9fab = function(arg0, arg1) {
        console.log(getStringFromWasm0(arg0, arg1));
    };
    imports.wbg.__wbg_message_3abccea43568e0bd = function(arg0, arg1) {
        const ret = arg1.message;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_msCrypto_a61aeb35a24c1329 = function(arg0) {
        const ret = arg0.msCrypto;
        return ret;
    };
    imports.wbg.__wbg_new_0_f9740686d739025c = function() {
        const ret = new Date();
        return ret;
    };
    imports.wbg.__wbg_new_1acc0b6eea89d040 = function() {
        const ret = new Object();
        return ret;
    };
    imports.wbg.__wbg_new_3c3d849046688a66 = function(arg0, arg1) {
        try {
            var state0 = {a: arg0, b: arg1};
            var cb0 = (arg0, arg1) => {
                const a = state0.a;
                state0.a = 0;
                try {
                    return wasm_bindgen__convert__closures_____invoke__h8a0305fb7488cc73(a, state0.b, arg0, arg1);
                } finally {
                    state0.a = a;
                }
            };
            const ret = new Promise(cb0);
            return ret;
        } finally {
            state0.a = state0.b = 0;
        }
    };
    imports.wbg.__wbg_new_5a79be3ab53b8aa5 = function(arg0) {
        const ret = new Uint8Array(arg0);
        return ret;
    };
    imports.wbg.__wbg_new_881c4fe631eee9ad = function() { return handleError(function (arg0, arg1) {
        const ret = new WebSocket(getStringFromWasm0(arg0, arg1));
        return ret;
    }, arguments) };
    imports.wbg.__wbg_new_a7442b4b19c1a356 = function(arg0, arg1) {
        const ret = new Error(getStringFromWasm0(arg0, arg1));
        return ret;
    };
    imports.wbg.__wbg_new_from_slice_92f4d78ca282a2d2 = function(arg0, arg1) {
        const ret = new Uint8Array(getArrayU8FromWasm0(arg0, arg1));
        return ret;
    };
    imports.wbg.__wbg_new_no_args_ee98eee5275000a4 = function(arg0, arg1) {
        const ret = new Function(getStringFromWasm0(arg0, arg1));
        return ret;
    };
    imports.wbg.__wbg_new_with_length_01aa0dc35aa13543 = function(arg0) {
        const ret = new Uint8Array(arg0 >>> 0);
        return ret;
    };
    imports.wbg.__wbg_node_905d3e251edff8a2 = function(arg0) {
        const ret = arg0.node;
        return ret;
    };
    imports.wbg.__wbg_now_793306c526e2e3b6 = function() {
        const ret = Date.now();
        return ret;
    };
    imports.wbg.__wbg_ok_5749966cb2b8535e = function(arg0) {
        const ret = arg0.ok;
        return ret;
    };
    imports.wbg.__wbg_parse_2a704d6b78abb2b8 = function() { return handleError(function (arg0, arg1) {
        const ret = JSON.parse(getStringFromWasm0(arg0, arg1));
        return ret;
    }, arguments) };
    imports.wbg.__wbg_process_dc0fbacc7c1c06f7 = function(arg0) {
        const ret = arg0.process;
        return ret;
    };
    imports.wbg.__wbg_prototypesetcall_2a6620b6922694b2 = function(arg0, arg1, arg2) {
        Uint8Array.prototype.set.call(getArrayU8FromWasm0(arg0, arg1), arg2);
    };
    imports.wbg.__wbg_queueMicrotask_34d692c25c47d05b = function(arg0) {
        const ret = arg0.queueMicrotask;
        return ret;
    };
    imports.wbg.__wbg_queueMicrotask_9d76cacb20c84d58 = function(arg0) {
        queueMicrotask(arg0);
    };
    imports.wbg.__wbg_randomFillSync_ac0988aba3254290 = function() { return handleError(function (arg0, arg1) {
        arg0.randomFillSync(arg1);
    }, arguments) };
    imports.wbg.__wbg_readyState_97984f126080aeda = function(arg0) {
        const ret = arg0.readyState;
        return ret;
    };
    imports.wbg.__wbg_reason_1cced37e3a93763e = function(arg0, arg1) {
        const ret = arg1.reason;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_removeItem_0e1e70f1687b5304 = function() { return handleError(function (arg0, arg1, arg2) {
        arg0.removeItem(getStringFromWasm0(arg1, arg2));
    }, arguments) };
    imports.wbg.__wbg_require_60cc747a6bc5215a = function() { return handleError(function () {
        const ret = module.require;
        return ret;
    }, arguments) };
    imports.wbg.__wbg_resolve_caf97c30b83f7053 = function(arg0) {
        const ret = Promise.resolve(arg0);
        return ret;
    };
    imports.wbg.__wbg_send_25caa2dbdb78318d = function() { return handleError(function (arg0, arg1) {
        arg0.send(arg1);
    }, arguments) };
    imports.wbg.__wbg_setItem_64dfb54d7b20d84c = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        arg0.setItem(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
    }, arguments) };
    imports.wbg.__wbg_set_binaryType_9d839cea8fcdc5c3 = function(arg0, arg1) {
        arg0.binaryType = __wbindgen_enum_BinaryType[arg1];
    };
    imports.wbg.__wbg_set_c2abbebe8b9ebee1 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = Reflect.set(arg0, arg1, arg2);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_set_onclose_c09e4f7422de8dae = function(arg0, arg1) {
        arg0.onclose = arg1;
    };
    imports.wbg.__wbg_set_onerror_337a3a2db9517378 = function(arg0, arg1) {
        arg0.onerror = arg1;
    };
    imports.wbg.__wbg_set_onmessage_8661558551a89792 = function(arg0, arg1) {
        arg0.onmessage = arg1;
    };
    imports.wbg.__wbg_set_onopen_efccb9305427b907 = function(arg0, arg1) {
        arg0.onopen = arg1;
    };
    imports.wbg.__wbg_static_accessor_GLOBAL_89e1d9ac6a1b250e = function() {
        const ret = typeof global === 'undefined' ? null : global;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_static_accessor_GLOBAL_THIS_8b530f326a9e48ac = function() {
        const ret = typeof globalThis === 'undefined' ? null : globalThis;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_static_accessor_SELF_6fdf4b64710cc91b = function() {
        const ret = typeof self === 'undefined' ? null : self;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_static_accessor_WINDOW_b45bfc5a37f6cfa2 = function() {
        const ret = typeof window === 'undefined' ? null : window;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_status_de7eed5a7a5bfd5d = function(arg0) {
        const ret = arg0.status;
        return ret;
    };
    imports.wbg.__wbg_stringify_b5fb28f6465d9c3e = function() { return handleError(function (arg0) {
        const ret = JSON.stringify(arg0);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_subarray_480600f3d6a9f26c = function(arg0, arg1, arg2) {
        const ret = arg0.subarray(arg1 >>> 0, arg2 >>> 0);
        return ret;
    };
    imports.wbg.__wbg_then_4f46f6544e6b4a28 = function(arg0, arg1) {
        const ret = arg0.then(arg1);
        return ret;
    };
    imports.wbg.__wbg_then_70d05cf780a18d77 = function(arg0, arg1, arg2) {
        const ret = arg0.then(arg1, arg2);
        return ret;
    };
    imports.wbg.__wbg_versions_c01dfd4722a88165 = function(arg0) {
        const ret = arg0.versions;
        return ret;
    };
    imports.wbg.__wbg_wasmautopayrule_new = function(arg0) {
        const ret = WasmAutoPayRule.__wrap(arg0);
        return ret;
    };
    imports.wbg.__wbg_wasmcontact_new = function(arg0) {
        const ret = WasmContact.__wrap(arg0);
        return ret;
    };
    imports.wbg.__wbg_wasmpaymentmethodconfig_new = function(arg0) {
        const ret = WasmPaymentMethodConfig.__wrap(arg0);
        return ret;
    };
    imports.wbg.__wbg_wasmpaymentrequest_new = function(arg0) {
        const ret = WasmPaymentRequest.__wrap(arg0);
        return ret;
    };
    imports.wbg.__wbg_wasmpeerspendinglimit_new = function(arg0) {
        const ret = WasmPeerSpendingLimit.__wrap(arg0);
        return ret;
    };
    imports.wbg.__wbg_wasmsignedsubscription_new = function(arg0) {
        const ret = WasmSignedSubscription.__wrap(arg0);
        return ret;
    };
    imports.wbg.__wbg_wasmsubscription_new = function(arg0) {
        const ret = WasmSubscription.__wrap(arg0);
        return ret;
    };
    imports.wbg.__wbindgen_cast_2241b6af4c4b2941 = function(arg0, arg1) {
        // Cast intrinsic for `Ref(String) -> Externref`.
        const ret = getStringFromWasm0(arg0, arg1);
        return ret;
    };
    imports.wbg.__wbindgen_cast_3e6d7a467edf4f66 = function(arg0, arg1) {
        // Cast intrinsic for `Closure(Closure { dtor_idx: 166, function: Function { arguments: [NamedExternref("CloseEvent")], shim_idx: 167, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
        const ret = makeMutClosure(arg0, arg1, wasm.wasm_bindgen__closure__destroy__h0ea10b8e17c2589a, wasm_bindgen__convert__closures_____invoke__h7460171fa07d4e7b);
        return ret;
    };
    imports.wbg.__wbindgen_cast_4625c577ab2ec9ee = function(arg0) {
        // Cast intrinsic for `U64 -> Externref`.
        const ret = BigInt.asUintN(64, arg0);
        return ret;
    };
    imports.wbg.__wbindgen_cast_752ed32918f8e5cb = function(arg0, arg1) {
        // Cast intrinsic for `Closure(Closure { dtor_idx: 424, function: Function { arguments: [], shim_idx: 425, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
        const ret = makeMutClosure(arg0, arg1, wasm.wasm_bindgen__closure__destroy__he9ff11ce1c64d320, wasm_bindgen__convert__closures_____invoke__hec0e381372c60b88);
        return ret;
    };
    imports.wbg.__wbindgen_cast_98c349af0503c7f4 = function(arg0, arg1) {
        // Cast intrinsic for `Closure(Closure { dtor_idx: 166, function: Function { arguments: [NamedExternref("MessageEvent")], shim_idx: 167, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
        const ret = makeMutClosure(arg0, arg1, wasm.wasm_bindgen__closure__destroy__h0ea10b8e17c2589a, wasm_bindgen__convert__closures_____invoke__h7460171fa07d4e7b);
        return ret;
    };
    imports.wbg.__wbindgen_cast_9ae0607507abb057 = function(arg0) {
        // Cast intrinsic for `I64 -> Externref`.
        const ret = arg0;
        return ret;
    };
    imports.wbg.__wbindgen_cast_cb9088102bce6b30 = function(arg0, arg1) {
        // Cast intrinsic for `Ref(Slice(U8)) -> NamedExternref("Uint8Array")`.
        const ret = getArrayU8FromWasm0(arg0, arg1);
        return ret;
    };
    imports.wbg.__wbindgen_cast_d6cd19b81560fd6e = function(arg0) {
        // Cast intrinsic for `F64 -> Externref`.
        const ret = arg0;
        return ret;
    };
    imports.wbg.__wbindgen_cast_e481686c74984159 = function(arg0, arg1) {
        var v0 = getArrayJsValueFromWasm0(arg0, arg1).slice();
        wasm.__wbindgen_free(arg0, arg1 * 4, 4);
        // Cast intrinsic for `Vector(Externref) -> Externref`.
        const ret = v0;
        return ret;
    };
    imports.wbg.__wbindgen_cast_e77cc7d1fa54f319 = function(arg0, arg1) {
        // Cast intrinsic for `Closure(Closure { dtor_idx: 427, function: Function { arguments: [Externref], shim_idx: 428, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
        const ret = makeMutClosure(arg0, arg1, wasm.wasm_bindgen__closure__destroy__he7277012e90784de, wasm_bindgen__convert__closures_____invoke__h75da7eae032c0859);
        return ret;
    };
    imports.wbg.__wbindgen_cast_fb0c6bde9bba27e4 = function(arg0, arg1) {
        // Cast intrinsic for `Closure(Closure { dtor_idx: 166, function: Function { arguments: [NamedExternref("ErrorEvent")], shim_idx: 167, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
        const ret = makeMutClosure(arg0, arg1, wasm.wasm_bindgen__closure__destroy__h0ea10b8e17c2589a, wasm_bindgen__convert__closures_____invoke__h7460171fa07d4e7b);
        return ret;
    };
    imports.wbg.__wbindgen_init_externref_table = function() {
        const table = wasm.__wbindgen_externrefs;
        const offset = table.grow(4);
        table.set(0, undefined);
        table.set(offset + 0, undefined);
        table.set(offset + 1, null);
        table.set(offset + 2, true);
        table.set(offset + 3, false);
        ;
    };

    return imports;
}

function __wbg_finalize_init(instance, module) {
    wasm = instance.exports;
    __wbg_init.__wbindgen_wasm_module = module;
    cachedDataViewMemory0 = null;
    cachedUint8ArrayMemory0 = null;


    wasm.__wbindgen_start();
    return wasm;
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (typeof module !== 'undefined') {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();

    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }

    const instance = new WebAssembly.Instance(module, imports);

    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (typeof module_or_path !== 'undefined') {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (typeof module_or_path === 'undefined') {
        module_or_path = new URL('paykit_demo_web_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync };
export default __wbg_init;

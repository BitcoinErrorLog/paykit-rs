/* tslint:disable */
/* eslint-disable */
/**
 * Initialize the WASM module
 *
 * This should be called once when the module is loaded.
 * It sets up panic hooks for better error messages in the browser console.
 */
export function init(): void;
/**
 * Get the version of the Paykit WASM module
 */
export function version(): string;
export function is_valid_pubkey(pubkey: string): boolean;
/**
 * Utility functions for subscriptions
 */
export function format_timestamp(timestamp: bigint): string;
/**
 * Storage manager for browser localStorage
 */
export class BrowserStorage {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Load an identity from localStorage
   */
  loadIdentity(name: string): Identity;
  /**
   * Save an identity to localStorage
   */
  saveIdentity(name: string, identity: Identity): void;
  /**
   * Delete an identity from localStorage
   */
  deleteIdentity(name: string): void;
  /**
   * List all saved identity names
   */
  listIdentities(): any[];
  /**
   * Get the current active identity name
   */
  getCurrentIdentity(): string | undefined;
  /**
   * Set the current active identity
   */
  setCurrentIdentity(name: string): void;
  /**
   * Create a new browser storage manager
   */
  constructor();
  /**
   * Clear all Paykit data from localStorage
   */
  clearAll(): void;
}
/**
 * Directory client for querying payment methods
 */
export class DirectoryClient {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Query payment methods for a public key
   */
  queryMethods(public_key: string): Promise<any>;
  /**
   * Publish payment methods (placeholder - requires authentication)
   */
  publishMethods(_methods: any): Promise<void>;
  /**
   * Create a new directory client
   */
  constructor(homeserver: string);
}
/**
 * JavaScript-facing identity wrapper
 */
export class Identity {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Get the public key as a hex string
   */
  publicKey(): string;
  /**
   * Create an identity with a nickname
   */
  static withNickname(nickname: string): Identity;
  /**
   * Get Ed25519 public key (for Noise identity)
   * Returns hex-encoded public key
   */
  ed25519PublicKeyHex(): string;
  /**
   * Get Ed25519 secret key (for Noise key derivation)
   * Returns hex-encoded secret key
   */
  ed25519SecretKeyHex(): string;
  /**
   * Generate a new random identity
   */
  constructor();
  /**
   * Export identity to JSON string
   */
  toJSON(): string;
  /**
   * Get the nickname (if set)
   */
  nickname(): string | undefined;
  /**
   * Import identity from JSON string
   */
  static fromJSON(json: string): Identity;
  /**
   * Get the Pubky URI
   */
  pubkyUri(): string;
}
/**
 * WASM-friendly auto-pay rule
 */
export class WasmAutoPayRule {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a new auto-pay rule
   */
  constructor(peer_pubkey: string, max_amount: bigint, period_seconds: bigint);
  /**
   * Enable the rule
   */
  enable(): void;
  /**
   * Disable the rule
   */
  disable(): void;
  /**
   * Get the maximum amount
   */
  readonly max_amount: bigint;
  /**
   * Get the peer public key
   */
  readonly peer_pubkey: string;
  /**
   * Get the period in seconds
   */
  readonly period_seconds: bigint;
  /**
   * Get the rule ID
   */
  readonly id: string;
  /**
   * Check if the rule is enabled
   */
  readonly enabled: boolean;
}
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
  free(): void;
  [Symbol.dispose](): void;
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
   */
  with_notes(notes: string): WasmContact;
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
   */
  constructor(public_key: string, name: string);
  /**
   * Convert contact to JSON string
   */
  to_json(): string;
  /**
   * Create contact from JSON string
   */
  static from_json(json: string): WasmContact;
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
   */
  pubky_uri(): string;
  /**
   * Get the contact's public key
   */
  readonly public_key: string;
  /**
   * Get the contact's payment history (receipt IDs)
   */
  readonly payment_history: any[];
  /**
   * Get the contact's name
   */
  readonly name: string;
  /**
   * Get the contact's notes
   */
  readonly notes: string | undefined;
  /**
   * Get the timestamp when contact was added
   */
  readonly added_at: bigint;
}
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
  free(): void;
  [Symbol.dispose](): void;
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
   */
  get_contact(public_key: string): Promise<WasmContact | undefined>;
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
   */
  save_contact(contact: WasmContact): Promise<void>;
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
   */
  list_contacts(): Promise<any[]>;
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
   */
  delete_contact(public_key: string): Promise<void>;
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
   */
  search_contacts(query: string): Promise<any[]>;
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
   */
  update_payment_history(public_key: string, receipt_id: string): Promise<void>;
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
  constructor();
}
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
  free(): void;
  [Symbol.dispose](): void;
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
   */
  is_setup_complete(): Promise<boolean>;
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
   */
  get_overview_stats(current_pubkey: string): Promise<any>;
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
   */
  get_recent_activity(current_pubkey: string, limit: number): Promise<any[]>;
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
   */
  get_setup_checklist(): Promise<any>;
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
  constructor();
}
/**
 * WASM-exposed client for initiating payments over WebSocket
 */
export class WasmPaymentClient {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a new payment client
   */
  constructor();
  /**
   * Connect to a payee and initiate a payment
   * Returns a promise that resolves with the receipt
   */
  pay(_ws_url: string, _payee_pubkey: string, _amount: string, _currency: string, _method: string): Promise<any>;
}
/**
 * Payment coordinator for initiating payments
 */
export class WasmPaymentCoordinator {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Get stored receipts
   */
  get_receipts(): Promise<any[]>;
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
   */
  initiate_payment(payer_identity_json: string, ws_url: string, payee_pubkey: string, server_static_key_hex: string, amount: string, currency: string, method: string): Promise<string>;
  /**
   * Create new payment coordinator
   */
  constructor();
}
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
  free(): void;
  [Symbol.dispose](): void;
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
   */
  constructor(method_id: string, endpoint: string, is_public: boolean, is_preferred: boolean, priority: number);
  /**
   * Convert method to JSON string
   */
  to_json(): string;
  /**
   * Create method from JSON string
   */
  static from_json(json: string): WasmPaymentMethodConfig;
  /**
   * Get the preferred status
   */
  readonly is_preferred: boolean;
  /**
   * Get the endpoint
   */
  readonly endpoint: string;
  /**
   * Get the priority
   */
  readonly priority: number;
  /**
   * Get the public visibility status
   */
  readonly is_public: boolean;
  /**
   * Get the method ID
   */
  readonly method_id: string;
}
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
  free(): void;
  [Symbol.dispose](): void;
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
   */
  get_method(method_id: string): Promise<WasmPaymentMethodConfig | undefined>;
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
   */
  save_method(method: WasmPaymentMethodConfig): Promise<void>;
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
   */
  list_methods(): Promise<any[]>;
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
   */
  mock_publish(): Promise<string>;
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
   */
  delete_method(method_id: string): Promise<void>;
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
   */
  set_preferred(method_id: string, preferred: boolean): Promise<void>;
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
   */
  update_priority(method_id: string, priority: number): Promise<void>;
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
   */
  get_preferred_methods(): Promise<any[]>;
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
  constructor();
}
/**
 * Payment receiver for accepting payments
 */
export class WasmPaymentReceiver {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Get stored receipts
   */
  get_receipts(): Promise<any[]>;
  /**
   * Accept a payment request
   *
   * Note: In browser, this typically requires a WebSocket relay server
   * since browsers cannot directly accept incoming connections.
   */
  accept_payment(request_json: string): Promise<string>;
  /**
   * Create new payment receiver
   */
  constructor();
}
/**
 * JavaScript-friendly payment request
 */
export class WasmPaymentRequest {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Check if expired
   */
  is_expired(): boolean;
  /**
   * Add expiration time (Unix timestamp)
   */
  with_expiration(expires_at: bigint): WasmPaymentRequest;
  /**
   * Add description to the request
   */
  with_description(description: string): WasmPaymentRequest;
  /**
   * Create a new payment request
   */
  constructor(from_pubkey: string, to_pubkey: string, amount: string, currency: string, method: string);
  /**
   * Convert to JSON
   */
  to_json(): string;
  /**
   * Create from JSON
   */
  static from_json(json: string): WasmPaymentRequest;
  /**
   * Get created timestamp
   */
  readonly created_at: bigint;
  /**
   * Get expiration timestamp
   */
  readonly expires_at: bigint | undefined;
  /**
   * Get request ID
   */
  readonly request_id: string;
  /**
   * Get description
   */
  readonly description: string | undefined;
  /**
   * Get to public key
   */
  readonly to: string;
  /**
   * Get from public key
   */
  readonly from: string;
  /**
   * Get amount
   */
  readonly amount: string;
  /**
   * Get currency
   */
  readonly currency: string;
}
/**
 * WASM-exposed server for receiving payments over WebSocket
 */
export class WasmPaymentServer {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a new payment server
   */
  constructor();
  /**
   * Start listening for payment requests
   * Note: In browser, this requires a WebSocket relay server
   */
  listen(_port: number): Promise<void>;
}
/**
 * WASM-friendly peer spending limit
 */
export class WasmPeerSpendingLimit {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Record a payment
   */
  record_payment(amount: bigint): void;
  /**
   * Create a new peer spending limit
   */
  constructor(peer_pubkey: string, total_limit: bigint, period_seconds: bigint);
  /**
   * Reset the spending counter
   */
  reset(): void;
  /**
   * Check if a payment amount is allowed
   */
  can_spend(amount: bigint): boolean;
  /**
   * Get the peer public key
   */
  readonly peer_pubkey: string;
  /**
   * Get the total limit
   */
  readonly total_limit: bigint;
  /**
   * Get the current spent amount
   */
  readonly current_spent: bigint;
  /**
   * Get the period in seconds
   */
  readonly period_seconds: bigint;
  /**
   * Get the remaining limit
   */
  readonly remaining_limit: bigint;
}
/**
 * Receipt storage in browser localStorage
 */
export class WasmReceiptStorage {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Get a receipt by ID
   */
  get_receipt(receipt_id: string): Promise<string | undefined>;
  /**
   * Save a receipt
   */
  save_receipt(receipt_id: string, receipt_json: string): Promise<void>;
  /**
   * List all receipts
   */
  list_receipts(): Promise<any[]>;
  /**
   * Delete a receipt
   */
  delete_receipt(receipt_id: string): Promise<void>;
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
   */
  export_as_json(): Promise<string>;
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
   */
  get_statistics(current_pubkey: string): Promise<any>;
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
   */
  filter_by_method(method: string): Promise<any[]>;
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
   */
  filter_by_contact(contact_pubkey: string, current_pubkey: string): Promise<any[]>;
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
   */
  filter_by_direction(direction: string, current_pubkey: string): Promise<any[]>;
  /**
   * Create new receipt storage
   */
  constructor();
  /**
   * Clear all receipts
   *
   * # Examples
   *
   * ```
   * let storage = WasmReceiptStorage::new();
   * storage.clear_all().await?;
   * ```
   */
  clear_all(): Promise<void>;
}
/**
 * Request-only storage manager for browser (simplified wrapper)
 * For full subscription storage, use WasmSubscriptionAgreementStorage
 */
export class WasmRequestStorage {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Get a payment request by ID
   */
  get_request(request_id: string): Promise<WasmPaymentRequest | undefined>;
  /**
   * Save a payment request to browser localStorage
   */
  save_request(request: WasmPaymentRequest): Promise<void>;
  /**
   * List all payment requests
   */
  list_requests(): Promise<any[]>;
  /**
   * Delete a payment request
   */
  delete_request(request_id: string): Promise<void>;
  /**
   * Create new storage manager
   */
  constructor(storage_key?: string | null);
  /**
   * Clear all payment requests
   */
  clear_all(): Promise<void>;
}
/**
 * JavaScript-friendly signed subscription
 */
export class WasmSignedSubscription {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Check if expired
   */
  is_expired(): boolean;
  /**
   * Get subscription details
   */
  subscription(): WasmSubscription;
  /**
   * Check if signatures are valid
   */
  verify_signatures(): boolean;
  /**
   * Check if active
   */
  is_active(): boolean;
}
/**
 * JavaScript-friendly subscription
 */
export class WasmSubscription {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Check if expired
   */
  is_expired(): boolean;
  /**
   * Create a new subscription
   */
  constructor(subscriber_pubkey: string, provider_pubkey: string, amount: string, currency: string, frequency: string, description: string);
  /**
   * Validate subscription
   */
  validate(): void;
  /**
   * Check if active
   */
  is_active(): boolean;
  /**
   * Get created timestamp
   */
  readonly created_at: bigint;
  /**
   * Get subscriber public key
   */
  readonly subscriber: string;
  /**
   * Get description
   */
  readonly description: string;
  /**
   * Get subscription ID
   */
  readonly subscription_id: string;
  /**
   * Get amount
   */
  readonly amount: string;
  /**
   * Get ends timestamp (or null)
   */
  readonly ends_at: bigint | undefined;
  /**
   * Get currency
   */
  readonly currency: string;
  /**
   * Get provider public key
   */
  readonly provider: string;
  /**
   * Get frequency
   */
  readonly frequency: string;
  /**
   * Get starts timestamp
   */
  readonly starts_at: bigint;
}
/**
 * Storage for subscription agreements (WASM)
 *
 * Full implementation using browser localStorage
 */
export class WasmSubscriptionAgreementStorage {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Get a subscription by ID
   */
  get_subscription(id: string): Promise<WasmSubscription | undefined>;
  /**
   * Save a subscription
   */
  save_subscription(subscription: WasmSubscription): Promise<void>;
  /**
   * Delete a subscription by ID
   */
  delete_subscription(id: string): Promise<void>;
  /**
   * List all subscriptions (including inactive)
   */
  list_all_subscriptions(): Promise<any[]>;
  /**
   * Get a signed subscription by ID
   */
  get_signed_subscription(id: string): Promise<WasmSignedSubscription | undefined>;
  /**
   * Save a signed subscription
   */
  save_signed_subscription(signed: WasmSignedSubscription): Promise<void>;
  /**
   * List active subscriptions
   */
  list_active_subscriptions(): Promise<any[]>;
  /**
   * Delete a signed subscription by ID
   */
  delete_signed_subscription(id: string): Promise<void>;
  /**
   * Create new storage (uses browser localStorage)
   */
  constructor();
  /**
   * Clear all subscriptions
   */
  clear_all(): Promise<void>;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_identity_free: (a: number, b: number) => void;
  readonly identity_ed25519PublicKeyHex: (a: number) => [number, number];
  readonly identity_ed25519SecretKeyHex: (a: number) => [number, number];
  readonly identity_fromJSON: (a: number, b: number) => [number, number, number];
  readonly identity_new: () => [number, number, number];
  readonly identity_nickname: (a: number) => [number, number];
  readonly identity_pubkyUri: (a: number) => [number, number];
  readonly identity_publicKey: (a: number) => [number, number];
  readonly identity_toJSON: (a: number) => [number, number, number, number];
  readonly identity_withNickname: (a: number, b: number) => [number, number, number];
  readonly version: () => [number, number];
  readonly init: () => void;
  readonly __wbg_wasmautopayrule_free: (a: number, b: number) => void;
  readonly __wbg_wasmpaymentrequest_free: (a: number, b: number) => void;
  readonly __wbg_wasmpeerspendinglimit_free: (a: number, b: number) => void;
  readonly __wbg_wasmrequeststorage_free: (a: number, b: number) => void;
  readonly __wbg_wasmsignedsubscription_free: (a: number, b: number) => void;
  readonly __wbg_wasmsubscription_free: (a: number, b: number) => void;
  readonly __wbg_wasmsubscriptionagreementstorage_free: (a: number, b: number) => void;
  readonly format_timestamp: (a: bigint) => [number, number];
  readonly is_valid_pubkey: (a: number, b: number) => number;
  readonly wasmautopayrule_disable: (a: number) => void;
  readonly wasmautopayrule_enable: (a: number) => void;
  readonly wasmautopayrule_enabled: (a: number) => number;
  readonly wasmautopayrule_id: (a: number) => [number, number];
  readonly wasmautopayrule_max_amount: (a: number) => bigint;
  readonly wasmautopayrule_new: (a: number, b: number, c: bigint, d: bigint) => [number, number, number];
  readonly wasmautopayrule_peer_pubkey: (a: number) => [number, number];
  readonly wasmautopayrule_period_seconds: (a: number) => bigint;
  readonly wasmpaymentrequest_amount: (a: number) => [number, number];
  readonly wasmpaymentrequest_created_at: (a: number) => bigint;
  readonly wasmpaymentrequest_currency: (a: number) => [number, number];
  readonly wasmpaymentrequest_description: (a: number) => [number, number];
  readonly wasmpaymentrequest_expires_at: (a: number) => [number, bigint];
  readonly wasmpaymentrequest_from: (a: number) => [number, number];
  readonly wasmpaymentrequest_from_json: (a: number, b: number) => [number, number, number];
  readonly wasmpaymentrequest_is_expired: (a: number) => number;
  readonly wasmpaymentrequest_new: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number) => [number, number, number];
  readonly wasmpaymentrequest_request_id: (a: number) => [number, number];
  readonly wasmpaymentrequest_to: (a: number) => [number, number];
  readonly wasmpaymentrequest_to_json: (a: number) => [number, number, number, number];
  readonly wasmpaymentrequest_with_description: (a: number, b: number, c: number) => number;
  readonly wasmpaymentrequest_with_expiration: (a: number, b: bigint) => number;
  readonly wasmpeerspendinglimit_can_spend: (a: number, b: bigint) => number;
  readonly wasmpeerspendinglimit_new: (a: number, b: number, c: bigint, d: bigint) => [number, number, number];
  readonly wasmpeerspendinglimit_peer_pubkey: (a: number) => [number, number];
  readonly wasmpeerspendinglimit_period_seconds: (a: number) => bigint;
  readonly wasmpeerspendinglimit_record_payment: (a: number, b: bigint) => [number, number];
  readonly wasmpeerspendinglimit_remaining_limit: (a: number) => bigint;
  readonly wasmpeerspendinglimit_reset: (a: number) => void;
  readonly wasmrequeststorage_clear_all: (a: number) => any;
  readonly wasmrequeststorage_delete_request: (a: number, b: number, c: number) => any;
  readonly wasmrequeststorage_get_request: (a: number, b: number, c: number) => any;
  readonly wasmrequeststorage_list_requests: (a: number) => any;
  readonly wasmrequeststorage_new: (a: number, b: number) => number;
  readonly wasmrequeststorage_save_request: (a: number, b: number) => any;
  readonly wasmsignedsubscription_is_active: (a: number) => number;
  readonly wasmsignedsubscription_is_expired: (a: number) => number;
  readonly wasmsignedsubscription_subscription: (a: number) => number;
  readonly wasmsignedsubscription_verify_signatures: (a: number) => [number, number, number];
  readonly wasmsubscription_amount: (a: number) => [number, number];
  readonly wasmsubscription_created_at: (a: number) => bigint;
  readonly wasmsubscription_currency: (a: number) => [number, number];
  readonly wasmsubscription_description: (a: number) => [number, number];
  readonly wasmsubscription_frequency: (a: number) => [number, number];
  readonly wasmsubscription_is_active: (a: number) => number;
  readonly wasmsubscription_is_expired: (a: number) => number;
  readonly wasmsubscription_new: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number) => [number, number, number];
  readonly wasmsubscription_provider: (a: number) => [number, number];
  readonly wasmsubscription_starts_at: (a: number) => bigint;
  readonly wasmsubscription_subscriber: (a: number) => [number, number];
  readonly wasmsubscription_subscription_id: (a: number) => [number, number];
  readonly wasmsubscription_validate: (a: number) => [number, number];
  readonly wasmsubscriptionagreementstorage_clear_all: (a: number) => any;
  readonly wasmsubscriptionagreementstorage_delete_signed_subscription: (a: number, b: number, c: number) => any;
  readonly wasmsubscriptionagreementstorage_delete_subscription: (a: number, b: number, c: number) => any;
  readonly wasmsubscriptionagreementstorage_get_signed_subscription: (a: number, b: number, c: number) => any;
  readonly wasmsubscriptionagreementstorage_get_subscription: (a: number, b: number, c: number) => any;
  readonly wasmsubscriptionagreementstorage_list_active_subscriptions: (a: number) => any;
  readonly wasmsubscriptionagreementstorage_list_all_subscriptions: (a: number) => any;
  readonly wasmsubscriptionagreementstorage_new: () => number;
  readonly wasmsubscriptionagreementstorage_save_signed_subscription: (a: number, b: number) => any;
  readonly wasmsubscriptionagreementstorage_save_subscription: (a: number, b: number) => any;
  readonly wasmsubscription_ends_at: (a: number) => [number, bigint];
  readonly wasmpeerspendinglimit_current_spent: (a: number) => bigint;
  readonly wasmpeerspendinglimit_total_limit: (a: number) => bigint;
  readonly __wbg_wasmcontact_free: (a: number, b: number) => void;
  readonly __wbg_wasmcontactstorage_free: (a: number, b: number) => void;
  readonly wasmcontact_added_at: (a: number) => bigint;
  readonly wasmcontact_from_json: (a: number, b: number) => [number, number, number];
  readonly wasmcontact_name: (a: number) => [number, number];
  readonly wasmcontact_new: (a: number, b: number, c: number, d: number) => [number, number, number];
  readonly wasmcontact_notes: (a: number) => [number, number];
  readonly wasmcontact_payment_history: (a: number) => [number, number];
  readonly wasmcontact_pubky_uri: (a: number) => [number, number];
  readonly wasmcontact_public_key: (a: number) => [number, number];
  readonly wasmcontact_to_json: (a: number) => [number, number, number, number];
  readonly wasmcontact_with_notes: (a: number, b: number, c: number) => number;
  readonly wasmcontactstorage_delete_contact: (a: number, b: number, c: number) => any;
  readonly wasmcontactstorage_get_contact: (a: number, b: number, c: number) => any;
  readonly wasmcontactstorage_list_contacts: (a: number) => any;
  readonly wasmcontactstorage_new: () => number;
  readonly wasmcontactstorage_save_contact: (a: number, b: number) => any;
  readonly wasmcontactstorage_search_contacts: (a: number, b: number, c: number) => any;
  readonly wasmcontactstorage_update_payment_history: (a: number, b: number, c: number, d: number, e: number) => any;
  readonly __wbg_browserstorage_free: (a: number, b: number) => void;
  readonly browserstorage_clearAll: (a: number) => [number, number];
  readonly browserstorage_deleteIdentity: (a: number, b: number, c: number) => [number, number];
  readonly browserstorage_getCurrentIdentity: (a: number) => [number, number, number, number];
  readonly browserstorage_listIdentities: (a: number) => [number, number, number, number];
  readonly browserstorage_loadIdentity: (a: number, b: number, c: number) => [number, number, number];
  readonly browserstorage_saveIdentity: (a: number, b: number, c: number, d: number) => [number, number];
  readonly browserstorage_setCurrentIdentity: (a: number, b: number, c: number) => [number, number];
  readonly browserstorage_new: () => number;
  readonly __wbg_wasmpaymentmethodconfig_free: (a: number, b: number) => void;
  readonly __wbg_wasmpaymentmethodstorage_free: (a: number, b: number) => void;
  readonly wasmpaymentmethodconfig_endpoint: (a: number) => [number, number];
  readonly wasmpaymentmethodconfig_from_json: (a: number, b: number) => [number, number, number];
  readonly wasmpaymentmethodconfig_is_preferred: (a: number) => number;
  readonly wasmpaymentmethodconfig_is_public: (a: number) => number;
  readonly wasmpaymentmethodconfig_method_id: (a: number) => [number, number];
  readonly wasmpaymentmethodconfig_new: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number, number];
  readonly wasmpaymentmethodconfig_priority: (a: number) => number;
  readonly wasmpaymentmethodconfig_to_json: (a: number) => [number, number, number, number];
  readonly wasmpaymentmethodstorage_delete_method: (a: number, b: number, c: number) => any;
  readonly wasmpaymentmethodstorage_get_method: (a: number, b: number, c: number) => any;
  readonly wasmpaymentmethodstorage_get_preferred_methods: (a: number) => any;
  readonly wasmpaymentmethodstorage_list_methods: (a: number) => any;
  readonly wasmpaymentmethodstorage_mock_publish: (a: number) => any;
  readonly wasmpaymentmethodstorage_new: () => number;
  readonly wasmpaymentmethodstorage_save_method: (a: number, b: number) => any;
  readonly wasmpaymentmethodstorage_set_preferred: (a: number, b: number, c: number, d: number) => any;
  readonly wasmpaymentmethodstorage_update_priority: (a: number, b: number, c: number, d: number) => any;
  readonly __wbg_wasmpaymentcoordinator_free: (a: number, b: number) => void;
  readonly wasmpaymentcoordinator_get_receipts: (a: number) => any;
  readonly wasmpaymentcoordinator_initiate_payment: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number, n: number, o: number) => any;
  readonly wasmpaymentcoordinator_new: () => number;
  readonly wasmpaymentreceiver_accept_payment: (a: number, b: number, c: number) => any;
  readonly wasmpaymentreceiver_get_receipts: (a: number) => any;
  readonly wasmreceiptstorage_clear_all: (a: number) => any;
  readonly wasmreceiptstorage_delete_receipt: (a: number, b: number, c: number) => any;
  readonly wasmreceiptstorage_export_as_json: (a: number) => any;
  readonly wasmreceiptstorage_filter_by_contact: (a: number, b: number, c: number, d: number, e: number) => any;
  readonly wasmreceiptstorage_filter_by_direction: (a: number, b: number, c: number, d: number, e: number) => any;
  readonly wasmreceiptstorage_filter_by_method: (a: number, b: number, c: number) => any;
  readonly wasmreceiptstorage_get_receipt: (a: number, b: number, c: number) => any;
  readonly wasmreceiptstorage_get_statistics: (a: number, b: number, c: number) => any;
  readonly wasmreceiptstorage_list_receipts: (a: number) => any;
  readonly wasmreceiptstorage_save_receipt: (a: number, b: number, c: number, d: number, e: number) => any;
  readonly wasmpaymentreceiver_new: () => number;
  readonly wasmreceiptstorage_new: () => number;
  readonly __wbg_wasmpaymentreceiver_free: (a: number, b: number) => void;
  readonly __wbg_wasmreceiptstorage_free: (a: number, b: number) => void;
  readonly __wbg_directoryclient_free: (a: number, b: number) => void;
  readonly __wbg_wasmdashboard_free: (a: number, b: number) => void;
  readonly directoryclient_new: (a: number, b: number) => number;
  readonly directoryclient_publishMethods: (a: number, b: any) => any;
  readonly directoryclient_queryMethods: (a: number, b: number, c: number) => any;
  readonly wasmdashboard_get_overview_stats: (a: number, b: number, c: number) => any;
  readonly wasmdashboard_get_recent_activity: (a: number, b: number, c: number, d: number) => any;
  readonly wasmdashboard_get_setup_checklist: (a: number) => any;
  readonly wasmdashboard_is_setup_complete: (a: number) => any;
  readonly wasmdashboard_new: () => number;
  readonly __wbg_wasmpaymentclient_free: (a: number, b: number) => void;
  readonly __wbg_wasmpaymentserver_free: (a: number, b: number) => void;
  readonly wasmpaymentclient_pay: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number) => any;
  readonly wasmpaymentserver_listen: (a: number, b: number) => any;
  readonly wasmpaymentclient_new: () => number;
  readonly wasmpaymentserver_new: () => number;
  readonly wasm_bindgen__convert__closures_____invoke__h7460171fa07d4e7b: (a: number, b: number, c: any) => void;
  readonly wasm_bindgen__closure__destroy__h0ea10b8e17c2589a: (a: number, b: number) => void;
  readonly wasm_bindgen__convert__closures_____invoke__h75da7eae032c0859: (a: number, b: number, c: any) => void;
  readonly wasm_bindgen__closure__destroy__he7277012e90784de: (a: number, b: number) => void;
  readonly wasm_bindgen__convert__closures_____invoke__hec0e381372c60b88: (a: number, b: number) => void;
  readonly wasm_bindgen__closure__destroy__he9ff11ce1c64d320: (a: number, b: number) => void;
  readonly wasm_bindgen__convert__closures_____invoke__h8a0305fb7488cc73: (a: number, b: number, c: any, d: any) => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_externrefs: WebAssembly.Table;
  readonly __externref_drop_slice: (a: number, b: number) => void;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __externref_table_dealloc: (a: number) => void;
  readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;

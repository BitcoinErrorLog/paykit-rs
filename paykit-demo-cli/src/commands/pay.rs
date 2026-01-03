//! Pay command - initiate payment
//!
//! Executes real payments with Noise protocol negotiation when the recipient is a Pubky URI.
//! Falls back to direct payment when a Lightning invoice or Bitcoin address is provided.

use anyhow::{Context, Result};
use paykit_demo_core::DemoStorage;
use paykit_interactive::{PaykitNoiseMessage, PaykitReceipt};
use paykit_lib::prelude::*;
use paykit_lib::rotation::{EndpointRotationManager, RotationConfig};
use paykit_lib::MethodId;
use pubky_noise::datalink_adapter::{client_complete_ik, client_start_ik_direct};
use pubky_noise::{DummyRing, NoiseClient};
use std::path::Path;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use super::wallet::WalletConfig;
use crate::ui;

#[allow(clippy::too_many_arguments)]
#[tracing::instrument(skip(storage_dir))]
pub async fn run(
    storage_dir: &Path,
    recipient: &str,
    amount: Option<String>,
    currency: Option<String>,
    method: &str,
    strategy: &str,
    dry_run: bool,
    verbose: bool,
) -> Result<()> {
    ui::header("Initiate Payment");

    tracing::debug!("Loading current identity");
    // Load current identity
    let identity = super::load_current_identity(storage_dir).await?;

    if verbose {
        ui::info(&format!("Payer: {}", identity.pubky_uri()));
        tracing::info!("Payer: {}", identity.pubky_uri());
    }

    // Resolve recipient (could be contact name or URI)
    let payee_uri = resolve_recipient(storage_dir, recipient)?;

    ui::info(&format!("Recipient: {}", payee_uri));

    let amount_str = if let Some(amt) = &amount {
        if let Some(curr) = &currency {
            ui::info(&format!("Amount: {} {}", amt, curr));
            format!("{} {}", amt, curr)
        } else {
            ui::info(&format!("Amount: {} SAT", amt));
            format!("{} SAT", amt)
        }
    } else {
        ui::info("Amount: Unspecified");
        "unspecified".to_string()
    };

    // Determine the payment method to use
    let selected_method = if method.to_lowercase() == "auto" {
        // Auto-select the best method
        select_payment_method(
            storage_dir,
            &payee_uri,
            amount.as_deref(),
            strategy,
            verbose,
        )
        .await?
    } else {
        ui::info(&format!("Method: {}", method));
        method.to_string()
    };

    ui::separator();

    // Check if recipient is a Pubky URI - if so, use Noise negotiation
    if payee_uri.starts_with("pubky://") {
        return execute_noise_payment(
            storage_dir,
            &identity,
            &payee_uri,
            amount.as_deref(),
            currency.as_deref(),
            &selected_method,
            dry_run,
            verbose,
        )
        .await;
    }

    // Check wallet configuration for direct payments
    let wallet_config = WalletConfig::load(storage_dir)?;

    match selected_method.to_lowercase().as_str() {
        "lightning" | "ln" | "ln-btc" => {
            execute_lightning_payment(
                storage_dir,
                &wallet_config,
                &payee_uri,
                amount.as_deref(),
                dry_run,
                verbose,
            )
            .await?;
        }
        "onchain" | "btc" | "onchain-btc" => {
            execute_onchain_payment(
                storage_dir,
                &wallet_config,
                &payee_uri,
                amount.as_deref(),
                dry_run,
                verbose,
            )
            .await?;
        }
        _ => {
            ui::warning(&format!("Unknown payment method: {}", selected_method));
            ui::info("Supported methods: lightning, onchain, auto");
            return Ok(());
        }
    }

    // Log payment attempt
    if !dry_run {
        log_payment_attempt(storage_dir, &payee_uri, &amount_str, &selected_method)?;
    }

    Ok(())
}

/// Select the best payment method using paykit-lib selection
async fn select_payment_method(
    _storage_dir: &Path,
    payee_uri: &str,
    amount: Option<&str>,
    strategy: &str,
    verbose: bool,
) -> Result<String> {
    use paykit_lib::methods::Amount;
    use paykit_lib::selection::{PaymentMethodSelector, SelectionPreferences};

    ui::info("Auto-selecting payment method...");

    // Parse strategy and create preferences
    let prefs = match strategy.to_lowercase().as_str() {
        "balanced" => SelectionPreferences::balanced(),
        "cost" => SelectionPreferences::cost_optimized(),
        "speed" => SelectionPreferences::speed_optimized(),
        "privacy" => SelectionPreferences::privacy_optimized(),
        _ => {
            ui::warning(&format!("Unknown strategy '{}', using balanced", strategy));
            SelectionPreferences::balanced()
        }
    };

    if verbose {
        ui::info(&format!("Selection strategy: {}", strategy));
    }

    // Parse amount
    let amount_sats = amount.and_then(|a| a.parse::<u64>().ok()).unwrap_or(10000); // Default to 10k sats if not specified

    let amt = Amount::sats(amount_sats);

    // Try to discover payee's supported methods
    let payee_pk_str = payee_uri.strip_prefix("pubky://").unwrap_or(payee_uri);

    let spinner = ui::spinner("Discovering recipient payment methods...");

    // Query the directory for supported methods
    let storage = pubky::PublicStorage::new().context("Failed to create PublicStorage")?;
    let transport = paykit_lib::PubkyUnauthenticatedTransport::new(storage);

    let payee_pk: paykit_lib::PublicKey = payee_pk_str
        .parse()
        .context("Failed to parse payee public key")?;

    let supported = paykit_lib::get_payment_list(&transport, &payee_pk).await;

    spinner.finish_and_clear();

    match supported {
        Ok(methods) if !methods.entries.is_empty() => {
            if verbose {
                ui::info(&format!(
                    "Found {} supported method(s):",
                    methods.entries.len()
                ));
                for (method_id, endpoint) in &methods.entries {
                    ui::info(&format!("  - {}: {}", method_id.0, endpoint.0));
                }
            }

            // Use the selector
            let selector = PaymentMethodSelector::with_defaults();

            match selector.select(&methods, &amt, &prefs) {
                Ok(result) => {
                    ui::success(&format!(
                        "Selected method: {} (score: {:.2})",
                        result.primary.0, result.score
                    ));
                    ui::info(&format!("Reason: {}", result.reason));

                    if !result.fallbacks.is_empty() {
                        ui::info(&format!(
                            "Fallbacks: {}",
                            result
                                .fallbacks
                                .iter()
                                .map(|m| m.0.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                    }

                    Ok(result.primary.0)
                }
                Err(e) => {
                    ui::warning(&format!("Selection failed: {}", e));
                    ui::info("Falling back to lightning");
                    Ok("lightning".to_string())
                }
            }
        }
        Ok(_) => {
            ui::warning("No payment methods found for recipient");
            ui::info("Falling back to lightning (default)");
            Ok("lightning".to_string())
        }
        Err(e) => {
            if verbose {
                ui::warning(&format!("Could not query methods: {}", e));
            }
            ui::info("Using lightning (default)");
            Ok("lightning".to_string())
        }
    }
}

/// Execute payment using Noise protocol to negotiate with recipient
#[allow(clippy::too_many_arguments)]
async fn execute_noise_payment(
    storage_dir: &Path,
    identity: &paykit_demo_core::Identity,
    payee_uri: &str,
    amount: Option<&str>,
    currency: Option<&str>,
    method: &str,
    dry_run: bool,
    verbose: bool,
) -> Result<()> {
    ui::info("Using Noise protocol to negotiate payment...");

    // Parse payee public key from URI
    let payee_pk_str = payee_uri.strip_prefix("pubky://").unwrap_or(payee_uri);
    let payee_pk: paykit_lib::PublicKey = payee_pk_str
        .parse()
        .context("Failed to parse payee public key")?;

    if dry_run {
        ui::info("DRY RUN - Would negotiate payment via Noise:");
        ui::info(&format!("  Payee: {}", payee_uri));
        if let Some(amt) = amount {
            ui::info(&format!("  Amount: {} {}", amt, currency.unwrap_or("SAT")));
        }
        ui::info(&format!("  Method: {}", method));
        ui::info("  No actual connection will be made");
        return Ok(());
    }

    // Use smart checkout to resolve endpoint (private → public fallback)
    let spinner = ui::spinner("Resolving payment endpoint...");

    // Create storage adapter for private endpoint lookup
    let demo_storage = DemoStorage::new(storage_dir.join("data"));
    let endpoint_manager = {
        use paykit_lib::private_endpoints::{encryption, FileStore, PrivateEndpointManager};
        let endpoints_dir = storage_dir.join("private_endpoints");
        let key_path = storage_dir.join(".endpoint_key");

        let key = if key_path.exists() {
            // Load existing key
            let key_bytes =
                std::fs::read(&key_path).context("Failed to read endpoint encryption key")?;
            if key_bytes.len() != 32 {
                anyhow::bail!("Invalid endpoint encryption key size");
            }
            let mut key = [0u8; 32];
            key.copy_from_slice(&key_bytes);
            key
        } else {
            // Generate new key and save
            let key = encryption::generate_key();
            std::fs::write(&key_path, *key).context("Failed to save endpoint encryption key")?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600)).ok();
            }
            *key // Dereference Zeroizing
        };

        let store = FileStore::new_encrypted(&endpoints_dir, key)
            .context("Failed to create encrypted endpoint store")?;
        Some(PrivateEndpointManager::new(store))
    };

    struct PayStorageAdapter {
        #[allow(dead_code)]
        storage: DemoStorage,
        endpoint_manager: Option<
            paykit_lib::private_endpoints::PrivateEndpointManager<
                paykit_lib::private_endpoints::FileStore,
            >,
        >,
    }

    #[async_trait::async_trait]
    impl paykit_interactive::PaykitStorage for PayStorageAdapter {
        async fn save_receipt(
            &self,
            _receipt: &paykit_interactive::PaykitReceipt,
        ) -> paykit_interactive::Result<()> {
            Ok(()) // Not used in this context
        }

        async fn get_receipt(
            &self,
            _receipt_id: &str,
        ) -> paykit_interactive::Result<Option<paykit_interactive::PaykitReceipt>> {
            Ok(None) // Not used in this context
        }

        async fn list_receipts(
            &self,
        ) -> paykit_interactive::Result<Vec<paykit_interactive::PaykitReceipt>> {
            Ok(vec![]) // Not used in this context
        }

        async fn save_private_endpoint(
            &self,
            _peer: &paykit_lib::PublicKey,
            _method: &paykit_lib::MethodId,
            _endpoint: &str,
        ) -> paykit_interactive::Result<()> {
            Ok(()) // Not used in this context
        }

        async fn get_private_endpoint(
            &self,
            peer: &paykit_lib::PublicKey,
            method: &paykit_lib::MethodId,
        ) -> paykit_interactive::Result<Option<String>> {
            if let Some(ref manager) = self.endpoint_manager {
                if let Some(endpoint) = manager
                    .get_endpoint(peer, method)
                    .await
                    .map_err(|e| paykit_interactive::InteractiveError::Transport(e.to_string()))?
                {
                    return Ok(Some(endpoint.0));
                }
            }
            Ok(None)
        }

        async fn list_private_endpoints_for_peer(
            &self,
            peer: &paykit_lib::PublicKey,
        ) -> paykit_interactive::Result<Vec<(paykit_lib::MethodId, String)>> {
            if let Some(ref manager) = self.endpoint_manager {
                let endpoints = manager
                    .get_endpoints_for_peer(peer)
                    .await
                    .map_err(|e| paykit_interactive::InteractiveError::Transport(e.to_string()))?;
                return Ok(endpoints
                    .into_iter()
                    .map(|e| (e.method_id, e.endpoint.0))
                    .collect());
            }
            Ok(vec![])
        }

        async fn remove_private_endpoint(
            &self,
            _peer: &paykit_lib::PublicKey,
            _method: &paykit_lib::MethodId,
        ) -> paykit_interactive::Result<()> {
            Ok(()) // Not used in this context
        }
    }

    let storage_adapter = PayStorageAdapter {
        storage: demo_storage,
        endpoint_manager,
    };

    // Use smart checkout to resolve endpoint
    let method_id = MethodId::new(method);
    let storage = pubky::PublicStorage::new().context("Failed to create PublicStorage")?;
    let transport = paykit_lib::PubkyUnauthenticatedTransport::new(storage);

    let checkout_result = paykit_interactive::smart_checkout_detailed(
        &storage_adapter,
        &transport,
        &payee_pk,
        &method_id,
    )
    .await
    .context("Failed to resolve payment endpoint")?;

    spinner.finish_and_clear();

    match checkout_result {
        Some(result) => {
            let source = if result.is_private {
                "private"
            } else {
                "public"
            };
            ui::info(&format!(
                "Resolved {} endpoint (source: {})",
                method, source
            ));
            ui::key_value("  Endpoint", &result.endpoint.0);
            if verbose {
                ui::info(&format!("  Source: {} directory", source));
            }
        }
        None => {
            ui::warning(&format!("No endpoint found for method: {}", method));
            ui::info("The recipient may need to publish endpoints or share private endpoints");
        }
    }

    // For demo, we need the recipient's Noise server address
    // In production, this would come from the directory or be negotiated
    ui::separator();
    ui::info("To complete Noise payment negotiation:");
    ui::info("  1. Recipient must be running: paykit-demo receive --port <PORT>");
    ui::info("  2. You need recipient's Noise public key and address");
    ui::separator();

    // Try to connect if we have connection info (for demo, use localhost)
    let connect_addr =
        std::env::var("PAYKIT_PAYEE_ADDR").unwrap_or_else(|_| "127.0.0.1:8888".to_string());
    let payee_noise_pk = std::env::var("PAYKIT_PAYEE_NOISE_PK").ok();

    if let Some(noise_pk_hex) = payee_noise_pk {
        ui::info(&format!("Connecting to {} ...", connect_addr));

        // Parse the noise public key
        let noise_pk_bytes = hex::decode(&noise_pk_hex).context("Invalid Noise public key hex")?;
        if noise_pk_bytes.len() != 32 {
            anyhow::bail!("Noise public key must be 32 bytes");
        }
        let mut server_pk = [0u8; 32];
        server_pk.copy_from_slice(&noise_pk_bytes);

        // Setup Noise client
        let seed = identity.keypair.secret_key();
        let ring = Arc::new(DummyRing::new(seed, "paykit-payer"));
        let noise_client = NoiseClient::<_, ()>::new_direct("paykit-payer", b"demo-device", ring);

        // Connect
        let mut socket = TcpStream::connect(&connect_addr)
            .await
            .context("Failed to connect to recipient")?;

        // Perform handshake
        let spinner = ui::spinner("Performing Noise handshake...");
        let (client_hs, first_msg) = client_start_ik_direct(&noise_client, &server_pk, None)
            .context("Failed to initiate handshake")?;

        socket.write_all(&first_msg).await?;

        let mut response = vec![0u8; 4096];
        let n = socket.read(&mut response).await?;
        response.truncate(n);

        let mut link =
            client_complete_ik(client_hs, &response).context("Failed to complete handshake")?;
        spinner.finish_and_clear();

        ui::success(&format!("Session established: {}", link.session_id()));

        // Create receipt request
        let receipt_id = uuid::Uuid::new_v4().to_string();
        let provisional_receipt = PaykitReceipt::new(
            receipt_id.clone(),
            identity.public_key(),
            payee_pk.clone(),
            MethodId::new(method),
            amount.map(String::from),
            Some(currency.unwrap_or("SAT").to_string()),
            serde_json::json!({}),
        );

        let request_msg = PaykitNoiseMessage::RequestReceipt {
            provisional_receipt: provisional_receipt.clone(),
        };

        // Send request
        let request_json = serde_json::to_vec(&request_msg)?;
        let encrypted = link.encrypt(&request_json)?;
        let len_bytes = (encrypted.len() as u32).to_be_bytes();
        socket.write_all(&len_bytes).await?;
        socket.write_all(&encrypted).await?;

        ui::info("Receipt request sent, waiting for confirmation...");

        // Read response
        let mut len_buf = [0u8; 4];
        socket.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;

        let mut ciphertext = vec![0u8; len];
        socket.read_exact(&mut ciphertext).await?;

        let plaintext = link.decrypt(&ciphertext)?;
        let response_msg: PaykitNoiseMessage = serde_json::from_slice(&plaintext)?;

        match response_msg {
            PaykitNoiseMessage::ConfirmReceipt { receipt } => {
                ui::separator();
                ui::success("Receipt confirmed!");
                ui::key_value("Receipt ID", &receipt.receipt_id);
                if let Some(amt) = &receipt.amount {
                    ui::key_value("Amount", amt);
                }
                ui::key_value("Method", &receipt.method_id.0);

                // Save receipt with proof if available
                let demo_storage = DemoStorage::new(storage_dir.join("data"));
                demo_storage.init()?;

                // Convert PaykitReceipt to core Receipt with proof
                let core_receipt = paykit_demo_core::Receipt::new(
                    receipt.receipt_id.clone(),
                    receipt.payer.clone(),
                    receipt.payee.clone(),
                    receipt.method_id.0.clone(),
                )
                .with_amount(
                    receipt.amount.clone().unwrap_or_default(),
                    receipt.currency.clone().unwrap_or_default(),
                )
                .with_metadata(receipt.metadata.clone());

                // Note: Proof would be captured from payment execution
                // For Noise payments, proof comes from the payment method plugin
                // For now, we save the receipt as-is
                let receipt_json = serde_json::to_string(&receipt)?;
                demo_storage.save_receipt_json(&receipt.receipt_id, &receipt_json)?;

                // Also save as core Receipt for consistency
                demo_storage.save_receipt(core_receipt)?;

                ui::info("Receipt saved locally");

                // Handle endpoint rotation after successful payment
                handle_endpoint_rotation(storage_dir, &receipt.method_id, verbose).await?;

                // Log payment
                log_payment_attempt(
                    storage_dir,
                    payee_uri,
                    amount.unwrap_or("unspecified"),
                    method,
                )?;
            }
            PaykitNoiseMessage::Error { code, message } => {
                ui::error(&format!("Payment rejected: {} - {}", code, message));
            }
            _ => {
                ui::error("Unexpected response from recipient");
            }
        }
    } else {
        ui::info("To test Noise payment negotiation:");
        ui::info("  1. In terminal 1: paykit-demo receive --port 8888");
        ui::info("  2. Note the 'Noise public key' displayed");
        ui::info("  3. In terminal 2: Set environment variables:");
        ui::info("     export PAYKIT_PAYEE_ADDR=127.0.0.1:8888");
        ui::info("     export PAYKIT_PAYEE_NOISE_PK=<noise_public_key_hex>");
        ui::info("  4. Run this pay command again");
    }

    Ok(())
}

async fn execute_lightning_payment(
    _storage_dir: &Path,
    wallet_config: &Option<WalletConfig>,
    payee_uri: &str,
    amount: Option<&str>,
    dry_run: bool,
    _verbose: bool,
) -> Result<()> {
    match wallet_config {
        Some(config) if config.has_lightning() => {
            let lnd_config = config.lnd.as_ref().unwrap();

            if dry_run {
                ui::info("DRY RUN - Would execute Lightning payment:");
                ui::info(&format!("  LND URL: {}", lnd_config.url));
                ui::info(&format!("  Recipient: {}", payee_uri));
                if let Some(amt) = amount {
                    ui::info(&format!("  Amount: {} sats", amt));
                }
                ui::info("  No actual payment will be made");
                return Ok(());
            }

            ui::info("Executing Lightning payment...");

            // Create LND executor
            #[cfg(feature = "http-executor")]
            {
                use paykit_lib::executors::{LndConfig as LibLndConfig, LndExecutor};
                use paykit_lib::methods::LightningExecutor;

                let lib_config = LibLndConfig::new(&lnd_config.url, &lnd_config.macaroon);
                let executor =
                    LndExecutor::new(lib_config).context("Failed to create LND executor")?;

                // For now, we expect the payee_uri to contain an invoice
                // In a full implementation, we'd use Noise to negotiate
                let invoice = if payee_uri.starts_with("ln") {
                    payee_uri.to_string()
                } else {
                    // In a real implementation, we would:
                    // 1. Connect to recipient via Noise
                    // 2. Request a payment invoice
                    // 3. Execute the payment
                    ui::warning("Full Noise negotiation not yet implemented");
                    ui::info("To pay directly, provide a BOLT11 invoice as the recipient");
                    return Ok(());
                };

                // Decode invoice first
                let decoded = executor
                    .decode_invoice(&invoice)
                    .await
                    .context("Failed to decode invoice")?;

                ui::info(&format!("Payment hash: {}", decoded.payment_hash));
                if let Some(amt) = decoded.amount_msat {
                    ui::info(&format!(
                        "Invoice amount: {} msat ({} sats)",
                        amt,
                        amt / 1000
                    ));
                }
                if let Some(desc) = &decoded.description {
                    ui::info(&format!("Description: {}", desc));
                }
                if decoded.expired {
                    ui::error("Invoice has expired!");
                    return Err(anyhow::anyhow!("Invoice expired"));
                }

                // Execute payment
                let amount_msat = amount.and_then(|s| s.parse::<u64>().ok().map(|a| a * 1000));
                let result = executor
                    .pay_invoice(&invoice, amount_msat, None)
                    .await
                    .context("Payment failed")?;

                ui::separator();
                match result.status {
                    paykit_lib::methods::LightningPaymentStatus::Succeeded => {
                        ui::success("Payment succeeded!");
                        ui::info(&format!("Preimage: {}", result.preimage));
                        ui::info(&format!("Fee: {} msat", result.fee_msat));
                        ui::info(&format!("Hops: {}", result.hops));

                        // Capture proof for receipt
                        let proof = paykit_lib::methods::PaymentProof::lightning_preimage(
                            &result.preimage,
                            &result.payment_hash,
                        );
                        let proof_json =
                            serde_json::to_value(&proof).context("Failed to serialize proof")?;

                        // Save receipt with proof
                        let demo_storage = DemoStorage::new(storage_dir.join("data"));
                        demo_storage.init()?;

                        let identity = super::load_current_identity(storage_dir).await?;
                        let receipt = paykit_demo_core::Receipt::new(
                            uuid::Uuid::new_v4().to_string(),
                            identity.public_key(),
                            payee_uri.parse().unwrap_or_else(|_| {
                                "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo"
                                    .parse()
                                    .unwrap()
                            }),
                            "lightning".to_string(),
                        )
                        .with_amount(amount.unwrap_or("0").to_string(), "SAT".to_string())
                        .with_proof(proof_json);

                        demo_storage.save_receipt(receipt)?;
                        ui::info("Receipt with proof saved");
                    }
                    paykit_lib::methods::LightningPaymentStatus::Pending => {
                        ui::warning("Payment pending...");
                        ui::info(&format!("Payment hash: {}", result.payment_hash));
                    }
                    paykit_lib::methods::LightningPaymentStatus::Failed => {
                        ui::error("Payment failed");
                    }
                }
            }

            #[cfg(not(feature = "http-executor"))]
            {
                ui::warning("http-executor feature not enabled");
                ui::info("Rebuild with: cargo build --features http-executor");
            }
        }
        Some(_) => {
            ui::warning("LND not configured for Lightning payments");
            ui::info(
                "Configure with: paykit-demo wallet configure-lnd --url <url> --macaroon <hex>",
            );
            show_simulation_message();
        }
        None => {
            ui::warning("No wallet configured");
            ui::info("Configure with: paykit-demo wallet status");
            show_simulation_message();
        }
    }

    Ok(())
}

async fn execute_onchain_payment(
    _storage_dir: &Path,
    wallet_config: &Option<WalletConfig>,
    payee_uri: &str,
    amount: Option<&str>,
    dry_run: bool,
    _verbose: bool,
) -> Result<()> {
    match wallet_config {
        Some(config) if config.has_onchain() => {
            let esplora_config = config.esplora.as_ref().unwrap();

            if dry_run {
                ui::info("DRY RUN - Would prepare on-chain payment:");
                ui::info(&format!("  Esplora URL: {}", esplora_config.url));
                ui::info(&format!("  Recipient: {}", payee_uri));
                if let Some(amt) = amount {
                    ui::info(&format!("  Amount: {} sats", amt));
                }
                ui::info("  No actual transaction will be created");
                return Ok(());
            }

            ui::info("Preparing on-chain payment...");

            #[cfg(feature = "http-executor")]
            {
                use paykit_lib::executors::{EsploraConfig as LibEsploraConfig, EsploraExecutor};

                let lib_config = LibEsploraConfig::new(&esplora_config.url);
                let executor = EsploraExecutor::new(lib_config)
                    .context("Failed to create Esplora executor")?;

                // Get fee estimates
                let fees = executor
                    .get_fee_estimates()
                    .await
                    .context("Failed to get fee estimates")?;

                ui::info("Current fee estimates:");
                ui::info(&format!(
                    "  Next block: {:.1} sat/vB",
                    fees.get_rate_for_blocks(1)
                ));
                ui::info(&format!(
                    "  ~1 hour: {:.1} sat/vB",
                    fees.get_rate_for_blocks(6)
                ));
                ui::info(&format!(
                    "  ~1 day: {:.1} sat/vB",
                    fees.get_rate_for_blocks(144)
                ));

                ui::separator();
                ui::warning("On-chain payment requires a wallet for signing");
                ui::info("Esplora can verify transactions but cannot create them.");
                ui::info("Use a wallet like Sparrow or Electrum to create the transaction.");
                if let Some(amt) = amount {
                    ui::info(&format!("Send {} sats to: {}", amt, payee_uri));
                } else {
                    ui::info(&format!("Send to: {}", payee_uri));
                }
            }

            #[cfg(not(feature = "http-executor"))]
            {
                ui::warning("http-executor feature not enabled");
                ui::info("Rebuild with: cargo build --features http-executor");
            }
        }
        Some(_) => {
            ui::warning("Esplora not configured for on-chain payments");
            ui::info("Configure with: paykit-demo wallet configure-esplora --url <url>");
            show_simulation_message();
        }
        None => {
            ui::warning("No wallet configured");
            ui::info("Configure with: paykit-demo wallet status");
            show_simulation_message();
        }
    }

    Ok(())
}

fn show_simulation_message() {
    ui::separator();
    ui::info("SIMULATION MODE");
    ui::info("In production, this would:");
    ui::info("  1. Connect to recipient via Noise protocol");
    ui::info("  2. Exchange payment method information");
    ui::info("  3. Coordinate payment execution");
    ui::info("  4. Generate and exchange receipts");
    ui::separator();
}

fn resolve_recipient(storage_dir: &Path, recipient: &str) -> Result<String> {
    // If it looks like a URI, return as-is
    if recipient.starts_with("pubky://")
        || recipient.starts_with("ln")
        || recipient.starts_with("bc1")
        || recipient.starts_with("tb1")
        || recipient.len() > 40
    {
        return Ok(recipient.to_string());
    }

    // Otherwise, try to look up as contact name
    let storage = DemoStorage::new(storage_dir.join("data"));
    let contacts = storage.list_contacts()?;

    for contact in contacts {
        if contact.name == recipient {
            return Ok(contact.pubky_uri());
        }
    }

    // If not found, assume it's a public key
    Ok(format!("pubky://{}", recipient))
}

/// Handle endpoint rotation after payment execution
async fn handle_endpoint_rotation(
    storage_dir: &Path,
    method_id: &MethodId,
    verbose: bool,
) -> Result<()> {
    // Load or create rotation manager with default config
    let rotation_config = load_rotation_config(storage_dir)?;
    let registry = default_registry();
    let rotation_manager = EndpointRotationManager::new(rotation_config, registry);

    // Record payment execution and check if rotation is needed
    if let Some(new_endpoint) = rotation_manager.on_payment_executed(method_id).await {
        ui::separator();
        ui::info(&format!("Endpoint rotated for method: {}", method_id.0));
        if verbose {
            ui::key_value("New endpoint", &new_endpoint.0);
        }
        ui::warning("New endpoint generated but not yet published to directory");
        ui::info("Run 'paykit-demo publish' to update the directory with the new endpoint");

        // Save rotation state for future reference
        save_rotation_state(storage_dir, method_id, &new_endpoint)?;
    } else if verbose {
        ui::info(&format!(
            "No rotation needed for method: {} (policy check)",
            method_id.0
        ));
    }

    Ok(())
}

/// Load rotation configuration from storage or return default
fn load_rotation_config(storage_dir: &Path) -> Result<RotationConfig> {
    let config_path = storage_dir.join("rotation_config.json");

    if config_path.exists() {
        let config_str =
            std::fs::read_to_string(&config_path).context("Failed to read rotation config")?;
        serde_json::from_str(&config_str).context("Failed to parse rotation config")
    } else {
        // Return default config (rotate on use for privacy)
        Ok(RotationConfig::default())
    }
}

/// Save rotation state for tracking
fn save_rotation_state(
    storage_dir: &Path,
    method_id: &MethodId,
    new_endpoint: &paykit_lib::EndpointData,
) -> Result<()> {
    let state_path = storage_dir.join("rotation_state.json");

    // Load existing state or create new
    let mut state: serde_json::Value = if state_path.exists() {
        let state_str =
            std::fs::read_to_string(&state_path).context("Failed to read rotation state")?;
        serde_json::from_str(&state_str).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // Update state for this method
    let method_key = &method_id.0;
    if !state[method_key].is_object() {
        state[method_key] = serde_json::json!({
            "last_rotated": chrono::Utc::now().timestamp(),
            "rotations": 0,
        });
    }

    state[method_key]["last_rotated"] = serde_json::json!(chrono::Utc::now().timestamp());
    state[method_key]["rotations"] =
        serde_json::json!(state[method_key]["rotations"].as_u64().unwrap_or(0) + 1);
    state[method_key]["pending_endpoint"] = serde_json::json!(new_endpoint.0);

    // Save updated state
    let state_str =
        serde_json::to_string_pretty(&state).context("Failed to serialize rotation state")?;
    std::fs::write(&state_path, state_str).context("Failed to save rotation state")?;

    Ok(())
}

fn log_payment_attempt(
    storage_dir: &Path,
    recipient: &str,
    amount: &str,
    method: &str,
) -> Result<()> {
    let log_dir = storage_dir.join("logs");
    std::fs::create_dir_all(&log_dir)?;

    let log_file = log_dir.join("payments.log");
    let timestamp = chrono::Utc::now().to_rfc3339();
    let entry = format!(
        "{} | method={} | recipient={} | amount={}\n",
        timestamp, method, recipient, amount
    );

    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)?
        .write_all(entry.as_bytes())?;

    Ok(())
}

use std::io::Write;

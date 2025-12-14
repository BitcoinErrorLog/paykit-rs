//! Paykit Demo CLI
//!
//! Command-line interface for testing and demonstrating Paykit functionality.

use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;
mod ui;

#[derive(Parser)]
#[command(name = "paykit-demo")]
#[command(about = "Paykit Demo CLI - Test and demonstrate Paykit payment protocol", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Custom storage directory (can also be set via PAYKIT_DEMO_DIR env var)
    #[arg(long, global = true)]
    storage_dir: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Setup a new identity
    Setup {
        /// Name for this identity
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Show current identity
    Whoami,

    /// List all saved identities
    List,

    /// Switch to a different identity
    Switch {
        /// Identity name to switch to
        name: String,
    },

    /// Migrate identities from plaintext to secure storage
    Migrate,

    /// Export identity to encrypted backup file
    Backup {
        /// Output file for the backup (JSON format)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Restore identity from encrypted backup file
    Restore {
        /// Input backup file to restore from
        input: String,

        /// Name for the restored identity
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Configure payment wallet (LND, Esplora)
    Wallet {
        #[command(subcommand)]
        action: WalletAction,
    },

    /// Publish payment methods to the public directory
    Publish {
        /// Bitcoin onchain address
        #[arg(long)]
        onchain: Option<String>,

        /// Lightning invoice or LNURL
        #[arg(long)]
        lightning: Option<String>,

        /// Homeserver URL
        #[arg(long, default_value = "https://demo.httprelay.io")]
        homeserver: String,
    },

    /// Query payment methods from a Pubky URI
    Discover {
        /// Pubky URI (e.g., pubky://...)
        uri: String,

        /// Homeserver URL
        #[arg(long, default_value = "https://demo.httprelay.io")]
        homeserver: String,
    },

    /// Manage contacts
    Contacts {
        #[command(subcommand)]
        action: ContactAction,
    },

    /// Start payment receiver (server mode)
    Receive {
        /// Port to listen on
        #[arg(short, long, default_value = "8888")]
        port: u16,
    },

    /// Initiate a payment (client mode)
    Pay {
        /// Recipient Pubky URI or contact name
        recipient: String,

        /// Amount (optional)
        #[arg(short, long)]
        amount: Option<String>,

        /// Currency (optional)
        #[arg(short, long)]
        currency: Option<String>,

        /// Payment method (onchain, lightning, or auto)
        #[arg(short, long, default_value = "auto")]
        method: String,

        /// Selection strategy when using auto (balanced, cost, speed, privacy)
        #[arg(long, default_value = "balanced")]
        strategy: String,

        /// Dry run - show what would happen without executing
        #[arg(long)]
        dry_run: bool,
    },

    /// Show payment receipts
    Receipts {
        /// Receipt ID to show details for
        #[arg(short, long)]
        id: Option<String>,
    },
    
    /// Verify payment proof for a receipt
    VerifyProof {
        /// Receipt ID to verify
        receipt_id: String,
    },

    /// QR code operations - display and parse
    Qr {
        #[command(subcommand)]
        action: QrAction,
    },

    /// Show dashboard with summary statistics
    Dashboard,

    /// Manage private endpoints exchanged with peers
    Endpoints {
        #[command(subcommand)]
        action: EndpointAction,
    },

    /// Configure endpoint rotation policies
    Rotation {
        #[command(subcommand)]
        action: RotationAction,
    },

    /// Manage payment requests and subscriptions
    Subscriptions {
        #[command(subcommand)]
        action: SubscriptionAction,
    },
}

#[derive(Subcommand)]
enum EndpointAction {
    /// List all private endpoints
    List,

    /// Show endpoints for a specific peer
    Show {
        /// Peer public key
        peer: String,
    },

    /// Remove a specific endpoint
    Remove {
        /// Peer public key
        peer: String,

        /// Payment method ID
        method: String,
    },

    /// Remove all endpoints for a peer
    RemovePeer {
        /// Peer public key
        peer: String,
    },

    /// Cleanup expired endpoints
    Cleanup,

    /// Show endpoint statistics
    Stats,
}

#[derive(Subcommand)]
enum RotationAction {
    /// Show rotation status for all methods
    Status,

    /// Set rotation policy for a specific method
    Policy {
        /// Payment method ID (e.g., "onchain", "lightning")
        method: String,

        /// Policy: on-use, manual, after:<count>, periodic:<seconds>
        policy: String,
    },

    /// Set the default rotation policy
    Default {
        /// Policy: on-use, manual, after:<count>, periodic:<seconds>
        policy: String,
    },

    /// Enable or disable auto-rotation after payments
    AutoRotate {
        /// Enable (true) or disable (false)
        #[arg(long)]
        enable: bool,
    },

    /// Manually trigger rotation for a method
    Rotate {
        /// Payment method ID
        method: String,
    },

    /// Show rotation history for auditing
    History {
        /// Filter by payment method (optional)
        #[arg(short, long)]
        method: Option<String>,
    },

    /// Clear rotation history
    ClearHistory,
}

#[derive(Subcommand)]
enum SubscriptionAction {
    /// Send a payment request to a peer
    Request {
        /// Recipient Pubky URI or contact name
        recipient: String,

        /// Amount
        #[arg(short, long)]
        amount: String,

        /// Currency (SAT, BTC, USD)
        #[arg(short, long, default_value = "SAT")]
        currency: String,

        /// Description
        #[arg(short, long)]
        description: Option<String>,

        /// Expiration time in seconds (default: 24 hours)
        #[arg(short, long)]
        expires_in: Option<u64>,
    },

    /// List payment requests
    List {
        /// Filter type: incoming, outgoing, or all
        #[arg(short, long, default_value = "all")]
        filter: String,

        /// Filter by peer (contact name or public key)
        #[arg(short, long)]
        peer: Option<String>,
    },

    /// Show request details
    Show {
        /// Request ID
        request_id: String,
    },

    /// Respond to a payment request
    Respond {
        /// Request ID
        request_id: String,

        /// Action: accept or decline
        #[arg(short, long)]
        action: String,

        /// Reason for declining (optional)
        #[arg(short, long)]
        reason: Option<String>,
    },

    // Phase 2: Subscription Agreements
    /// Propose a subscription agreement to a peer
    Propose {
        /// Recipient Pubky URI or contact name
        recipient: String,

        /// Amount per payment
        #[arg(short, long)]
        amount: String,

        /// Currency (SAT, BTC, USD)
        #[arg(short, long, default_value = "SAT")]
        currency: String,

        /// Payment frequency: `daily`, `weekly`, `monthly[:DAY]`, `yearly:MONTH:DAY`, `custom:SECONDS`
        #[arg(short, long)]
        frequency: String,

        /// Description
        #[arg(short, long)]
        description: String,
    },

    /// Accept a subscription proposal
    Accept {
        /// Subscription ID
        subscription_id: String,
    },

    /// List subscription agreements
    ListAgreements {
        /// Filter by peer (contact name or public key)
        #[arg(short, long)]
        peer: Option<String>,

        /// Show only active subscriptions
        #[arg(short, long)]
        active: bool,
    },

    /// Show subscription details
    ShowSubscription {
        /// Subscription ID
        subscription_id: String,
    },

    // Phase 3: Auto-Pay Automation
    /// Enable auto-pay for a subscription
    EnableAutoPay {
        /// Subscription ID
        subscription_id: String,

        /// Maximum amount per payment (optional)
        #[arg(long)]
        max_amount: Option<String>,

        /// Require manual confirmation before each payment
        #[arg(long)]
        require_confirmation: bool,
    },

    /// Disable auto-pay for a subscription
    DisableAutoPay {
        /// Subscription ID
        subscription_id: String,
    },

    /// Show auto-pay status for a subscription
    ShowAutoPay {
        /// Subscription ID
        subscription_id: String,
    },

    /// Set spending limit for a peer
    SetLimit {
        /// Peer Pubky URI or contact name
        peer: String,

        /// Maximum amount
        #[arg(short, long)]
        limit: String,

        /// Period: daily, weekly, or monthly
        #[arg(short, long, default_value = "monthly")]
        period: String,
    },

    /// Show spending limits
    ShowLimits {
        /// Filter by peer (optional)
        #[arg(short, long)]
        peer: Option<String>,
    },

    /// Delete a spending limit
    DeleteLimit {
        /// Peer Pubky URI or contact name
        peer: String,
    },

    /// Reset a spending limit's usage counter
    ResetLimit {
        /// Peer Pubky URI or contact name
        peer: String,
    },

    /// List all auto-pay rules
    ListAutoPay,

    /// Delete an auto-pay rule
    DeleteAutoPay {
        /// Subscription ID
        subscription_id: String,
    },

    /// Show global auto-pay settings
    GlobalSettings,

    /// Configure global auto-pay settings
    ConfigureGlobal {
        /// Enable global auto-pay
        #[arg(long)]
        enable: bool,

        /// Disable global auto-pay
        #[arg(long)]
        disable: bool,

        /// Set global daily limit (sats)
        #[arg(long)]
        daily_limit: Option<String>,
    },

    /// Show recent auto-payments
    RecentPayments {
        /// Number of recent payments to show
        #[arg(short, long, default_value = "10")]
        count: usize,
    },

    /// Calculate proration for subscription changes
    Prorate {
        /// Current amount per period (sats)
        #[arg(long)]
        current_amount: i64,

        /// New amount per period (sats)
        #[arg(long)]
        new_amount: i64,

        /// Period start timestamp (unix seconds)
        #[arg(long)]
        period_start: i64,

        /// Period end timestamp (unix seconds)
        #[arg(long)]
        period_end: i64,

        /// Change date timestamp (unix seconds, defaults to now)
        #[arg(long)]
        change_date: Option<i64>,
    },
}

#[derive(Subcommand)]
enum WalletAction {
    /// Show wallet configuration status
    Status,

    /// Check health of all configured payment methods
    Health {
        /// Specific method to check (lightning, onchain)
        #[arg(short, long)]
        method: Option<String>,
    },

    /// Configure LND for Lightning payments
    ConfigureLnd {
        /// LND REST API URL
        #[arg(long)]
        url: String,

        /// Admin macaroon in hexadecimal format
        #[arg(long)]
        macaroon: String,

        /// TLS certificate in PEM format (optional)
        #[arg(long)]
        tls_cert: Option<String>,

        /// Network (mainnet, testnet, signet, regtest)
        #[arg(long)]
        network: Option<String>,
    },

    /// Configure Esplora for on-chain payments
    ConfigureEsplora {
        /// Esplora API URL
        #[arg(long)]
        url: String,

        /// Network (mainnet, testnet, signet, regtest)
        #[arg(long)]
        network: Option<String>,
    },

    /// Apply a preset configuration
    Preset {
        /// Preset name (polar, testnet, signet, mutinynet)
        name: String,

        /// Macaroon for Polar preset
        #[arg(long)]
        macaroon: Option<String>,
    },

    /// Clear wallet configuration
    Clear,
}

#[derive(Subcommand)]
enum QrAction {
    /// Display a QR code for your identity
    Identity,

    /// Display a QR code for a contact
    Contact {
        /// Contact name
        name: String,
    },

    /// Display a QR code for a payment request
    Request {
        /// Amount in sats
        #[arg(short, long)]
        amount: Option<String>,

        /// Description
        #[arg(short, long)]
        description: Option<String>,

        /// Payment method (lightning, onchain)
        #[arg(short, long, default_value = "lightning")]
        method: String,
    },

    /// Parse a scanned QR code or URI
    Parse {
        /// The URI or QR data to parse
        data: String,
    },
}

#[derive(Subcommand)]
enum ContactAction {
    /// Add a new contact
    Add {
        /// Contact name
        name: String,

        /// Pubky URI
        uri: String,

        /// Optional notes
        #[arg(short, long)]
        notes: Option<String>,
    },

    /// List all contacts
    List {
        /// Search query to filter contacts by name or public key
        #[arg(short, long)]
        search: Option<String>,
    },

    /// Remove a contact
    Remove {
        /// Contact name or public key
        name: String,
    },

    /// Show contact details
    Show {
        /// Contact name
        name: String,
    },

    /// Discover contacts from Pubky follows directory
    Discover {
        /// Auto-import discovered contacts
        #[arg(short, long)]
        import: bool,

        /// Homeserver for discovery
        #[arg(long, default_value = "https://demo.httprelay.io")]
        homeserver: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_env_filter("paykit_demo_cli=debug,paykit_lib=debug,paykit_interactive=debug,paykit_subscriptions=debug")
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter("paykit_demo_cli=info,paykit_lib=warn,paykit_interactive=warn")
            .init();
    }

    // Setup storage directory
    let storage_dir = if let Some(dir) = cli.storage_dir {
        std::path::PathBuf::from(dir)
    } else {
        dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("paykit-demo")
    };

    // Dispatch commands
    match cli.command {
        Commands::Setup { name } => {
            commands::setup::run(&storage_dir, name, cli.verbose).await?;
        }
        Commands::Whoami => {
            commands::whoami::run(&storage_dir, cli.verbose).await?;
        }
        Commands::List => {
            commands::list::run(&storage_dir, cli.verbose).await?;
        }
        Commands::Switch { name } => {
            commands::switch::run(&storage_dir, &name, cli.verbose).await?;
        }
        Commands::Migrate => {
            commands::migrate::run(&storage_dir, cli.verbose).await?;
        }
        Commands::Backup { output } => {
            commands::backup::export(&storage_dir, output.as_deref(), cli.verbose).await?;
        }
        Commands::Restore { input, name } => {
            commands::backup::import(&storage_dir, &input, name.as_deref(), cli.verbose).await?;
        }
        Commands::Wallet { action } => match action {
            WalletAction::Status => {
                commands::wallet::status(&storage_dir, cli.verbose).await?;
            }
            WalletAction::Health { method } => {
                commands::wallet::health(&storage_dir, method, cli.verbose).await?;
            }
            WalletAction::ConfigureLnd {
                url,
                macaroon,
                tls_cert,
                network,
            } => {
                commands::wallet::configure_lnd(
                    &storage_dir,
                    &url,
                    &macaroon,
                    tls_cert.as_deref(),
                    network.as_deref(),
                    cli.verbose,
                )
                .await?;
            }
            WalletAction::ConfigureEsplora { url, network } => {
                commands::wallet::configure_esplora(
                    &storage_dir,
                    &url,
                    network.as_deref(),
                    cli.verbose,
                )
                .await?;
            }
            WalletAction::Preset { name, macaroon } => {
                commands::wallet::apply_preset(
                    &storage_dir,
                    &name,
                    macaroon.as_deref(),
                    cli.verbose,
                )
                .await?;
            }
            WalletAction::Clear => {
                commands::wallet::clear(&storage_dir, cli.verbose).await?;
            }
        },
        Commands::Publish {
            onchain,
            lightning,
            homeserver,
        } => {
            commands::publish::run(&storage_dir, onchain, lightning, &homeserver, cli.verbose)
                .await?;
        }
        Commands::Discover { uri, homeserver } => {
            commands::discover::run(&storage_dir, &uri, &homeserver, cli.verbose).await?;
        }
        Commands::Contacts { action } => match action {
            ContactAction::Add { name, uri, notes } => {
                commands::contacts::add(&storage_dir, &name, &uri, notes.as_deref(), cli.verbose)
                    .await?;
            }
            ContactAction::List { search } => {
                commands::contacts::list(&storage_dir, search.as_deref(), cli.verbose).await?;
            }
            ContactAction::Remove { name } => {
                commands::contacts::remove(&storage_dir, &name, cli.verbose).await?;
            }
            ContactAction::Show { name } => {
                commands::contacts::show(&storage_dir, &name, cli.verbose).await?;
            }
            ContactAction::Discover { import, homeserver } => {
                commands::contacts::discover(&storage_dir, import, &homeserver, cli.verbose)
                    .await?;
            }
        },
        Commands::Receive { port } => {
            commands::receive::run(&storage_dir, port, cli.verbose).await?;
        }
        Commands::Pay {
            recipient,
            amount,
            currency,
            method,
            strategy,
            dry_run,
        } => {
            commands::pay::run(
                &storage_dir,
                &recipient,
                amount,
                currency,
                &method,
                &strategy,
                dry_run,
                cli.verbose,
            )
            .await?;
        }
        Commands::Receipts { id } => {
            if let Some(receipt_id) = id {
                commands::receipts::show(&storage_dir, &receipt_id, cli.verbose).await?;
            } else {
                commands::receipts::run(&storage_dir, cli.verbose).await?;
            }
        }
        
        Commands::VerifyProof { receipt_id } => {
            commands::receipts::verify_proof(&storage_dir, &receipt_id, cli.verbose).await?;
        }
        Commands::Qr { action } => match action {
            QrAction::Identity => {
                commands::qr::identity(&storage_dir, cli.verbose).await?;
            }
            QrAction::Contact { name } => {
                commands::qr::contact(&storage_dir, &name, cli.verbose).await?;
            }
            QrAction::Request {
                amount,
                description,
                method,
            } => {
                commands::qr::request(&storage_dir, amount, description, &method, cli.verbose)
                    .await?;
            }
            QrAction::Parse { data } => {
                commands::qr::parse(&data, cli.verbose).await?;
            }
        },
        Commands::Dashboard => {
            commands::dashboard::run(&storage_dir, cli.verbose).await?;
        }
        Commands::Endpoints { action } => match action {
            EndpointAction::List => {
                commands::endpoints::list(&storage_dir, cli.verbose).await?;
            }
            EndpointAction::Show { peer } => {
                commands::endpoints::show(&storage_dir, &peer, cli.verbose).await?;
            }
            EndpointAction::Remove { peer, method } => {
                commands::endpoints::remove(&storage_dir, &peer, &method, cli.verbose).await?;
            }
            EndpointAction::RemovePeer { peer } => {
                commands::endpoints::remove_peer(&storage_dir, &peer, cli.verbose).await?;
            }
            EndpointAction::Cleanup => {
                commands::endpoints::cleanup(&storage_dir, cli.verbose).await?;
            }
            EndpointAction::Stats => {
                commands::endpoints::stats(&storage_dir, cli.verbose).await?;
            }
        },
        Commands::Rotation { action } => match action {
            RotationAction::Status => {
                commands::rotation::status(&storage_dir, cli.verbose).await?;
            }
            RotationAction::Policy { method, policy } => {
                commands::rotation::set_policy(&storage_dir, &method, &policy, cli.verbose).await?;
            }
            RotationAction::Default { policy } => {
                commands::rotation::set_default(&storage_dir, &policy, cli.verbose).await?;
            }
            RotationAction::AutoRotate { enable } => {
                commands::rotation::auto_rotate(&storage_dir, enable, cli.verbose).await?;
            }
            RotationAction::Rotate { method } => {
                commands::rotation::rotate(&storage_dir, &method, cli.verbose).await?;
            }
            RotationAction::History { method } => {
                commands::rotation::history(&storage_dir, method, cli.verbose).await?;
            }
            RotationAction::ClearHistory => {
                commands::rotation::clear_history(&storage_dir, cli.verbose).await?;
            }
        },
        Commands::Subscriptions { action } => match action {
            SubscriptionAction::Request {
                recipient,
                amount,
                currency,
                description,
                expires_in,
            } => {
                commands::subscriptions::send_request(
                    &storage_dir,
                    &recipient,
                    &amount,
                    &currency,
                    description,
                    expires_in,
                )
                .await?;
            }
            SubscriptionAction::List { filter, peer } => {
                commands::subscriptions::list_requests(&storage_dir, &filter, peer).await?;
            }
            SubscriptionAction::Show { request_id } => {
                commands::subscriptions::show_request(&storage_dir, &request_id).await?;
            }
            SubscriptionAction::Respond {
                request_id,
                action,
                reason,
            } => {
                commands::subscriptions::respond_to_request(
                    &storage_dir,
                    &request_id,
                    &action,
                    reason,
                )
                .await?;
            }
            // Phase 2: Subscription Agreements
            SubscriptionAction::Propose {
                recipient,
                amount,
                currency,
                frequency,
                description,
            } => {
                commands::subscriptions::propose_subscription(
                    &storage_dir,
                    &recipient,
                    &amount,
                    &currency,
                    &frequency,
                    &description,
                )
                .await?;
            }
            SubscriptionAction::Accept { subscription_id } => {
                commands::subscriptions::accept_subscription(&storage_dir, &subscription_id)
                    .await?;
            }
            SubscriptionAction::ListAgreements { peer, active } => {
                commands::subscriptions::list_subscriptions(&storage_dir, peer, active).await?;
            }
            SubscriptionAction::ShowSubscription { subscription_id } => {
                commands::subscriptions::show_subscription(&storage_dir, &subscription_id).await?;
            }

            // Phase 3: Auto-Pay Commands
            SubscriptionAction::EnableAutoPay {
                subscription_id,
                max_amount,
                require_confirmation,
            } => {
                commands::subscriptions::enable_autopay(
                    &storage_dir,
                    &subscription_id,
                    max_amount,
                    require_confirmation,
                )
                .await?;
            }

            SubscriptionAction::DisableAutoPay { subscription_id } => {
                commands::subscriptions::disable_autopay(&storage_dir, &subscription_id).await?;
            }

            SubscriptionAction::ShowAutoPay { subscription_id } => {
                commands::subscriptions::show_autopay_status(&storage_dir, &subscription_id)
                    .await?;
            }

            SubscriptionAction::SetLimit {
                peer,
                limit,
                period,
            } => {
                commands::subscriptions::set_peer_limit(&storage_dir, &peer, &limit, &period)
                    .await?;
            }

            SubscriptionAction::ShowLimits { peer } => {
                commands::subscriptions::show_peer_limits(&storage_dir, peer).await?;
            }

            SubscriptionAction::DeleteLimit { peer } => {
                commands::subscriptions::delete_peer_limit(&storage_dir, &peer).await?;
            }

            SubscriptionAction::ResetLimit { peer } => {
                commands::subscriptions::reset_peer_limit(&storage_dir, &peer).await?;
            }

            SubscriptionAction::ListAutoPay => {
                commands::subscriptions::list_autopay_rules(&storage_dir).await?;
            }

            SubscriptionAction::DeleteAutoPay { subscription_id } => {
                commands::subscriptions::delete_autopay_rule(&storage_dir, &subscription_id)
                    .await?;
            }

            SubscriptionAction::GlobalSettings => {
                commands::subscriptions::show_global_settings(&storage_dir).await?;
            }

            SubscriptionAction::ConfigureGlobal {
                enable,
                disable,
                daily_limit,
            } => {
                commands::subscriptions::configure_global_settings(
                    &storage_dir,
                    enable,
                    disable,
                    daily_limit,
                )
                .await?;
            }

            SubscriptionAction::RecentPayments { count } => {
                commands::subscriptions::show_recent_autopayments(&storage_dir, count).await?;
            }
            SubscriptionAction::Prorate {
                current_amount,
                new_amount,
                period_start,
                period_end,
                change_date,
            } => {
                commands::subscriptions::calculate_proration(
                    current_amount,
                    new_amount,
                    period_start,
                    period_end,
                    change_date,
                )
                .await?;
            }
        },
    }

    Ok(())
}

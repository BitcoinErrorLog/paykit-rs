//! QR code display and URI parsing commands

use anyhow::Result;
use std::path::Path;

use crate::ui;

/// Display QR code for current identity
pub async fn identity(storage_dir: &Path, _verbose: bool) -> Result<()> {
    ui::header("Identity QR Code");

    let identity = super::load_current_identity(storage_dir).await?;
    let uri = identity.pubky_uri();

    ui::info(&format!("Identity: {}", uri));
    println!();
    ui::qr_code(&uri)?;

    ui::separator();
    ui::info("Share this QR code to let others discover your payment methods");

    Ok(())
}

/// Display QR code for a contact
pub async fn contact(storage_dir: &Path, name: &str, _verbose: bool) -> Result<()> {
    use paykit_demo_core::DemoStorage;

    ui::header(&format!("Contact QR: {}", name));

    let storage = DemoStorage::new(storage_dir.join("data"));
    let contacts = storage.list_contacts()?;

    let contact = contacts
        .iter()
        .find(|c| c.name == name)
        .ok_or_else(|| anyhow::anyhow!("Contact '{}' not found", name))?;

    let uri = contact.pubky_uri();

    ui::info(&format!("Contact: {}", contact.name));
    ui::info(&format!("URI: {}", uri));
    println!();
    ui::qr_code(&uri)?;

    Ok(())
}

/// Generate a payment request QR code
pub async fn request(
    storage_dir: &Path,
    amount: Option<String>,
    description: Option<String>,
    method: &str,
    _verbose: bool,
) -> Result<()> {
    ui::header("Payment Request QR Code");

    let identity = super::load_current_identity(storage_dir).await?;

    // Build paykit URI with parameters
    let mut uri = format!("paykit://{}/request", identity.public_key());

    let mut params = Vec::new();
    if let Some(amt) = &amount {
        params.push(format!("amount={}", amt));
    }
    if let Some(desc) = &description {
        params.push(format!("description={}", urlencoding::encode(desc)));
    }
    params.push(format!("method={}", method));

    if !params.is_empty() {
        uri = format!("{}?{}", uri, params.join("&"));
    }

    ui::info("Payment Request:");
    if let Some(amt) = &amount {
        ui::key_value("  Amount", &format!("{} sats", amt));
    }
    if let Some(desc) = &description {
        ui::key_value("  Description", desc);
    }
    ui::key_value("  Method", method);
    ui::key_value("  URI", &uri);

    println!();
    ui::qr_code(&uri)?;

    ui::separator();
    ui::info("Scan this QR code to pay");

    Ok(())
}

/// Parse a scanned QR code or URI
pub async fn parse(data: &str, verbose: bool) -> Result<()> {
    ui::header("Parse URI");

    ui::key_value("Input", data);
    ui::separator();

    // Try to parse as different URI types
    if data.starts_with("pubky://") {
        parse_pubky_uri(data, verbose)?;
    } else if data.starts_with("paykit://") {
        parse_paykit_uri(data, verbose)?;
    } else if data.starts_with("lnurl") || data.starts_with("LNURL") {
        parse_lnurl(data, verbose)?;
    } else if data.starts_with("lnbc") || data.starts_with("lntb") || data.starts_with("lnbs") {
        parse_bolt11(data, verbose)?;
    } else if data.starts_with("bc1")
        || data.starts_with("tb1")
        || data.starts_with("bcrt1")
        || data.starts_with("1")
        || data.starts_with("3")
    {
        parse_bitcoin_address(data, verbose)?;
    } else if data.starts_with("bitcoin:") {
        parse_bip21(data, verbose)?;
    } else {
        ui::warning("Unknown URI format");
        ui::info("");
        ui::info("Supported formats:");
        ui::info("  - pubky://... (Pubky identity)");
        ui::info("  - paykit://... (Paykit payment request)");
        ui::info("  - lnurl... (Lightning URL)");
        ui::info("  - lnbc.../lntb... (BOLT11 invoice)");
        ui::info("  - bc1.../tb1.../1.../3... (Bitcoin address)");
        ui::info("  - bitcoin:... (BIP-21 URI)");
    }

    Ok(())
}

fn parse_pubky_uri(uri: &str, _verbose: bool) -> Result<()> {
    ui::success("Pubky Identity URI");

    let pubkey = uri.strip_prefix("pubky://").unwrap_or(uri);
    ui::key_value("Type", "Pubky Identity");
    ui::key_value("Public Key", pubkey);

    ui::separator();
    ui::info("Actions:");
    ui::info(&format!(
        "  - Add as contact: paykit-demo contacts add <name> {}",
        uri
    ));
    ui::info(&format!(
        "  - Discover methods: paykit-demo discover {}",
        uri
    ));
    ui::info(&format!("  - Pay directly: paykit-demo pay {}", uri));

    Ok(())
}

fn parse_paykit_uri(uri: &str, verbose: bool) -> Result<()> {
    ui::success("Paykit Protocol URI");

    // Parse the URI
    let without_scheme = uri.strip_prefix("paykit://").unwrap_or(uri);
    let (path, query) = if let Some(pos) = without_scheme.find('?') {
        (&without_scheme[..pos], Some(&without_scheme[pos + 1..]))
    } else {
        (without_scheme, None)
    };

    let parts: Vec<&str> = path.split('/').collect();

    ui::key_value("Type", "Paykit Request");

    if !parts.is_empty() {
        ui::key_value("Public Key", parts[0]);
    }

    if parts.len() > 1 {
        ui::key_value("Action", parts[1]);
    }

    if let Some(q) = query {
        if verbose {
            ui::info(&format!("Query Parameters: {}", q));
        }

        for param in q.split('&') {
            if let Some((key, value)) = param.split_once('=') {
                let decoded = urlencoding::decode(value).unwrap_or_else(|_| value.into());
                ui::key_value(&format!("  {}", key), &decoded);
            }
        }
    }

    Ok(())
}

fn parse_lnurl(data: &str, _verbose: bool) -> Result<()> {
    ui::success("LNURL");

    ui::key_value("Type", "Lightning URL (LNURL)");
    ui::key_value("Format", "Bech32 encoded URL");

    // LNURL is bech32 encoded - we just display it for now
    ui::info("");
    ui::info("LNURL types:");
    ui::info("  - lnurl-pay: Request payment");
    ui::info("  - lnurl-withdraw: Request withdrawal");
    ui::info("  - lnurl-auth: Authentication");
    ui::info("  - lnurl-channel: Channel request");

    ui::separator();
    ui::info("To pay this LNURL:");
    ui::info(&format!("  paykit-demo pay {} --method lightning", data));

    Ok(())
}

fn parse_bolt11(invoice: &str, verbose: bool) -> Result<()> {
    ui::success("BOLT11 Lightning Invoice");

    // Determine network from prefix
    let network = if invoice.starts_with("lnbc") {
        "mainnet"
    } else if invoice.starts_with("lntb") {
        "testnet"
    } else if invoice.starts_with("lnbs") || invoice.starts_with("lnsb") {
        "signet"
    } else {
        "unknown"
    };

    ui::key_value("Type", "Lightning Invoice (BOLT11)");
    ui::key_value("Network", network);

    if verbose {
        ui::key_value("Length", &format!("{} chars", invoice.len()));
    }

    ui::separator();
    ui::info("To pay this invoice:");
    ui::info(&format!("  paykit-demo pay {} --method lightning", invoice));

    Ok(())
}

fn parse_bitcoin_address(address: &str, _verbose: bool) -> Result<()> {
    ui::success("Bitcoin Address");

    // Determine address type
    let (addr_type, network) = if address.starts_with("bc1p") {
        ("Taproot (P2TR)", "mainnet")
    } else if address.starts_with("bc1q") {
        ("Native SegWit (P2WPKH/P2WSH)", "mainnet")
    } else if address.starts_with("bc1") {
        ("Bech32", "mainnet")
    } else if address.starts_with("tb1") {
        ("Testnet Bech32", "testnet")
    } else if address.starts_with("bcrt1") {
        ("Regtest Bech32", "regtest")
    } else if address.starts_with("1") {
        ("Legacy (P2PKH)", "mainnet")
    } else if address.starts_with("3") {
        ("SegWit (P2SH-P2WPKH)", "mainnet")
    } else {
        ("Unknown", "unknown")
    };

    ui::key_value("Type", "Bitcoin Address");
    ui::key_value("Address Type", addr_type);
    ui::key_value("Network", network);
    ui::key_value("Address", address);

    ui::separator();
    ui::info("To pay to this address:");
    ui::info(&format!(
        "  paykit-demo pay {} --method onchain --amount <sats>",
        address
    ));

    Ok(())
}

fn parse_bip21(uri: &str, verbose: bool) -> Result<()> {
    ui::success("BIP-21 Bitcoin URI");

    let without_scheme = uri.strip_prefix("bitcoin:").unwrap_or(uri);
    let (address, query) = if let Some(pos) = without_scheme.find('?') {
        (&without_scheme[..pos], Some(&without_scheme[pos + 1..]))
    } else {
        (without_scheme, None)
    };

    ui::key_value("Type", "BIP-21 Payment Request");
    ui::key_value("Address", address);

    if let Some(q) = query {
        for param in q.split('&') {
            if let Some((key, value)) = param.split_once('=') {
                let decoded = urlencoding::decode(value).unwrap_or_else(|_| value.into());
                let label = match key {
                    "amount" => "Amount (BTC)",
                    "label" => "Label",
                    "message" => "Message",
                    "lightning" => "Lightning Invoice",
                    _ => key,
                };
                ui::key_value(&format!("  {}", label), &decoded);
            }
        }
    }

    // Check for unified QR (has lightning parameter)
    if verbose {
        if let Some(q) = query {
            if q.contains("lightning=") {
                ui::info("");
                ui::info("This is a Unified QR code with both on-chain and Lightning options");
            }
        }
    }

    ui::separator();
    ui::info("To pay:");
    ui::info(&format!("  paykit-demo pay {} --method onchain", address));

    Ok(())
}

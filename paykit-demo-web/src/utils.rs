//! Utility functions for WASM

use wasm_bindgen::prelude::*;

/// Set up better panic messages in the browser console
pub fn set_panic_hook() {
    // Panic hook setup is available in tests
    // In production, panics will be caught by browser's error handling
}

/// Log a message to the browser console
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    pub fn warn(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    pub fn error(s: &str);
}

/// Convert a JS error into a Result
pub fn js_error(msg: &str) -> JsValue {
    js_sys::Error::new(msg).into()
}

/// Result of parsing a URI/QR code data
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct ParsedUri {
    /// The type of URI detected
    uri_type: String,
    /// The original raw data
    raw: String,
    /// Extracted public key (if applicable)
    public_key: Option<String>,
    /// Extracted amount (if applicable)
    amount: Option<String>,
    /// Extracted description/label (if applicable)
    description: Option<String>,
    /// Whether this is a valid/recognized format
    valid: bool,
}

#[wasm_bindgen]
impl ParsedUri {
    /// Get the detected URI type
    #[wasm_bindgen(getter)]
    pub fn uri_type(&self) -> String {
        self.uri_type.clone()
    }

    /// Get the raw input data
    #[wasm_bindgen(getter)]
    pub fn raw(&self) -> String {
        self.raw.clone()
    }

    /// Get the extracted public key (if any)
    #[wasm_bindgen(getter)]
    pub fn public_key(&self) -> Option<String> {
        self.public_key.clone()
    }

    /// Get the extracted amount (if any)
    #[wasm_bindgen(getter)]
    pub fn amount(&self) -> Option<String> {
        self.amount.clone()
    }

    /// Get the extracted description (if any)
    #[wasm_bindgen(getter)]
    pub fn description(&self) -> Option<String> {
        self.description.clone()
    }

    /// Check if the URI was recognized and valid
    #[wasm_bindgen(getter)]
    pub fn valid(&self) -> bool {
        self.valid
    }

    /// Convert to a JavaScript object
    pub fn to_object(&self) -> JsValue {
        let obj = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&obj, &"uri_type".into(), &self.uri_type.clone().into());
        let _ = js_sys::Reflect::set(&obj, &"raw".into(), &self.raw.clone().into());
        let _ = js_sys::Reflect::set(&obj, &"valid".into(), &self.valid.into());

        if let Some(pk) = &self.public_key {
            let _ = js_sys::Reflect::set(&obj, &"public_key".into(), &pk.clone().into());
        }
        if let Some(amt) = &self.amount {
            let _ = js_sys::Reflect::set(&obj, &"amount".into(), &amt.clone().into());
        }
        if let Some(desc) = &self.description {
            let _ = js_sys::Reflect::set(&obj, &"description".into(), &desc.clone().into());
        }

        obj.into()
    }
}

/// Parse a URI or QR code data and extract relevant information
///
/// Supported formats:
/// - `pubky://` - Pubky identity URIs
/// - `paykit://` - Paykit payment requests
/// - `lnurl` - Lightning URLs
/// - `lnbc/lntb/lnbs` - BOLT11 invoices
/// - `bc1/tb1/1/3` - Bitcoin addresses
/// - `bitcoin:` - BIP-21 URIs
///
/// # Examples
///
/// ```javascript
/// import { parseUri } from 'paykit-demo-web';
///
/// const result = parseUri("pubky://8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo");
/// console.log(result.uri_type); // "pubky"
/// console.log(result.public_key); // "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo"
///
/// const btc = parseUri("bitcoin:bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq?amount=0.001");
/// console.log(btc.uri_type); // "bitcoin"
/// console.log(btc.amount); // "0.001"
/// ```
#[wasm_bindgen(js_name = parseUri)]
pub fn parse_uri(data: &str) -> ParsedUri {
    let data = data.trim();

    // Pubky URI
    if data.starts_with("pubky://") {
        let pk = data.strip_prefix("pubky://").unwrap_or("");
        let pk_clean = pk.split('/').next().unwrap_or(pk);
        return ParsedUri {
            uri_type: "pubky".to_string(),
            raw: data.to_string(),
            public_key: Some(pk_clean.to_string()),
            amount: None,
            description: None,
            valid: pk_clean.len() >= 32,
        };
    }

    // Paykit URI
    if data.starts_with("paykit://") {
        let path = data.strip_prefix("paykit://").unwrap_or("");
        // Parse paykit://pubkey?amount=X&description=Y
        let parts: Vec<&str> = path.splitn(2, '?').collect();
        let pk = parts.first().unwrap_or(&"");

        let mut amount = None;
        let mut description = None;

        if parts.len() > 1 {
            for param in parts[1].split('&') {
                let kv: Vec<&str> = param.splitn(2, '=').collect();
                if kv.len() == 2 {
                    match kv[0] {
                        "amount" => amount = Some(kv[1].to_string()),
                        "description" | "label" | "message" => {
                            description = Some(kv[1].to_string())
                        }
                        _ => {}
                    }
                }
            }
        }

        return ParsedUri {
            uri_type: "paykit".to_string(),
            raw: data.to_string(),
            public_key: Some(pk.to_string()),
            amount,
            description,
            valid: !pk.is_empty(),
        };
    }

    // LNURL
    if data.to_lowercase().starts_with("lnurl") {
        return ParsedUri {
            uri_type: "lnurl".to_string(),
            raw: data.to_string(),
            public_key: None,
            amount: None,
            description: None,
            valid: data.len() > 10,
        };
    }

    // BOLT11 Invoice
    if data.to_lowercase().starts_with("lnbc")
        || data.to_lowercase().starts_with("lntb")
        || data.to_lowercase().starts_with("lnbs")
    {
        return ParsedUri {
            uri_type: "bolt11".to_string(),
            raw: data.to_string(),
            public_key: None,
            amount: None, // Would need to decode the invoice to extract amount
            description: None,
            valid: data.len() > 20,
        };
    }

    // BIP-21 Bitcoin URI
    if data.to_lowercase().starts_with("bitcoin:") {
        let path = data
            .strip_prefix("bitcoin:")
            .or_else(|| data.strip_prefix("Bitcoin:"))
            .unwrap_or("");
        let parts: Vec<&str> = path.splitn(2, '?').collect();
        let address = parts.first().unwrap_or(&"");

        let mut amount = None;
        let mut description = None;

        if parts.len() > 1 {
            for param in parts[1].split('&') {
                let kv: Vec<&str> = param.splitn(2, '=').collect();
                if kv.len() == 2 {
                    match kv[0].to_lowercase().as_str() {
                        "amount" => amount = Some(kv[1].to_string()),
                        "label" | "message" => description = Some(kv[1].to_string()),
                        _ => {}
                    }
                }
            }
        }

        return ParsedUri {
            uri_type: "bitcoin".to_string(),
            raw: data.to_string(),
            public_key: Some(address.to_string()),
            amount,
            description,
            valid: !address.is_empty(),
        };
    }

    // Raw Bitcoin addresses
    let lower = data.to_lowercase();
    if lower.starts_with("bc1") || lower.starts_with("tb1") {
        // Bech32 (SegWit) address
        return ParsedUri {
            uri_type: "bitcoin_address".to_string(),
            raw: data.to_string(),
            public_key: Some(data.to_string()),
            amount: None,
            description: None,
            valid: data.len() >= 26,
        };
    }

    if data.starts_with('1')
        || data.starts_with('3')
        || data.starts_with('m')
        || data.starts_with('n')
        || data.starts_with('2')
    {
        // Legacy or P2SH address
        if data.len() >= 26 && data.len() <= 35 && data.chars().all(|c| c.is_alphanumeric()) {
            return ParsedUri {
                uri_type: "bitcoin_address".to_string(),
                raw: data.to_string(),
                public_key: Some(data.to_string()),
                amount: None,
                description: None,
                valid: true,
            };
        }
    }

    // Unknown format
    ParsedUri {
        uri_type: "unknown".to_string(),
        raw: data.to_string(),
        public_key: None,
        amount: None,
        description: None,
        valid: false,
    }
}

/// Check if a string looks like a valid Pubky public key
#[wasm_bindgen(js_name = isValidPublicKey)]
pub fn is_valid_public_key(key: &str) -> bool {
    // Z-base32 encoded Ed25519 public key (52 chars)
    if key.len() < 32 || key.len() > 64 {
        return false;
    }

    // Check for valid z-base32 characters
    key.chars().all(|c| {
        matches!(
            c,
            'y' | 'b'
                | 'n'
                | 'd'
                | 'r'
                | 'f'
                | 'g'
                | '8'
                | 'e'
                | 'j'
                | 'k'
                | 'm'
                | 'c'
                | 'p'
                | 'q'
                | 'x'
                | 'o'
                | 't'
                | '1'
                | 'u'
                | 'w'
                | 'i'
                | 's'
                | 'z'
                | 'a'
                | '3'
                | '4'
                | '5'
                | 'h'
                | '7'
                | '6'
                | '9'
        )
    })
}

/// Generate a Pubky URI from a public key
#[wasm_bindgen(js_name = toPubkyUri)]
pub fn to_pubky_uri(public_key: &str) -> String {
    format!("pubky://{}", public_key)
}

/// Generate a Paykit payment URI
///
/// # Arguments
///
/// * `public_key` - The recipient's public key
/// * `amount` - Optional amount in satoshis
/// * `description` - Optional payment description
///
/// # Examples
///
/// ```javascript
/// import { toPaykitUri } from 'paykit-demo-web';
///
/// const uri = toPaykitUri("8pin...", "1000", "Coffee");
/// // Returns: "paykit://8pin...?amount=1000&description=Coffee"
/// ```
#[wasm_bindgen(js_name = toPaykitUri)]
pub fn to_paykit_uri(
    public_key: &str,
    amount: Option<String>,
    description: Option<String>,
) -> String {
    let mut uri = format!("paykit://{}", public_key);
    let mut params = Vec::new();

    if let Some(amt) = amount {
        params.push(format!("amount={}", amt));
    }
    if let Some(desc) = description {
        params.push(format!("description={}", desc));
    }

    if !params.is_empty() {
        uri.push('?');
        uri.push_str(&params.join("&"));
    }

    uri
}

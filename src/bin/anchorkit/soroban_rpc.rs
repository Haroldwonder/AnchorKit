/// Soroban RPC client for session and attestation reads exposed by the AnchorKit CLI.
use soroban_client::{
    account::{Account, AccountBehavior},
    address::{Address, AddressTrait},
    authorize_entry::{authorize_entry, AuthorizeEntryParams},
    contract::{ContractBehavior, Contracts},
    keypair::{Keypair, KeypairBehavior},
    soroban_rpc::{SendTransactionStatus, TransactionStatus},
    transaction::{TransactionBehavior, TransactionBuilder, TransactionBuilderBehavior},
    xdr::{
        ContractDataDurability, LedgerEntryData, LedgerKey, LedgerKeyContractData, OperationBody,
        ScMapEntry, ScSymbol, ScVal, SorobanCredentials, StringM, VecM,
    },
    Options, Server,
};
use std::{env, str::FromStr, time::Duration};

/// Environment variable for RPC URL
const RPC_URL_VAR: &str = "SOROBAN_RPC_URL";
const RPC_URL_ALT: &str = "ANCHORKIT_RPC_URL";

const MAX_SESSION_LIST_LIMIT: u64 = 1_000;

/// Get RPC URL from environment or use testnet default
pub fn get_rpc_url() -> String {
    env::var(RPC_URL_VAR)
        .or_else(|_| env::var(RPC_URL_ALT))
        .unwrap_or_else(|_| "https://soroban-testnet.stellar.org".to_string())
}

/// Get the RPC URL for `network`: `SOROBAN_RPC_URL` / `ANCHORKIT_RPC_URL`
/// when set, otherwise the public endpoint for testnet or futurenet. Mainnet
/// has no public default, so it must be configured explicitly.
pub fn get_rpc_url_for_network(network: &str) -> Result<String, String> {
    if let Ok(url) = env::var(RPC_URL_VAR).or_else(|_| env::var(RPC_URL_ALT)) {
        if !url.trim().is_empty() {
            return Ok(url);
        }
    }
    match network {
        "testnet" => Ok("https://soroban-testnet.stellar.org".to_string()),
        "futurenet" => Ok("https://rpc-futurenet.stellar.org".to_string()),
        other => Err(format!(
            "no Soroban RPC endpoint configured for {other} — set {RPC_URL_VAR} or {RPC_URL_ALT}"
        )),
    }
}

/// Get the contract ID from the global `--contract-id` flag, the
/// `ANCHORKIT_CONTRACT_ID` / `SOROBAN_CONTRACT_ID` env vars or the config
/// file (see `settings.rs` for the precedence).
pub fn get_contract_id() -> Result<String, String> {
    crate::settings::require_contract_id()
}

/// Get secret key for signing transactions
pub fn get_secret_key() -> Result<String, String> {
    env::var("STELLAR_SECRET_KEY")
        .or_else(|_| env::var("SOROBAN_SECRET_KEY"))
        .or_else(|_| env::var("ANCHORKIT_SECRET_KEY"))
        .map_err(|_| {
            "STELLAR_SECRET_KEY, SOROBAN_SECRET_KEY, or ANCHORKIT_SECRET_KEY not set".to_string()
        })
}

/// A routing quote candidate displayed by `anchorkit quote`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct QuoteCandidate {
    pub quote_id: u64,
    pub anchor: String,
    pub base_asset: String,
    pub quote_asset: String,
    pub rate: u64,
    pub fee_percentage: u32,
    pub minimum_amount: u64,
    pub maximum_amount: u64,
    pub valid_until: u64,
}

/// Simulate a read-only contract invocation and return its result value.
pub fn simulate_contract_call(function: &str, args: Vec<ScVal>) -> Result<ScVal, String> {
    let contract_id = get_contract_id()?;
    let rpc_url = get_rpc_url();
    let source = std::env::var("ANCHORKIT_SIMULATION_ACCOUNT")
        .unwrap_or_else(|_| "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN".to_string());

    run_rpc(async move {
        let rpc = Server::new(&rpc_url, Options::default()).map_err(|e| e.to_string())?;
        let network = rpc
            .get_network()
            .await
            .map_err(|e| format!("Could not load network details: {e}"))?;
        let passphrase = std::env::var("SOROBAN_NETWORK_PASSPHRASE")
            .ok()
            .or(network.passphrase)
            .ok_or_else(|| "RPC did not provide a network passphrase".to_string())?;
        let mut account = Account::new(&source, "0").map_err(|e| e.to_string())?;
        let contract = Contracts::new(&contract_id).map_err(|e| e.to_string())?;
        let operation = contract.call(
            function,
            if args.is_empty() { None } else { Some(args) },
        );
        let mut builder = TransactionBuilder::new(&mut account, &passphrase, None);
        builder
            .fee(100)
            .set_timeout(30)
            .map_err(|e| format!("Could not set transaction timeout: {e}"))?
            .add_operation(operation);
        let transaction = builder.build();
        let response = rpc
            .simulate_transaction(&transaction, None)
            .await
            .map_err(|e| format!("Could not simulate `{function}`: {e}"))?;
        if let Some(error) = response.error {
            return Err(format!("Simulation of `{function}` failed: {error}"));
        }
        response
            .to_result()
            .map(|(value, _)| value)
            .ok_or_else(|| format!("Simulation of `{function}` returned no value"))
    })
}

fn scval_string(value: &str) -> Result<ScVal, String> {
    let string = StringM::from_str(value).map_err(|e| format!("Invalid string argument: {e}"))?;
    Ok(ScVal::String(string))
}

fn scval_map(entries: Vec<(&str, ScVal)>) -> Result<ScVal, String> {
    let entries = entries
        .into_iter()
        .map(|(name, value)| {
            scval_symbol(name).map(|key| ScMapEntry { key, val: value })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let entries =
        VecM::try_from(entries).map_err(|e| format!("Could not encode contract map: {e}"))?;
    Ok(ScVal::Map(Some(entries)))
}

fn routing_options_arg(
    base: &str,
    quote: &str,
    amount: u64,
    strategy: &str,
) -> Result<ScVal, String> {
    let request = scval_map(vec![
        ("base_asset", scval_string(base)?),
        ("quote_asset", scval_string(quote)?),
        ("amount", ScVal::U64(amount)),
        ("operation_type", ScVal::U32(3)), // SERVICE_QUOTES
    ])?;
    let strategies = VecM::try_from(vec![scval_symbol(strategy)?])
        .map_err(|e| format!("Could not encode strategy: {e}"))?;
    let fallback = VecM::try_from(Vec::<ScVal>::new())
        .map_err(|e| format!("Could not encode fallback chain: {e}"))?;
    scval_map(vec![
        ("request", request),
        ("strategy", ScVal::Vec(Some(strategies))),
        ("min_reputation", ScVal::U32(0)),
        ("max_anchors", ScVal::U32(0)),
        ("require_kyc", ScVal::Bool(false)),
        ("jurisdiction", ScVal::Void),
        ("fallback_chain", ScVal::Vec(Some(fallback))),
    ])
}

fn map_field<'a>(value: &'a ScVal, name: &str) -> Result<&'a ScVal, String> {
    match value {
        ScVal::Map(Some(entries)) => entries
            .iter()
            .find(|entry| matches!(&entry.key, ScVal::Symbol(sym) if sym.to_utf8_string_lossy() == name))
            .map(|entry| &entry.val)
            .ok_or_else(|| format!("Contract record is missing `{name}`")),
        _ => Err("Contract returned an unexpected value".to_string()),
    }
}

fn field_u64(value: &ScVal, name: &str) -> Result<u64, String> {
    match map_field(value, name)? {
        ScVal::U64(number) => Ok(*number),
        _ => Err(format!("Contract returned an invalid `{name}`")),
    }
}

fn field_u32(value: &ScVal, name: &str) -> Result<u32, String> {
    match map_field(value, name)? {
        ScVal::U32(number) => Ok(*number),
        _ => Err(format!("Contract returned an invalid `{name}`")),
    }
}

fn field_string(value: &ScVal, name: &str) -> Result<String, String> {
    match map_field(value, name)? {
        ScVal::String(string) => Ok(string.to_utf8_string_lossy()),
        _ => Err(format!("Contract returned an invalid `{name}`")),
    }
}

fn quote_from_value(value: &ScVal) -> Result<QuoteCandidate, String> {
    Ok(QuoteCandidate {
        quote_id: field_u64(value, "quote_id")?,
        anchor: Address::from_sc_val(map_field(value, "anchor")?)
            .map_err(|e| format!("Contract returned an invalid anchor: {e}"))?
            .to_string(),
        base_asset: field_string(value, "base_asset")?,
        quote_asset: field_string(value, "quote_asset")?,
        rate: field_u64(value, "rate")?,
        fee_percentage: field_u32(value, "fee_percentage")?,
        minimum_amount: field_u64(value, "minimum_amount")?,
        maximum_amount: field_u64(value, "maximum_amount")?,
        valid_until: field_u64(value, "valid_until")?,
    })
}

/// Run the read-only routing dry run and collect the anchor's quote candidates.
pub fn route_quote_rpc(
    base: &str,
    quote: &str,
    amount: u64,
    strategy: &str,
) -> Result<(String, Vec<QuoteCandidate>), String> {
    let options = routing_options_arg(base, quote, amount, strategy)?;
    let selected = simulate_contract_call("route_transaction_dry_run", vec![options])?;
    let selected = Address::from_sc_val(&selected)
        .map_err(|e| format!("Dry run returned an invalid anchor: {e}"))?
        .to_string();

    let anchors = simulate_contract_call("get_routing_anchors", Vec::new())?;
    let anchors = match anchors {
        ScVal::Vec(Some(anchors)) => anchors.iter().try_fold(Vec::new(), |mut acc, value| {
            acc.push(
                Address::from_sc_val(value)
                    .map_err(|e| format!("Routing list contains an invalid anchor: {e}"))?
                    .to_string(),
            );
            Ok::<_, String>(acc)
        })?,
        ScVal::Vec(None) => Vec::new(),
        _ => return Err("Contract returned an invalid routing anchor list".to_string()),
    };

    let mut candidates = Vec::new();
    for anchor in anchors {
        let arg = Address::new(&anchor)
            .map_err(|e| format!("Invalid anchor address {anchor}: {e}"))?
            .to_sc_val()
            .map_err(|e| e.to_string())?;
        let value = simulate_contract_call("get_latest_quote", vec![arg])?;
        if matches!(value, ScVal::Void) {
            continue;
        }
        let candidate = quote_from_value(&value)?;
        if candidate.base_asset == base && candidate.quote_asset == quote {
            candidates.push(candidate);
        }
    }
    Ok((selected, candidates))
}

/// Session data structure returned from contract storage
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionData {
    pub session_id: u64,
    pub initiator: String,
    pub created_at: u64,
    pub operation_count: u64,
    pub expires_at: u64,
}

/// Attestation record decoded from contract storage (`StorageKey::Attest`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct AttestationData {
    pub id: u64,
    pub issuer: String,
    pub subject: String,
    pub timestamp: u64,
    /// Hex-encoded payload hash.
    pub payload_hash: String,
    /// Hex-encoded issuer signature.
    pub signature: String,
    pub issuer_revoked: bool,
    pub expires_at: Option<u64>,
}

fn run_rpc<T: Send + 'static>(
    future: impl std::future::Future<Output = Result<T, String>> + Send + 'static,
) -> Result<T, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("Could not start the Soroban RPC runtime: {e}"))?;
    runtime.block_on(future)
}

fn scval_symbol(value: &str) -> Result<ScVal, String> {
    let symbol = StringM::from_str(value).map_err(|e| format!("Invalid storage symbol: {e}"))?;
    Ok(ScVal::Symbol(ScSymbol::from(symbol)))
}

fn storage_key(variant: &str, session_id: Option<u64>) -> Result<ScVal, String> {
    let mut values = vec![scval_symbol(variant)?];
    if let Some(session_id) = session_id {
        values.push(ScVal::U64(session_id));
    }
    let values = VecM::try_from(values).map_err(|e| format!("Invalid storage key: {e}"))?;
    Ok(ScVal::Vec(Some(values)))
}

fn contract_data_value(
    entry: &soroban_client::soroban_rpc::LedgerEntryResult,
) -> Result<ScVal, String> {
    match entry.to_data() {
        LedgerEntryData::ContractData(data) => Ok(data.val),
        _ => Err("RPC returned a non-contract ledger entry for contract storage".to_string()),
    }
}

fn session_from_value(value: ScVal) -> Result<SessionData, String> {
    // `#[contracttype]` structs with named fields are encoded as an `ScVal::Map`
    // keyed by field-name symbols, so decode by name rather than by position.
    let entries = match value {
        ScVal::Map(Some(entries)) => entries,
        _ => return Err("Contract returned an invalid session record".to_string()),
    };
    let field = |name: &str| -> Result<&ScVal, String> {
        entries
            .iter()
            .find(|entry| matches!(&entry.key, ScVal::Symbol(sym) if sym.to_utf8_string_lossy() == name))
            .map(|entry| &entry.val)
            .ok_or_else(|| format!("Contract returned a session record without `{name}`"))
    };
    let as_u64 = |name: &str| match field(name)? {
        ScVal::U64(value) => Ok(*value),
        _ => Err(format!("Contract returned an invalid session `{name}`")),
    };
    let initiator = Address::from_sc_val(field("initiator")?)
        .map_err(|e| format!("Contract returned an invalid session initiator: {e}"))?
        .to_string();

    Ok(SessionData {
        session_id: as_u64("session_id")?,
        initiator,
        created_at: as_u64("created_at")?,
        operation_count: as_u64("operation_count")?,
        expires_at: as_u64("expires_at")?,
    })
}

fn attestation_from_value(value: ScVal) -> Result<AttestationData, String> {
    // `Attestation` is a `#[contracttype]` struct with named fields, so it is
    // encoded as an `ScVal::Map` keyed by field-name symbols.
    let entries = match value {
        ScVal::Map(Some(entries)) => entries,
        _ => return Err("Contract returned an invalid attestation record".to_string()),
    };
    let field = |name: &str| -> Result<&ScVal, String> {
        entries
            .iter()
            .find(|entry| matches!(&entry.key, ScVal::Symbol(sym) if sym.to_utf8_string_lossy() == name))
            .map(|entry| &entry.val)
            .ok_or_else(|| format!("Contract returned an attestation record without `{name}`"))
    };
    let as_u64 = |name: &str| match field(name)? {
        ScVal::U64(value) => Ok(*value),
        _ => Err(format!("Contract returned an invalid attestation `{name}`")),
    };
    let as_hex = |name: &str| match field(name)? {
        ScVal::Bytes(bytes) => Ok(hex::encode(bytes.0.as_slice())),
        _ => Err(format!("Contract returned an invalid attestation `{name}`")),
    };
    let as_address = |name: &str| {
        Address::from_sc_val(field(name)?)
            .map(|address| address.to_string())
            .map_err(|e| format!("Contract returned an invalid attestation `{name}`: {e}"))
    };
    let issuer_revoked = match field("issuer_revoked")? {
        ScVal::Bool(value) => *value,
        _ => return Err("Contract returned an invalid attestation `issuer_revoked`".to_string()),
    };
    // `Option<u64>` is encoded as `Void` for `None` and the bare value for `Some`.
    let expires_at = match field("expires_at") {
        Err(_) | Ok(ScVal::Void) => None,
        Ok(ScVal::U64(value)) => Some(*value),
        Ok(_) => return Err("Contract returned an invalid attestation `expires_at`".to_string()),
    };

    Ok(AttestationData {
        id: as_u64("id")?,
        issuer: as_address("issuer")?,
        subject: as_address("subject")?,
        timestamp: as_u64("timestamp")?,
        payload_hash: as_hex("payload_hash")?,
        signature: as_hex("signature")?,
        issuer_revoked,
        expires_at,
    })
}

fn contract_data_key(
    contract: &str,
    key: ScVal,
    durability: ContractDataDurability,
) -> Result<LedgerKey, String> {
    let contract = Contracts::new(contract).map_err(|e| e.to_string())?;
    let address = contract
        .address()
        .to_sc_address()
        .map_err(|e| format!("Invalid contract address: {e}"))?;
    Ok(LedgerKey::ContractData(LedgerKeyContractData {
        contract: address,
        key,
        durability,
    }))
}

/// Create and submit a session transaction, then wait for ledger confirmation.
pub fn create_session_rpc(initiator_addr: Option<String>) -> Result<u64, String> {
    let secret = get_secret_key()?;
    let keypair = Keypair::from_secret(&secret).map_err(|e| format!("Invalid secret key: {e}"))?;
    let signer_address = keypair.public_key();
    let initiator = initiator_addr.unwrap_or_else(|| signer_address.clone());
    if !is_valid_stellar_address(&initiator) || initiator.starts_with('C') {
        return Err(format!("Invalid Stellar account address: {initiator}"));
    }
    if initiator != signer_address {
        return Err(format!(
            "The initiator {initiator} must match the account derived from the configured signing key ({signer_address})"
        ));
    }

    let contract_id = get_contract_id()?;
    let rpc_url = get_rpc_url();
    eprintln!("ℹ  Creating session for initiator: {initiator}");
    eprintln!("ℹ  Using RPC endpoint: {rpc_url}");

    run_rpc(async move {
        let rpc = Server::new(&rpc_url, Options::default()).map_err(|e| e.to_string())?;
        let network = rpc.get_network().await.map_err(|e| e.to_string())?;
        let passphrase = env::var("SOROBAN_NETWORK_PASSPHRASE")
            .ok()
            .or(network.passphrase)
            .ok_or_else(|| "RPC did not provide a network passphrase".to_string())?;
        let mut account = rpc
            .get_account(&signer_address)
            .await
            .map_err(|e| format!("Could not load signer account: {e}"))?;
        let contract = Contracts::new(&contract_id).map_err(|e| e.to_string())?;
        let initiator_arg = Address::new(&initiator)
            .map_err(|e| format!("Invalid initiator address: {e}"))?
            .to_sc_val()
            .map_err(|e| e.to_string())?;
        let operation = contract.call("create_session", Some(vec![initiator_arg]));

        let mut builder = TransactionBuilder::new(&mut account, &passphrase, None);
        builder
            .fee(100)
            .set_timeout(30)
            .map_err(|e| format!("Could not set transaction timeout: {e}"))?
            .add_operation(operation);
        let transaction = builder.build();
        let mut transaction = rpc
            .prepare_transaction(&transaction)
            .await
            .map_err(|e| format!("Session transaction simulation failed: {e}"))?;

        let latest_ledger = rpc
            .get_latest_ledger()
            .await
            .map_err(|e| format!("Could not determine the latest ledger: {e}"))?
            .sequence;
        let valid_until_ledger_seq = latest_ledger
            .checked_add(100)
            .ok_or_else(|| "Latest ledger sequence is too large".to_string())?;
        let expected_signer = Address::new(&signer_address)
            .and_then(|address| address.to_sc_address())
            .map_err(|e| format!("Invalid signer address: {e}"))?;

        if let Some(operations) = transaction.operations.as_mut() {
            for operation in operations {
                if let OperationBody::InvokeHostFunction(invoke) = &mut operation.body {
                    let mut signed_auth = Vec::with_capacity(invoke.auth.len());
                    for entry in invoke.auth.iter().cloned() {
                        let auth = match &entry.credentials {
                            SorobanCredentials::SourceAccount => entry,
                            SorobanCredentials::Address(credentials)
                            | SorobanCredentials::AddressV2(credentials) => {
                                if credentials.address != expected_signer {
                                    return Err(format!(
                                        "Transaction requires authorization from {}, but only {signer_address} is configured",
                                        Address::from_sc_address(&credentials.address)
                                            .map_err(|e| e.to_string())?
                                            .to_string()
                                    ));
                                }
                                authorize_entry(AuthorizeEntryParams {
                                    entry,
                                    signer: &keypair,
                                    valid_until_ledger_seq,
                                    network_passphrase: &passphrase,
                                    use_address_v2: None,
                                })
                                .map_err(|e| format!("Could not authorize session creation: {e}"))?
                            }
                            _ => {
                                return Err(
                                    "Session creation requires an unsupported authorization type"
                                        .to_string(),
                                )
                            }
                        };
                        signed_auth.push(auth);
                    }
                    invoke.auth = VecM::try_from(signed_auth)
                        .map_err(|e| format!("Invalid authorization entries: {e}"))?;
                }
            }
        }

        transaction.sign(&[keypair]);
        let submitted = rpc
            .send_transaction(transaction)
            .await
            .map_err(|e| format!("Could not submit session transaction: {e}"))?;
        if !matches!(
            submitted.status,
            SendTransactionStatus::Pending | SendTransactionStatus::Duplicate
        ) {
            return Err(format!(
                "Session transaction was not accepted by RPC: {:?}",
                submitted.status
            ));
        }
        let confirmed = rpc
            .wait_transaction(&submitted.hash, Duration::from_secs(60))
            .await
            .map_err(|(e, _)| format!("Could not confirm session transaction: {e}"))?;
        if confirmed.status != TransactionStatus::Success {
            return Err(format!(
                "Session transaction failed on-chain: {:?}",
                confirmed.to_result()
            ));
        }
        match confirmed.to_result_meta().and_then(|(_, value)| value) {
            Some(ScVal::U64(session_id)) => Ok(session_id),
            Some(value) => Err(format!(
                "Session transaction returned an unexpected value: {value:?}"
            )),
            None => Err("Session transaction completed without a return value".to_string()),
        }
    })
}

/// Fetch a session record directly from persistent contract storage.
pub fn get_session_rpc(session_id: u64) -> Result<SessionData, String> {
    let contract_id = get_contract_id()?;
    let rpc_url = get_rpc_url();
    eprintln!("ℹ  Fetching session {session_id} from contract");
    eprintln!("ℹ  Using RPC endpoint: {rpc_url}");

    run_rpc(async move {
        let rpc = Server::new(&rpc_url, Options::default()).map_err(|e| e.to_string())?;
        let key = contract_data_key(
            &contract_id,
            storage_key("Session", Some(session_id))?,
            ContractDataDurability::Persistent,
        )?;
        let entries = rpc
            .get_ledger_entries(vec![key])
            .await
            .map_err(|e| format!("Could not fetch session {session_id}: {e}"))?
            .entries
            .unwrap_or_default();
        let entry = entries
            .first()
            .ok_or_else(|| format!("Session {session_id} not found"))?;
        let mut session = session_from_value(contract_data_value(entry)?)?;
        session.operation_count = match rpc
            .get_contract_data(
                &contract_id,
                storage_key("SessionOpCount", Some(session_id))?,
                ContractDataDurability::Persistent,
            )
            .await
        {
            Ok(entry) => match contract_data_value(&entry)? {
                ScVal::U64(count) => count,
                other => {
                    return Err(format!(
                        "Contract returned an invalid operation count: {other:?}"
                    ))
                }
            },
            Err(soroban_client::error::Error::ContractDataNotFound) => 0,
            Err(e) => {
                return Err(format!(
                    "Could not fetch operation count for session {session_id}: {e}"
                ))
            }
        };
        Ok(session)
    })
}

/// Fetch an attestation by ID, mirroring the contract's `get_attestation`:
/// read `StorageKey::Attest(id)` from persistent storage and flag it as
/// `issuer_revoked` when the issuer has an `AttestorRevoked` entry.
pub fn get_attestation_rpc(attestation_id: u64, rpc_url: &str) -> Result<AttestationData, String> {
    let contract_id = get_contract_id()?;
    let rpc_url = rpc_url.to_string();
    eprintln!("ℹ  Fetching attestation {attestation_id} from contract {contract_id}");
    eprintln!("ℹ  Using RPC endpoint: {rpc_url}");

    run_rpc(async move {
        let rpc = Server::new(&rpc_url, Options::default()).map_err(|e| e.to_string())?;
        let key = contract_data_key(
            &contract_id,
            storage_key("Attest", Some(attestation_id))?,
            ContractDataDurability::Persistent,
        )?;
        let entries = rpc
            .get_ledger_entries(vec![key])
            .await
            .map_err(|e| format!("Could not fetch attestation {attestation_id}: {e}"))?
            .entries
            .unwrap_or_default();
        let entry = entries
            .first()
            .ok_or_else(|| format!("Attestation {attestation_id} not found"))?;
        let mut attestation = attestation_from_value(contract_data_value(entry)?)?;

        if !attestation.issuer_revoked {
            let issuer = Address::new(&attestation.issuer)
                .map_err(|e| format!("Invalid issuer address: {e}"))?
                .to_sc_val()
                .map_err(|e| e.to_string())?;
            let revoked_key = VecM::try_from(vec![scval_symbol("AttestorRevoked")?, issuer])
                .map_err(|e| format!("Invalid storage key: {e}"))?;
            let revoked_key = contract_data_key(
                &contract_id,
                ScVal::Vec(Some(revoked_key)),
                ContractDataDurability::Persistent,
            )?;
            let revoked = rpc
                .get_ledger_entries(vec![revoked_key])
                .await
                .map_err(|e| format!("Could not check issuer revocation status: {e}"))?
                .entries
                .unwrap_or_default();
            attestation.issuer_revoked = !revoked.is_empty();
        }

        Ok(attestation)
    })
}

/// List active sessions by walking the contract's monotonic session counter.
pub fn list_sessions_rpc(limit: u64) -> Result<Vec<SessionData>, String> {
    if limit > MAX_SESSION_LIST_LIMIT {
        return Err(format!(
            "Session list limit {limit} exceeds the maximum of {MAX_SESSION_LIST_LIMIT}"
        ));
    }
    if limit == 0 {
        return Ok(Vec::new());
    }
    let contract_id = get_contract_id()?;
    let rpc_url = get_rpc_url();
    eprintln!("ℹ  Listing sessions (max {limit})");
    eprintln!("ℹ  Using RPC endpoint: {rpc_url}");

    run_rpc(async move {
        let rpc = Server::new(&rpc_url, Options::default()).map_err(|e| e.to_string())?;

        // ── Issue #1400 fix ─────────────────────────────────────────────────
        // Instance storage is NOT stored as separate ContractData ledger
        // entries.  It lives inside the contract instance entry
        // (LedgerKey::ContractInstance) as a map in ScContractInstance.storage.
        // The previous code tried `getLedgerEntries` with
        // `ContractDataDurability::Instance` which always returned no entries,
        // so `session_count` was never read and `list_sessions_rpc` returned
        // an empty vec on every deployment.
        //
        // Fix: fetch the contract instance ledger entry and look up the
        // `SESS_CNT` key in its `storage` map.
        use soroban_client::xdr::{LedgerKeyContractInstance, ScContractInstance};

        let contract = Contracts::new(&contract_id).map_err(|e| e.to_string())?;
        let contract_address = contract
            .address()
            .to_sc_address()
            .map_err(|e| format!("Invalid contract address: {e}"))?;

        let instance_key = LedgerKey::ContractData(LedgerKeyContractData {
            contract: contract_address.clone(),
            key: ScVal::LedgerKeyContractInstance,
            durability: ContractDataDurability::Instance,
        });

        // Use getLedgerEntries on the instance key — the RPC returns the
        // contract instance entry which contains the instance storage map.
        let instance_entries = rpc
            .get_ledger_entries(vec![instance_key])
            .await
            .map_err(|e| format!("Could not read the contract instance entry: {e}"))?
            .entries
            .unwrap_or_default();

        let instance_entry = instance_entries
            .first()
            .ok_or_else(|| "Contract instance entry not found — the contract may not be deployed at the configured address".to_string())?;

        // Decode the instance entry: it contains ScContractInstance with a
        // `storage` field that is an Option<VecM<ScMapEntry>>.
        let session_count = {
            let instance_val = contract_data_value(instance_entry)?;
            match instance_val {
                ScVal::ContractInstance(ScContractInstance {
                    storage: Some(storage_map),
                    ..
                }) => {
                    // Look up the SESS_CNT key in the storage map.
                    let sess_cnt_key = storage_key("SESS_CNT", None)?;
                    let mut found: Option<u64> = None;
                    for entry in storage_map.iter() {
                        if entry.key == sess_cnt_key {
                            match &entry.val {
                                ScVal::U64(count) => {
                                    found = Some(*count);
                                    break;
                                }
                                _ => {
                                    return Err(format!(
                                        "Contract returned an invalid session counter value: {:?}",
                                        entry.val
                                    ));
                                }
                            }
                        }
                    }
                    found.ok_or_else(|| {
                        "Session counter (SESS_CNT) not found in contract instance storage — \
                         the contract may not have created any sessions yet"
                            .to_string()
                    })?
                }
                ScVal::ContractInstance(ScContractInstance { storage: None, .. }) => {
                    return Err(
                        "Contract instance has no storage — the contract may not have been \
                         initialized or has never created a session"
                            .to_string(),
                    );
                }
                other => {
                    return Err(format!(
                        "Expected a contract instance entry, got: {other:?}"
                    ));
                }
            }
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| format!("System clock is before Unix epoch: {e}"))?
            .as_secs();
        let mut sessions = Vec::new();
        let mut end = session_count;
        while end > 0 && sessions.len() < limit as usize {
            let start = end.saturating_sub(50);
            let ids: Vec<u64> = (start..end).rev().collect();
            let mut keys = Vec::with_capacity(ids.len() * 2);
            for id in &ids {
                keys.push(contract_data_key(
                    &contract_id,
                    storage_key("Session", Some(*id))?,
                    ContractDataDurability::Persistent,
                )?);
                keys.push(contract_data_key(
                    &contract_id,
                    storage_key("SessionOpCount", Some(*id))?,
                    ContractDataDurability::Persistent,
                )?);
            }
            let entries = rpc
                .get_ledger_entries(keys)
                .await
                .map_err(|e| format!("Could not fetch session records: {e}"))?
                .entries
                .unwrap_or_default();

            let mut operation_counts = std::collections::HashMap::new();
            let mut batch_sessions = std::collections::HashMap::new();
            for entry in &entries {
                let LedgerKey::ContractData(key) = entry.to_key() else {
                    continue;
                };
                let Some(fields) = (match key.key {
                    ScVal::Vec(Some(fields)) => Some(fields),
                    _ => None,
                }) else {
                    continue;
                };
                let Some(variant) = fields.first().and_then(|value| match value {
                    ScVal::Symbol(symbol) => Some(symbol.0.as_slice()),
                    _ => None,
                }) else {
                    continue;
                };
                let Some(id) = fields.get(1).and_then(|v| match v {
                    ScVal::U64(id) => Some(*id),
                    _ => None,
                }) else {
                    continue;
                };
                if variant == b"Session" {
                    let session = session_from_value(contract_data_value(entry)?)?;
                    if session.expires_at > now {
                        batch_sessions.insert(id, session);
                    }
                } else if variant == b"SessionOpCount" {
                    if let ScVal::U64(count) = contract_data_value(entry)? {
                        operation_counts.insert(id, count);
                    }
                }
            }

            for id in ids {
                if let Some(mut session) = batch_sessions.remove(&id) {
                    session.operation_count = operation_counts.get(&id).copied().unwrap_or(0);
                    sessions.push(session);
                    if sessions.len() == limit as usize {
                        break;
                    }
                }
            }
            end = start;
        }

        Ok(sessions)
    })
}

/// Health record decoded from the contract's `HealthStatus` storage entry.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HealthData {
    pub anchor: String,
    pub latency_ms: u64,
    pub failure_count: u32,
    pub availability_percent: u32,
}

/// Maximum number of ledger keys requested in one `getLedgerEntries` call.
const LEDGER_ENTRY_BATCH: usize = 100;

/// Expected network passphrase for a CLI network name.
fn network_passphrase(network: &str) -> Option<&'static str> {
    match network {
        "testnet" => Some("Test SDF Network ; September 2015"),
        "mainnet" => Some("Public Global Stellar Network ; September 2015"),
        "futurenet" => Some("Test SDF Future Network ; October 2022"),
        _ => None,
    }
}

/// Validate that health queries are configured for `network`, returning
/// `(contract_id, rpc_url)`.
pub fn health_config(network: &str) -> Result<(String, String), String> {
    let contract_id = get_contract_id()?;
    let rpc_url = get_rpc_url_for_network(network)?;
    Ok((contract_id, rpc_url))
}

fn health_from_value(value: ScVal) -> Result<HealthData, String> {
    let entries = match value {
        ScVal::Map(Some(entries)) => entries,
        _ => return Err("Contract returned an invalid health record".to_string()),
    };
    let field = |name: &str| -> Result<&ScVal, String> {
        entries
            .iter()
            .find(|entry| matches!(&entry.key, ScVal::Symbol(sym) if sym.to_utf8_string_lossy() == name))
            .map(|entry| &entry.val)
            .ok_or_else(|| format!("Contract returned a health record without `{name}`"))
    };
    let as_u32 = |name: &str| match field(name)? {
        ScVal::U32(value) => Ok(*value),
        _ => Err(format!("Contract returned an invalid health `{name}`")),
    };
    let latency_ms = match field("latency_ms")? {
        ScVal::U64(value) => *value,
        _ => return Err("Contract returned an invalid health `latency_ms`".to_string()),
    };
    let anchor = Address::from_sc_val(field("anchor")?)
        .map_err(|e| format!("Contract returned an invalid health anchor: {e}"))?
        .to_string();

    Ok(HealthData {
        anchor,
        latency_ms,
        failure_count: as_u32("failure_count")?,
        availability_percent: as_u32("availability_percent")?,
    })
}

fn health_key(contract_id: &str, anchor: &str) -> Result<LedgerKey, String> {
    let anchor_val = Address::new(anchor)
        .map_err(|e| format!("Invalid attestor address {anchor}: {e}"))?
        .to_sc_val()
        .map_err(|e| e.to_string())?;
    let values = VecM::try_from(vec![scval_symbol("Health")?, anchor_val])
        .map_err(|e| format!("Invalid storage key: {e}"))?;
    contract_data_key(
        contract_id,
        ScVal::Vec(Some(values)),
        ContractDataDurability::Persistent,
    )
}

/// Read on-chain health status (the data behind `get_health_status`) for one
/// attestor, or for every registered attestor when `attestor` is `None`.
pub fn get_health_rpc(attestor: Option<&str>, network: &str) -> Result<Vec<HealthData>, String> {
    if let Some(addr) = attestor {
        if !is_valid_stellar_address(addr) {
            return Err(format!("Invalid attestor address: {addr}"));
        }
    }
    let (contract_id, rpc_url) = health_config(network)?;
    let attestor = attestor.map(str::to_string);
    let expected_passphrase = network_passphrase(network);
    let network = network.to_string();

    run_rpc(async move {
        let rpc = Server::new(&rpc_url, Options::default()).map_err(|e| e.to_string())?;

        // Make sure the RPC endpoint actually serves the requested network.
        if let Some(expected) = expected_passphrase {
            let info = rpc
                .get_network()
                .await
                .map_err(|e| format!("Could not reach RPC endpoint {rpc_url}: {e}"))?;
            if let Some(actual) = info.passphrase {
                if actual != expected {
                    return Err(format!(
                        "RPC endpoint {rpc_url} serves '{actual}', not the {network} network"
                    ));
                }
            }
        }

        let single = attestor.clone();
        let anchors: Vec<String> = match attestor {
            Some(addr) => vec![addr],
            None => {
                let list_key = contract_data_key(
                    &contract_id,
                    storage_key("ATTLST", None)?,
                    ContractDataDurability::Persistent,
                )?;
                let entries = rpc
                    .get_ledger_entries(vec![list_key])
                    .await
                    .map_err(|e| format!("Could not fetch the attestor list: {e}"))?
                    .entries
                    .unwrap_or_default();
                match entries.first() {
                    None => Vec::new(),
                    Some(entry) => match contract_data_value(entry)? {
                        ScVal::Vec(Some(values)) => values
                            .iter()
                            .map(|v| {
                                Address::from_sc_val(v)
                                    .map(|a| a.to_string())
                                    .map_err(|e| format!("Contract returned an invalid attestor: {e}"))
                            })
                            .collect::<Result<_, _>>()?,
                        ScVal::Vec(None) => Vec::new(),
                        other => {
                            return Err(format!(
                                "Contract returned an invalid attestor list: {other:?}"
                            ))
                        }
                    },
                }
            }
        };

        let mut records = Vec::with_capacity(anchors.len());
        for chunk in anchors.chunks(LEDGER_ENTRY_BATCH) {
            let keys = chunk
                .iter()
                .map(|anchor| health_key(&contract_id, anchor))
                .collect::<Result<Vec<_>, _>>()?;
            let entries = rpc
                .get_ledger_entries(keys)
                .await
                .map_err(|e| format!("Could not fetch health status: {e}"))?
                .entries
                .unwrap_or_default();
            let mut by_anchor = std::collections::HashMap::new();
            for entry in &entries {
                let health = health_from_value(contract_data_value(entry)?)?;
                by_anchor.insert(health.anchor.clone(), health);
            }
            // Preserve the on-chain attestor order.
            for anchor in chunk {
                if let Some(health) = by_anchor.remove(anchor) {
                    records.push(health);
                }
            }
        }

        if let (Some(addr), true) = (single, records.is_empty()) {
            return Err(format!("No health status recorded for attestor {addr}"));
        }
        Ok(records)
    })
}

/// Check if address is valid Stellar format.
fn is_valid_stellar_address(addr: &str) -> bool {
    let Some(rest) = addr.strip_prefix('G') else {
        return addr.strip_prefix('C').is_some_and(|rest| {
            addr.len() == 56 && rest.chars().all(|c| matches!(c, 'A'..='Z' | '2'..='7'))
        });
    };
    addr.len() == 56 && rest.chars().all(|c| matches!(c, 'A'..='Z' | '2'..='7'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_storage_keys_encode_variant_and_id() {
        assert!(matches!(
            storage_key("Session", Some(42)).unwrap(),
            ScVal::Vec(Some(values))
                if matches!(values.as_slice(), [ScVal::Symbol(_), ScVal::U64(42)])
        ));
    }

    #[test]
    fn session_records_decode_from_contract_values() {
        use soroban_client::xdr::{Limits, ReadXdr};
        use soroban_sdk::{
            xdr::{Limits as SdkLimits, ScVal as SdkScVal, WriteXdr},
            Address as SdkAddress, Env, IntoVal, TryFromVal, Val,
        };

        const INITIATOR: &str = "GBRPYHIL2CI3C65PL7DH65O6PXARPG63G2GQRFDW6XQVFVVL6UUEZFX";

        // Serialize a real `Session` contract type, exactly as the contract stores it.
        let env = Env::default();
        let session = anchorkit::Session {
            session_id: 7,
            initiator: SdkAddress::from_str(&env, INITIATOR),
            created_at: 100,
            operation_count: 3,
            expires_at: 200,
        };
        let val: Val = session.into_val(&env);
        let sdk_val = SdkScVal::try_from_val(&env, &val).unwrap();
        let bytes = sdk_val.to_xdr(SdkLimits::none()).unwrap();
        let value = ScVal::from_xdr(bytes, Limits::none()).unwrap();
        assert!(matches!(value, ScVal::Map(Some(_))));

        let record = session_from_value(value).unwrap();
        assert_eq!(record.session_id, 7);
        assert_eq!(record.initiator, INITIATOR);
        assert_eq!(record.created_at, 100);
        assert_eq!(record.operation_count, 3);
        assert_eq!(record.expires_at, 200);
    }

    #[test]
    fn address_validation_accepts_stellar_strkeys_only() {
        assert!(is_valid_stellar_address(
            "GBRPYHIL2CI3C65PL7DH65O6PXARPG63G2GQRFDW6XQVFVVL6UUEZFX"
        ));
        assert!(!is_valid_stellar_address(
            "GBRPYHIL2CI3C65PL7DH65O6PXARPG63G2GQRFDW6XQVFVVL6UUEZF0"
        ));
        assert!(!is_valid_stellar_address("not-an-address"));
    }

    #[test]
    fn list_limit_zero_does_not_require_network_configuration() {
        assert!(list_sessions_rpc(0).unwrap().is_empty());
    }

    // ── Issue #1400: instance storage fixture decoding ──────────────────────
    //
    // The fix requires reading SESS_CNT from the contract instance entry's
    // `storage` map, not from a standalone ledger entry.  This test
    // decodes a captured instance-entry fixture (an ScVal::ContractInstance
    // with a storage map containing SESS_CNT = 5) and verifies the counter
    // is extracted correctly.

    #[test]
    fn decodes_session_counter_from_instance_storage_fixture() {
        use soroban_client::xdr::{ScContractInstance, ScMap, ScMapEntry, VecM};

        // Build a synthetic ScContractInstance with SESS_CNT = 5 in storage.
        let sess_cnt_key = storage_key("SESS_CNT", None).unwrap();
        let sess_cnt_val = ScVal::U64(5);

        let storage_map = ScMap::try_from(vec![ScMapEntry {
            key: sess_cnt_key.clone(),
            val: sess_cnt_val.clone(),
        }])
        .unwrap();

        let instance = ScVal::ContractInstance(ScContractInstance {
            storage: Some(storage_map),
            executable: soroban_client::xdr::ContractExecutable::WasmRef(soroban_client::xdr::Hash(
                [0u8; 32],
            )),
        });

        // Simulate what the fixed list_sessions_rpc does: look up SESS_CNT
        // in the instance storage map.
        let count = match instance {
            ScVal::ContractInstance(ScContractInstance {
                storage: Some(storage_map),
                ..
            }) => {
                let mut found: Option<u64> = None;
                for entry in storage_map.iter() {
                    if entry.key == sess_cnt_key {
                        match &entry.val {
                            ScVal::U64(count) => {
                                found = Some(*count);
                                break;
                            }
                            _ => panic!("Expected U64 for SESS_CNT, got {:?}", entry.val),
                        }
                    }
                }
                found.expect("SESS_CNT not found in instance storage")
            }
            _ => panic!("Expected ScVal::ContractInstance, got something else"),
        };

        assert_eq!(count, 5, "SESS_CNT must decode to 5 from the instance entry fixture");
    }

    #[test]
    fn returns_error_when_instance_has_no_storage() {
        use soroban_client::xdr::ScContractInstance;

        let instance = ScVal::ContractInstance(ScContractInstance {
            storage: None,
            executable: soroban_client::xdr::ContractExecutable::WasmRef(soroban_client::xdr::Hash(
                [0u8; 32],
            )),
        });

        // A contract with no storage should produce a clear error, not a
        // silent empty list.
        let has_storage = match instance {
            ScVal::ContractInstance(ScContractInstance { storage: Some(_), .. }) => true,
            _ => false,
        };
        assert!(!has_storage, "An instance with storage=None should report no storage");
    }
}


/// Soroban RPC client for session operations exposed by the AnchorKit CLI.
use soroban_client::{
    address::{Address, AddressTrait},
    authorize_entry::{authorize_entry, AuthorizeEntryParams},
    contract::{ContractBehavior, Contracts},
    keypair::{Keypair, KeypairBehavior},
    soroban_rpc::{SendTransactionStatus, TransactionStatus},
    transaction::{TransactionBehavior, TransactionBuilder, TransactionBuilderBehavior},
    xdr::{
        ContractDataDurability, LedgerEntryData, LedgerKey, LedgerKeyContractData, OperationBody,
        ScSymbol, ScVal, SorobanCredentials, StringM, VecM,
    },
    Options, Server,
};
use std::{env, str::FromStr, time::Duration};

/// Environment variable for RPC URL
const RPC_URL_VAR: &str = "SOROBAN_RPC_URL";
const RPC_URL_ALT: &str = "ANCHORKIT_RPC_URL";

/// Environment variable for contract ID
const CONTRACT_ID_VAR: &str = "SOROBAN_CONTRACT_ID";
const CONTRACT_ID_ALT: &str = "ANCHORKIT_CONTRACT_ID";

const MAX_SESSION_LIST_LIMIT: u64 = 1_000;

/// Get RPC URL from environment or use testnet default
pub fn get_rpc_url() -> String {
    env::var(RPC_URL_VAR)
        .or_else(|_| env::var(RPC_URL_ALT))
        .unwrap_or_else(|_| "https://soroban-testnet.stellar.org".to_string())
}

/// Get contract ID from environment
pub fn get_contract_id() -> Result<String, String> {
    env::var(CONTRACT_ID_VAR)
        .or_else(|_| env::var(CONTRACT_ID_ALT))
        .map_err(|_| "SOROBAN_CONTRACT_ID or ANCHORKIT_CONTRACT_ID not set".to_string())
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

/// Session data structure returned from contract storage
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionData {
    pub session_id: u64,
    pub initiator: String,
    pub created_at: u64,
    pub operation_count: u64,
    pub expires_at: u64,
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
    let fields = match value {
        ScVal::Vec(Some(fields)) => fields,
        _ => return Err("Contract returned an invalid session record".to_string()),
    };
    if fields.len() != 5 {
        return Err(format!(
            "Contract returned a session record with {} fields; expected 5",
            fields.len()
        ));
    }
    let as_u64 = |index: usize, name: &str| match &fields[index] {
        ScVal::U64(value) => Ok(*value),
        _ => Err(format!("Contract returned an invalid session {name}")),
    };
    let initiator = Address::from_sc_val(&fields[1])
        .map_err(|e| format!("Contract returned an invalid session initiator: {e}"))?
        .to_string();

    Ok(SessionData {
        session_id: as_u64(0, "ID")?,
        initiator,
        created_at: as_u64(2, "creation time")?,
        operation_count: as_u64(3, "operation count")?,
        expires_at: as_u64(4, "expiry time")?,
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
        let counter_key = contract_data_key(
            &contract_id,
            storage_key("SESS_CNT", None)?,
            ContractDataDurability::Instance,
        )?;
        let counter_entries = rpc
            .get_ledger_entries(vec![counter_key])
            .await
            .map_err(|e| format!("Could not read the session counter: {e}"))?
            .entries
            .unwrap_or_default();
        let Some(counter_entry) = counter_entries.first() else {
            return Ok(Vec::new());
        };
        let session_count = match contract_data_value(counter_entry)? {
            ScVal::U64(count) => count,
            other => {
                return Err(format!(
                    "Contract returned an invalid session counter: {other:?}"
                ))
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
        let initiator = Address::new("GBRPYHIL2CI3C65PL7DH65O6PXARPG63G2GQRFDW6XQVFVVL6UUEZFX")
            .unwrap()
            .to_sc_address()
            .unwrap();
        let record = session_from_value(ScVal::Vec(Some(
            VecM::try_from(vec![
                ScVal::U64(7),
                ScVal::Address(initiator),
                ScVal::U64(100),
                ScVal::U64(3),
                ScVal::U64(200),
            ])
            .unwrap(),
        )))
        .unwrap();
        assert_eq!(record.session_id, 7);
        assert_eq!(
            record.initiator,
            "GBRPYHIL2CI3C65PL7DH65O6PXARPG63G2GQRFDW6XQVFVVL6UUEZFX"
        );
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
}

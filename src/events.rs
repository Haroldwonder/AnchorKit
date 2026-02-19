use soroban_sdk::{contracttype, Address, BytesN, Env, String};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttestorAdded {
    pub attestor: Address,
}

impl AttestorAdded {
    pub fn publish(&self, env: &Env) {
        env.events().publish(
            (soroban_sdk::symbol_short!("attestor"), soroban_sdk::symbol_short!("added")),
            self.clone(),
        );
    }
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttestorRemoved {
    pub attestor: Address,
}

impl AttestorRemoved {
    pub fn publish(&self, env: &Env) {
        env.events().publish(
            (soroban_sdk::symbol_short!("attestor"), soroban_sdk::symbol_short!("removed")),
            self.clone(),
        );
    }
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttestationRecorded {
    pub id: u64,
    pub issuer: Address,
    pub subject: Address,
    pub timestamp: u64,
    pub payload_hash: BytesN<32>,
}

impl AttestationRecorded {
    pub fn publish(&self, env: &Env) {
        env.events().publish(
            (soroban_sdk::symbol_short!("attest"), soroban_sdk::symbol_short!("recorded")),
            self.clone(),
        );
    }
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EndpointConfigured {
    pub attestor: Address,
    pub url: String,
}

impl EndpointConfigured {
    pub fn publish(&self, env: &Env) {
        env.events().publish(
            (soroban_sdk::symbol_short!("endpoint"), soroban_sdk::symbol_short!("config")),
            self.clone(),
        );
    }
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EndpointRemoved {
    pub attestor: Address,
}

impl EndpointRemoved {
    pub fn publish(&self, env: &Env) {
        env.events().publish(
            (soroban_sdk::symbol_short!("endpoint"), soroban_sdk::symbol_short!("removed")),
            self.clone(),
        );
    }
}

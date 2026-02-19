use soroban_sdk::contracterror;

/// Error codes for AnchorKit contract operations.
/// All error codes are in the range 100-120 for stable API compatibility.
/// See API_SPEC.md for detailed documentation.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    UnauthorizedAttestor = 3,
    AttestorAlreadyRegistered = 4,
    AttestorNotRegistered = 5,
    ReplayAttack = 6,
    InvalidTimestamp = 7,
    AttestationNotFound = 8,
    InvalidPublicKey = 9,
    InvalidEndpointFormat = 10,
    EndpointNotFound = 11,
    EndpointAlreadyExists = 12,
    /// Session-related errors (13-15 reserved for future use)
    SessionNotFound = 13,
    InvalidSessionId = 14,
    SessionReplayAttack = 15,
}

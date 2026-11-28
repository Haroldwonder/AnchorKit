//! Integration tests for the public SEP-31 sending-anchor client API.

mod sep31_client_tests {
    use anchorkit::{
        callback_signature_header, MockSep31Transport, RawSep31CreationResponse,
        RawSep31InfoResponse, RawSep31Transaction, Sep31AssetInfo, Sep31Auth, Sep31Callback,
        Sep31Client, Sep31CustomerReference, Sep31Request, Sep31Response, Sep31Status,
        Sep31TransactionRequest,
    };
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine as _;
    use ed25519_dalek::{Signer, SigningKey};

    const JWT: &str = "header.payload.signature";
    const HOST: &str = "https://sending.example.com/callback";

    #[test]
    fn public_api_runs_a_full_sep31_flow() {
        let auth = Sep31Auth::new(JWT).unwrap();
        let asset = Sep31AssetInfo::new("USDC", &["SWIFT"]);
        let request = Sep31TransactionRequest::new("10", "USDC", "SWIFT")
            .with_sender(Sep31CustomerReference::with_type(
                "sender-id",
                "sep31_sender",
            ))
            .with_receiver_id("receiver-id");

        let transport = MockSep31Transport::new()
            .with_response(
                Sep31Request::info(JWT, None),
                Sep31Response::Info(
                    RawSep31InfoResponse::new(vec![asset])
                        .with_customer_types(&["sep31_sender"], &["sep31_receiver"]),
                ),
            )
            .with_response(
                Sep31Request::create(&request, JWT),
                Sep31Response::Created(RawSep31CreationResponse::new("tx-1")),
            )
            .with_response(
                Sep31Request::get("tx-1", JWT),
                Sep31Response::Transaction(Box::new(RawSep31Transaction::new(
                    "tx-1",
                    Sep31Status::PendingSender,
                ))),
            );

        let mut client = Sep31Client::new(transport, auth);
        assert_eq!(client.get_info(None).unwrap().receive.len(), 1);
        assert_eq!(client.create_transaction(&request).unwrap().id, "tx-1");
        assert_eq!(
            client.get_transaction("tx-1").unwrap().status,
            Sep31Status::PendingSender
        );
        assert_eq!(client.transport().call_count(), 3);
    }

    #[test]
    fn public_api_parses_and_verifies_callbacks() {
        let key = SigningKey::from_bytes(&[3u8; 32]);
        let timestamp = 1000u64;
        let body_json = r#"{"transaction":{"id":"tx-1","status":"completed"}}"#;
        let signature = key.sign(format!("{timestamp}.{HOST}.{body_json}").as_bytes());
        let header = format!("t={timestamp}, s={}", STANDARD.encode(signature.to_bytes()));
        let header = callback_signature_header(None, Some(&header)).unwrap();

        let callback = Sep31Callback::parse(
            header,
            HOST,
            RawSep31Transaction::new("tx-1", Sep31Status::Completed),
            timestamp + 1,
            60,
        )
        .unwrap();

        assert_eq!(callback.transaction.status, Sep31Status::Completed);
        assert!(callback
            .verify(body_json, key.verifying_key().as_bytes())
            .is_ok());
    }
}

//! Tests for auth module: token expiry, PKCE helpers, URL encoding/decoding, callback parsing

#[cfg(test)]
mod token_tests {
    use terms::auth::TokenResponse;

    fn make_token(expires_in: u64, age_secs: u64) -> TokenResponse {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        TokenResponse {
            access_token: "test-token".to_string(),
            refresh_token: Some("test-refresh".to_string()),
            expires_in,
            token_type: "Bearer".to_string(),
            obtained_at: now.saturating_sub(age_secs),
        }
    }

    #[test]
    fn fresh_token_not_expired() {
        let token = make_token(3600, 0);
        assert!(!token.is_expired());
    }

    #[test]
    fn old_token_is_expired() {
        let token = make_token(3600, 3700);
        assert!(token.is_expired());
    }

    #[test]
    fn token_expires_with_60s_buffer() {
        // Token with 100s lifetime, obtained 50s ago: 50s remaining > 60s buffer → not expired
        let token = make_token(100, 39);
        assert!(!token.is_expired());

        // Token with 100s lifetime, obtained 41s ago: ~59s remaining < 60s → expired
        let token2 = make_token(100, 41);
        assert!(token2.is_expired());
    }

    #[test]
    fn with_timestamp_sets_current_time() {
        let token = TokenResponse {
            access_token: "test".to_string(),
            refresh_token: None,
            expires_in: 3600,
            token_type: "Bearer".to_string(),
            obtained_at: 0,
        };
        let stamped = token.with_timestamp();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(stamped.obtained_at >= now - 2 && stamped.obtained_at <= now + 1);
    }
}

#[cfg(test)]
mod pkce_tests {
    use terms::auth::{compute_code_challenge, generate_code_verifier};

    #[test]
    fn verifier_is_base64url_no_padding() {
        let verifier = generate_code_verifier();
        assert!(!verifier.is_empty());
        assert!(!verifier.contains('='));
        assert!(!verifier.contains('+'));
        assert!(!verifier.contains('/'));
        // Should be 43 chars (32 bytes → base64url no pad)
        assert_eq!(verifier.len(), 43);
    }

    #[test]
    fn verifier_is_random() {
        let v1 = generate_code_verifier();
        let v2 = generate_code_verifier();
        assert_ne!(v1, v2);
    }

    #[test]
    fn challenge_is_sha256_of_verifier() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = compute_code_challenge(verifier);
        // Known value: SHA256("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk") base64url
        assert_eq!(challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn challenge_is_deterministic() {
        let verifier = "test-verifier-123";
        let c1 = compute_code_challenge(verifier);
        let c2 = compute_code_challenge(verifier);
        assert_eq!(c1, c2);
    }
}

#[cfg(test)]
mod url_encoding_tests {
    use terms::auth::{percent_encode, simple_url_decode};

    #[test]
    fn encode_plain_text() {
        assert_eq!(percent_encode("hello"), "hello");
    }

    #[test]
    fn encode_spaces() {
        assert_eq!(percent_encode("hello world"), "hello%20world");
    }

    #[test]
    fn encode_special_chars() {
        assert_eq!(percent_encode("a=b&c=d"), "a%3Db%26c%3Dd");
    }

    #[test]
    fn encode_preserves_unreserved() {
        assert_eq!(percent_encode("a-b_c.d~e"), "a-b_c.d~e");
    }

    #[test]
    fn decode_plain_text() {
        assert_eq!(simple_url_decode("hello"), "hello");
    }

    #[test]
    fn decode_percent_encoded() {
        assert_eq!(simple_url_decode("hello%20world"), "hello world");
    }

    #[test]
    fn decode_plus_as_space() {
        assert_eq!(simple_url_decode("hello+world"), "hello world");
    }

    #[test]
    fn decode_special_chars() {
        assert_eq!(simple_url_decode("a%3Db%26c"), "a=b&c");
    }

    #[test]
    fn roundtrip_encode_decode() {
        let original = "hello world & goodbye=test";
        let encoded = percent_encode(original);
        let decoded = simple_url_decode(&encoded);
        assert_eq!(decoded, original);
    }
}

#[cfg(test)]
mod callback_parsing_tests {
    use terms::auth::parse_auth_callback;

    #[test]
    fn parse_valid_callback() {
        let request = "GET /?code=ABC123&state=xyz HTTP/1.1\r\nHost: localhost\r\n";
        let code = parse_auth_callback(request).unwrap();
        assert_eq!(code, "ABC123");
    }

    #[test]
    fn parse_encoded_callback() {
        let request = "GET /?code=ABC%20123 HTTP/1.1\r\n";
        let code = parse_auth_callback(request).unwrap();
        assert_eq!(code, "ABC 123");
    }

    #[test]
    fn parse_error_callback() {
        let request = "GET /?error=access_denied&error_description=User%20denied HTTP/1.1\r\n";
        let result = parse_auth_callback(request);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("access_denied"));
    }

    #[test]
    fn parse_empty_callback() {
        let request = "GET / HTTP/1.1\r\n";
        let result = parse_auth_callback(request);
        assert!(result.is_err());
    }
}

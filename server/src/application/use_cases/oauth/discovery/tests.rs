#[cfg(test)]
mod tests {
    use crate::application::use_cases::oauth::discovery::{query::Query, use_case::UseCase};

    #[tokio::test]
    async fn builds_metadata_from_issuer() {
        let use_case = UseCase::new("https://auth.test/".to_string());

        let response = use_case.execute(Query).await.unwrap();

        // Trailing slash normalized; endpoints derived from issuer.
        assert_eq!(response.issuer, "https://auth.test");
        assert_eq!(response.token_endpoint, "https://auth.test/oauth/token");
        assert_eq!(response.jwks_uri, "https://auth.test/.well-known/jwks.json");
        // Secure-by-default profile.
        assert_eq!(response.response_types_supported, vec!["code".to_string()]);
        assert_eq!(
            response.code_challenge_methods_supported,
            vec!["S256".to_string()]
        );
    }
}

//! Classify documented, observed entitlement codes without reflecting bodies.

const MAX_ERROR_BODY: usize = 8 * 1024;

pub(super) async fn model_access_denial(
    url: &str,
    response: &mut reqwest::Response,
) -> Option<u32> {
    if response.status() != reqwest::StatusCode::TOO_MANY_REQUESTS
        || reqwest::Url::parse(url).ok()?.host_str()? != "open.bigmodel.cn"
    {
        return None;
    }
    let mut body = Vec::new();
    while let Ok(Some(chunk)) = response.chunk().await {
        if body.len().saturating_add(chunk.len()) > MAX_ERROR_BODY {
            return None;
        }
        body.extend_from_slice(&chunk);
    }
    permission_code(&body)
}

fn permission_code(body: &[u8]) -> Option<u32> {
    let value: serde_json::Value = serde_json::from_slice(body).ok()?;
    let code = value.get("error")?.get("code")?;
    let code = code.as_u64().or_else(|| code.as_str()?.parse().ok())?;
    // Actual GLM-5.3-FlashX refusal: HTTP429 with code1311 means this
    // subscription has no model access. No retry can grant permission.
    (code == 1311).then_some(1311)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_entitlement_code_is_distinct_from_real_rate_limits() {
        assert_eq!(
            permission_code(br#"{"error":{"code":"1311","message":"untrusted echoed secret"}}"#),
            Some(1311)
        );
        assert_eq!(permission_code(br#"{"error":{"code":1311}}"#), Some(1311));
        assert_eq!(permission_code(br#"{"error":{"code":"1302"}}"#), None);
        assert_eq!(permission_code(b"not JSON"), None);
    }

    #[tokio::test]
    async fn subscription_refusal_fails_once_without_echoing_the_provider_message() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut connection, _) = listener.accept().unwrap();
            let mut request = [0; 2048];
            let _ = connection.read(&mut request);
            let body = r#"{"error":{"code":"1311","message":"SECRET_ECHO_DO_NOT_DISPLAY"}}"#;
            write!(connection,"HTTP/1.1 429 Too Many Requests\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
        });
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .build()
            .unwrap();
        let result = crate::backoff::send_with_backoff(
            "test",
            "https://open.bigmodel.cn/api/coding/paas/v4/chat/completions",
            1,
            std::time::Duration::ZERO,
            || client.post(format!("http://{address}/")),
        )
        .await;
        let error = result.unwrap_err();
        assert!(matches!(
            error,
            crate::chat_builtin_http::BuiltinHttpError::ModelAccessDenied {
                provider_code: 1311,
                ..
            }
        ));
        assert!(!error.to_string().contains("SECRET_ECHO"));
        server.join().unwrap();
    }
}

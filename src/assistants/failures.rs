//! Fixed diagnostic codes only: never expose provider response bodies or URLs.
pub fn code(error: &anyhow::Error) -> &'static str {
    if let Some(api) = error.downcast_ref::<crate::providers::ApiFailure>() {
        return match api.status.as_u16() {
            401 | 403 => "provider_auth",
            402 => "provider_billing",
            404 => "provider_model",
            429 => "provider_rate_limit",
            400 | 422 => "provider_request",
            _ => "provider_unavailable",
        };
    }
    if error.downcast_ref::<reqwest::Error>().is_some() {
        return "provider_connection";
    }
    "chat_failed"
}

#[cfg(test)]
mod tests {
    #[test]
    fn arbitrary_error_contents_are_never_returned() {
        let error = anyhow::anyhow!("private prompt and credential-like data");
        assert_eq!(super::code(&error), "chat_failed");
    }
}

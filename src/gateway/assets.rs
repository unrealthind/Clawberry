//! Same-origin production frontend. Never falls back to private runtime files.
use super::AppState;
use axum::{
    extract::State,
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use std::path::{Component, Path};
use tokio::io::AsyncReadExt;

pub(super) async fn serve(State(state): State<AppState>, uri: Uri) -> Response {
    let Some(root) = state
        .assistant_config
        .as_ref()
        .and_then(|c| c.gateway.app_dir.as_deref())
    else {
        return StatusCode::NOT_FOUND.into_response();
    };
    match load(root, uri.path()).await {
        Ok((bytes, mime)) => {
            let mut response = (StatusCode::OK, bytes).into_response();
            let headers = response.headers_mut();
            headers.insert(header::CONTENT_TYPE, mime.parse().expect("static MIME"));
            headers.insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
            headers.insert(header::X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap());
            headers.insert(header::REFERRER_POLICY, "no-referrer".parse().unwrap());
            headers.insert(header::CONTENT_SECURITY_POLICY,"default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' blob: data:; connect-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'".parse().unwrap());
            response
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}
async fn load(root: &Path, url: &str) -> anyhow::Result<(Vec<u8>, &'static str)> {
    let relative = url
        .strip_prefix('/')
        .ok_or_else(|| anyhow::anyhow!("Invalid asset path"))?;
    let relative = if relative.is_empty() {
        "index.html"
    } else {
        relative
    };
    if relative.contains(['%', '\\'])
        || !Path::new(relative)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
    {
        anyhow::bail!("Invalid asset path");
    }
    let mime = match Path::new(relative).extension().and_then(|s| s.to_str()) {
        Some("html") if matches!(relative, "index.html" | "prototype.html") => {
            "text/html; charset=utf-8"
        }
        Some("js") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        Some("webmanifest") => "application/manifest+json",
        _ => anyhow::bail!("Unsupported asset"),
    };
    if !root.is_absolute() {
        anyhow::bail!("Asset root must be absolute");
    }
    let root = tokio::fs::canonicalize(root).await?;
    // Resolve symlinks before opening, then enforce containment. The static
    // directory is administrator-managed and never mounted in agent sandboxes.
    let path = tokio::fs::canonicalize(root.join(relative)).await?;
    if !path.starts_with(&root) {
        anyhow::bail!("Asset outside static directory");
    }
    let file = tokio::fs::File::open(path).await?;
    if !file.metadata().await?.is_file() {
        anyhow::bail!("Not an asset file");
    }
    let mut bytes = Vec::new();
    file.take(5_000_001).read_to_end(&mut bytes).await?;
    if bytes.len() > 5_000_000 {
        anyhow::bail!("Asset exceeds limit");
    }
    Ok((bytes, mime))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn static_root_has_no_traversal_api_fallback_or_secret_file_access() {
        let root = tempfile::TempDir::new().unwrap();
        std::fs::write(root.path().join("index.html"), "<html>App</html>").unwrap();
        std::fs::write(root.path().join("config.toml"), "secret").unwrap();
        assert_eq!(
            load(root.path(), "/").await.unwrap().1,
            "text/html; charset=utf-8"
        );
        for path in [
            "/../index.html",
            "/%2e%2e/index.html",
            "/config.toml",
            "/api/unknown",
            "/missing.js",
            "//index.html",
        ] {
            assert!(load(root.path(), path).await.is_err(), "{path}");
        }
        #[cfg(unix)]
        {
            let other = tempfile::NamedTempFile::new().unwrap();
            std::os::unix::fs::symlink(other.path(), root.path().join("escape.js")).unwrap();
            assert!(load(root.path(), "/escape.js").await.is_err());
        }
    }
}

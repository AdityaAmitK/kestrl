//! Shares for the app: list them and close them. Opening a share needs a long-lived owner
//! process, so shares are opened with `kestrl share open`.

use kestrl_core::share::{self, Access, ShareState};

use crate::error::KestrlError;

/// One open share.
#[derive(Debug, uniffi::Record)]
pub struct ShareInfo {
    pub id: String,
    pub port: u16,
    /// The URL, once the tunnel is ready
    pub url: Option<String>,
    /// Anyone with the URL can reach it; otherwise Cloudflare Access protects it
    pub is_public: bool,
    pub is_ready: bool,
    pub hostname: Option<String>,
    pub expires_at_ms: i64,
}

/// Every open share, oldest first.
#[uniffi::export]
pub fn list_shares() -> Result<Vec<ShareInfo>, KestrlError> {
    Ok(share::list()?
        .into_iter()
        .map(|share| ShareInfo {
            id: share.id,
            port: share.port,
            url: share.url,
            is_public: share.access == Access::Public,
            is_ready: share.state == ShareState::Ready,
            hostname: share.hostname,
            expires_at_ms: share.expires_at,
        })
        .collect())
}

/// Close a share and wait until its tunnel is gone.
#[uniffi::export]
pub fn close_share(id: String) -> Result<(), KestrlError> {
    share::close(&id)?;
    Ok(())
}

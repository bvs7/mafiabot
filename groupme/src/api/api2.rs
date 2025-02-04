// Want a single handler?
use crate::util::UnexpectedJsonError;

use tokio::sync::Notify;

use std::sync::OnceLock;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Unexpected JSON error")]
    UnexpectedJsonError(#[from] UnexpectedJsonError),
    #[error("Serde JSON error")]
    SerdeJsonError(#[from] serde_json::Error),
    #[error("Reqwest error")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Token error")]
    TokenError(#[from] std::env::VarError),
    #[error("Unknown error {0}")]
    OtherError(String),
}

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
static NOTIFY: OnceLock<Notify> = OnceLock::new();

fn client() -> &'static reqwest::Client {
    &CLIENT.get_or_init(|| {
        tokio::spawn(run_notify());
        reqwest::Client::new()
    })
}
fn notify() -> &'static Notify {
    &NOTIFY.get_or_init(|| Notify::new())
}

#[tracing::instrument]
async fn run_notify() {
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(200));
    loop {
        interval.tick().await;
        notify().notify_one();
    }
}

async fn get(uri: &str, queries: &[(&str, &str)]) -> Result<String, Error> {
    match client().await.get(uri).query(&[("token", &self.token)]).query(queries).send().await? {
        resp if resp.status().is_success() => Ok(resp.text().await?),
        resp => {
            let status = resp.status();
            let text = resp.text().await?;
            Err(Error::OtherError(format!(
                "GET request to {uri} failed with status {status}: {text}"
            )))
        }
    }
}

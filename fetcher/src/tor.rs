use arti_client::TorClient;
use lazy_static::lazy_static;
use tor_config::Listen;
use tor_rtcompat::tokio::TokioRustlsRuntime;

lazy_static! {
    pub static ref SOCKS_PORT: u16 = portpicker::pick_unused_port().expect("no ports free");
}

pub async fn start_socks_proxy() -> anyhow::Result<()> {
    let tor_runtime = TokioRustlsRuntime::current()?;
    let tor_client = TorClient::with_runtime(tor_runtime.clone()).create_bootstrapped().await?;
    let listen = Listen::new_localhost(*SOCKS_PORT);
    arti::socks::run_socks_proxy(tor_runtime, tor_client, listen).await
}

use arti_client::TorClient;
use lazy_static::lazy_static;
use tor_rtcompat::PreferredRuntime;

lazy_static! {
    pub static ref SOCKS_PORT: u16 = portpicker::pick_unused_port().expect("no ports free");
}

pub async fn start_socks_proxy() -> anyhow::Result<()> {
    let tor_runtime = PreferredRuntime::current()?;
    let tor_client = TorClient::with_runtime(tor_runtime.clone())
        .create_bootstrapped()
        .await?;
    arti::socks::run_socks_proxy(tor_runtime, tor_client, *SOCKS_PORT).await
}

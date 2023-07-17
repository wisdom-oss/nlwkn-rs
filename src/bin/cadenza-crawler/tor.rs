use arti_client::TorClient;
use tor_rtcompat::PreferredRuntime;

pub async fn start_socks_proxy() -> anyhow::Result<()> {
    let tor_runtime = PreferredRuntime::current().expect("in tokio");
    let tor_client = TorClient::with_runtime(tor_runtime.clone()).create_bootstrapped().await.expect("tor client is necessary");
    arti::socks::run_socks_proxy(tor_runtime, tor_client, 9150).await
}

mod tor;
mod xlsx;

#[tokio::main]
async fn main() {
    let out = tokio::select! {
        _ = tor::start_socks_proxy() => 1,
        _ = start_crawling() => 0
    };

    std::process::exit(out);
}

async fn start_crawling() {

}

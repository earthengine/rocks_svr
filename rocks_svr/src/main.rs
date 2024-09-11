use rocks_lib::run_vless_over_tcp;
use tokio::select;
use tracing::info;
use warp::Filter;

async fn wrap() -> Result<(), Box<dyn std::error::Error>> {
    let addr = warp::any().map(|| "Hello, World!");
    warp::serve(addr).run(([127, 0, 0, 1], 8888)).await;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    select!(
        r = run_vless_over_tcp() => {
            info!("test_vless finished: {:?}", r);
        },
        r = wrap() => {
            info!("wrap finished: {:?}", r);
        },
        _ = tokio::signal::ctrl_c() => info!("Ctrl-C received")
    );

    Ok(())
}

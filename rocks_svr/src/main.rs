use rocks_lib::{run_vless_over_tcp, run_vless_over_tungstenite_ws};
use tokio::select;
use tracing::info;
use warp::Filter;

async fn wrap() -> Result<(), Box<dyn std::error::Error>> {
    let root = warp::path::end().and(warp::fs::dir("public"));

    warp::serve(root).run(([127, 0, 0, 1], 8888)).await;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    select!(
        r = run_vless_over_tcp() => {
            info!("test_vless finished: {:?}", r);
        },
        r = run_vless_over_tungstenite_ws() => {
            info!("test_vless finished: {:?}", r);
        },
        r = wrap() => {
            info!("wrap finished: {:?}", r);
        },
        _ = tokio::signal::ctrl_c() => info!("Ctrl-C received")
    );

    Ok(())
}

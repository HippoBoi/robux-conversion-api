mod rates;

use serde::Serialize;
use worker::*;

#[derive(Serialize)]
struct RateResponse {
    rate: f64,
    pair: &'static str,
    basis: &'static str,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

fn rate_payload() -> RateResponse {
    RateResponse {
        rate: rates::USD_PER_RBX,
        pair: "RBX/USD",
        basis: "devex-standard",
    }
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    Router::new()
        .get("/rate", |_, _| Response::from_json(&rate_payload()))
        .get("/health", |_, _| {
            Response::from_json(&HealthResponse { status: "ok" })
        })
        .run(req, env)
        .await
}

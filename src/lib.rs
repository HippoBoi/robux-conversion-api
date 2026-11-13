mod rates;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use worker::*;

#[derive(Deserialize, Serialize)]
struct ExternalRatesResponse {
    result: String,
    time_last_update: String,
    rates: HashMap<String, f64>,
}

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

async fn sync_fiat_rates(env: &Env) -> Result<ExternalRatesResponse> {
    let url = "https://open.er-api.com/v6/latest/USD";
    let req = Request::new(url, Method::Get)?;
    let mut res = Fetch::Request(req).send().await?;

    if res.status_code() != 200 {
        return Err(Error::RustError("failed to fetch external api".into()));
    }

    let data: ExternalRatesResponse = res.json().await?;
    let json_string = serde_json::to_string(&data)?;

    let kv = env.kv("FX_RATES")?;
    kv.put("usd_rates", json_string)?.execute().await?;

    Ok(data)
}

#[event(scheduled)]
pub async fn scheduled(_event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
   if let Err(e) = sync_fiat_rates(&env).await {
        console_error!("cron error: {:?}", e);
   } else {
        console_log!("cron success");
   }
}

fn rate_payload() -> RateResponse {
    RateResponse {
        rate: rates::USD_PER_RBX,
        pair: "RBX/USD",
        basis: "devex-standard",
    }
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    Router::new()
        .get_async("/v1/rates", |_req, ctx| async move {
            let kv = ctx.env.kv("FX_RATES")?;

            let rates_data: ExternalRatesResponse = match kv.get("usd_rates").text().await? {
                Some(cached_json) => serde_json::from_str(&cached_json)?,
                None => sync_fiat_rates(&ctx.env).await?,
            };

            let supported = ["USD", "CLP", "ARS", "MXN", "BRL", "EUR", "GBP", "COP", "CAD", "AUD"];
            let filtered_rates: HashMap<String, f64> = rates_data
                .rates
                .into_iter()
                .filter(|(k, _)| supported.contains(&k.as_str()))
                .collect();

            Response::from_json(&serde_json::json!({
                "base": "USD",
                "robux_rates": {
                    "web_purchase_usd": 0.0125,
                    "mobile_purchase_usd": 0.0142,
                    "devex_payout_usd": 0.0035
                },
                "fiat_rates": filtered_rates,
                "updated_at": rates_data.time_last_update
            }))
        })
        .get("/rate", |_, _| Response::from_json(&rate_payload()))
        .get("/health", |_, _| {
            Response::from_json(&HealthResponse { status: "ok" })
        })
        .run(req, env)
        .await
}

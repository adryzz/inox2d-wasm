use std::fmt::format;
use std::io::Read;

use bytes::Buf;
use log::{info, debug};

fn main() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Info));
    console_error_panic_hook::set_once();

    wasm_bindgen_futures::spawn_local(runwrap())
}

async fn runwrap() {
    match run().await {
        Ok(_) => info!("app shutdown"),
        Err(e) => log::error!("error: {}", e)
    }
}

async fn run() -> anyhow::Result<()> {
    info!("loading puppet");
    let res = reqwest::Client::new()
    .get(format!("{}/assets/puppet.inp", base_url()))
    .send()
    .await?;

    let model = inox2d::formats::inp::parse_inp(res.bytes().await?.reader())?;
    info!("== Puppet Meta ==\n{}", &model.puppet.meta);
    //debug!("== Nodes ==\n{}", &model.puppet.nodes);
    if model.vendors.is_empty() {
        info!("(No Vendor Data)\n");
    } else {
        info!("== Vendor Data ==");
        for vendor in &model.vendors {
            debug!("{vendor}");
        }
    }
    Ok(())
}

pub fn base_url() -> String {
    web_sys::window().unwrap().location().origin().unwrap()
}
use crate::tor;

use headless_chrome::{Browser, LaunchOptionsBuilder};

use std::ffi::OsString;
use std::ops::Deref;


use std::time::Duration;

pub static CADENZA_URL: &str = crate::CONFIG.cadenza.url;
pub static CADENZA_TIMEOUT: Duration = Duration::from_millis(crate::CONFIG.cadenza.timeout as u64);

#[cfg(not(feature = "head"))]
const HEADLESS: bool = true;

#[cfg(feature = "head")]
const HEADLESS: bool = false;

#[cfg(not(feature = "no-sandbox"))]
const NO_SANDBOX: bool = false;

#[cfg(feature = "no-sandbox")]
const NO_SANDBOX: bool = true;

pub fn fetch_water_right_report(water_right_no: u64) -> anyhow::Result<String> {
    let proxy = OsString::from(format!(
        "--proxy-server=socks5://localhost:{}",
        tor::SOCKS_PORT.deref()
    ));
    let launch_options = LaunchOptionsBuilder::default()
        .headless(HEADLESS)
        .sandbox(!NO_SANDBOX)
        .args(vec![&proxy])
        .build()?;
    let browser = Browser::new(launch_options)?;
    let tab = browser.new_tab()?;
    tab.set_default_timeout(CADENZA_TIMEOUT);

    // println!("set timeout");

    tab.navigate_to(CADENZA_URL)?;
    let map_button_attributes = tab
        .wait_for_element("a.map[title='Karte']")?
        .get_attributes()?
        .ok_or(anyhow::Error::msg("map button has no attributes"))?;
    // dbg!(&map_button_attributes);
    let map_button_link = map_button_attributes
        .iter()
        .skip_while(|attr| attr.as_str() != "href").nth(1)
        .ok_or(anyhow::Error::msg("map button has no link"))?;
    // dbg!(&map_button_link);
    let session_id = map_button_link
        .split(";jsessionid=")
        .nth(1)
        .ok_or(anyhow::Error::msg("no session id found"))?;

    let report_command = format!("{CADENZA_URL}commands.xhtml;jsessionid={session_id}?ShowLegacy.RepositoryItem.Id=FIS-W.WBE.wbe/wbe_net_wasserrecht.cwf&ShowLegacy.RepositoryItem.Value='{water_right_no}'&ShowLegacy.RepositoryItem.Attribute=wbe_net_wasserrecht.wasserrecht_nr");
    // dbg!(&report_command);
    tab.navigate_to(&report_command)?;

    let report_anchor = tab.wait_for_element("div.downloadReportInfoMessage a")?;
    let report_anchor_attributes = report_anchor
        .get_attributes()?
        .ok_or(anyhow::Error::msg("report has no link"))?;
    let report_link = report_anchor_attributes
        .iter()
        .skip_while(|attr| attr.as_str() != "href").nth(1)
        .ok_or(anyhow::Error::msg("report has no href"))?;

    Ok(report_link.to_string())
}

use lazy_static::lazy_static;
use nlwkn_rs::WaterRightNo;
use regex::Regex;

static CADENZA_ROOT: &str = crate::CONFIG.cadenza.root;
static CADENZA_URL: &str = crate::CONFIG.cadenza.url;
const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/115.0";

lazy_static! {
    static ref REPORT_URL_RE: Regex =
        Regex::new(r"\?file=rep(?<report_id>\d+)\.pdf").expect("valid regex");
}

pub async fn fetch_report_url(
    water_right_no: WaterRightNo,
    client: &reqwest::Client
) -> anyhow::Result<String> {
    let command_url = format!(
        "{CADENZA_URL}commands.xhtml?ShowLegacy.RepositoryItem.Id=FIS-W.WBE.wbe/\
         wbe_net_wasserrecht.cwf&ShowLegacy.RepositoryItem.Value='{water_right_no}'&ShowLegacy.\
         RepositoryItem.Attribute=wbe_net_wasserrecht.wasserrecht_nr"
    );
    let command_res = client.get(command_url).header("User-Agent", USER_AGENT).send().await?;
    match command_res.status().as_u16() {
        302 => (),
        code => {
            return Err(anyhow::Error::msg(format!(
                "command responded with {code}, expected 302"
            )))
        }
    }

    let wait_xhtml_url = command_res
        .headers()
        .get("Location")
        .ok_or(anyhow::Error::msg("command res has no 'Location' header"))?;
    let wait_xhtml_url = wait_xhtml_url.to_str()?;
    let j_session_id = wait_xhtml_url.split(";jsessionid=").nth(1).ok_or(anyhow::Error::msg(
        "command res has no session id in 'Location' header"
    ))?;

    let wait_cweb_url = format!("{CADENZA_URL}wait.cweb;jsessionid={j_session_id}");
    let wait_cweb_res = client.get(wait_cweb_url).header("User-Agent", USER_AGENT).send().await?;
    match wait_cweb_res.status().as_u16() {
        302 => (),
        code => {
            return Err(anyhow::Error::msg(format!(
                "wait cweb responded with {code}, expected 302"
            )))
        }
    }

    let finished_url = wait_cweb_res
        .headers()
        .get("Location")
        .ok_or(anyhow::Error::msg("wait cweb has no 'Location' header"))?;
    let finished_url = format!("{CADENZA_ROOT}{}", finished_url.to_str()?);
    let finished_res = client.get(finished_url).header("User-Agent", USER_AGENT).send().await?;
    match finished_res.status().as_u16() {
        302 => (),
        code => {
            return Err(anyhow::Error::msg(format!(
                "finish responded with {code}, expected 302"
            )))
        }
    }

    let download_url = finished_res
        .headers()
        .get("Location")
        .ok_or(anyhow::Error::msg("finish res has 'Location' header"))?;
    let download_url = download_url.to_str()?;

    let captured = REPORT_URL_RE.captures(download_url).ok_or(anyhow::Error::msg(
        "download url does not contain report file id"
    ))?;
    let report_id = &captured["report_id"];
    let report_url = format!(
        "{CADENZA_URL}/pages/download/get;jsessionid={j_session_id}?file=rep{report_id}.pdf&\
         mimetype=application/pdf"
    );
    Ok(report_url)
}

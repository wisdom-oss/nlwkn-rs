use lazy_static::lazy_static;
use nlwkn::WaterRightNo;
use regex::Regex;
use reqwest::header::ToStrError;
use thiserror::Error;

static CADENZA_ROOT: &str = crate::CONFIG.cadenza.root;
static CADENZA_URL: &str = crate::CONFIG.cadenza.url;
const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/115.0";

lazy_static! {
    static ref REPORT_URL_RE: Regex =
        Regex::new(r"\?file=rep(?<report_id>\d+)\.pdf").expect("valid regex");
}

#[derive(Debug, Error)]
pub enum FetchReportUrlError {
    #[error("command responded with {0}, expected 302")]
    CommandInvalidCode(u16),

    #[error("command response has not 'Location' header")]
    CommandNoLocation,

    #[error("command response has no session id in 'Location' header")]
    CommandNoSessionId,

    #[error(transparent)]
    HeaderToStr(#[from] ToStrError),

    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    #[error("wait cweb responded with {0}, expected 302")]
    WaitCwebInvalidCode(u16),

    #[error("wait cweb response has no 'Location' header")]
    WaitCwebNoLocation,

    #[error("finish response has not 'Location' header")]
    FinishNoLocation,

    #[error("cadenza has no results for this request")]
    NoResults,

    #[error("download url does not contain report file id")]
    NoReportFileId
}

pub async fn fetch_report_url(
    water_right_no: WaterRightNo,
    client: &reqwest::Client
) -> Result<String, FetchReportUrlError> {
    let command_url = format!(
        "{CADENZA_URL}commands.xhtml?ShowLegacy.RepositoryItem.Id=FIS-W.WBE.wbe/\
         wbe_net_wasserrecht.cwf&ShowLegacy.RepositoryItem.Value='{water_right_no}'&ShowLegacy.\
         RepositoryItem.Attribute=wbe_net_wasserrecht.wasserrecht_nr"
    );
    let command_res = client.get(command_url).header("User-Agent", USER_AGENT).send().await?;
    match command_res.status().as_u16() {
        302 => (),
        code => return Err(FetchReportUrlError::CommandInvalidCode(code))
    }

    let wait_xhtml_url =
        command_res.headers().get("Location").ok_or(FetchReportUrlError::CommandNoLocation)?;
    let wait_xhtml_url = wait_xhtml_url.to_str()?;
    let j_session_id = wait_xhtml_url
        .split(";jsessionid=")
        .nth(1)
        .ok_or(FetchReportUrlError::CommandNoSessionId)?;

    let wait_cweb_url = format!("{CADENZA_URL}wait.cweb;jsessionid={j_session_id}");
    let wait_cweb_res = client.get(wait_cweb_url).header("User-Agent", USER_AGENT).send().await?;
    match wait_cweb_res.status().as_u16() {
        302 => (),
        code => return Err(FetchReportUrlError::WaitCwebInvalidCode(code))
    }

    let finished_url =
        wait_cweb_res.headers().get("Location").ok_or(FetchReportUrlError::WaitCwebNoLocation)?;
    let finished_url = format!("{CADENZA_ROOT}{}", finished_url.to_str()?);
    let finished_res = client.get(&finished_url).header("User-Agent", USER_AGENT).send().await?;
    let download_url = match finished_res.headers().get("Location") {
        Some(location) => location.to_str()?,
        None => {
            return match finished_res.text().await {
                Ok(body) if body.contains("Die Abfrage liefert keine Ergebnisse.") => {
                    Err(FetchReportUrlError::NoResults)
                }
                _ => Err(FetchReportUrlError::FinishNoLocation)
            }
        }
    };

    let captured =
        REPORT_URL_RE.captures(download_url).ok_or(FetchReportUrlError::NoReportFileId)?;
    let report_id = &captured["report_id"];
    let report_url = format!(
        "{CADENZA_URL}/pages/download/get;jsessionid={j_session_id}?file=rep{report_id}.pdf&\
         mimetype=application/pdf"
    );
    Ok(report_url)
}

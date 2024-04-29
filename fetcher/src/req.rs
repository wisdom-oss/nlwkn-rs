use std::ffi::OsString;

use lazy_static::lazy_static;
use nlwkn::WaterRightNo;
use regex::Regex;
use reqwest::header::ToStrError;
use reqwest::RequestBuilder;
use thiserror::Error;

static CADENZA_ROOT: &str = crate::CONFIG.cadenza.root;
static CADENZA_URL: &str = crate::CONFIG.cadenza.url;
const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/115.0";

lazy_static! {
    static ref REPORT_URL_RE: Regex =
        Regex::new(r"\?file=rep(?<report_id>\d+)\.pdf").expect("valid regex");
    static ref SESSION_ID_DISY_RE: Regex =
        Regex::new(r#"jsessionId : "(?<jsession_id>\S+)""#).expect("valid regex");
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

#[derive(Debug, Error)]
pub enum FetchCadenzaTableError {
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error)
}

// TODO: add more error types for proper notifications
pub async fn fetch_cadenza_table(
    client: &reqwest::Client
) -> Result<(OsString, impl AsRef<[u8]>), FetchCadenzaTableError> {
    let homepage = client.get(CADENZA_URL).header("User-Agent", USER_AGENT).send().await?;
    let homepage = homepage.text().await.unwrap();
    let jsession_id = SESSION_ID_DISY_RE.captures(&homepage).unwrap();
    let jsession_id = jsession_id.name("jsession_id").expect("part of regex");
    let jsession_id = jsession_id.as_str();

    let servlet_res = client
        .get(format!(
            "{CADENZA_URL}servlet/CadenzaServlet;jsessionid={jsession_id}"
        ))
        .header("User-Agent", USER_AGENT)
        .query(&[
            (
                "repositoryId",
                "FIS-W.Wasserrechte.wbe/wbe_tab_nutzungsort.sel"
            ),
            ("pid", "FIS-W.Wasserrechte")
        ])
        .send()
        .await?;
    let wait_xhtml_location = servlet_res.headers().get("Location").unwrap().to_str().unwrap();
    let wait_xhtml_location = format!("{CADENZA_ROOT}{wait_xhtml_location}");
    // client.get(&wait_xhtml_location).header("User-Agent",
    // USER_AGENT).send().await?;
    let wait_cweb_location = format!("{CADENZA_URL}wait.cweb;jsessionid={jsession_id}");

    let wait_cweb_res =
        client.get(wait_cweb_location).header("User-Agent", USER_AGENT).send().await?;
    let finished_location = wait_cweb_res.headers().get("Location").unwrap().to_str().unwrap();
    let finished_location = format!("{CADENZA_ROOT}{finished_location}");

    let finished_res =
        client.get(finished_location).header("User-Agent", USER_AGENT).send().await?;
    let table_view_location = finished_res.headers().get("Location").unwrap().to_str().unwrap();
    let table_view_location = format!("{CADENZA_ROOT}{table_view_location}");

    let table_view_res =
        client.get(table_view_location).header("User-Agent", USER_AGENT).send().await?;
    dbg!((&table_view_res, table_view_res.headers()));

    let table_url = format!(
        "{CADENZA_URL}actions/tableResult/displayExcelTableResult.xhtml;jsessionid={jsession_id}"
    );
    let table_res = client.get(table_url).header("User-Agent", USER_AGENT).send().await?;
    let content_disposition =
        table_res.headers().get("content-disposition").unwrap().to_str().unwrap();
    let filename = content_disposition.strip_prefix("attachment; filename=").unwrap().into();
    let bytes = table_res.bytes().await.unwrap();

    Ok((filename, bytes))
}

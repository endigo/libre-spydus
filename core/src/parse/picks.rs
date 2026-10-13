//! The reader's Pick list ("My Picks", new; no TS counterpart). The list page itself is a results
//! page (`parse_search_results`); this reads the website's save/remove reply (`spydus.js`
//! `onClickSVL`), an XML document such as
//! `<AJAXRES><XSVL COUNT="2"><COOKIE NAME="SVLID" VALUE="…"/></XSVL></AJAXRES>`.

use std::sync::LazyLock;

use regex::Regex;

use crate::error::SpydusError;
use crate::parse::decode::{collapse, decode};

static COUNT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"<XSVL\b[^>]*\bCOUNT="([0-9]+)""#).unwrap());
static REDIRECT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"<REDIRECT\b[^>]*\bURL="([^"]+)""#).unwrap());
/// "None found": the library's error for a record it couldn't find (`<p class="xerror">`).
static ERROR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?s)class="xerror"[^>]*>(.*?)</"#).unwrap());

/// A save/remove reply → how many records the Pick list now holds. A reply sending the website
/// elsewhere (`<REDIRECT URL>`), the library's error page, or anything else is an error.
pub fn parse_pick_reply(xml: &str) -> Result<i64, SpydusError> {
    if let Some(url) = REDIRECT.captures(xml) {
        return Err(SpydusError::Upstream { message: format!("the library sent the Pick list to {}", decode(&url[1])) });
    }
    if let Some(count) = COUNT.captures(xml) {
        return count[1].parse().map_err(|_| SpydusError::Parse { message: "Pick list count".into() });
    }
    if let Some(error) = ERROR.captures(xml) {
        return Err(SpydusError::Upstream { message: collapse(&decode(&error[1])) });
    }
    Err(SpydusError::Parse { message: "unrecognised Pick list reply".into() })
}

//! The handful of plain HTTP calls the editor makes to hexend (the
//! directory browser, project creation, image upload) — everything
//! about a project's live state goes over the /ws session instead.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, Response};

/// Where hexend is: the desktop app's own server when running inside
/// it; else a `?server=` in this page's URL (how `hexen launch` points
/// the editor at a hexend on another port); else baked in at build time
/// (`HEXEND_URL=... trunk build`).
pub fn server_url() -> String {
    crate::desktop::desktop_info()
        .map(|info| info.server_url)
        .or_else(|| query_param("server"))
        .unwrap_or_else(|| {
            option_env!("HEXEND_URL")
                .unwrap_or("http://localhost:4000")
                .to_string()
        })
}

/// A non-empty query parameter from this page's URL.
pub fn query_param(name: &str) -> Option<String> {
    let search = web_sys::window()?.location().search().ok()?;
    web_sys::UrlSearchParams::new_with_str(&search)
        .ok()?
        .get(name)
        .filter(|v| !v.is_empty())
}

pub fn encode(s: &str) -> String {
    js_sys::encode_uri_component(s).into()
}

/// Sends a request and returns the body text of a 2xx response; any
/// other status comes back as `Err` with the body (hexend's error text).
pub async fn fetch_text(
    method: &str,
    url: &str,
    body: Option<&JsValue>,
    json: bool,
) -> Result<String, String> {
    let init = RequestInit::new();
    init.set_method(method);
    if let Some(body) = body {
        init.set_body(body);
    }
    let request = Request::new_with_str_and_init(url, &init).map_err(describe)?;
    if json {
        request
            .headers()
            .set("Content-Type", "application/json")
            .map_err(describe)?;
    }
    let window = web_sys::window().ok_or("no window")?;
    let response: Response = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|_| format!("Couldn't reach {}", server_url()))?
        .unchecked_into();
    let text = JsFuture::from(response.text().map_err(describe)?)
        .await
        .map_err(describe)?
        .as_string()
        .unwrap_or_default();
    if response.ok() {
        Ok(text)
    } else {
        Err(text)
    }
}

/// A JS exception as a message a person can read.
pub fn describe(err: JsValue) -> String {
    if let Some(e) = err.dyn_ref::<js_sys::Error>() {
        return e.message().into();
    }
    if let Some(e) = err.dyn_ref::<web_sys::DomException>() {
        return e.message();
    }
    err.as_string().unwrap_or_else(|| format!("{err:?}"))
}

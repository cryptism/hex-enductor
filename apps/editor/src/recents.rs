//! Recently opened projects. Server projects are keyed by path and kept
//! in localStorage; browser-folder projects are directory handles,
//! which only IndexedDB can hold (Chromium structured-clones them), so
//! they live there instead. Both are conveniences: any storage failure
//! just means an empty list.

use js_sys::{Array, Object, Reflect};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{FileSystemDirectoryHandle, IdbDatabase, IdbRequest, IdbTransactionMode};

const STORAGE_KEY: &str = "hex-enductor:recent-projects";
const MAX_RECENT: usize = 8;

/// `path` moved (or added) to the front of `recent`, capped.
pub fn with_recent(recent: &[String], path: &str) -> Vec<String> {
    std::iter::once(path.to_string())
        .chain(recent.iter().filter(|p| *p != path).cloned())
        .take(MAX_RECENT)
        .collect()
}

fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

pub fn recent_projects() -> Vec<String> {
    local_storage()
        .and_then(|s| s.get_item(STORAGE_KEY).ok().flatten())
        .and_then(|raw| serde_json::from_str::<Vec<serde_json::Value>>(&raw).ok())
        .map(|items| {
            items
                .into_iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

pub fn add_recent_project(path: &str) {
    let updated = with_recent(&recent_projects(), path);
    if let (Some(storage), Ok(json)) = (local_storage(), serde_json::to_string(&updated)) {
        let _ = storage.set_item(STORAGE_KEY, &json);
    }
}

const DB_NAME: &str = "hex-enductor-local-recents";
const STORE: &str = "handles";
const MAX_LOCAL: usize = 5;

#[derive(Clone)]
pub struct LocalRecent {
    pub name: String,
    pub handle: FileSystemDirectoryHandle,
    pub opened_at: f64,
}

/// An IDBRequest's result, once it has one.
async fn settle(request: &IdbRequest) -> Result<JsValue, JsValue> {
    let promise = js_sys::Promise::new(&mut |resolve, reject| {
        let ok = request.clone();
        let on_success = Closure::once_into_js(move || {
            resolve.call1(&JsValue::NULL, &ok.result().unwrap_or_default())
        });
        let on_error = Closure::once_into_js(move || reject.call0(&JsValue::NULL));
        request.set_onsuccess(Some(on_success.unchecked_ref()));
        request.set_onerror(Some(on_error.unchecked_ref()));
    });
    wasm_bindgen_futures::JsFuture::from(promise).await
}

async fn open_db() -> Result<IdbDatabase, JsValue> {
    let factory = web_sys::window()
        .and_then(|w| w.indexed_db().ok().flatten())
        .ok_or(JsValue::NULL)?;
    let request = factory.open_with_u32(DB_NAME, 1)?;
    let on_upgrade = Closure::once_into_js({
        let request = request.clone();
        move || {
            if let Ok(db) = request.result().and_then(|r| r.dyn_into::<IdbDatabase>()) {
                let params = web_sys::IdbObjectStoreParameters::new();
                params.set_key_path(&"name".into());
                let _ = db.create_object_store_with_optional_parameters(STORE, &params);
            }
        }
    });
    request.set_onupgradeneeded(Some(on_upgrade.unchecked_ref()));
    Ok(settle(&request).await?.unchecked_into())
}

async fn all(db: &IdbDatabase) -> Result<Vec<LocalRecent>, JsValue> {
    let store = db.transaction_with_str(STORE)?.object_store(STORE)?;
    let rows: Array = settle(&store.get_all()?).await?.unchecked_into();
    let mut entries: Vec<LocalRecent> = rows
        .iter()
        .filter_map(|row| {
            Some(LocalRecent {
                name: Reflect::get(&row, &"name".into()).ok()?.as_string()?,
                handle: Reflect::get(&row, &"handle".into()).ok()?.dyn_into().ok()?,
                opened_at: Reflect::get(&row, &"openedAt".into())
                    .ok()?
                    .as_f64()
                    .unwrap_or(0.0),
            })
        })
        .collect();
    entries.sort_by(|a, b| b.opened_at.total_cmp(&a.opened_at));
    Ok(entries)
}

pub async fn local_recents() -> Vec<LocalRecent> {
    match open_db().await {
        Ok(db) => all(&db).await.unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

pub async fn add_local_recent(handle: &FileSystemDirectoryHandle) -> Result<(), JsValue> {
    let db = open_db().await?;
    let row = Object::new();
    Reflect::set(&row, &"name".into(), &handle.name().into())?;
    Reflect::set(&row, &"handle".into(), handle)?;
    Reflect::set(&row, &"openedAt".into(), &js_sys::Date::now().into())?;
    let store = db
        .transaction_with_str_and_mode(STORE, IdbTransactionMode::Readwrite)?
        .object_store(STORE)?;
    settle(&store.put(&row)?).await?;

    let excess: Vec<_> = all(&db).await?.into_iter().skip(MAX_LOCAL).collect();
    if !excess.is_empty() {
        let store = db
            .transaction_with_str_and_mode(STORE, IdbTransactionMode::Readwrite)?
            .object_store(STORE)?;
        for entry in excess {
            let _ = store.delete(&entry.name.into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn adds_to_the_front() {
        let recent = with_recent(&with_recent(&[], "/a.hexen.yml"), "/b.hexen.yml");
        assert_eq!(recent, v(&["/b.hexen.yml", "/a.hexen.yml"]));
    }

    #[test]
    fn re_adding_moves_to_the_front_instead_of_duplicating() {
        let recent = with_recent(&v(&["/b.hexen.yml", "/a.hexen.yml"]), "/a.hexen.yml");
        assert_eq!(recent, v(&["/a.hexen.yml", "/b.hexen.yml"]));
    }

    #[test]
    fn caps_at_eight() {
        let recent = (0..10).fold(Vec::new(), |r, i| {
            with_recent(&r, &format!("/p{i}.hexen.yml"))
        });
        assert_eq!(recent.len(), 8);
        assert_eq!(recent[0], "/p9.hexen.yml");
    }
}

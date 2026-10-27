//! Production compiled-in adapter. Sibling v2_sdk and CoreContext are the beta host's APIs.
use super::v2_sdk as sdk;
use crate::native_core::common::{CoreContext, CoreResult};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Mutex, OnceLock,
};
use tezbar_uuid_generator_v2::{
    history, set_history, Engine, OsSource, Output, COMMANDS, MAX_TEXT,
};

pub const SLUG: &str = "uuid-generator";
pub const ID: &str = "com.tezbar.native.uuid-generator";
static PROCESS: OnceLock<String> = OnceLock::new();
static NONCE: AtomicU64 = AtomicU64::new(1);
static ENGINE: Mutex<Engine> = Mutex::new(Engine::new());
fn fresh() -> Result<String, String> {
    let process = if let Some(s) = PROCESS.get() {
        s.clone()
    } else {
        let mut b = [0; 16];
        getrandom::getrandom(&mut b).map_err(|_| "Session entropy unavailable")?;
        let n = tezbar_uuid_generator_v2::uuid_string(b);
        let _ = PROCESS.set(n);
        PROCESS
            .get()
            .ok_or("Session initialization failed")?
            .clone()
    };
    NONCE
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| v.checked_add(1))
        .map(|n| format!("uuid-v2-{process}-{n}"))
        .map_err(|_| "Session ID capacity exhausted; restart the application".into())
}
pub fn commands() -> &'static [&'static str] {
    COMMANDS
}
fn fail(s: &str) -> String {
    s.into()
}
fn node(kind: &str, props: Value, children: Vec<Value>) -> Value {
    json!({"type":kind,"props":props,"children":children})
}
fn action(
    state: &mut Value,
    kind: &str,
    data: Value,
    title: &str,
    submit: bool,
) -> Result<Value, String> {
    let id = fresh()?;
    state["caps"][&id] = json!({"kind":kind,"data":data});
    Ok(json!({"id":id,"title":title,"kind":if submit{"submit-form"}else{"action"}}))
}
fn fields(command: &str) -> &'static [(&'static str, &'static str)] {
    match command {
        "generate" | "generateV7" => &[("numberOfUUIDsToGenerate", "Number of UUIDs")],
        "generateUlid" => &[("numberOfULIDsToGenerate", "Number of ULIDs")],
        "generateV5" => &[
            ("name", "UUID Name"),
            ("numberOfUUIDsToGenerate", "Number of UUIDs"),
        ],
        "generateTypeID" => &[
            ("prefix", "Prefix"),
            ("numberOfUUIDsToGenerate", "Number of UUIDs"),
        ],
        "parseTypeID" => &[("typeID", "TypeID")],
        "formatUuid" => &[("uuid", "UUID")],
        "packTypeID" => &[("suffix", "Suffix"), ("uuid", "UUID")],
        _ => &[],
    }
}
fn required(command: &str) -> &'static [&'static str] {
    match command {
        "generateV5" => &["name"],
        "generateTypeID" => &["prefix"],
        "parseTypeID" => &["typeID"],
        "formatUuid" => &["uuid"],
        "packTypeID" => &["suffix", "uuid"],
        _ => &[],
    }
}
fn title(command: &str) -> &'static str {
    match command {
        "generate" => "Generate UUIDs",
        "generateV5" => "Generate V5 UUIDs",
        "generateV7" => "Generate V7 UUIDs",
        "generateUlid" => "Generate ULIDs",
        "generateTypeID" => "Generate TypeIDs",
        "parseTypeID" => "Parse TypeIDs",
        "formatUuid" => "Format UUID",
        "packTypeID" => "Pack TypeID",
        _ => "View History",
    }
}
fn form(command: &str, state: &mut Value) -> CoreResult {
    // Field and submit capabilities persist while this form is visible, including queued edits.
    let same = state["screen"] == "form" && state["formActions"].is_array();
    if !same {
        state["caps"] = json!({});
        let mut actions = Vec::new();
        let mut ids = json!({});
        for (key, _) in fields(command) {
            let a = action(state, "field", json!(key), "Update field", false)?;
            ids[*key] = a["id"].clone();
        }
        actions.push(action(
            state,
            "submit",
            Value::Null,
            "Generate / Convert",
            true,
        )?);
        state["fieldIds"] = ids;
        state["formActions"] = json!(actions);
    }
    state["screen"] = json!("form");
    let children=fields(command).iter().map(|(key,label)|{
        let mut props=json!({"id":format!("{}:{key}",state["nonce"].as_str().unwrap_or("")),"title":label,"actionId":state["fieldIds"][*key]});
        // An absent server value preserves local queued edits in beta reconcileFormValues.
        if !same {props["value"]=json!(state["draft"][*key].as_str().unwrap_or(""));}
        node("Form.TextField",props,vec![])
    }).collect();
    Ok(sdk::view(
        title(command),
        node("Form", json!({}), children),
        state["formActions"].as_array().cloned().unwrap_or_default(),
    ))
}
fn detail(command: &str, state: &mut Value, message: &str, uncertain: bool) -> CoreResult {
    state["caps"] = json!({});
    state["screen"] = json!("detail");
    let mut actions = vec![
        action(
            state,
            "resume",
            Value::Null,
            if uncertain {
                "Retry effect explicitly (may duplicate)"
            } else {
                "Retry unfinished step"
            },
            false,
        )?,
        action(state, "back", Value::Null, "Back to inputs", false)?,
    ];
    if state["pending"]["effect"] == "ack" {
        actions[0]["title"] = json!("Finish remaining steps");
    }
    let text=format!("{message}\n\nOutput is retained for retry. A retry never generates replacement identifiers. Unacknowledged clipboard or paste operations may already have completed; only an explicit retry repeats them.");
    let mut value = sdk::view(
        title(command),
        node(
            "Detail",
            json!({"markdown":text,"backActionId":actions[1]["id"]}),
            vec![],
        ),
        actions,
    );
    // Inline failure stays a view so NativeSessions registers it and retry capabilities survive.
    value["message"] = json!(format!("Failed · {message}"));
    Ok(value)
}
fn ack(value: CoreResult, label: &str) -> Result<(), String> {
    let error = || {
        format!("{label} was not acknowledged. The operation may have completed; review before explicitly retrying.")
    };
    let v = value.map_err(|_| error())?;
    let o = v.as_object().ok_or_else(error)?;
    if o.get("ok") != Some(&json!(true))
        || o.get("error").is_some_and(|v| !v.is_null())
        || o.get("success").is_some_and(|v| v != &json!(true))
    {
        return Err(error());
    }
    Ok(())
}
fn revision(store: &Value) -> Result<Value, String> {
    match store.get("_uuidV2Revision") {
        None => Ok(Value::Null),
        Some(Value::String(s)) if s.len() <= 256 => Ok(json!(s)),
        _ => Err("Corrupt native history revision".into()),
    }
}
async fn save_pending(ctx: &CoreContext, state: &mut Value) -> Result<(), String> {
    if state["pending"]["stored"] == true {
        return Ok(());
    }
    let out: Output = serde_json::from_value(state["pending"]["output"].clone())
        .map_err(|_| fail("Invalid pending output"))?;
    if out.entries.is_empty() {
        state["pending"]["stored"] = json!(true);
        return Ok(());
    }
    let token = state["pending"]["token"]
        .as_str()
        .ok_or("Missing operation token")?
        .to_string();
    sdk::update_store(ctx, SLUG, move |v| {
        let mut h = history(v).map_err(|e| e.to_string())?;
        revision(v)?;
        if let Some(receipts) = v.get("_uuidV2Receipts") {
            if !receipts.is_object()
                || receipts
                    .as_object()
                    .is_some_and(|r| r.values().any(|v| v != &json!(true)))
            {
                return Err("Corrupt native operation receipts".into());
            }
            if receipts.get(&token).is_some() {
                return Ok(());
            }
        }
        if v["_uuidV2Receipts"]
            .as_object()
            .is_some_and(|r| r.len() >= 100_000)
        {
            return Err("Native receipt resource limit reached".into());
        }
        h.extend(out.entries);
        set_history(v, &h).map_err(|e| e.to_string())?;
        if v.get("_uuidV2Receipts").is_none() {
            v["_uuidV2Receipts"] = json!({});
        }
        v["_uuidV2Revision"] = json!(token);
        v["_uuidV2Receipts"][token] = json!(true);
        Ok(())
    })
    .await
    .map_err(|_| {
        "History could not be saved. Repair native storage or remove entries before retrying."
            .to_string()
    })?;
    state["pending"]["stored"] = json!(true);
    Ok(())
}
async fn finish(ctx: &CoreContext, command: &str, state: &mut Value, explicit: bool) -> CoreResult {
    if command == "viewHistory"
        && state["pending"]["effect"] != "ack"
        && revision(&sdk::read_store(ctx, SLUG).await?)? != state["historyRevision"]
    {
        return Err("History changed; return to history before copying".into());
    }
    if let Err(e) = save_pending(ctx, state).await {
        return detail(command, state, &e, false);
    }
    let out: Output = serde_json::from_value(state["pending"]["output"].clone())
        .map_err(|_| fail("Invalid pending output"))?;
    if state["pending"]["effect"] == "uncertain" && !explicit {
        return detail(
            command,
            state,
            "Clipboard effect outcome is uncertain",
            true,
        );
    }
    if state["pending"]["effect"] != "ack" {
        // Checkpoint before await: cancellation or transport loss requires an explicit retry.
        state["pending"]["effect"] = json!("uncertain");
        let result = if out.action == "paste" {
            ctx.request("core:platform:paste-text", vec![json!(out.text)])
                .await
        } else {
            sdk::copy(ctx, &out.text).await
        };
        if let Err(e) = ack(
            result,
            if out.action == "paste" {
                "Paste"
            } else {
                "Copy"
            },
        ) {
            return detail(command, state, &e, true);
        }
        state["pending"]["effect"] = json!("ack");
    }
    // paste-text already writes/hides/restores focus/waits/pastes. Copy needs only hide.
    if command != "viewHistory" && out.action == "copy" && state["pending"]["hidden"] != true {
        if let Err(e) = ack(ctx.request("window:hide", vec![]).await, "Hide window") {
            return detail(command, state, &e, false);
        }
        state["pending"]["hidden"] = json!(true);
    }
    if command == "viewHistory" {
        state["pending"] = Value::Null;
        let mut value = list(ctx, state).await?;
        value["message"] = json!(out.message);
        return Ok(value);
    }
    state["caps"] = json!({});
    state["screen"] = json!("done");
    Ok(sdk::no_view(&out.message))
}
async fn produce(
    ctx: &CoreContext,
    command: &str,
    _payload: &Value,
    state: &mut Value,
) -> CoreResult {
    if !state["pending"].is_null() {
        return finish(ctx, command, state, false).await;
    }
    let output = {
        let mut engine = ENGINE
            .lock()
            .map_err(|_| fail("Generator state unavailable"))?;
        engine.execute(
            command,
            &state["draft"],
            &state["preferences"],
            &mut OsSource,
        )
    };
    match output {
        Ok(out) => {
            state["pending"] = json!({"token":fresh()?,"output":out,"stored":false,"effect":"ready","hidden":false});
            finish(ctx, command, state, false).await
        }
        Err(e) => {
            let mut value = form(command, state)?;
            value["message"] = json!(format!("Failed · {e}"));
            Ok(value)
        }
    }
}
async fn list(ctx: &CoreContext, state: &mut Value) -> CoreResult {
    let store = sdk::read_store(ctx, SLUG)
        .await
        .map_err(|_| fail("Cannot read native UUID history"))?;
    let mut h = history(&store).map_err(|e| e.to_string())?;
    state["historyRevision"] = revision(&store)?;
    if state["ascending"] != true {
        h.reverse();
    }
    state["caps"] = json!({});
    state["screen"] = json!("history");
    let query = state["query"].as_str().unwrap_or("").to_lowercase();
    let mut rows = Vec::new();
    let mut actions = Vec::new();
    let all = action(
        state,
        "all",
        json!(h),
        if state["defaultAction"] == "paste" {
            "Paste All UUIDs"
        } else {
            "Copy All UUIDs"
        },
        false,
    )?;
    let clear = action(state, "clear-prompt", Value::Null, "Clear History", false)?;
    actions.extend([all.clone(), clear.clone()]);
    for entry in h
        .into_iter()
        .filter(|e| e.uuid.to_lowercase().contains(&query))
    {
        let copy = action(
            state,
            "copy-entry",
            json!(entry),
            "Copy to Clipboard",
            false,
        )?;
        let delete = action(state, "delete", json!(entry.uuid), "Delete Entry", false)?;
        rows.push(node("List.Item",json!({"id":fresh()?,"title":entry.uuid,"accessories":[{"tag":entry.kind,"tooltip":"Type"},{"date":entry.timestamp,"tooltip":entry.timestamp}],"actionIds":[copy["id"],all["id"],delete["id"],clear["id"]]}),vec![]));
        actions.extend([copy, delete]);
    }
    Ok(sdk::view(
        "View History",
        node(
            "List",
            json!({"__hasServerSearch":true,"filtering":false,"searchText":state["query"]}),
            rows,
        ),
        actions,
    ))
}
async fn history_action(
    ctx: &CoreContext,
    command: &str,
    cap: &Value,
    state: &mut Value,
) -> CoreResult {
    match cap["kind"].as_str().unwrap_or("") {
        "clear-prompt" => {
            state["caps"] = json!({});
            state["screen"] = json!("confirm");
            let yes = action(state, "clear", Value::Null, "Clear History", false)?;
            let no = action(state, "cancel-clear", Value::Null, "Cancel", false)?;
            Ok(sdk::view(
                "Clear History",
                node(
                    "Detail",
                    json!({"markdown":"You can't undo this action. Are you sure you want to clear the history?","backActionId":no["id"]}),
                    vec![],
                ),
                vec![yes, no],
            ))
        }
        "cancel-clear" => list(ctx, state).await,
        "delete" | "clear" => {
            let clear = cap["kind"] == "clear";
            let id = cap["data"].as_str().unwrap_or("").to_string();
            let expected = state["historyRevision"].clone();
            let next_revision = fresh()?;
            sdk::update_store(ctx, SLUG, move |v| {
                let mut h = history(v).map_err(|e| e.to_string())?;
                if revision(v)? != expected {
                    return Err("History changed; reopen before deleting".into());
                }
                if clear {
                    v.as_object_mut()
                        .ok_or("Invalid store")?
                        .remove("uuidHistory");
                } else {
                    h.retain(|e| e.uuid != id);
                    set_history(v, &h).map_err(|e| e.to_string())?;
                }
                v["_uuidV2Revision"] = json!(next_revision);
                Ok(())
            })
            .await
            .map_err(|_| fail("History update failed"))?;
            if !clear {
                state["ascending"] = json!(true);
            }
            list(ctx, state).await
        }
        "copy-entry" | "all" => {
            let store = sdk::read_store(ctx, SLUG).await?;
            if revision(&store)? != state["historyRevision"] {
                return Err("History changed; search or reopen before copying".into());
            }
            let current = history(&store).map_err(|e| e.to_string())?;
            let entries: Vec<tezbar_uuid_generator_v2::HistoryEntry> = if cap["kind"] == "all" {
                serde_json::from_value(cap["data"].clone())
                    .map_err(|_| fail("Invalid history action"))?
            } else {
                vec![serde_json::from_value(cap["data"].clone())
                    .map_err(|_| fail("Invalid history entry"))?]
            };
            // Revalidate the snapshot before copying; stale IDs cannot copy deleted entries.
            let mut remaining = std::collections::HashMap::<String, usize>::new();
            for entry in current {
                let key = serde_json::to_string(&entry).map_err(|_| "Invalid history entry")?;
                *remaining.entry(key).or_default() += 1;
            }
            for entry in &entries {
                let key = serde_json::to_string(entry).map_err(|_| "Invalid history entry")?;
                let count = remaining
                    .get_mut(&key)
                    .ok_or("History changed; search or reopen before copying")?;
                if *count == 0 {
                    return Err("History changed; search or reopen before copying".into());
                }
                *count -= 1;
            }
            let out = Output {
                text: entries
                    .iter()
                    .map(|e| e.uuid.clone())
                    .collect::<Vec<_>>()
                    .join("\r\n"),
                entries: vec![],
                message: if cap["kind"] == "all" {
                    "All UUIDs copied / pasted".into()
                } else {
                    "Copied UUID".into()
                },
                action: if cap["kind"] == "copy-entry" {
                    "copy".into()
                } else {
                    state["defaultAction"].as_str().unwrap_or("copy").into()
                },
            };
            state["pending"] = json!({"token":fresh()?,"output":out,"stored":true,"effect":"ready","hidden":false});
            finish(ctx, command, state, false).await
        }
        _ => Err("Invalid history action".into()),
    }
}
pub async fn handle(
    ctx: &CoreContext,
    command: &str,
    operation: &str,
    payload: &Value,
    state: &mut Value,
) -> CoreResult {
    if !commands().contains(&command) {
        return Err("Unsupported UUID command".into());
    }
    if operation == "extension:dispose-session" {
        *state = json!({"disposed":true});
        return Ok(json!(true));
    }
    if operation == "extension:refresh-session" {
        return Ok(sdk::unchanged());
    }
    if operation == "extension:load-more" {
        return Ok(sdk::unchanged());
    }
    if operation == "extension:run-command" {
        if state["nonce"].is_string() || state["disposed"] == true {
            return Err("Session is already initialized or disposed".into());
        }
        let args = payload
            .get("argumentValues")
            .cloned()
            .unwrap_or_else(|| json!({}));
        if !args.is_object() {
            return Err("argumentValues must be an object".into());
        }
        for (key, _) in fields(command) {
            if let Some(v) = args.get(*key) {
                if !v.is_string() && !key.starts_with("numberOf") {
                    return Err("Text arguments must be strings".into());
                }
                if v.as_str().is_some_and(|s| s.len() > MAX_TEXT) {
                    return Err("Input exceeds 1 MiB".into());
                }
            }
        }
        let default = payload["preferences"]["defaultAction"]
            .as_str()
            .unwrap_or("copy");
        if !["copy", "paste"].contains(&default) {
            return Err("Invalid defaultAction preference".into());
        }
        *state = json!({"nonce":fresh()?,"command":command,"draft":args,"preferences":payload.get("preferences").cloned().unwrap_or_else(||json!({})),"caps":{},"query":"","ascending":false,"pending":null,"defaultAction":default});
        if command == "viewHistory" {
            return list(ctx, state).await;
        }
        if payload.get("argumentValues").is_none()
            || required(command)
                .iter()
                .any(|k| state["draft"][*k].as_str().unwrap_or("").is_empty())
        {
            return form(command, state);
        }
        return produce(ctx, command, payload, state).await;
    }
    if state["disposed"] == true || state["command"] != command || !state["nonce"].is_string() {
        return Err("Invalid or disposed UUID session".into());
    }
    if operation == "extension:search-text-changed" {
        if command != "viewHistory" || state["screen"] != "history" {
            return Err("Search is available only in history".into());
        }
        let text = payload["searchText"]
            .as_str()
            .ok_or("searchText must be a string")?;
        if text.len() > MAX_TEXT {
            return Err("Search input exceeds 1 MiB".into());
        }
        state["query"] = json!(text);
        return list(ctx, state).await;
    }
    if operation != "extension:invoke-action" {
        return Err("Unsupported native operation".into());
    }
    let id = payload["actionId"].as_str().ok_or("Missing actionId")?;
    let cap = state["caps"]
        .get(id)
        .cloned()
        .ok_or("Stale or unknown action; reopen or search again")?;
    match cap["kind"].as_str().unwrap_or("") {
        "field" => {
            let key = cap["data"].as_str().ok_or("Invalid field")?;
            let value = payload["formValues"]["value"]
                .as_str()
                .ok_or("Field value must be a string")?;
            if value.len() > MAX_TEXT {
                return Err("Input exceeds 1 MiB".into());
            }
            state["draft"][key] = json!(value);
            form(command, state)
        }
        "submit" => {
            let values = payload["formValues"]
                .as_object()
                .ok_or("formValues must be an object")?;
            for (key, _) in fields(command) {
                let wire = format!("{}:{key}", state["nonce"].as_str().unwrap_or(""));
                if let Some(v) = values.get(&wire) {
                    let s = v.as_str().ok_or("Form fields must be strings")?;
                    if s.len() > MAX_TEXT {
                        return Err("Input exceeds 1 MiB".into());
                    }
                    state["draft"][*key] = v.clone();
                }
            }
            produce(ctx, command, payload, state).await
        }
        "resume" => finish(ctx, command, state, true).await,
        "back" => {
            if command == "viewHistory" {
                list(ctx, state).await
            } else {
                form(command, state)
            }
        }
        _ if command == "viewHistory" => history_action(ctx, command, &cap, state).await,
        _ => Err("Unknown action".into()),
    }
}

use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tezbar_uuid_generator_v2::{history, HistoryEntry};
use uuid_beta_host_fixture::{
    common::{write_json_atomic, CoreContext, CorePaths},
    v2_sdk as sdk, v2_uuid_generator as adapter, Sessions,
};
type Calls = Arc<Mutex<Vec<(String, Vec<Value>)>>>;
type Responses = Arc<Mutex<Option<(String, Result<Value, String>)>>>;
#[derive(Clone)]
struct Fixture {
    _dir: Arc<tempfile::TempDir>,
    ctx: CoreContext,
    calls: Calls,
    response: Responses,
}
impl Fixture {
    fn new() -> Self {
        let dir = Arc::new(tempfile::tempdir().unwrap());
        let ctx = CoreContext::new(
            CorePaths {
                data_dir: dir.path().join("data"),
                config_dir: dir.path().join("config"),
                resource_dir: dir.path().join("resources"),
                temp_dir: dir.path().join("tmp"),
                home_dir: dir.path().join("home"),
            },
            None,
        );
        let calls = Arc::new(Mutex::new(vec![]));
        let response = Arc::new(Mutex::new(None::<(String, Result<Value, String>)>));
        let (c, r) = (calls.clone(), response.clone());
        ctx.set_router(Arc::new(move |channel, args| {
            c.lock().unwrap().push((channel.clone(), args));
            let result = r
                .lock()
                .unwrap()
                .as_ref()
                .filter(|(ch, _)| ch == &channel)
                .map(|(_, r)| r.clone())
                .unwrap_or_else(|| Ok(json!({"ok":true})));
            Box::pin(async move { result })
        }))
        .unwrap();
        Self {
            _dir: dir,
            ctx,
            calls,
            response,
        }
    }
    async fn call(
        &self,
        command: &str,
        op: &str,
        p: Value,
        s: &mut Value,
    ) -> Result<Value, String> {
        adapter::handle(&self.ctx, command, op, &p, s).await
    }
    async fn launch(&self, command: &str, p: Value, s: &mut Value) -> Value {
        self.call(command, "extension:run-command", p, s)
            .await
            .unwrap()
    }
    async fn action(
        &self,
        command: &str,
        id: Value,
        form: Value,
        s: &mut Value,
    ) -> Result<Value, String> {
        self.call(
            command,
            "extension:invoke-action",
            json!({"actionId":id,"formValues":form}),
            s,
        )
        .await
    }
    async fn stored(&self) -> Vec<HistoryEntry> {
        history(&sdk::read_store(&self.ctx, adapter::SLUG).await.unwrap()).unwrap()
    }
    fn fail(&self, channel: &str, value: Result<Value, String>) {
        *self.response.lock().unwrap() = Some((channel.into(), value));
    }
    fn count(&self, ch: &str) -> usize {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(c, _)| c == ch)
            .count()
    }
    async fn seed(&self) {
        sdk::update_store(&self.ctx,adapter::SLUG,|v|{v["unknown"]=json!({"keep":true});v["uuidHistory"]=json!(r#"[{"uuid":"first","timestamp":"2023-11-14T22:13:20.000Z","type":"uuidV4"},{"uuid":"middle","timestamp":"2023-11-14T22:13:21.000Z","type":"uuidV5"},{"uuid":"last","timestamp":"2023-11-14T22:13:22.000Z","type":"ulid"},{"uuid":"first","timestamp":"2023-11-14T22:13:23.000Z","type":"uuidV4"}]"#);Ok(())}).await.unwrap();
    }
    fn install(&self) {
        write_json_atomic(
            &self.ctx.paths.data_dir.join("fixture-installed.json"),
            &json!({"defaultAction":"copy"}),
        )
        .unwrap();
    }
}
fn id(view: &Value, title: &str) -> Value {
    view["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["title"] == title)
        .unwrap_or_else(|| panic!("Missing {title}: {view}"))
        .get("id")
        .unwrap()
        .clone()
}
fn props(view: &Value) -> &Value {
    &view["root"]["props"]
}
#[tokio::test]
async fn commands_launch_arguments_actual_preferences_and_offline_effect_routes() {
    let f = Fixture::new();
    assert_eq!(adapter::commands().len(), 9);
    for cmd in adapter::commands().iter().filter(|c| **c != "viewHistory") {
        let mut s = json!({});
        let args = json!({"name":"hello","prefix":"user","uuid":"550e8400-e29b-41d4-a716-446655440000","suffix":"doctor","typeID":"user_2n1t201rmv87aa8nj4csam8000","numberOfUUIDsToGenerate":"2","numberOfULIDsToGenerate":"2"});
        let v=f.launch(cmd,json!({"argumentValues":args,"preferences":{"defaultAction":"paste","upperCaseLetters":true,"uuidNamespace":"6ba7b810-9dad-11d1-80b4-00c04fd430c8","base32Encoding":true}}),&mut s).await;
        assert_eq!(v["mode"], "no-view", "{cmd}: {v}");
    }
    assert_eq!(f.count("core:platform:paste-text"), 8);
    assert_eq!(f.count("window:hide"), 0);
    assert_eq!(f.count("clipboard:write"), 0);
    assert_eq!(f.stored().await.len(), 10);
    let mut s = json!({});
    let v=f.launch("generate",json!({"arguments":{"numberOfUUIDsToGenerate":"2"},"prefs":{"preferences":{"defaultAction":"paste"}}}),&mut s).await;
    assert_eq!(v["root"]["type"], "Form");
}
#[tokio::test]
async fn strict_effect_ack_rejects_malformed_and_sanitizes_transport() {
    for response in [
        Ok(Value::Null),
        Ok(json!(true)),
        Ok(json!(false)),
        Ok(json!({})),
        Ok(json!({"ok":true,"success":false})),
        Ok(json!({"ok":true,"error":"secret"})),
        Ok(json!({"ok":"true"})),
        Ok(json!({"ok":true,"success":null})),
        Err("SECRET token /profile/history".into()),
    ] {
        let f = Fixture::new();
        f.fail("clipboard:write", response);
        let mut s = json!({});
        let v = f
            .launch("generate", json!({"argumentValues":{}}), &mut s)
            .await;
        assert_eq!(v["mode"], "view");
        assert_eq!(v["root"]["type"], "Detail");
        assert!(!v.to_string().contains("SECRET"));
        assert!(!v.to_string().contains("secret"));
        assert_eq!(s["pending"]["effect"], "uncertain");
        assert_eq!(f.count("window:hide"), 0);
        assert_eq!(f.stored().await.len(), 1);
        let copied = f.calls.lock().unwrap()[0].1.clone();
        *f.response.lock().unwrap() = None;
        let retry = id(&v, "Retry effect explicitly (may duplicate)");
        let done = f
            .action("generate", retry, Value::Null, &mut s)
            .await
            .unwrap();
        assert_eq!(done["mode"], "no-view");
        assert_eq!(f.count("clipboard:write"), 2);
        assert_eq!(f.stored().await.len(), 1);
        assert_eq!(f.calls.lock().unwrap()[1].1, copied);
    }
    for response in [
        json!({"ok":true}),
        json!({"ok":true,"error":null,"success":true}),
    ] {
        let f = Fixture::new();
        f.fail("clipboard:write", Ok(response));
        let mut s = json!({});
        assert_eq!(
            f.launch("generate", json!({"argumentValues":{}}), &mut s)
                .await["mode"],
            "no-view"
        );
    }
}
#[tokio::test]
async fn copy_ack_checkpoint_survives_hide_failure_without_duplicate() {
    let f = Fixture::new();
    f.fail("window:hide", Ok(Value::Null));
    let mut s = json!({});
    let view = f
        .launch(
            "generate",
            json!({"argumentValues":{"numberOfUUIDsToGenerate":"3"}}),
            &mut s,
        )
        .await;
    assert_eq!(s["pending"]["effect"], "ack");
    let original = s["pending"]["output"].clone();
    *f.response.lock().unwrap() = None;
    let v = f
        .action(
            "generate",
            id(&view, "Finish remaining steps"),
            Value::Null,
            &mut s,
        )
        .await
        .unwrap();
    assert_eq!(v["mode"], "no-view");
    assert_eq!(f.count("clipboard:write"), 1);
    assert_eq!(f.count("window:hide"), 2);
    assert_eq!(s["pending"]["output"], original);
    assert_eq!(f.stored().await.len(), 3);
}
#[tokio::test]
async fn storage_failure_retains_output_and_retry_saves_once() {
    let f = Fixture::new();
    let path = f
        .ctx
        .paths
        .data_dir
        .join("native-extensions-v2/uuid-generator/storage.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"{broken").unwrap();
    let mut s = json!({});
    let view = f
        .launch("generate", json!({"argumentValues":{}}), &mut s)
        .await;
    let generated = s["pending"]["output"].clone();
    assert_eq!(std::fs::read(&path).unwrap(), b"{broken");
    assert_eq!(f.count("clipboard:write"), 0);
    std::fs::remove_file(path).unwrap();
    let done = f
        .action(
            "generate",
            id(&view, "Retry unfinished step"),
            Value::Null,
            &mut s,
        )
        .await
        .unwrap();
    assert_eq!(done["mode"], "no-view");
    assert_eq!(f.stored().await.len(), 1);
    assert_eq!(s["pending"]["output"], generated);
}
#[tokio::test]
async fn storage_receipt_replay_does_not_duplicate_append() {
    let f = Fixture::new();
    f.fail("clipboard:write", Err("lost".into()));
    let mut s = json!({});
    let v = f
        .launch("generate", json!({"argumentValues":{}}), &mut s)
        .await;
    s["pending"]["stored"] = json!(false);
    *f.response.lock().unwrap() = None;
    f.action(
        "generate",
        id(&v, "Retry effect explicitly (may duplicate)"),
        Value::Null,
        &mut s,
    )
    .await
    .unwrap();
    assert_eq!(f.stored().await.len(), 1);
}
#[tokio::test]
async fn uncertain_paste_needs_explicit_retry_and_back_keeps_draft() {
    let f = Fixture::new();
    f.fail("core:platform:paste-text", Err("unknown".into()));
    let mut s = json!({});
    let v=f.launch("generateV5",json!({"argumentValues":{"name":"draft","numberOfUUIDsToGenerate":"1"},"preferences":{"defaultAction":"paste","uuidNamespace":"6ba7b810-9dad-11d1-80b4-00c04fd430c8"}}),&mut s).await;
    let back = props(&v)["backActionId"].clone();
    let form = f
        .action("generateV5", back, Value::Null, &mut s)
        .await
        .unwrap();
    assert_eq!(form["root"]["children"][0]["props"]["value"], "draft");
    let v = f
        .action(
            "generateV5",
            id(&form, "Generate / Convert"),
            json!({}),
            &mut s,
        )
        .await
        .unwrap();
    assert_eq!(f.count("core:platform:paste-text"), 1);
    *f.response.lock().unwrap() = None;
    f.action(
        "generateV5",
        id(&v, "Retry effect explicitly (may duplicate)"),
        Value::Null,
        &mut s,
    )
    .await
    .unwrap();
    assert_eq!(f.count("core:platform:paste-text"), 2);
    assert_eq!(f.stored().await.len(), 1);
}
#[tokio::test]
async fn form_queued_field_ids_stable_and_unknown_actions_do_not_consume() {
    let f = Fixture::new();
    let mut s = json!({});
    let form = f
        .launch(
            "generateV5",
            json!({"preferences":{"uuidNamespace":"6ba7b810-9dad-11d1-80b4-00c04fd430c8"}}),
            &mut s,
        )
        .await;
    let field = form["root"]["children"][0]["props"]["actionId"].clone();
    let submit = id(&form, "Generate / Convert");
    let v = f
        .action(
            "generateV5",
            field.clone(),
            json!({"value":"first"}),
            &mut s,
        )
        .await
        .unwrap();
    assert_eq!(v["root"]["children"][0]["props"]["actionId"], field);
    assert!(v["root"]["children"][0]["props"].get("value").is_none());
    assert!(f
        .action("generateV5", json!("forged"), json!({}), &mut s)
        .await
        .is_err());
    f.action("generateV5", field, json!({"value":"second"}), &mut s)
        .await
        .unwrap();
    assert_eq!(s["draft"]["name"], "second");
    let key = form["root"]["children"][0]["props"]["id"].as_str().unwrap();
    let values = json!({key:"submitted"});
    let done = f
        .action("generateV5", submit, values, &mut s)
        .await
        .unwrap();
    assert_eq!(done["mode"], "no-view");
    assert_eq!(s["draft"]["name"], "submitted");
}
#[tokio::test]
async fn history_order_copy_all_delete_duplicates_and_scoped_clear() {
    let f = Fixture::new();
    f.seed().await;
    let mut s = json!({});
    let view = f.launch("viewHistory", json!({}), &mut s).await;
    let rows = view["root"]["children"].as_array().unwrap();
    assert_eq!(
        rows.iter()
            .map(|r| r["props"]["title"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["first", "last", "middle", "first"]
    );
    let all = id(&view, "Copy All UUIDs");
    let v = f
        .action("viewHistory", all, Value::Null, &mut s)
        .await
        .unwrap();
    assert_eq!(v["mode"], "view");
    assert_eq!(
        f.calls.lock().unwrap()[0].1,
        vec![json!("first\r\nlast\r\nmiddle\r\nfirst")]
    );
    assert_eq!(f.count("window:hide"), 0);
    let delete = id(&v, "Delete Entry");
    let v = f
        .action("viewHistory", delete, Value::Null, &mut s)
        .await
        .unwrap();
    assert_eq!(v["root"]["children"][0]["props"]["title"], "middle");
    assert_eq!(f.stored().await.len(), 2);
    let confirm = f
        .action("viewHistory", id(&v, "Clear History"), Value::Null, &mut s)
        .await
        .unwrap();
    assert_eq!(confirm["root"]["type"], "Detail");
    let back = props(&confirm)["backActionId"].clone();
    let v = f
        .action("viewHistory", back, Value::Null, &mut s)
        .await
        .unwrap();
    assert_eq!(f.stored().await.len(), 2);
    let confirm = f
        .action("viewHistory", id(&v, "Clear History"), Value::Null, &mut s)
        .await
        .unwrap();
    f.action(
        "viewHistory",
        id(&confirm, "Clear History"),
        Value::Null,
        &mut s,
    )
    .await
    .unwrap();
    assert!(f.stored().await.is_empty());
    let store = sdk::read_store(&f.ctx, adapter::SLUG).await.unwrap();
    assert_eq!(store["unknown"], json!({"keep":true}));
    assert!(store.get("uuidHistory").is_none());
}
#[tokio::test]
async fn stale_cross_session_and_discarded_search_ids_never_revive() {
    let f = Fixture::new();
    f.seed().await;
    let (mut a, mut b) = (json!({}), json!({}));
    let first = f.launch("viewHistory", json!({}), &mut a).await;
    f.launch("viewHistory", json!({}), &mut b).await;
    let cap = id(&first, "Copy All UUIDs");
    assert!(f
        .action("viewHistory", cap.clone(), Value::Null, &mut b)
        .await
        .is_err());
    let mut staged = a.clone();
    let discarded = f
        .call(
            "viewHistory",
            "extension:search-text-changed",
            json!({"searchText":"first"}),
            &mut staged,
        )
        .await
        .unwrap();
    let invalid = id(&discarded, "Copy All UUIDs");
    assert!(f
        .action("viewHistory", invalid.clone(), Value::Null, &mut a)
        .await
        .is_err());
    let committed = f
        .call(
            "viewHistory",
            "extension:search-text-changed",
            json!({"searchText":"first"}),
            &mut a,
        )
        .await
        .unwrap();
    assert_ne!(id(&committed, "Copy All UUIDs"), invalid);
    assert!(f
        .action("viewHistory", invalid, Value::Null, &mut a)
        .await
        .is_err());
    assert!(f
        .action("viewHistory", cap, Value::Null, &mut a)
        .await
        .is_err());
    f.action(
        "viewHistory",
        id(&committed, "Copy All UUIDs"),
        Value::Null,
        &mut a,
    )
    .await
    .unwrap();
}
#[tokio::test]
async fn history_search_keeps_order_all_action_includes_full_history() {
    let f = Fixture::new();
    f.seed().await;
    let mut s = json!({});
    f.launch("viewHistory", json!({}), &mut s).await;
    let view = f
        .call(
            "viewHistory",
            "extension:search-text-changed",
            json!({"searchText":"FIRST"}),
            &mut s,
        )
        .await
        .unwrap();
    assert_eq!(view["root"]["children"].as_array().unwrap().len(), 2);
    assert_eq!(props(&view)["__hasServerSearch"], true);
    assert_eq!(props(&view)["filtering"], false);
    f.action(
        "viewHistory",
        id(&view, "Copy All UUIDs"),
        Value::Null,
        &mut s,
    )
    .await
    .unwrap();
    assert_eq!(
        f.calls.lock().unwrap()[0].1,
        vec![json!("first\r\nlast\r\nmiddle\r\nfirst")]
    );
}
#[tokio::test]
async fn history_deleted_elsewhere_cannot_be_copied_with_old_id() {
    let f = Fixture::new();
    f.seed().await;
    let (mut a, mut b) = (json!({}), json!({}));
    let v = f.launch("viewHistory", json!({}), &mut a).await;
    let other = f.launch("viewHistory", json!({}), &mut b).await;
    f.action(
        "viewHistory",
        id(&other, "Delete Entry"),
        Value::Null,
        &mut b,
    )
    .await
    .unwrap();
    assert!(f
        .action(
            "viewHistory",
            id(&v, "Copy to Clipboard"),
            Value::Null,
            &mut a
        )
        .await
        .is_err());
    assert_eq!(f.count("clipboard:write"), 0);
}
#[tokio::test]
async fn refresh_clone_is_unchanged_without_new_capabilities() {
    let f = Fixture::new();
    let mut s = json!({});
    f.launch("generateV5", json!({}), &mut s).await;
    let previous = s.clone();
    let mut clone = s.clone();
    for op in ["extension:refresh-session", "extension:load-more"] {
        assert_eq!(
            f.call("generateV5", op, json!({}), &mut clone)
                .await
                .unwrap()["mode"],
            "unchanged"
        );
        assert_eq!(clone, previous);
    }
    assert_eq!(f.count("clipboard:write"), 0);
}
#[tokio::test]
async fn malformed_requests_and_disposal() {
    let f = Fixture::new();
    for args in [json!(null), json!([]), json!(4), json!({"prefix":2})] {
        assert!(f
            .call(
                "generateTypeID",
                "extension:run-command",
                json!({"argumentValues":args}),
                &mut json!({})
            )
            .await
            .is_err());
    }
    let mut s = json!({});
    let v = f.launch("generate", json!({}), &mut s).await;
    assert!(f
        .call(
            "generate",
            "extension:search-text-changed",
            json!({"searchText":"x"}),
            &mut s
        )
        .await
        .is_err());
    f.call("generate", "extension:dispose-session", json!({}), &mut s)
        .await
        .unwrap();
    assert!(f
        .action("generate", id(&v, "Generate / Convert"), json!({}), &mut s)
        .await
        .is_err());
    assert!(f
        .call("other", "extension:run-command", json!({}), &mut json!({}))
        .await
        .is_err());
}
#[tokio::test]
async fn sdk_concurrent_generators_preserve_all_history_and_unknown_keys() {
    let f = Fixture::new();
    sdk::update_store(&f.ctx, adapter::SLUG, |v| {
        v["future"] = json!([1, 2, 3]);
        Ok(())
    })
    .await
    .unwrap();
    let mut tasks = vec![];
    for _ in 0..20 {
        let f = f.clone();
        tasks.push(tokio::spawn(async move {
            let mut s = json!({});
            f.launch(
                "generate",
                json!({"argumentValues":{"numberOfUUIDsToGenerate":"2"}}),
                &mut s,
            )
            .await
        }));
    }
    for task in tasks {
        assert_eq!(task.await.unwrap()["mode"], "no-view");
    }
    assert_eq!(f.stored().await.len(), 40);
    assert_eq!(
        sdk::read_store(&f.ctx, adapter::SLUG).await.unwrap()["future"],
        json!([1, 2, 3])
    );
}
#[tokio::test]
async fn native_sessions_actual_beta_launch_dispatch_refresh_dispose_and_uninstall() {
    let f = Fixture::new();
    f.install();
    let native = Sessions::default();
    let v = native
        .handle(
            &f.ctx,
            "extension:run-command",
            json!({"extensionId":adapter::ID,"commandName":"generateV5"}),
        )
        .await;
    assert_eq!(v["mode"], "view");
    assert_eq!(v["commandName"], "generateV5");
    let session = v["sessionId"].clone();
    assert!(session.as_str().unwrap().starts_with("native-extension:"));
    let cap = id(&v, "Generate / Convert");
    assert_eq!(
        native
            .handle(
                &f.ctx,
                "extension:refresh-session",
                json!({"sessionId":session})
            )
            .await["mode"],
        "unchanged"
    );
    assert_eq!(
        native
            .handle(
                &f.ctx,
                "extension:invoke-action",
                json!({"sessionId":session,"actionId":"forged"})
            )
            .await["ok"],
        false
    );
    assert_eq!(
        native
            .handle(
                &f.ctx,
                "extension:dispose-session",
                json!({"sessionId":session})
            )
            .await,
        true
    );
    assert_eq!(
        native
            .handle(
                &f.ctx,
                "extension:invoke-action",
                json!({"sessionId":session,"actionId":cap})
            )
            .await["ok"],
        false
    );
    let v = native
        .handle(
            &f.ctx,
            "extension:run-command",
            json!({"extensionId":"raycast.uuid-generator","commandName":"viewHistory"}),
        )
        .await;
    assert_eq!(v["mode"], "view");
    std::fs::remove_file(f.ctx.paths.data_dir.join("fixture-installed.json")).unwrap();
    assert_eq!(
        native
            .handle(
                &f.ctx,
                "extension:search-text-changed",
                json!({"sessionId":v["sessionId"],"searchText":""})
            )
            .await["ok"],
        false
    );
    native.clear().await;
}
#[tokio::test]
async fn native_clear_slug_and_stale_launch_never_reach_effects() {
    let f = Fixture::new();
    f.install();
    let n = Sessions::default();
    let a = n
        .handle(
            &f.ctx,
            "extension:run-command",
            json!({"extensionId":adapter::ID,"commandName":"generate"}),
        )
        .await;
    n.clear_slug().await;
    assert_eq!(n.handle(&f.ctx,"extension:run-command",json!({"sessionId":a["sessionId"],"extensionId":adapter::ID,"commandName":"generate","argumentValues":{}})).await["ok"],false);
    assert_eq!(f.count("clipboard:write"), 0);
}
#[tokio::test]
async fn wire_artifact_actual_adapter_views() {
    let f = Fixture::new();
    f.seed().await;
    let mut s = json!({});
    let list = f.launch("viewHistory", json!({}), &mut s).await;
    let confirm = f
        .action(
            "viewHistory",
            id(&list, "Clear History"),
            Value::Null,
            &mut s,
        )
        .await
        .unwrap();
    let mut s = json!({});
    let form = f.launch("generateV5", json!({}), &mut s).await;
    let field = form["root"]["children"][0]["props"]["actionId"].clone();
    let changed = f
        .action("generateV5", field, json!({"value":"queued"}), &mut s)
        .await
        .unwrap();
    f.fail("clipboard:write", Err("lost".into()));
    let mut s = json!({});
    let detail = f
        .launch("generate", json!({"argumentValues":{}}), &mut s)
        .await;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.work");
    std::fs::create_dir_all(&root).unwrap();
    write_json_atomic(
        &root.join("wire.json"),
        &json!({"list":list,"confirm":confirm,"form":form,"changed":changed,"detail":detail}),
    )
    .unwrap();
}

#[tokio::test]
async fn actual_native_dispose_cancels_pending_effect_without_false_success() {
    let f = Fixture::new();
    f.install();
    let ctx = CoreContext::new(f.ctx.paths.clone(), None);
    let entered = Arc::new(tokio::sync::Notify::new());
    let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (started, record) = (entered.clone(), count.clone());
    ctx.set_router(Arc::new(move |channel, _args| {
        assert_eq!(channel, "clipboard:write");
        let (started, record) = (started.clone(), record.clone());
        Box::pin(async move {
            record.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            started.notify_one();
            std::future::pending().await
        })
    }))
    .unwrap();
    let sessions = Sessions::default();
    let form = sessions
        .handle(
            &ctx,
            "extension:run-command",
            json!({"extensionId":adapter::ID,"commandName":"generate"}),
        )
        .await;
    let (n, c) = (sessions.clone(), ctx.clone());
    let body = json!({"sessionId":form["sessionId"],"actionId":id(&form,"Generate / Convert"),"formValues":{}});
    let pending = tokio::spawn(async move { n.handle(&c, "extension:invoke-action", body).await });
    entered.notified().await;
    let v = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        sessions.handle(
            &ctx,
            "extension:dispose-session",
            json!({"sessionId":form["sessionId"]}),
        ),
    )
    .await
    .unwrap();
    assert_eq!(v, true);
    assert_eq!(pending.await.unwrap()["ok"], false);
    assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 1);
}
#[tokio::test]
async fn effect_cancellation_in_place_is_uncertain_and_requires_new_explicit_capability() {
    let f = Fixture::new();
    let ctx = CoreContext::new(f.ctx.paths.clone(), None);
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let count = calls.clone();
    let entered = Arc::new(tokio::sync::Notify::new());
    let started = entered.clone();
    ctx.set_router(Arc::new(move |_, _| {
        count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        started.notify_one();
        Box::pin(std::future::pending())
    }))
    .unwrap();
    let mut state = json!({});
    let payload = json!({"argumentValues":{}});
    let mut pending = Box::pin(adapter::handle(
        &ctx,
        "generate",
        "extension:run-command",
        &payload,
        &mut state,
    ));
    tokio::time::timeout(std::time::Duration::from_secs(2),async {tokio::select!{_ = entered.notified()=>(),v=&mut pending=>panic!("Effect unexpectedly completed: {v:?}")}}).await.unwrap();
    drop(pending);
    assert_eq!(state["pending"]["effect"], "uncertain");
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(f.stored().await.len(), 1);
}

#[tokio::test]
async fn corrupt_operation_receipts_reject_without_copy_or_partial_history() {
    let f = Fixture::new();
    sdk::update_store(&f.ctx, adapter::SLUG, |v| {
        v["_uuidV2Receipts"] = json!({"future":false});
        v["keep"] = json!(true);
        Ok(())
    })
    .await
    .unwrap();
    let before = sdk::read_store(&f.ctx, adapter::SLUG).await.unwrap();
    let mut state = json!({});
    let view = f
        .launch("generate", json!({"argumentValues":{}}), &mut state)
        .await;
    assert_eq!(view["root"]["type"], "Detail");
    assert_eq!(f.count("clipboard:write"), 0);
    assert_eq!(
        sdk::read_store(&f.ctx, adapter::SLUG).await.unwrap(),
        before
    );
}

#[tokio::test]
async fn revision_rejects_recreated_values_and_pending_deleted_history() {
    let f = Fixture::new();
    f.seed().await;
    let (mut a, mut b) = (json!({}), json!({}));
    let old = f.launch("viewHistory", json!({}), &mut a).await;
    let current = f.launch("viewHistory", json!({}), &mut b).await;
    let before = f.stored().await;
    f.action(
        "viewHistory",
        id(&current, "Delete Entry"),
        Value::Null,
        &mut b,
    )
    .await
    .unwrap();
    sdk::update_store(&f.ctx, adapter::SLUG, move |v| {
        tezbar_uuid_generator_v2::set_history(v, &before).map_err(|e| e.to_string())
    })
    .await
    .unwrap();
    assert!(f
        .action(
            "viewHistory",
            id(&old, "Copy to Clipboard"),
            Value::Null,
            &mut a
        )
        .await
        .is_err());
    let mut a = json!({});
    let current = f.launch("viewHistory", json!({}), &mut a).await;
    f.fail("clipboard:write", Err("uncertain".into()));
    let pending = f
        .action(
            "viewHistory",
            id(&current, "Copy to Clipboard"),
            Value::Null,
            &mut a,
        )
        .await
        .unwrap();
    let mut b = json!({});
    let list = f.launch("viewHistory", json!({}), &mut b).await;
    f.action(
        "viewHistory",
        id(&list, "Delete Entry"),
        Value::Null,
        &mut b,
    )
    .await
    .unwrap();
    *f.response.lock().unwrap() = None;
    assert!(f
        .action(
            "viewHistory",
            id(&pending, "Retry effect explicitly (may duplicate)"),
            Value::Null,
            &mut a
        )
        .await
        .is_err());
    assert_eq!(f.count("clipboard:write"), 1);
}

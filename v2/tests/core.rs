use serde_json::json;
use tezbar_uuid_generator_v2::*;
struct Fixed(u8, u64);
impl Source for Fixed {
    fn fill(&mut self, b: &mut [u8]) -> Result<()> {
        b.fill(self.0);
        Ok(())
    }
    fn now_ms(&mut self) -> Result<u64> {
        Ok(self.1)
    }
}
#[test]
fn count_js_coercions_and_limits() {
    for (input, n) in [
        (json!(""), 1),
        (json!(null), 1),
        (json!(0), 1),
        (json!("0"), 0),
        (json!("-0"), 0),
        (json!("  +3x"), 3),
        (json!("1e3"), 1),
        (json!([2]), 2),
        (json!("0x10"), 0),
    ] {
        assert_eq!(count(&input, false).unwrap(), n);
    }
    for input in [
        json!(true),
        json!("-1"),
        json!("10001"),
        json!("Infinity"),
        json!([]),
        json!({}),
    ] {
        assert!(count(&input, false).is_err());
    }
}
#[test]
fn typeid_rules_and_version_validation() {
    for p in ["", "a", "user_name"] {
        assert!(check_prefix(p).is_ok());
    }
    for p in ["_a", "a_", "A", "user1", "é"] {
        assert!(check_prefix(p).is_err());
    }
    assert!(check_prefix(&"a".repeat(63)).is_ok());
    assert!(check_prefix(&"a".repeat(64)).is_err());
    for u in [
        "00000000-0000-0000-0000-000000000000",
        "ffffffff-ffff-ffff-ffff-ffffffffffff",
        "550e8400-e29b-41d4-a716-446655440000",
    ] {
        let b = parse_uuid(u).unwrap();
        assert_eq!(parse_typeid(&pack("user", b).unwrap()).unwrap(), u);
    }
    for s in [
        "_00000000000000000000000000",
        "user_80000000000000000000000000",
        "USER_00000000000000000000000000",
        "user_0000000000000000000000000i",
        "user_0000000000000000000000000I",
    ] {
        assert!(parse_typeid(s).is_err());
    }
}
#[test]
fn entropy_is_os_and_not_a_product_seed() {
    let mut s = OsSource;
    let a = Engine::default().v4(&mut s).unwrap();
    let b = Engine::default().v4(&mut s).unwrap();
    assert_ne!(a, b);
    assert!(parse_uuid(&a).is_ok());
}
#[test]
fn history_validation_and_unknown_keys() {
    let mut v = json!({"unknown":{"preserve":true}});
    let e = HistoryEntry {
        uuid: "id".into(),
        timestamp: timestamp(0).unwrap(),
        kind: "ulid".into(),
        extra: Default::default(),
    };
    set_history(&mut v, std::slice::from_ref(&e)).unwrap();
    assert_eq!(history(&v).unwrap(), vec![e]);
    assert_eq!(v["unknown"], json!({"preserve":true}));
    for v in [
        json!([]),
        json!({"uuidHistory":[]}),
        json!({"uuidHistory":"broken"}),
        json!({"uuidHistory":"[{}]"}),
    ] {
        assert!(history(&v).is_err());
    }
}
#[test]
fn ulid_repeated_backward_and_overflow() {
    let mut e = Engine::default();
    let mut s = Fixed(255, 1700000000000);
    let a = e.ulid(&mut s).unwrap();
    assert!(a.ends_with("ZZZZZZZZZZZZZZZZ"));
    assert!(e.ulid(&mut s).is_err());
    let mut e = Engine::default();
    s.0 = 0;
    let a = e.ulid(&mut s).unwrap();
    s.1 -= 1;
    let b = e.ulid(&mut s).unwrap();
    assert!(a < b);
    assert_eq!(&a[..10], &b[..10]);
}
#[test]
fn v7_rollover_and_clock_boundaries() {
    let mut e: Engine = serde_json::from_value(
        json!({"v7":[1700000000000u64,4294967295u64],"type_v7":null,"ulid":null}),
    )
    .unwrap();
    let b = e.v7(&mut Fixed(0, 1700000000000), false).unwrap();
    assert_eq!(&b[..6], &1700000000001u64.to_be_bytes()[2..]);
    assert!(Engine::default().v7(&mut Fixed(0, 1 << 48), false).is_err());
    let mut e: Engine = serde_json::from_value(
        json!({"v7":null,"type_v7":[1700000000000u64,4095,524287],"ulid":null}),
    )
    .unwrap();
    let b = e.v7(&mut Fixed(0, 1700000000000), true).unwrap();
    assert_eq!(&b[..6], &1700000000001u64.to_be_bytes()[2..]);
}
#[test]
fn format_preserves_case_and_strips_nonhex() {
    assert_eq!(
        format_uuid("!550E8400 E29B41D4 A716 446655440000!").unwrap(),
        "550E8400-E29B-41D4-A716-446655440000"
    );
    assert!(format_uuid("short").is_err());
}

#[test]
fn storage_unknown_entry_fields_and_explicit_resource_limits() {
    let entry = json!({"uuid":"id","timestamp":"2023-11-14T22:13:20.000Z","type":"uuidV4","future":{"retain":[1,2]}});
    let mut store =
        json!({"other":true,"uuidHistory":serde_json::to_string(&vec![entry.clone()]).unwrap()});
    let h = history(&store).unwrap();
    set_history(&mut store, &h).unwrap();
    let saved: serde_json::Value =
        serde_json::from_str(store["uuidHistory"].as_str().unwrap()).unwrap();
    assert_eq!(saved, json!([entry]));
    assert_eq!(store["other"], true);
    let too_many = vec![h[0].clone(); MAX_HISTORY + 1];
    let before = store.clone();
    assert!(set_history(&mut store, &too_many).is_err());
    assert_eq!(store, before);
    let mut engine = Engine::default();
    let mut source = Fixed(0, 1700000000000);
    let big = "a".repeat(MAX_TEXT + 1);
    assert!(engine
        .execute("formatUuid", &json!({"uuid":big}), &json!({}), &mut source)
        .is_err());
    let name = "x".repeat(MAX_TEXT);
    let err = engine
        .execute(
            "generateV5",
            &json!({"name":name,"numberOfUUIDsToGenerate":"65"}),
            &json!({"uuidNamespace":"6ba7b810-9dad-11d1-80b4-00c04fd430c8"}),
            &mut source,
        )
        .unwrap_err();
    assert_eq!(err.title, "Resource limit");
}

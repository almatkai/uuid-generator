//! Executes checked-in output of the actual original TS + locked JS libraries.
use serde_json::{json, Value};
use tezbar_uuid_generator_v2::{Engine, Result, Source};
struct Recorded {
    next: u64,
    now: u64,
    fixed: Option<u8>,
}
impl Source for Recorded {
    fn fill(&mut self, bytes: &mut [u8]) -> Result<()> {
        for b in bytes {
            *b = self.fixed.unwrap_or((self.next % 256) as u8);
            self.next += 1;
        }
        Ok(())
    }
    fn now_ms(&mut self) -> Result<u64> {
        Ok(self.now)
    }
}
#[test]
fn actual_original_oracle_cases() {
    let cases: Vec<Value> = serde_json::from_str(include_str!("../oracle/cases.json")).unwrap();
    assert!(cases.len() >= 100);
    for case in cases {
        let mut engine = if let Some(seq) = case["prelude"]["typeV7Seq"].as_u64() {
            serde_json::from_value(
                json!({"v7":null,"type_v7":[case["now"],seq>>19,seq&0x7ffff],"ulid":null}),
            )
            .unwrap()
        } else {
            Engine::default()
        };
        let mut source = Recorded {
            next: case["seed"].as_u64().unwrap_or(0),
            now: case["now"].as_u64().unwrap_or(1700000000000),
            fixed: case["fixed"].as_u64().map(|v| v as u8),
        };
        for step in case["steps"].as_array().unwrap() {
            source.now = step["now"].as_u64().unwrap_or(source.now);
            let result = engine.execute(
                step["command"].as_str().unwrap(),
                &step["args"],
                &step["prefs"],
                &mut source,
            );
            if let Some(expected) = step.get("output") {
                let out = result.unwrap_or_else(|e| panic!("{}: {e}", case["name"]));
                assert_eq!(
                    serde_json::to_value(out).unwrap(),
                    *expected,
                    "{}",
                    case["name"]
                );
            } else {
                assert!(
                    result.is_err(),
                    "{} expected original error: {:?}",
                    case["name"],
                    step
                );
                if let Some(title) = step.get("errorTitle") {
                    let error = result.unwrap_err();
                    assert_eq!(json!(error.title), *title, "{}", case["name"]);
                    let toast = step["originalError"]["events"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|e| e[0] == "toast")
                        .unwrap();
                    assert_eq!(
                        error.message,
                        toast[1]["message"].as_str().unwrap_or(""),
                        "{}",
                        case["name"]
                    );
                }
            }
        }
    }
}

#[test]
fn locked_uuid11_state_helper_rollovers() {
    let cases: Vec<Value> = serde_json::from_str(include_str!("../oracle/low-level.json")).unwrap();
    for c in cases {
        let seq = c["before"]["seq"].as_i64().unwrap() as u32;
        let mut engine: Engine = serde_json::from_value(
            json!({"v7":[c["before"]["msecs"],seq],"type_v7":null,"ulid":null}),
        )
        .unwrap();
        let mut source = Recorded {
            next: 0,
            now: c["now"].as_u64().unwrap(),
            fixed: None,
        };
        let actual = tezbar_uuid_generator_v2::uuid_string(engine.v7(&mut source, false).unwrap());
        assert_eq!(json!(actual), c["result"]);
    }
}

#[test]
fn actual_source_history_serialization_bytes() {
    let recording: Value =
        serde_json::from_str(include_str!("../oracle/history-recording.json")).unwrap();
    let mut store = json!({"unknown":{"retain":true},"uuidHistory":recording["rawInitial"]});
    let mut entries = tezbar_uuid_generator_v2::history(&store).unwrap();
    tezbar_uuid_generator_v2::set_history(&mut store, &entries).unwrap();
    assert_eq!(store["uuidHistory"], recording["rawInitial"]);
    entries.retain(|e| e.uuid != "first");
    tezbar_uuid_generator_v2::set_history(&mut store, &entries).unwrap();
    assert_eq!(store["uuidHistory"], recording["rawAfterDelete"]);
    assert_eq!(store["unknown"], json!({"retain":true}));
}

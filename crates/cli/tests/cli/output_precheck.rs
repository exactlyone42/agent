use crate::{
    case::Case, completion::completion, fake_server::FakeServer, text_message::text_message,
    tool_call_message::tool_call_message, write_json::write_json,
};
use serde_json::json;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[test]
fn invalid_output_is_refused_before_creation_and_model_can_correct_it() {
    let case = Case::new();
    std::fs::write(case.root().join("input"), "source\n").unwrap();
    let refused_before_creation = Arc::new(AtomicBool::new(false));
    let server = FakeServer::start({
        let output = case.root().join("output");
        let refused = refused_before_creation.clone();
        move |index, request, stream| {
            let reply = match index {
                1 => completion(
                    tool_call_message("read", "read_input", &json!({})),
                    "tool_calls",
                ),
                2 => completion(
                    tool_call_message("wrong", "write_output", &json!({"content":"fixed"})),
                    "tool_calls",
                ),
                3 => {
                    let feedback = request["messages"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|message| {
                            message["tool_call_id"] == "wrong"
                                && message["content"]
                                    .as_str()
                                    .is_some_and(|text| text.contains("未建立檔案"))
                        });
                    refused.store(!output.exists() && feedback, Ordering::SeqCst);
                    completion(
                        tool_call_message(
                            "corrected",
                            "write_output",
                            &json!({"content":"fixed\n"}),
                        ),
                        "tool_calls",
                    )
                }
                _ => completion(text_message("done"), "stop"),
            };
            write_json(stream, &reply);
        }
    });
    let task = case.write_task(&json!({"endpoint":server.endpoint(),"model":"fixture-model","task":{"prompt":"read source and create the exact result","input":"input","output":"output","allow_read":true,"allow_write":true,"expected":"fixed\n"}}));
    let result = case.run(&["run", task.to_str().unwrap()]);
    assert!(
        refused_before_creation.load(Ordering::SeqCst),
        "wrong output was created instead of receiving correction feedback: {}",
        result.stderr
    );
    assert_eq!(result.code(), Some(0), "{}", result.stderr);
    assert_eq!(
        std::fs::read_to_string(case.root().join("input")).unwrap(),
        "source\n"
    );
    assert_eq!(
        std::fs::read_to_string(case.root().join("output")).unwrap(),
        "fixed\n"
    );
    let task = crate::task_id_of::task_id_of(&result.stderr);
    let store = agent_platform_agent::store::TaskStore::read(&case.database()).unwrap();
    let calls = store.approvals(&task).unwrap();
    assert_eq!(
        calls
            .iter()
            .map(|call| call.intent_status.as_str())
            .collect::<Vec<_>>(),
        vec!["succeeded", "failed", "succeeded"]
    );
    let readback = case.run(&["result", &task]);
    assert_eq!(readback.code(), Some(0), "{}", readback.stderr);
    let value: serde_json::Value = serde_json::from_str(&readback.stdout).unwrap();
    assert_eq!(value["verified"], true);
    assert_eq!(value["before"], "source\n");
    assert_eq!(server.requests().len(), 4);
}

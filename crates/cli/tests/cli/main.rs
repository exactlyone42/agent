mod accept_loop;
mod await_flag;
mod case;
mod collect_lines;
mod command_arguments;
mod compile_fixture;
mod completion;
mod error_codes;
mod existing_directory_without_database;
mod fake_server;
mod file_task_handler;
mod file_task_run;
mod file_tasks;
mod interactive;
mod invalid_configuration;
mod invalid_revision;
mod invalid_state_directory;
mod open_database;
mod outcome;
mod planned_call;
mod queries_refuse_missing;
mod read_request;
mod run_each;
mod running;
mod serve_connection;
mod stored_result;
mod task_id_of;
mod text_message;
mod tool_call_message;
mod url_policy;
mod write_json;
mod write_redirect;

#[cfg(unix)]
mod approval_scenario;
#[cfg(unix)]
mod fifo_source;
#[cfg(unix)]
mod hold_until_disconnect;
#[cfg(unix)]
mod interrupt_pending_model;
#[cfg(unix)]
mod invalid_key_encoding;
#[cfg(unix)]
mod keep_open;
#[cfg(unix)]
mod make_fifo;
#[cfg(unix)]
mod manual_approval;
#[cfg(target_os = "linux")]
mod second_interrupt;
#[cfg(unix)]
mod set_mode;
#[cfg(target_os = "linux")]
mod source_replaced;
#[cfg(unix)]
mod state_links_rejected;
#[cfg(unix)]
mod sync_pipes;
#[cfg(unix)]
mod unreadable_database_path;

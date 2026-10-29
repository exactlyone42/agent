use crate::case::Case;
use crate::make_fifo::make_fifo;
use crate::run_each::run_each;
use crate::set_mode::set_mode;

#[test]
fn state_directory_and_database_links_are_rejected() {
    let case = Case::new();
    let state = case.state();
    std::fs::create_dir(&state).expect("state directory");
    set_mode(&state, 0o700);
    let target = case.root().join("target");
    std::fs::write(&target, "preserve this file").expect("target file");
    let database = case.database();
    run_each(
        ["public_directory", "symlink", "hardlink", "fifo"],
        |scenario| {
            match scenario {
                "public_directory" => set_mode(&state, 0o755),
                "symlink" => std::os::unix::fs::symlink(&target, &database).expect("symlink"),
                "hardlink" => std::fs::hard_link(&target, &database).expect("hard link"),
                _ => make_fifo(&database),
            }
            let result = case.run(&["status", "missing"]);
            if scenario == "public_directory" {
                set_mode(&state, 0o700);
            } else {
                std::fs::remove_file(&database).expect("remove injected database");
            }
            assert_eq!(result.code(), Some(1));
            assert_eq!(result.stdout, "");
            assert_eq!(
                std::fs::read_to_string(&target).expect("target"),
                "preserve this file"
            );
        },
    );
}

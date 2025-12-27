use p4rs::extensible::CmdType;
use p4rs::{
    ChangeSpec, ChangeType, ClientMapping, ClientSpec, EditAction, OpenAction, P4Command, P4Error,
};
mod common;
use common::SERVER;
use std::fs;
use tempfile::TempDir;
use test_log::test;

#[test]
fn test_invalid_command_returns_json_error() {
    let p4 = SERVER.p4();
    assert!(matches!(
        p4.run(p4.build_cmd("-h", CmdType::Query)),
        Err(P4Error::JsonError(_))
    ));
}

#[test]
fn test_unknown_command_returns_command_failed() {
    let p4 = SERVER.p4();
    assert!(matches!(
        p4.run(p4.build_cmd("inf", CmdType::Query)),
        Err(P4Error::CommandFailed(ref error, severity)) if error.starts_with("Unknown command") && severity == 3
    ));
}

#[test]
fn test_info() {
    let p4 = SERVER.p4();
    let info = p4.info().short().run().expect("Failed to get info");
    assert!(!info.user_name.is_empty());
}

#[test]
fn test_changes() {
    let test_client = SERVER.test_client();
    test_client
        .p4
        .change()
        .set(&ChangeSpec::new(ChangeType::New).description("Test change"))
        .run()
        .expect("Failed to create change");
    test_client
        .p4
        .change()
        .set(&ChangeSpec::new(ChangeType::New).description("Test change 2"))
        .run()
        .expect("Failed to create change");
    let changes = test_client
        .p4
        .changes(&[])
        .long()
        .client(&test_client.client_name)
        .run()
        .expect("Failed to get changes");
    assert_eq!(changes.len(), 2);
}

#[test]
fn test_change_spec() {
    let test_client = SERVER.test_client();
    let change_spec = ChangeSpec::new(ChangeType::New).description("Test change");
    let change_number = test_client
        .p4
        .change()
        .set(&change_spec)
        .run()
        .expect("Failed to set change");
    assert!(change_number > 0);
    let result_spec = test_client
        .p4
        .change()
        .get(Some(change_number))
        .run()
        .expect("Failed to get change");
    assert!(result_spec.description.trim() == change_spec.description.trim());
}

#[test]
fn test_change_delete() {
    let test_client = SERVER.test_client();
    let change_spec = ChangeSpec::new(ChangeType::New).description("Change to delete");
    let change_number = test_client
        .p4
        .change()
        .set(&change_spec)
        .run()
        .expect("Failed to create change");

    test_client
        .p4
        .change()
        .delete(change_number)
        .run()
        .expect("Failed to delete change");

    let result = test_client.p4.change().get(Some(change_number)).run();
    assert!(matches!(result, Err(P4Error::CommandFailed(_, severity)) if severity == 3));
}

#[test]
fn test_add_opened_revert() {
    let test_client = SERVER.test_client();
    let file_path = test_client.client_root().join("test_file.txt");
    fs::write(&file_path, "test content").expect("Failed to write file");
    let file_str = file_path.to_str().unwrap();

    let results = test_client
        .p4
        .add(&[file_str])
        .run()
        .expect("Failed to add file");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].action, "add");

    let opened = test_client
        .p4
        .opened(&["//..."])
        .run()
        .expect("Failed to get opened files");
    assert_eq!(opened.len(), 1);
    assert_eq!(opened[0].action, OpenAction::Add);

    let reverted = test_client
        .p4
        .revert(&[file_str])
        .run()
        .expect("Failed to revert file");
    assert_eq!(reverted.len(), 1);

    let opened_after = test_client
        .p4
        .opened(&["//..."])
        .run()
        .expect("Failed to get opened files after revert");
    assert!(opened_after.is_empty());
}

#[test]
fn test_add_submit_edit() {
    let test_client = SERVER.test_client();
    let file_path = test_client.client_root().join("edit_test.txt");
    fs::write(&file_path, "initial content").expect("Failed to write file");
    let file_str = file_path.to_str().unwrap();

    let change_spec = ChangeSpec::new(ChangeType::New).description("Add file for edit test");
    let cl = test_client
        .p4
        .change()
        .set(&change_spec)
        .run()
        .expect("Failed to create change");

    test_client
        .p4
        .add(&[file_str])
        .changelist(cl)
        .run()
        .expect("Failed to add file");

    test_client
        .p4
        .submit(cl)
        .run()
        .expect("Failed to submit change");

    let edit_results = test_client
        .p4
        .edit(&[file_str])
        .run()
        .expect("Failed to edit file");
    assert_eq!(edit_results.len(), 1);
    assert_eq!(edit_results[0].action, EditAction::Edit);

    let opened = test_client
        .p4
        .opened(&["//..."])
        .run()
        .expect("Failed to get opened files");
    assert_eq!(opened.len(), 1);
    assert_eq!(opened[0].action, OpenAction::Edit);

    test_client
        .p4
        .revert(&[file_str])
        .run()
        .expect("Failed to revert");
}

#[test]
fn test_edit_with_changelist() {
    let test_client = SERVER.test_client();
    let file_path = test_client.client_root().join("cl_test.txt");
    fs::write(&file_path, "content").expect("Failed to write file");
    let file_str = file_path.to_str().unwrap();

    let add_cl = test_client
        .p4
        .change()
        .set(&ChangeSpec::new(ChangeType::New).description("Add file"))
        .run()
        .expect("Failed to create change");
    test_client
        .p4
        .add(&[file_str])
        .changelist(add_cl)
        .run()
        .expect("Failed to add");
    test_client
        .p4
        .submit(add_cl)
        .run()
        .expect("Failed to submit");

    let edit_cl = test_client
        .p4
        .change()
        .set(&ChangeSpec::new(ChangeType::New).description("Edit changelist"))
        .run()
        .expect("Failed to create edit changelist");

    let results = test_client
        .p4
        .edit(&[file_str])
        .changelist(edit_cl)
        .run()
        .expect("Failed to edit with changelist");
    assert_eq!(results.len(), 1);

    let opened = test_client
        .p4
        .opened(&["//..."])
        .run()
        .expect("Failed to get opened files");
    assert_eq!(opened.len(), 1);
    assert_eq!(opened[0].change.number(), Some(edit_cl));

    test_client
        .p4
        .revert(&["//..."])
        .run()
        .expect("Failed to revert");
}

#[test]
fn test_change_with_multiple_files() {
    let test_client = SERVER.test_client();
    let file1 = test_client.client_root().join("file1.txt");
    let file2 = test_client.client_root().join("file2.txt");
    fs::write(&file1, "content1").expect("Failed to write file1");
    fs::write(&file2, "content2").expect("Failed to write file2");
    let file1_str = file1.to_str().unwrap();
    let file2_str = file2.to_str().unwrap();
    let files = [file1_str, file2_str];

    let change_spec = ChangeSpec::new(ChangeType::New).description("Multi-file change");
    let cl = test_client
        .p4
        .change()
        .set(&change_spec)
        .run()
        .expect("Failed to create change");

    test_client
        .p4
        .add(&files)
        .changelist(cl)
        .run()
        .expect("Failed to add files");

    let spec = test_client
        .p4
        .change()
        .get(Some(cl))
        .run()
        .expect("Failed to get change spec");
    assert_eq!(spec.description.trim(), "Multi-file change");
    assert_eq!(spec.files.len(), 2);

    test_client
        .p4
        .revert(&["//..."])
        .run()
        .expect("Failed to revert");
}

#[test]
fn test_create_client() {
    let p4 = SERVER.p4();
    let tmp_dir = TempDir::new().expect("Failed to create temp dir");
    let client_spec = ClientSpec::new_with_default_mapping(
        "my-client",
        tmp_dir.path().to_str().unwrap(),
        "//depot/...",
    );
    let client_name = p4
        .client()
        .set(&client_spec)
        .run()
        .expect("Failed to create client");
    assert!(!client_name.is_empty());
    let result_spec = p4
        .client()
        .get(Some(&client_name))
        .run()
        .expect("Failed to get client spec");
    assert_eq!(result_spec.client, "my-client");
    assert_eq!(
        result_spec.view,
        vec![ClientMapping::new("//depot/...", "//my-client/...")]
    );

    p4.client()
        .delete(&client_name)
        .run()
        .expect("Failed to delete client");
}

#[test]
fn test_submit() {
    let test_client = SERVER.test_client();
    let file_path = test_client.client_root().join("submit_test.txt");
    fs::write(&file_path, "submit content").expect("Failed to write file");
    let file_str = file_path.to_str().unwrap();

    let cl = test_client
        .p4
        .change()
        .set(&ChangeSpec::new(ChangeType::New).description("Submit test"))
        .run()
        .expect("Failed to create change");

    test_client
        .p4
        .add(&[file_str])
        .changelist(cl)
        .run()
        .expect("Failed to add");

    test_client.p4.submit(cl).run().expect("Failed to submit");
}

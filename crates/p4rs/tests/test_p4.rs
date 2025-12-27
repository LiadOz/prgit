use p4rs::extensible::CmdType;
use p4rs::{
    ChangeSpec, ChangeStatus, ChangeType, ClientMapping, ClientSpec, EditAction, OpenAction,
    P4Command, P4Error, P4,
};
use tempfile::TempDir;
use test_log::test;

#[test]
fn test_invalid_command_returns_json_error() {
    let p4 = P4::new();
    assert!(matches!(
        p4.run(p4.build_cmd("-h", CmdType::Query)),
        Err(P4Error::JsonError(_))
    ));
}

#[test]
fn test_unknown_command_returns_command_failed() {
    let p4 = P4::new();
    assert!(matches!(
        p4.run(p4.build_cmd("inf", CmdType::Query)),
        Err(P4Error::CommandFailed(ref error)) if error.starts_with("Unknown command")
    ));
}

#[test]
fn test_info() {
    let p4 = P4::new();
    let info = p4.info().short().run().expect("Failed to get info");
    assert!(!info.server_id.is_empty());
    assert!(!info.user_name.is_empty());
}

#[test]
fn test_changes() {
    let p4 = P4::new();
    let changes = p4.changes(&[]).long().run().expect("Failed to get changes");
    assert!(!changes.is_empty());
    assert!(changes.len() >= 3);
    assert!(changes.last().expect("No changes found").status == ChangeStatus::Submitted);
}

#[test]
fn test_change_spec() {
    let p4 = P4::new();
    let change_spec = ChangeSpec::new(ChangeType::New).description("Test change");
    let change_number = p4
        .change()
        .set(&change_spec)
        .run()
        .expect("Failed to set change");
    assert!(change_number > 0);
    let result_spec = p4
        .change()
        .get(Some(change_number))
        .run()
        .expect("Failed to get change");
    assert!(result_spec.description.trim() == change_spec.description.trim());
}

#[test]
fn test_change_delete() {
    let p4 = P4::new();
    let change_spec = ChangeSpec::new(ChangeType::New).description("Change to delete");
    let change_number = p4
        .change()
        .set(&change_spec)
        .run()
        .expect("Failed to create change");

    p4.change()
        .delete(change_number)
        .run()
        .expect("Failed to delete change");

    let result = p4.change().get(Some(change_number)).run();
    assert!(matches!(result, Err(P4Error::CommandFailed(_))));
}

#[test]
fn test_edit_opened_revert() {
    let p4 = P4::new();
    let test_file = "//depot/testing/test_file";

    p4.revert(&[test_file]).run().ok();

    let results = p4.edit(&[test_file]).run().expect("Failed to edit file");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].action, EditAction::Edit);
    assert!(results[0].depot_file.ends_with("test_file"));

    let opened = p4
        .opened(&[test_file])
        .run()
        .expect("Failed to get opened files");
    assert_eq!(opened.len(), 1);
    assert_eq!(opened[0].action, OpenAction::Edit);

    let reverted = p4
        .revert(&[test_file])
        .run()
        .expect("Failed to revert file");
    assert_eq!(reverted.len(), 1);
    assert!(reverted[0].depot_file.ends_with("test_file"));

    let opened_after = p4
        .opened(&[test_file])
        .run()
        .expect("Failed to get opened files after revert");
    assert!(opened_after.is_empty());
}

#[test]
fn test_edit_with_changelist() {
    let p4 = P4::new();
    let test_file = "//depot/testing/another_file";

    p4.revert(&[test_file]).run().ok();

    let change_spec = ChangeSpec::new(ChangeType::New).description("Test CL for edit");
    let cl = p4
        .change()
        .set(&change_spec)
        .run()
        .expect("Failed to set change");

    let results = p4
        .edit(&[test_file])
        .changelist(cl)
        .run()
        .expect("Failed to edit file with changelist");
    assert_eq!(results.len(), 1);

    let opened = p4
        .opened(&[test_file])
        .run()
        .expect("Failed to get opened files");
    assert_eq!(opened.len(), 1);
    assert!(opened[0].depot_file.ends_with("another_file"));
    assert_eq!(opened[0].change.number(), Some(cl));

    p4.revert(&[test_file])
        .run()
        .expect("Failed to revert file");
}

#[test]
fn test_change_with_multiple_files() {
    let p4 = P4::new();
    let files = ["//depot/testing/test_file", "//depot/testing/another_file"];

    p4.revert(&files).run().ok();

    let change_spec = ChangeSpec::new(ChangeType::New).description("Multi-file change");
    let cl = p4
        .change()
        .set(&change_spec)
        .run()
        .expect("Failed to create change");

    p4.edit(&files)
        .changelist(cl)
        .run()
        .expect("Failed to edit files");

    let spec = p4
        .change()
        .get(Some(cl))
        .run()
        .expect("Failed to get change spec");
    assert_eq!(spec.description.trim(), "Multi-file change");
    assert_eq!(spec.files.len(), 2);
    for f in &files {
        assert!(spec.files.iter().any(|sf| sf.contains(*f)));
    }

    p4.revert(&files).run().expect("Failed to revert files");
}

#[test]
fn test_create_client() {
    let p4 = P4::new();
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

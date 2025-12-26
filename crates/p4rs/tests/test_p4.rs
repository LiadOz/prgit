use p4rs::{P4, P4Command, P4Error, ChangeSpec, ChangeType, ChangeStatus, EditAction, OpenAction};
use p4rs::extensible::CmdType;

#[test]
fn test_invalid_command_returns_json_error() {
    let p4 = P4::new();
    assert!(matches!(p4.run(p4.build_cmd("-h", CmdType::Query)), Err(P4Error::JsonError(_))));
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
    let change_spec = ChangeSpec::new(ChangeType::New).description("Test change".to_string());
    let change_number = p4.set_change(&change_spec).run().expect("Failed to set change");
    assert!(change_number > 0);
    let result_spec = p4.get_change(change_number).run().expect("Failed to get change");
    assert!(result_spec.description.trim() == change_spec.description.trim());
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

    let opened = p4.opened(&[test_file]).run().expect("Failed to get opened files");
    assert_eq!(opened.len(), 1);
    assert_eq!(opened[0].action, OpenAction::Edit);

    let reverted = p4.revert(&[test_file]).run().expect("Failed to revert file");
    assert_eq!(reverted.len(), 1);
    assert!(reverted[0].depot_file.ends_with("test_file"));

    let opened_after = p4.opened(&[test_file]).run().expect("Failed to get opened files after revert");
    assert!(opened_after.is_empty());
}

#[test]
fn test_edit_with_changelist() {
    let p4 = P4::new();
    let test_file = "//depot/testing/another_file";

    p4.revert(&[test_file]).run().ok();

    let change_spec = ChangeSpec::new(ChangeType::New).description("Test CL for edit".to_string());
    let cl = p4.set_change(&change_spec).run().expect("Failed to set change");

    let results = p4.edit(&[test_file]).changelist(cl).run().expect("Failed to edit file with changelist");
    assert_eq!(results.len(), 1);

    let opened = p4.opened(&[test_file]).run().expect("Failed to get opened files");
    assert_eq!(opened.len(), 1);
    assert!(opened[0].depot_file.ends_with("another_file"));
    assert_eq!(opened[0].change.number(), Some(cl));

    p4.revert(&[test_file]).run().expect("Failed to revert file");
}


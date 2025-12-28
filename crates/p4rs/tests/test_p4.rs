use p4rs::extensible::CmdType;
use p4rs::testkit::SERVER;
use p4rs::{
    ChangeSpec, ChangeType, ClientMapping, ClientSpec, EditAction, FileAction, OpenAction,
    P4Command, P4Error,
};
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

#[test]
fn test_opened_have_rev_after_submit() {
    let test_client = SERVER.test_client();
    let file_path = test_client.client_root().join("have_rev_test.txt");
    fs::write(&file_path, "content").expect("Failed to write file");
    let file_str = file_path.to_str().unwrap();

    let cl = test_client
        .p4
        .change()
        .set(&ChangeSpec::new(ChangeType::New).description("Add"))
        .run()
        .expect("Failed to create change");
    test_client
        .p4
        .add(&[file_str])
        .changelist(cl)
        .run()
        .expect("Failed to add");
    test_client.p4.submit(cl).run().expect("Failed to submit");

    test_client
        .p4
        .edit(&[file_str])
        .run()
        .expect("Failed to edit");
    let opened = test_client
        .p4
        .opened(&["//..."])
        .run()
        .expect("Failed to get opened");
    assert_eq!(opened.len(), 1);
    assert_eq!(opened[0].have_rev, Some(1));

    test_client
        .p4
        .revert(&["//..."])
        .run()
        .expect("Failed to revert");
}

#[test]
fn test_revert_unchanged() {
    let test_client = SERVER.test_client();
    let file_path = test_client.client_root().join("unchanged_test.txt");
    fs::write(&file_path, "content").expect("Failed to write file");
    let file_str = file_path.to_str().unwrap();

    let cl = test_client
        .p4
        .change()
        .set(&ChangeSpec::new(ChangeType::New).description("Add"))
        .run()
        .expect("Failed to create change");
    test_client
        .p4
        .add(&[file_str])
        .changelist(cl)
        .run()
        .expect("Failed to add");
    test_client.p4.submit(cl).run().expect("Failed to submit");

    test_client
        .p4
        .edit(&[file_str])
        .run()
        .expect("Failed to edit");
    let reverted = test_client
        .p4
        .revert(&["//..."])
        .unchanged()
        .run()
        .expect("Failed to revert");
    assert_eq!(reverted.len(), 1);
}

#[test]
fn test_connection_failed() {
    let p4 = p4rs::P4::new().port("localhost:99999");
    let result = p4.info().run();
    assert!(matches!(result, Err(P4Error::ConnectionFailed)));
}

#[test]
fn test_edit_preview() {
    let test_client = SERVER.test_client();
    let file_path = test_client.client_root().join("preview_test.txt");
    fs::write(&file_path, "content").expect("Failed to write file");
    let file_str = file_path.to_str().unwrap();

    let cl = test_client
        .p4
        .change()
        .set(&ChangeSpec::new(ChangeType::New).description("Add"))
        .run()
        .expect("Failed to create change");
    test_client
        .p4
        .add(&[file_str])
        .changelist(cl)
        .run()
        .expect("Failed to add");
    test_client.p4.submit(cl).run().expect("Failed to submit");

    let results = test_client
        .p4
        .edit(&[file_str])
        .preview()
        .run()
        .expect("Failed to preview edit");
    assert_eq!(results.len(), 1);

    let opened = test_client
        .p4
        .opened(&["//..."])
        .run()
        .expect("Failed to get opened");
    assert!(opened.is_empty());
}

#[test]
fn test_add_preview() {
    let test_client = SERVER.test_client();
    let file_path = test_client.client_root().join("add_preview.txt");
    fs::write(&file_path, "content").expect("Failed to write file");
    let file_str = file_path.to_str().unwrap();

    let results = test_client
        .p4
        .add(&[file_str])
        .preview()
        .run()
        .expect("Failed to preview add");
    assert_eq!(results.len(), 1);

    let opened = test_client
        .p4
        .opened(&["//..."])
        .run()
        .expect("Failed to get opened");
    assert!(opened.is_empty());
}

#[test]
fn test_shelve() {
    let test_client = SERVER.test_client();
    let file1 = test_client.client_root().join("shelve_test1.txt");
    let file2 = test_client.client_root().join("shelve_test2.txt");
    fs::write(&file1, "shelve content 1").expect("Failed to write file1");
    fs::write(&file2, "shelve content 2").expect("Failed to write file2");
    let file1_str = file1.to_str().unwrap();
    let file2_str = file2.to_str().unwrap();

    let cl = test_client
        .p4
        .change()
        .set(&ChangeSpec::new(ChangeType::New).description("Shelve test"))
        .run()
        .expect("Failed to create change");

    test_client
        .p4
        .add(&[file1_str, file2_str])
        .changelist(cl)
        .run()
        .expect("Failed to add files");

    let shelve_result = test_client
        .p4
        .shelve()
        .set(cl)
        .run()
        .expect("Failed to shelve");
    assert_eq!(shelve_result.len(), 2);

    let replace_result = test_client
        .p4
        .shelve()
        .set(cl)
        .replace()
        .run()
        .expect("Failed to replace shelve");
    assert_eq!(replace_result.len(), 2);

    let delete_result = test_client
        .p4
        .shelve()
        .delete(cl)
        .run()
        .expect("Failed to delete shelve");
    assert!(delete_result.data.contains("deleted"));

    test_client
        .p4
        .revert(&["//..."])
        .run()
        .expect("Failed to revert");
}

#[test]
fn test_reopen() {
    let test_client = SERVER.test_client();
    let file1 = test_client.client_root().join("reopen_test1.txt");
    let file2 = test_client.client_root().join("reopen_test2.txt");
    fs::write(&file1, "reopen content 1").expect("Failed to write file1");
    fs::write(&file2, "reopen content 2").expect("Failed to write file2");
    let file1_str = file1.to_str().unwrap();
    let file2_str = file2.to_str().unwrap();

    let cl1 = test_client
        .p4
        .change()
        .set(&ChangeSpec::new(ChangeType::New).description("Reopen source CL"))
        .run()
        .expect("Failed to create change");

    let cl2 = test_client
        .p4
        .change()
        .set(&ChangeSpec::new(ChangeType::New).description("Reopen target CL"))
        .run()
        .expect("Failed to create target change");

    test_client
        .p4
        .add(&[file1_str, file2_str])
        .changelist(cl1)
        .run()
        .expect("Failed to add files");

    let reopen_result = test_client
        .p4
        .reopen(&[file1_str, file2_str])
        .changelist(cl2)
        .run()
        .expect("Failed to reopen");
    assert_eq!(reopen_result.len(), 2);
    assert!(reopen_result[0]
        .change
        .as_ref()
        .unwrap()
        .contains(&cl2.to_string()));

    let opened = test_client
        .p4
        .opened(&["//..."])
        .run()
        .expect("Failed to get opened");
    assert_eq!(opened.len(), 2);
    assert!(opened.iter().all(|f| f.change.number() == Some(cl2)));

    let reopen_type = test_client
        .p4
        .reopen(&[file1_str])
        .file_type(p4rs::FileType::binary())
        .run()
        .expect("Failed to reopen with filetype");
    assert_eq!(reopen_type.len(), 1);
    assert_eq!(reopen_type[0].file_type.as_deref(), Some("binary"));

    test_client
        .p4
        .revert(&["//..."])
        .run()
        .expect("Failed to revert");
}

#[test]
fn test_delete() {
    let test_client = SERVER.test_client();
    let file1 = test_client.client_root().join("delete_test1.txt");
    let file2 = test_client.client_root().join("delete_test2.txt");
    fs::write(&file1, "delete content 1").expect("Failed to write file1");
    fs::write(&file2, "delete content 2").expect("Failed to write file2");
    let file1_str = file1.to_str().unwrap();
    let file2_str = file2.to_str().unwrap();

    let cl = test_client
        .p4
        .change()
        .set(&ChangeSpec::new(ChangeType::New).description("Delete test"))
        .run()
        .expect("Failed to create change");

    test_client
        .p4
        .add(&[file1_str, file2_str])
        .changelist(cl)
        .run()
        .expect("Failed to add files");

    test_client.p4.submit(cl).run().expect("Failed to submit");

    let delete_cl = test_client
        .p4
        .change()
        .set(&ChangeSpec::new(ChangeType::New).description("Delete CL"))
        .run()
        .expect("Failed to create delete change");

    let delete_result = test_client
        .p4
        .delete(&[file1_str, file2_str])
        .changelist(delete_cl)
        .run()
        .expect("Failed to delete");
    assert_eq!(delete_result.len(), 2);
    assert!(delete_result.iter().all(|r| r.action == "delete"));

    let opened = test_client
        .p4
        .opened(&["//..."])
        .run()
        .expect("Failed to get opened");
    assert_eq!(opened.len(), 2);
    assert!(opened.iter().all(|f| f.action == OpenAction::Delete));

    test_client
        .p4
        .revert(&["//..."])
        .run()
        .expect("Failed to revert");

    let preview_result = test_client
        .p4
        .delete(&[file1_str])
        .preview()
        .run()
        .expect("Failed to preview delete");
    assert_eq!(preview_result.len(), 1);

    let opened_after = test_client
        .p4
        .opened(&["//..."])
        .run()
        .expect("Failed to get opened after preview");
    assert!(opened_after.is_empty());
}

#[test]
fn test_describe() {
    let test_client = SERVER.test_client();
    let file1 = test_client.client_root().join("describe_add.txt");
    let file2 = test_client.client_root().join("describe_edit.txt");
    let file3 = test_client.client_root().join("describe_delete.txt");
    let file4 = test_client.client_root().join("describe_move_src.txt");
    fs::write(&file1, "add").expect("Failed to write file1");
    fs::write(&file2, "edit").expect("Failed to write file2");
    fs::write(&file3, "delete").expect("Failed to write file3");
    fs::write(&file4, "move").expect("Failed to write file4");

    let add_cl = test_client
        .p4
        .change()
        .set(&ChangeSpec::new(ChangeType::New).description("Add files"))
        .run()
        .expect("Failed to create add change");
    test_client
        .p4
        .add(&[
            file1.to_str().unwrap(),
            file2.to_str().unwrap(),
            file3.to_str().unwrap(),
            file4.to_str().unwrap(),
        ])
        .changelist(add_cl)
        .run()
        .expect("Failed to add files");
    let add_submitted = test_client
        .p4
        .submit(add_cl)
        .run()
        .expect("Failed to submit add");

    let add_desc = test_client
        .p4
        .describe(&[add_submitted.change])
        .short()
        .run()
        .expect("Failed to describe add");
    assert_eq!(add_desc.len(), 1);
    assert_eq!(add_desc[0].files.len(), 4);
    assert!(add_desc[0]
        .files
        .iter()
        .all(|f| f.action == FileAction::Add));

    let edit_cl = test_client
        .p4
        .change()
        .set(&ChangeSpec::new(ChangeType::New).description("Edit file"))
        .run()
        .expect("Failed to create edit change");
    test_client
        .p4
        .edit(&[file2.to_str().unwrap()])
        .changelist(edit_cl)
        .run()
        .expect("Failed to edit");
    fs::write(&file2, "edited content").expect("Failed to update file2");
    let edit_submitted = test_client
        .p4
        .submit(edit_cl)
        .run()
        .expect("Failed to submit edit");

    let edit_desc = test_client
        .p4
        .describe(&[edit_submitted.change])
        .short()
        .run()
        .expect("Failed to describe edit");
    assert_eq!(edit_desc[0].files.len(), 1);
    assert_eq!(edit_desc[0].files[0].action, FileAction::Edit);

    let delete_cl = test_client
        .p4
        .change()
        .set(&ChangeSpec::new(ChangeType::New).description("Delete file"))
        .run()
        .expect("Failed to create delete change");
    test_client
        .p4
        .delete(&[file3.to_str().unwrap()])
        .changelist(delete_cl)
        .run()
        .expect("Failed to delete");
    let delete_submitted = test_client
        .p4
        .submit(delete_cl)
        .run()
        .expect("Failed to submit delete");

    let delete_desc = test_client
        .p4
        .describe(&[delete_submitted.change])
        .short()
        .run()
        .expect("Failed to describe delete");
    assert_eq!(delete_desc[0].files.len(), 1);
    assert_eq!(delete_desc[0].files[0].action, FileAction::Delete);

    let move_cl = test_client
        .p4
        .change()
        .set(&ChangeSpec::new(ChangeType::New).description("Move file"))
        .run()
        .expect("Failed to create move change");
    let file4_dest = test_client.client_root().join("describe_move_dst.txt");
    test_client
        .p4
        .edit(&[file4.to_str().unwrap()])
        .changelist(move_cl)
        .run()
        .expect("Failed to edit for move");
    std::process::Command::new("p4")
        .args(["-p", &format!("localhost:{}", SERVER.port)])
        .args(["-c", &test_client.client_name])
        .args(["move", "-c", &move_cl.to_string()])
        .arg(file4.to_str().unwrap())
        .arg(file4_dest.to_str().unwrap())
        .output()
        .expect("Failed to move");
    let move_submitted = test_client
        .p4
        .submit(move_cl)
        .run()
        .expect("Failed to submit move");

    let move_desc = test_client
        .p4
        .describe(&[move_submitted.change])
        .short()
        .run()
        .expect("Failed to describe move");
    assert_eq!(move_desc[0].files.len(), 2);
    assert!(move_desc[0]
        .files
        .iter()
        .any(|f| f.action == FileAction::MoveDelete));
    assert!(move_desc[0]
        .files
        .iter()
        .any(|f| f.action == FileAction::MoveAdd));

    let multi_desc = test_client
        .p4
        .describe(&[add_submitted.change, edit_submitted.change])
        .short()
        .run()
        .expect("Failed to describe multiple");
    assert_eq!(multi_desc.len(), 2);
}

#[test]
fn test_print_content() {
    let test_client = SERVER.test_client();
    let file_path = test_client.client_root().join("print_test.txt");
    let content = "Hello, this is test content for print command.\nLine 2.\nLine 3.";
    fs::write(&file_path, content).expect("Failed to write file");
    let file_str = file_path.to_str().unwrap();

    let cl = test_client
        .p4
        .change()
        .set(&ChangeSpec::new(ChangeType::New).description("Print test"))
        .run()
        .expect("Failed to create change");
    test_client
        .p4
        .add(&[file_str])
        .changelist(cl)
        .run()
        .expect("Failed to add");
    let submitted = test_client.p4.submit(cl).run().expect("Failed to submit");

    // Test basic print content
    let results = test_client
        .p4
        .print()
        .content(&[&format!("//...@={}", submitted.change)])
        .run()
        .expect("Failed to print");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].data, content);
    assert_eq!(results[0].info.rev, 1);

    // Test print with offset and size
    let partial = test_client
        .p4
        .print()
        .content(&[&format!("//...@={}", submitted.change)])
        .offset(7)
        .size(4)
        .run()
        .expect("Failed to print with offset/size");
    assert_eq!(partial.len(), 1);
    assert_eq!(partial[0].data, "this");
}

#[test]
fn test_print_to_file() {
    let test_client = SERVER.test_client();
    let file1 = test_client.client_root().join("print_file1.txt");
    let file2 = test_client.client_root().join("print_file2.txt");
    fs::write(&file1, "File 1 content").expect("Failed to write file1");
    fs::write(&file2, "File 2 content").expect("Failed to write file2");

    let cl = test_client
        .p4
        .change()
        .set(&ChangeSpec::new(ChangeType::New).description("Print to file test"))
        .run()
        .expect("Failed to create change");
    test_client
        .p4
        .add(&[file1.to_str().unwrap(), file2.to_str().unwrap()])
        .changelist(cl)
        .run()
        .expect("Failed to add");
    let submitted = test_client.p4.submit(cl).run().expect("Failed to submit");

    // Create output directory
    let output_dir = test_client.client_root().join("print_output");
    fs::create_dir_all(&output_dir).expect("Failed to create output dir");

    // Test print to file - print specific files to specific locations
    let output_file1 = output_dir.join("out1.txt");
    let depot_file1 = format!(
        "//depot/{}/print_file1.txt@={}",
        test_client.client_name, submitted.change
    );
    let results = test_client
        .p4
        .print()
        .to_file(&[&depot_file1], output_file1.to_str().unwrap())
        .run()
        .expect("Failed to print to file");
    assert_eq!(results.len(), 1);
    assert!(results[0].depot_file.contains("print_file1.txt"));

    // Verify file was written
    let written_content = fs::read_to_string(&output_file1).expect("Failed to read output file");
    assert_eq!(written_content, "File 1 content");
}

#[test]
fn test_print_to_file_unmapped_path() {
    let test_client = SERVER.test_client();
    let file_path = test_client.client_root().join("print_unmapped.txt");
    fs::write(&file_path, "Unmapped test").expect("Failed to write file");
    let file_str = file_path.to_str().unwrap();

    let cl = test_client
        .p4
        .change()
        .set(&ChangeSpec::new(ChangeType::New).description("Print unmapped test"))
        .run()
        .expect("Failed to create change");
    test_client
        .p4
        .add(&[file_str])
        .changelist(cl)
        .run()
        .expect("Failed to add");
    let submitted = test_client.p4.submit(cl).run().expect("Failed to submit");

    // Test print to file with @= syntax (no depot path prefix) - fails to map
    let output_pattern = "./nonexistent_output/...";
    let result = test_client
        .p4
        .print()
        .to_file(&[&format!("@={}", submitted.change)], output_pattern)
        .run();

    // Should fail with an error about mapping
    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_str = format!("{:?}", err);
    assert!(
        err_str.contains("map"),
        "Expected mapping error, got: {}",
        err_str
    );
}

#[test]
fn test_sync() {
    let test_client = SERVER.test_client();
    let file_path = test_client.client_root().join("sync_test.txt");
    fs::write(&file_path, "sync content").expect("Failed to write file");
    let file_str = file_path.to_str().unwrap();

    // Add and submit a file
    let cl = test_client
        .p4
        .change()
        .set(&ChangeSpec::new(ChangeType::New).description("Sync test"))
        .run()
        .expect("Failed to create change");
    test_client
        .p4
        .add(&[file_str])
        .changelist(cl)
        .run()
        .expect("Failed to add");
    test_client.p4.submit(cl).run().expect("Failed to submit");

    // Sync to #none to remove the file from workspace
    test_client
        .p4
        .sync(&[&format!("{}#none", file_str)])
        .run()
        .expect("Failed to sync to #none");
    assert!(!file_path.exists());

    // Test sync preview (should not actually sync)
    let preview_results = test_client
        .p4
        .sync(&[file_str])
        .preview()
        .run()
        .expect("Failed to sync preview");
    assert_eq!(preview_results.len(), 1);
    assert!(preview_results[0].depot_file.contains("sync_test.txt"));
    // File should still be missing after preview
    assert!(!file_path.exists());

    // Test actual sync
    let results = test_client
        .p4
        .sync(&[file_str])
        .run()
        .expect("Failed to sync");
    assert_eq!(results.len(), 1);
    assert!(results[0].depot_file.contains("sync_test.txt"));
    assert_eq!(results[0].rev, 1);
    // File should exist after sync
    assert!(file_path.exists());

    // Test force sync (should re-sync even though file exists)
    let force_results = test_client
        .p4
        .sync(&[file_str])
        .force()
        .run()
        .expect("Failed to force sync");
    assert_eq!(force_results.len(), 1);
}

#[test]
fn test_sync_metadata_only() {
    let test_client = SERVER.test_client();
    let file_path = test_client.client_root().join("sync_metadata.txt");

    // Create and submit rev 1
    fs::write(&file_path, "revision 1 content").expect("Failed to write file");
    let file_str = file_path.to_str().unwrap();
    let cl1 = test_client
        .p4
        .change()
        .set(&ChangeSpec::new(ChangeType::New).description("Rev 1"))
        .run()
        .expect("Failed to create change 1");
    test_client
        .p4
        .add(&[file_str])
        .changelist(cl1)
        .run()
        .expect("Failed to add");
    test_client.p4.submit(cl1).run().expect("Failed to submit rev 1");

    // Edit and submit rev 2
    test_client
        .p4
        .edit(&[file_str])
        .run()
        .expect("Failed to edit");
    fs::write(&file_path, "revision 2 content").expect("Failed to write rev 2");
    let cl2 = test_client
        .p4
        .change()
        .set(&ChangeSpec::new(ChangeType::New).description("Rev 2"))
        .run()
        .expect("Failed to create change 2");
    test_client
        .p4
        .reopen(&[file_str])
        .changelist(cl2)
        .run()
        .expect("Failed to reopen");
    test_client.p4.submit(cl2).run().expect("Failed to submit rev 2");

    // Verify we're at rev 2
    let rev2_content = fs::read_to_string(&file_path).expect("Failed to read");
    assert_eq!(rev2_content, "revision 2 content");

    // Use metadata_only to update have list to rev 1 without changing file
    let metadata_results = test_client
        .p4
        .sync(&[&format!("{}#1", file_str)])
        .metadata_only()
        .run()
        .expect("Failed to sync metadata only");
    assert_eq!(metadata_results.len(), 1);
    assert_eq!(metadata_results[0].rev, 1);

    // File should still have rev 2 content (not overwritten)
    let after_metadata = fs::read_to_string(&file_path).expect("Failed to read after metadata sync");
    assert_eq!(after_metadata, "revision 2 content");

    // Verify have list shows rev 1 by checking opened
    let have = test_client
        .p4
        .sync(&[file_str])
        .preview()
        .run()
        .expect("Failed to check have");
    // Should want to sync to rev 2 since we're at rev 1 in have list
    assert_eq!(have.len(), 1);
    assert_eq!(have[0].rev, 2);
}

#[test]
fn test_user() {
    let test_client = SERVER.test_client();

    // Get current user from info
    let info = test_client.p4.info().run().expect("Failed to get info");
    let username = &info.user_name;

    // Get user info
    let user_info = test_client
        .p4
        .user()
        .get(username)
        .run()
        .expect("Failed to get user");

    // Just verify fields have content, don't assert specific values
    assert!(!user_info.user.is_empty());
    assert!(!user_info.email.is_empty());
    assert!(!user_info.full_name.is_empty());
}

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
fn test_unknown_command_returns_command_error() {
    let p4 = SERVER.p4();
    let result = p4.run(p4.build_cmd("inf", CmdType::Query));
    assert!(matches!(result, Err(P4Error::Command { .. })));
    if let Err(P4Error::Command { errors, .. }) = result {
        assert!(errors[0].data.starts_with("Unknown command"));
        assert!(errors[0].is_error());
    }
}

#[test]
fn test_info() {
    let p4 = SERVER.p4();
    let info = p4.info().short().run().expect("Failed to get info").single().expect("Expected single result");
    assert!(!info.user_name.is_empty());
}

#[test]
fn test_changes() {
    let test_client = SERVER.test_client();
    let cl1 = test_client
        .p4
        .change()
        .set(&ChangeSpec::new(ChangeType::New).description("Test change"))
        .run()
        .expect("Failed to create change")
        .single()
        .unwrap();
    let cl2 = test_client
        .p4
        .change()
        .set(&ChangeSpec::new(ChangeType::New).description("Test change 2"))
        .run()
        .expect("Failed to create change")
        .single()
        .unwrap();
    let changes = test_client
        .p4
        .changes(&[])
        .long()
        .client(&test_client.client_name)
        .run()
        .expect("Failed to get changes");
    assert_eq!(changes.len(), 2);
    test_client.p4.change().delete(cl1).run().unwrap();
    test_client.p4.change().delete(cl2).run().unwrap();
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
        .expect("Failed to set change")
        .single()
        .expect("Expected single result");
    assert!(change_number > 0);
    let result_spec = test_client
        .p4
        .change()
        .get(Some(change_number))
        .run()
        .expect("Failed to get change")
        .single()
        .expect("Expected single result");
    assert!(result_spec.description.trim() == change_spec.description.trim());
    test_client.p4.change().delete(change_number).run().unwrap();
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
        .expect("Failed to create change")
        .single()
        .expect("Expected single result");

    test_client
        .p4
        .change()
        .delete(change_number)
        .run()
        .expect("Failed to delete change");

    let result = test_client.p4.change().get(Some(change_number)).run();
    assert!(matches!(result, Err(P4Error::Command { .. })));
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
    let file_str = file_path.to_str().unwrap();

    test_client.changelist("Add file for edit test")
        .add_file("edit_test.txt", b"initial content")
        .submit()
        .unwrap();

    let edit_results = test_client.p4.edit(&[file_str]).run().expect("Failed to edit file");
    assert_eq!(edit_results.len(), 1);
    assert_eq!(edit_results[0].action, EditAction::Edit);

    let opened = test_client.p4.opened(&["//..."]).run().expect("Failed to get opened files");
    assert_eq!(opened.len(), 1);
    assert_eq!(opened[0].action, OpenAction::Edit);

    test_client.p4.revert(&[file_str]).run().expect("Failed to revert");
}

#[test]
fn test_edit_with_changelist() {
    let test_client = SERVER.test_client();
    let file_path = test_client.client_root().join("cl_test.txt");
    let file_str = file_path.to_str().unwrap();

    test_client.changelist("Add file")
        .add_file("cl_test.txt", b"content")
        .submit()
        .unwrap();

    let edit_cl = test_client
        .p4
        .change()
        .set(&ChangeSpec::new(ChangeType::New).description("Edit changelist"))
        .run()
        .expect("Failed to create edit changelist")
        .single()
        .expect("Expected single result");

    let results = test_client
        .p4
        .edit(&[file_str])
        .changelist(edit_cl)
        .run()
        .expect("Failed to edit with changelist");
    assert_eq!(results.len(), 1);

    let opened = test_client.p4.opened(&["//..."]).run().expect("Failed to get opened files");
    assert_eq!(opened.len(), 1);
    assert_eq!(opened[0].change.number(), Some(edit_cl));

    test_client.p4.revert(&["//..."]).run().expect("Failed to revert");
    test_client.p4.change().delete(edit_cl).run().unwrap();
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
        .expect("Failed to create change")
        .single()
        .expect("Expected single result");

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
        .expect("Failed to get change spec")
        .single()
        .expect("Expected single result");
    assert_eq!(spec.description.trim(), "Multi-file change");
    assert_eq!(spec.files.len(), 2);

    test_client.p4.revert(&["//..."]).run().expect("Failed to revert");
    test_client.p4.change().delete(cl).run().unwrap();
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
        .expect("Failed to create client")
        .single()
        .expect("Expected single result");
    assert!(!client_name.is_empty());
    let result_spec = p4
        .client()
        .get(Some(&client_name))
        .run()
        .expect("Failed to get client spec")
        .single()
        .expect("Expected single result");
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
        .expect("Failed to create change")
        .single()
        .expect("Expected single result");

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
    let file_str = file_path.to_str().unwrap();

    test_client.changelist("Add")
        .add_file("have_rev_test.txt", b"content")
        .submit()
        .unwrap();

    test_client.p4.edit(&[file_str]).run().expect("Failed to edit");
    let opened = test_client.p4.opened(&["//..."]).run().expect("Failed to get opened");
    assert_eq!(opened.len(), 1);
    assert_eq!(opened[0].have_rev, Some(1));

    test_client.p4.revert(&["//..."]).run().expect("Failed to revert");
}

#[test]
fn test_revert_unchanged() {
    let test_client = SERVER.test_client();
    let file_path = test_client.client_root().join("unchanged_test.txt");
    let file_str = file_path.to_str().unwrap();

    test_client.changelist("Add")
        .add_file("unchanged_test.txt", b"content")
        .submit()
        .unwrap();

    test_client.p4.edit(&[file_str]).run().expect("Failed to edit");
    let reverted = test_client.p4.revert(&["//..."]).unchanged().run().expect("Failed to revert");
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
    let file_str = file_path.to_str().unwrap();

    test_client.changelist("Add")
        .add_file("preview_test.txt", b"content")
        .submit()
        .unwrap();

    let results = test_client.p4.edit(&[file_str]).preview().run().expect("Failed to preview edit");
    assert_eq!(results.len(), 1);

    let opened = test_client.p4.opened(&["//..."]).run().expect("Failed to get opened");
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
        .expect("Failed to create change")
        .single()
        .expect("Expected single result");

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
    assert!(delete_result.single().expect("Expected single result").data.contains("deleted"));

    test_client.p4.revert(&["//..."]).run().expect("Failed to revert");
    test_client.p4.change().delete(cl).run().unwrap();
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
        .expect("Failed to create change")
        .single()
        .expect("Expected single result");

    let cl2 = test_client
        .p4
        .change()
        .set(&ChangeSpec::new(ChangeType::New).description("Reopen target CL"))
        .run()
        .expect("Failed to create target change")
        .single()
        .expect("Expected single result");

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
    assert_eq!(reopen_type[0].file_type, Some(p4rs::FileType::binary()));

    test_client.p4.revert(&["//..."]).run().expect("Failed to revert");
    test_client.p4.change().delete(cl1).run().unwrap();
    test_client.p4.change().delete(cl2).run().unwrap();
}

#[test]
fn test_symlink() {
    use std::os::unix::fs::symlink;

    let test_client = SERVER.test_client();
    let target_file = test_client.client_root().join("symlink_target.txt");
    let link_file = test_client.client_root().join("symlink_link.txt");
    fs::write(&target_file, "target content").expect("Failed to write target file");
    symlink(&target_file, &link_file).expect("Failed to create symlink");

    let target_str = target_file.to_str().unwrap();
    let link_str = link_file.to_str().unwrap();

    let cl = test_client
        .p4
        .change()
        .set(&ChangeSpec::new(ChangeType::New).description("Symlink test"))
        .run()
        .expect("Failed to create change")
        .single()
        .expect("Expected single result");

    test_client
        .p4
        .add(&[target_str])
        .changelist(cl)
        .run()
        .expect("Failed to add target file");

    let add_result = test_client
        .p4
        .add(&[link_str])
        .changelist(cl)
        .file_type(p4rs::FileType::symlink())
        .run()
        .expect("Failed to add symlink");
    assert_eq!(add_result.len(), 1);
    assert_eq!(add_result[0].file_type.base, p4rs::BaseFileType::Symlink);

    let opened = test_client
        .p4
        .opened(&["//..."])
        .run()
        .expect("Failed to get opened");
    let link_opened = opened
        .iter()
        .find(|f| f.client_file.contains("symlink_link"))
        .unwrap();
    assert_eq!(link_opened.file_type.base, p4rs::BaseFileType::Symlink);

    let submit = test_client.p4.submit(cl).run().expect("Failed to submit").single().expect("Expected submit result");
    assert!(submit.submitted_change > 0);

    let describe = test_client
        .p4
        .describe(&[submit.submitted_change])
        .run()
        .expect("Failed to describe");
    let link_desc = describe[0]
        .files
        .iter()
        .find(|f| f.depot_file.contains("symlink_link"))
        .unwrap();
    assert_eq!(link_desc.file_type.base, p4rs::BaseFileType::Symlink);
}

#[test]
fn test_delete() {
    let test_client = SERVER.test_client();
    let file1 = test_client.client_root().join("delete_test1.txt");
    let file2 = test_client.client_root().join("delete_test2.txt");
    let file1_str = file1.to_str().unwrap();
    let file2_str = file2.to_str().unwrap();

    test_client.changelist("Delete test")
        .add_file("delete_test1.txt", b"delete content 1")
        .add_file("delete_test2.txt", b"delete content 2")
        .submit()
        .unwrap();

    let delete_cl = test_client
        .p4
        .change()
        .set(&ChangeSpec::new(ChangeType::New).description("Delete CL"))
        .run()
        .expect("Failed to create delete change")
        .single()
        .expect("Expected single result");

    let delete_result = test_client
        .p4
        .delete(&[file1_str, file2_str])
        .changelist(delete_cl)
        .run()
        .expect("Failed to delete");
    assert_eq!(delete_result.len(), 2);
    assert!(delete_result.iter().all(|r| r.action == "delete"));

    let opened = test_client.p4.opened(&["//..."]).run().expect("Failed to get opened");
    assert_eq!(opened.len(), 2);
    assert!(opened.iter().all(|f| f.action == OpenAction::Delete));

    test_client.p4.revert(&["//..."]).run().expect("Failed to revert");
    test_client.p4.change().delete(delete_cl).run().unwrap();

    let preview_result = test_client.p4.delete(&[file1_str]).preview().run().expect("Failed to preview delete");
    assert_eq!(preview_result.len(), 1);

    let opened_after = test_client.p4.opened(&["//..."]).run().expect("Failed to get opened after preview");
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
        .expect("Failed to create add change")
        .single()
        .expect("Expected single result");
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
        .expect("Failed to submit add")
        .single()
        .expect("Expected submit result");

    let add_desc = test_client
        .p4
        .describe(&[add_submitted.submitted_change])
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
        .expect("Failed to create edit change")
        .single()
        .expect("Expected single result");
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
        .expect("Failed to submit edit")
        .single()
        .expect("Expected submit result");

    let edit_desc = test_client
        .p4
        .describe(&[edit_submitted.submitted_change])
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
        .expect("Failed to create delete change")
        .single()
        .expect("Expected single result");
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
        .expect("Failed to submit delete")
        .single()
        .expect("Expected submit result");

    let delete_desc = test_client
        .p4
        .describe(&[delete_submitted.submitted_change])
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
        .expect("Failed to create move change")
        .single()
        .expect("Expected single result");
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
        .expect("Failed to submit move")
        .single()
        .expect("Expected submit result");

    let move_desc = test_client
        .p4
        .describe(&[move_submitted.submitted_change])
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
        .describe(&[add_submitted.submitted_change, edit_submitted.submitted_change])
        .short()
        .run()
        .expect("Failed to describe multiple");
    assert_eq!(multi_desc.len(), 2);
}

#[test]
fn test_print_content() {
    let test_client = SERVER.test_client();
    let content = "Hello, this is test content for print command.\nLine 2.\nLine 3.";

    let submitted = test_client.changelist("Print test")
        .add_file("print_test.txt", content.as_bytes())
        .submit()
        .unwrap().submitted_change;

    let results = test_client
        .p4
        .print()
        .content(&[&format!("//...@={}", submitted)])
        .run()
        .expect("Failed to print");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].data, content);
    assert_eq!(results[0].info.rev, 1);

    let partial = test_client
        .p4
        .print()
        .content(&[&format!("//...@={}", submitted)])
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

    let submitted = test_client.changelist("Print to file test")
        .add_file("print_file1.txt", b"File 1 content")
        .add_file("print_file2.txt", b"File 2 content")
        .submit()
        .unwrap().submitted_change;

    let output_dir = test_client.client_root().join("print_output");
    fs::create_dir_all(&output_dir).expect("Failed to create output dir");

    let output_file1 = output_dir.join("out1.txt");
    let depot_file1 = format!("//depot/{}/print_file1.txt@={}", test_client.client_name, submitted);
    let results = test_client
        .p4
        .print()
        .to_file(&[&depot_file1], output_file1.to_str().unwrap())
        .run()
        .expect("Failed to print to file");
    assert_eq!(results.len(), 1);
    assert!(results[0].depot_file.contains("print_file1.txt"));

    let written_content = fs::read_to_string(&output_file1).expect("Failed to read output file");
    assert_eq!(written_content, "File 1 content");
}

#[test]
fn test_print_to_file_unmapped_path() {
    let test_client = SERVER.test_client();

    let submitted = test_client.changelist("Print unmapped test")
        .add_file("print_unmapped.txt", b"Unmapped test")
        .submit()
        .unwrap().submitted_change;

    let output_pattern = "./nonexistent_output/...";
    let result = test_client
        .p4
        .print()
        .to_file(&[&format!("@={}", submitted)], output_pattern)
        .run();

    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_str = format!("{:?}", err);
    assert!(err_str.contains("map"), "Expected mapping error, got: {}", err_str);
}

#[test]
fn test_sync() {
    let test_client = SERVER.test_client();
    let file_path = test_client.client_root().join("sync_test.txt");
    let file_str = file_path.to_str().unwrap();

    test_client.changelist("Sync test")
        .add_file("sync_test.txt", b"sync content")
        .submit()
        .unwrap();

    test_client.p4.sync(&[&format!("{}#none", file_str)]).run().expect("Failed to sync to #none");
    assert!(!file_path.exists());

    let preview_results = test_client.p4.sync(&[file_str]).preview().run().expect("Failed to sync preview");
    assert_eq!(preview_results.len(), 1);
    assert!(preview_results[0].depot_file.contains("sync_test.txt"));
    assert!(!file_path.exists());

    let results = test_client.p4.sync(&[file_str]).run().expect("Failed to sync");
    assert_eq!(results.len(), 1);
    assert!(results[0].depot_file.contains("sync_test.txt"));
    assert_eq!(results[0].rev, 1);
    assert!(file_path.exists());

    let force_results = test_client.p4.sync(&[file_str]).force().run().expect("Failed to force sync");
    assert_eq!(force_results.len(), 1);
}

#[test]
fn test_sync_metadata_only() {
    let test_client = SERVER.test_client();
    let file_path = test_client.client_root().join("sync_metadata.txt");
    let file_str = file_path.to_str().unwrap();

    test_client.changelist("Rev 1")
        .add_file("sync_metadata.txt", b"revision 1 content")
        .submit()
        .unwrap();

    test_client.changelist("Rev 2")
        .edit_file("sync_metadata.txt", b"revision 2 content")
        .submit()
        .unwrap();

    let rev2_content = fs::read_to_string(&file_path).expect("Failed to read");
    assert_eq!(rev2_content, "revision 2 content");

    let metadata_results = test_client
        .p4
        .sync(&[&format!("{}#1", file_str)])
        .metadata_only()
        .run()
        .expect("Failed to sync metadata only");
    assert_eq!(metadata_results.len(), 1);
    assert_eq!(metadata_results[0].rev, 1);

    let after_metadata = fs::read_to_string(&file_path).expect("Failed to read after metadata sync");
    assert_eq!(after_metadata, "revision 2 content");

    let have = test_client.p4.sync(&[file_str]).preview().run().expect("Failed to check have");
    assert_eq!(have.len(), 1);
    assert_eq!(have[0].rev, 2);
}

#[test]
fn test_user() {
    let test_client = SERVER.test_client();

    let info = test_client.p4.info().run().expect("Failed to get info").single().expect("Expected single result");
    let username = &info.user_name;

    let user_info = test_client
        .p4
        .user()
        .get(username)
        .run()
        .expect("Failed to get user")
        .single()
        .expect("Expected single result");

    assert!(!user_info.user.is_empty());
    assert!(!user_info.email.is_empty());
    assert!(!user_info.full_name.is_empty());
}

#[test]
fn test_where() {
    let test_client = SERVER.test_client();
    let client_root = test_client.client_root().to_str().unwrap();

    let file1 = test_client.client_root().join("file1.txt");
    let file2 = test_client.client_root().join("file2.txt");
    std::fs::write(&file1, "content1").unwrap();
    std::fs::write(&file2, "content2").unwrap();

    let results = test_client
        .p4
        .where_cmd(&[file1.to_str().unwrap(), file2.to_str().unwrap()])
        .run()
        .expect("Failed to run where");

    assert_eq!(results.len(), 2);
    for result in results.iter() {
        assert!(result.depot_file.starts_with("//depot/"));
        assert!(result.client_file.starts_with("//"));
        assert!(result.path.starts_with(client_root));
    }

    let results = test_client
        .p4
        .where_cmd(&["//depot/..."])
        .run()
        .expect("Failed to run where with depot path");

    assert_eq!(results.len(), 1);
    assert!(results[0].depot_file.starts_with("//depot/"));
}

#[test]
fn test_print_deleted_file() {
    let test_client = SERVER.test_client();

    test_client.changelist("Add file")
        .add_file("print_delete_test.txt", b"content to delete")
        .submit()
        .unwrap();

    let delete_submitted = test_client.changelist("Delete file")
        .delete_file("print_delete_test.txt")
        .submit()
        .unwrap().submitted_change;

    let output_dir = test_client.client_root().join("print_delete_output");
    fs::create_dir_all(&output_dir).expect("Failed to create output dir");

    let results = test_client
        .p4
        .print()
        .to_file(
            &[&format!("//...@={}", delete_submitted)],
            format!("{}/...", output_dir.display()).as_str(),
        )
        .run()
        .expect("Failed to print deleted file");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].action, FileAction::Delete);
    assert_eq!(results[0].file_size, None);
}

#[test]
fn test_move_file() {
    let test_client = SERVER.test_client();
    test_client.changelist("Add file for move test")
        .add_file("move_source.txt", b"original content")
        .submit()
        .unwrap();

    let move_cl = test_client
        .p4
        .change()
        .set(&ChangeSpec::new(ChangeType::New).description("Move file"))
        .run()
        .expect("Failed to create move changelist")
        .single()
        .expect("Expected single result");

    let source_path = test_client.client_root().join("move_source.txt");
    let dest_path = test_client.client_root().join("move_dest.txt");
    let source_str = source_path.to_str().unwrap();
    let dest_str = dest_path.to_str().unwrap();

    test_client.p4.edit(&[source_str])
        .changelist(move_cl)
        .run()
        .expect("Failed to open file for edit");

    let move_results = test_client
        .p4
        .move_file(source_str, dest_str)
        .changelist(move_cl)
        .run()
        .expect("Failed to move file");

    assert_eq!(move_results.len(), 1);
    let move_add = move_results.iter().find(|r| r.action == FileAction::MoveAdd);
    assert!(move_add.is_some());
    assert!(move_add.unwrap().depot_file.ends_with("move_dest.txt"));

    let opened = test_client
        .p4
        .opened(&["//..."])
        .run()
        .expect("Failed to get opened files");
    assert_eq!(opened.len(), 2);

    test_client.p4.revert(&["//..."]).run().expect("Failed to revert");
    test_client.p4.change().delete(move_cl).run().unwrap();
}

#[test]
fn test_files() {
    let test_client = SERVER.test_client();

    test_client.changelist("Add files for test")
        .add_file("test_file1.txt", b"content1")
        .add_file("test_file2.txt", b"content2")
        .submit()
        .unwrap();

    let files = test_client
        .p4
        .files(&["//..."])
        .run()
        .expect("Failed to get files");

    assert!(files.len() >= 2);
    assert!(files.iter().any(|f| f.depot_file.ends_with("test_file1.txt")));
    assert!(files.iter().any(|f| f.depot_file.ends_with("test_file2.txt")));

    let file1 = files.iter().find(|f| f.depot_file.ends_with("test_file1.txt")).unwrap();
    assert_eq!(file1.rev, 1);
    assert_eq!(file1.action, "add");
}

#[test]
fn test_changelist_builder_batched_add() {
    let test_client = SERVER.test_client();
    let result = test_client.changelist("Batched add test")
        .add_file("batch_add1.txt", b"content1")
        .add_file("batch_add2.txt", b"content2")
        .add_file("batch_add3.txt", b"content3")
        .submit()
        .unwrap();

    assert!(result.submitted_change > 0);

    let desc = test_client.p4.describe(&[result.submitted_change]).run().expect("Failed to describe");
    assert_eq!(desc[0].files.len(), 3);
}

#[test]
fn test_changelist_builder_batched_edit() {
    let test_client = SERVER.test_client();
    test_client.changelist("Setup for edit")
        .add_file("batch_edit1.txt", b"original1")
        .add_file("batch_edit2.txt", b"original2")
        .submit()
        .unwrap();

    let result = test_client.changelist("Batched edit test")
        .edit_file("batch_edit1.txt", b"modified1")
        .edit_file("batch_edit2.txt", b"modified2")
        .submit()
        .unwrap();

    assert!(result.submitted_change > 0);
    let desc = test_client.p4.describe(&[result.submitted_change]).run().expect("Failed to describe");
    assert_eq!(desc[0].files.len(), 2);
}

#[test]
fn test_changelist_builder_batched_delete() {
    let test_client = SERVER.test_client();
    test_client.changelist("Setup for delete")
        .add_file("batch_del1.txt", b"content1")
        .add_file("batch_del2.txt", b"content2")
        .submit()
        .unwrap();

    let result = test_client.changelist("Batched delete test")
        .delete_file("batch_del1.txt")
        .delete_file("batch_del2.txt")
        .submit()
        .unwrap();

    assert!(result.submitted_change > 0);
    let desc = test_client.p4.describe(&[result.submitted_change]).run().expect("Failed to describe");
    assert_eq!(desc[0].files.len(), 2);
    assert!(desc[0].files.iter().all(|f| f.action == FileAction::Delete));
}

#[test]
fn test_changelist_builder_mixed_operations() {
    let test_client = SERVER.test_client();
    test_client.changelist("Setup for mixed")
        .add_file("mix_edit.txt", b"original")
        .add_file("mix_del.txt", b"to delete")
        .submit()
        .unwrap();

    let result = test_client.changelist("Mixed operations")
        .add_file("mix_add.txt", b"new file")
        .edit_file("mix_edit.txt", b"modified")
        .delete_file("mix_del.txt")
        .submit()
        .unwrap();

    assert!(result.submitted_change > 0);
    let desc = test_client.p4.describe(&[result.submitted_change]).run().expect("Failed to describe");
    assert_eq!(desc[0].files.len(), 3);
}

#[test]
fn test_changelist_builder_immediate_mode() {
    let test_client = SERVER.test_client();
    let root = test_client.client_root().to_path_buf();

    fs::write(root.join("imm1.txt"), "immediate1").unwrap();
    fs::write(root.join("imm2.txt"), "immediate2").unwrap();

    let mut builder = p4rs::ChangelistBuilder::new(&test_client.p4, root.clone(), "Immediate mode test")
        .expect("Failed to create builder")
        .immediate();

    builder.add("imm1.txt").expect("Failed to add imm1");
    builder.add("imm2.txt").expect("Failed to add imm2");

    let opened = test_client.p4.opened(&["//..."]).run().expect("Failed to get opened");
    assert_eq!(opened.len(), 2);

    builder.submit().expect("Failed to submit");
}

#[test]
fn test_changelist_builder_file_type_auto_detection() {
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::fs::symlink;

    let test_client = SERVER.test_client();
    let root = test_client.client_root().to_path_buf();

    fs::write(root.join("text_file.txt"), "regular text").unwrap();

    let exec_path = root.join("exec_file.sh");
    fs::write(&exec_path, "#!/bin/bash\necho hello").unwrap();
    fs::set_permissions(&exec_path, fs::Permissions::from_mode(0o755)).unwrap();

    fs::write(root.join("link_target.txt"), "target").unwrap();
    symlink("link_target.txt", root.join("link_file.txt")).unwrap();

    let result = test_client.changelist("File type detection test")
        .add_file("text_file.txt", b"regular text")
        .add_file("exec_file.sh", b"#!/bin/bash\necho hello")
        .add_file_with_opts("link_file.txt", b"", Some(p4rs::FileType::symlink()))
        .add_file("link_target.txt", b"target")
        .submit()
        .unwrap();

    let desc = test_client.p4.describe(&[result.submitted_change]).run().expect("Failed to describe");
    assert_eq!(desc[0].files.len(), 4);

    let exec_file = desc[0].files.iter().find(|f| f.depot_file.contains("exec_file")).unwrap();
    assert!(exec_file.file_type.executable);

    let link_file = desc[0].files.iter().find(|f| f.depot_file.contains("link_file")).unwrap();
    assert_eq!(link_file.file_type.base, p4rs::BaseFileType::Symlink);
}

#[test]
fn test_changelist_builder_explicit_file_type() {
    let test_client = SERVER.test_client();
    let root = test_client.client_root().to_path_buf();

    fs::write(root.join("binary_file.dat"), &[0u8, 1, 2, 3, 255]).unwrap();

    let mut builder = p4rs::ChangelistBuilder::new(&test_client.p4, root, "Explicit file type")
        .expect("Failed to create builder");

    builder.add_with_type("binary_file.dat", p4rs::FileType::binary())
        .expect("Failed to add with type");

    let result = builder.submit().expect("Failed to submit");

    let desc = test_client.p4.describe(&[result.submitted_change]).run().expect("Failed to describe");
    assert_eq!(desc[0].files[0].file_type.base, p4rs::BaseFileType::Binary);
}

#[test]
fn test_changelist_builder_add_nonexistent_file_error() {
    let test_client = SERVER.test_client();
    let root = test_client.client_root().to_path_buf();

    let mut builder = p4rs::ChangelistBuilder::new(&test_client.p4, root, "Nonexistent file test")
        .expect("Failed to create builder");

    let result = builder.add("does_not_exist.txt");
    assert!(result.is_err());
    test_client.p4.change().delete(builder.changelist).run().unwrap();
}

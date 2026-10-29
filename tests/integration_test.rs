use brain::builder::{BuildOptions, build_project};
use brain::config::{BrainConfig, TargetPlatform};
use brain::lexer::{Lexer, TokenKind};
use brain::parser::Parser;
use brain::safety::{HASH_FILE_NAME, SafetyStatus, check_file_safety};
use brain::typecheck::TypeChecker;
use sha2::Digest;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_lexer_tokens() {
    let code = r#"
        // Comment
        [Skill("test", "A test skill")]
        public class MySkill {
            public void Run(string path = "test", bool flag = true) {
                if (flag) {
                    Prompt($"Running {path}");
                }
            }
        }
    "#;
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize().expect("Lexing failed");
    assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Class)));
    assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::If)));
    assert!(
        tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::InterpolatedString(_)))
    );
}

#[test]
fn test_parser_and_typecheck() {
    let code = r#"
        [Skill("code-review", "Reviews git diff")]
        [Trigger("review", "check diff")]
        [Requires("git")]
        [Rule("Do not modify files directly")]
        public class Reviewer
        {
            [Precondition("Git repo exists")]
            public bool CheckPrereq()
            {
                return true;
            }

            public void Run(string branch = "main", bool terse = false)
            {
                string diff = "git diff";
                if (terse)
                {
                    Prompt($"Short diff summary for {branch}");
                }
                else
                {
                    Prompt($"Detailed diff review for {branch}");
                }

                try
                {
                    ExecuteReview();
                }
                catch (ReviewException)
                {
                    Prompt("Review failed, fall back to file listing.");
                }
            }

            private void ExecuteReview()
            {
                Prompt("Inspecting commits and diff lines.");
            }

            [Example(Input = "review branch main", Output = "All checks passed")]
            public void Examples() {}
        }
    "#;

    let mut parser = Parser::from_source(code).expect("Parser creation failed");
    let ast = parser.parse_file().expect("Parsing failed");
    assert_eq!(ast.classes.len(), 1);
    assert_eq!(ast.classes[0].name, "Reviewer");

    let mut checker = TypeChecker::new();
    let res = checker.check_file(&ast);
    assert!(res.is_ok(), "Type checking failed: {:?}", res.err());
}

#[test]
fn test_typecheck_errors() {
    // 1. Comparison of mismatched types
    let invalid_comparison = r#"
        public class BadSkill {
            public void Run() {
                if (5 == "hello") {
                    Prompt("oops");
                }
            }
        }
    "#;
    let mut parser = Parser::from_source(invalid_comparison).unwrap();
    let ast = parser.parse_file().unwrap();
    let mut checker = TypeChecker::new();
    assert!(checker.check_file(&ast).is_err());

    // 2. Type mismatch in variable initialization
    let invalid_type = r#"
        public class BadSkill2 {
            public void Run() {
                int count = "hello";
            }
        }
    "#;
    let mut parser = Parser::from_source(invalid_type).unwrap();
    let ast = parser.parse_file().unwrap();
    let mut checker = TypeChecker::new();
    assert!(checker.check_file(&ast).is_err());
}

#[test]
fn test_safety_watermark_and_overwrite_prevention() {
    let tmp = tempdir().expect("tempdir failed");
    let file_path = tmp.path().join("SKILL.md");

    // Case 1: File doesn't exist
    assert_eq!(
        check_file_safety(&file_path).unwrap(),
        SafetyStatus::SafeToCreate
    );

    // Case 2: File exists with valid hash in .brainhash
    let content = "# Skill Content\nInstructions";
    let hash = hex::encode(&sha2::Sha256::digest(content.as_bytes())[..8]);
    fs::write(&file_path, content).unwrap();
    fs::write(
        tmp.path().join(HASH_FILE_NAME),
        format!("sha256: {}\n", hash),
    )
    .unwrap();
    assert_eq!(
        check_file_safety(&file_path).unwrap(),
        SafetyStatus::SafeToOverwrite
    );

    // Case 3: File was locally modified (hash mismatch)
    fs::write(
        &file_path,
        "# Skill Content\nInstructions with local edits!",
    )
    .unwrap();
    assert!(matches!(
        check_file_safety(&file_path).unwrap(),
        SafetyStatus::LocallyModified(_)
    ));

    // Case 4: Foreign file exists without .brainhash
    let _ = fs::remove_file(tmp.path().join(HASH_FILE_NAME));
    assert_eq!(
        check_file_safety(&file_path).unwrap(),
        SafetyStatus::ForeignConflict
    );
}

#[test]
fn test_end_to_end_build_pipeline() {
    let tmp = tempdir().expect("tempdir failed");
    let workspace = tmp.path();

    // Create .brains directory and config
    let brains_dir = workspace.join(".brains");
    fs::create_dir_all(&brains_dir).unwrap();

    let config = BrainConfig::new(
        "test-project".to_string(),
        vec![TargetPlatform::OpenCode, TargetPlatform::Claude],
    );
    config.save_to_dir(&brains_dir).unwrap();

    // Add a .brn file
    let brn_content = r#"[Skill("commit-helper", "Auto generate commits")]
[Trigger("commit")]
[Requires("git")]
[Rule("Keep subject concise")]
public class CommitHelper
{
    public void Run(string msg = "default")
    {
        Prompt($"Commit with message {msg}");
    }
}
"#;
    fs::write(brains_dir.join("CommitHelper.brn"), brn_content).unwrap();

    // Run build
    let build_res = build_project(
        workspace,
        BuildOptions {
            force: false,
            target_override: None,
        },
    )
    .unwrap();
    assert_eq!(build_res.files_compiled, 1);
    assert_eq!(build_res.skills_written, 2); // 1 for opencode, 1 for claude

    let opencode_skill = workspace.join(".opencode/skills/commit-helper/SKILL.md");
    let claude_skill = workspace.join(".claude/skills/commit-helper/SKILL.md");

    assert!(opencode_skill.exists());
    assert!(claude_skill.exists());

    // Companion .brainhash file exists
    assert!(
        workspace
            .join(".opencode/skills/commit-helper/.brainhash")
            .exists()
    );
    assert!(
        workspace
            .join(".claude/skills/commit-helper/.brainhash")
            .exists()
    );

    let opencode_md = fs::read_to_string(&opencode_skill).unwrap();
    assert!(opencode_md.contains("name: commit-helper"));
    assert!(opencode_md.contains("platform: opencode"));

    let claude_md = fs::read_to_string(&claude_skill).unwrap();
    assert!(claude_md.contains("name: commit-helper"));
    assert!(claude_md.contains("platform: claude"));

    // Overwrite safety test: Foreign file in opencode (remove .brainhash to simulate foreign skill)
    let _ = fs::remove_file(workspace.join(".opencode/skills/commit-helper/.brainhash"));
    fs::write(&opencode_skill, "# Hand written skill without watermark").unwrap();

    // Rebuild should FAIL with foreign conflict error
    let fail_res = build_project(
        workspace,
        BuildOptions {
            force: false,
            target_override: None,
        },
    );
    assert!(fail_res.is_err());
    let err_msg = fail_res.unwrap_err().to_string();
    assert!(err_msg.contains("Refusing to overwrite existing unmanaged skill"));

    // Rebuild with force = true should SUCCEED
    let force_res = build_project(
        workspace,
        BuildOptions {
            force: true,
            target_override: None,
        },
    );
    assert!(force_res.is_ok());

    // Test supporting files copying from subfolder
    let code_review_dir = brains_dir.join("code-review");
    fs::create_dir_all(&code_review_dir).unwrap();
    let script_dir = code_review_dir.join("scripts");
    fs::create_dir_all(&script_dir).unwrap();
    fs::write(script_dir.join("helper.py"), "print('hello')").unwrap();
    fs::write(
        code_review_dir.join("Review.brn"),
        "[Skill(\"code-review\", \"Code review skill\")] public class Review { public void Run() { Prompt(\"Review\"); } }",
    ).unwrap();

    let sub_build = build_project(
        workspace,
        BuildOptions {
            force: true,
            target_override: None,
        },
    )
    .unwrap();
    assert_eq!(sub_build.files_compiled, 2);

    // Test import resolution: using another .brn file
    let shared_dir = brains_dir.join("shared");
    fs::create_dir_all(&shared_dir).unwrap();
    fs::write(
        shared_dir.join("CommonRules.brn"),
        "[Rule(\"Always check formatting\")] public class CommonRules { public void CheckFormat() { Prompt(\"Check format\"); } }"
    ).unwrap();

    let consumer_dir = brains_dir.join("consumer");
    fs::create_dir_all(&consumer_dir).unwrap();
    fs::write(
        consumer_dir.join("Consumer.brn"),
        "using \"../shared/CommonRules.brn\";\n[Skill(\"consumer\", \"Consumer skill\")] public class Consumer { public void Run() { Prompt(\"Run consumer\"); } }"
    ).unwrap();

    let import_build = build_project(
        workspace,
        BuildOptions {
            force: true,
            target_override: None,
        },
    )
    .unwrap();
    assert_eq!(import_build.files_compiled, 4);

    let consumer_out = workspace.join(".opencode/skills/consumer/SKILL.md");
    assert!(consumer_out.exists());
    let consumer_content = fs::read_to_string(consumer_out).unwrap();
    assert!(consumer_content.contains("Consumer skill"));
    assert!(consumer_content.contains("name=\"CheckFormat\""));

    // Snapshot testing with insta to ensure deterministic prompt emission
    insta::assert_snapshot!("consumer_skill_markdown", consumer_content);
}

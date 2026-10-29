use sha2::Digest;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::ast::SourceFile;
use crate::config::{BrainConfig, TargetPlatform};
use crate::emitter::Emitter;
use crate::parser::Parser;
use crate::safety::{HASH_FILE_NAME, SafetyStatus, check_file_safety};
use crate::typecheck::TypeChecker;

pub struct BuildOptions {
    pub force: bool,
    pub target_override: Option<Vec<TargetPlatform>>,
}

#[derive(Debug)]
pub struct BuildResult {
    pub files_compiled: usize,
    pub skills_written: usize,
    pub written_paths: Vec<PathBuf>,
}

pub fn compile_file(
    file_path: &Path,
    source_root: &Path,
    target: TargetPlatform,
) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
    let source = fs::read_to_string(file_path)?;
    let mut parser = Parser::from_source(&source)
        .map_err(|e| format!("Syntax error in {}: {}", file_path.display(), e))?;
    let mut ast = parser
        .parse_file()
        .map_err(|e| format!("Parse error in {}: {}", file_path.display(), e))?;

    // Resolve `using "path.brn"` or `using Module.Name`
    resolve_imports(&mut ast, file_path, source_root)?;

    let mut checker = TypeChecker::new();
    checker
        .check_file(&ast)
        .map_err(|e| format!("Typecheck error in {}: {}", file_path.display(), e))?;

    let emitter = Emitter::new(target);
    let emitted = emitter
        .emit_file(&ast)
        .map_err(|e| format!("Emit error in {}: {}", file_path.display(), e))?;

    Ok(emitted)
}

fn resolve_imports(
    ast: &mut SourceFile,
    current_file: &Path,
    source_root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let current_dir = current_file.parent().unwrap_or(source_root);
    let mut imported_classes = Vec::new();

    for import_path in &ast.usings {
        let candidate_path = if import_path.ends_with(".brn") {
            current_dir.join(import_path)
        } else {
            // Module notation: Foo.Bar -> Foo/Bar.brn or Foo.brn
            let as_path = import_path.replace('.', "/");
            let p1 = current_dir.join(format!("{}.brn", as_path));
            if p1.exists() {
                p1
            } else {
                source_root.join(format!("{}.brn", as_path))
            }
        };

        if candidate_path.exists() {
            let imported_src = fs::read_to_string(&candidate_path)?;
            let mut sub_parser = Parser::from_source(&imported_src).map_err(|e| {
                format!(
                    "Syntax error in imported {}: {}",
                    candidate_path.display(),
                    e
                )
            })?;
            let sub_ast = sub_parser.parse_file().map_err(|e| {
                format!(
                    "Parse error in imported {}: {}",
                    candidate_path.display(),
                    e
                )
            })?;

            // Append classes and their procedures/rules into the current ast
            for cls in sub_ast.classes {
                imported_classes.push(cls);
            }
        }
    }

    ast.classes.extend(imported_classes);
    Ok(())
}

fn copy_supporting_files(src_dir: &Path, dst_dir: &Path) -> Result<(), std::io::Error> {
    for entry in WalkDir::new(src_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path == src_dir {
            continue;
        }

        // Don't copy .brn source files into compiled destination
        if path.extension().is_some_and(|ext| ext == "brn") {
            continue;
        }

        let rel = match path.strip_prefix(src_dir) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let target_path = dst_dir.join(rel);

        if path.is_dir() {
            fs::create_dir_all(&target_path)?;
        } else if path.is_file() {
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(path, &target_path)?;
        }
    }
    Ok(())
}

pub fn build_project(
    workspace_dir: &Path,
    options: BuildOptions,
) -> Result<BuildResult, Box<dyn std::error::Error>> {
    let brains_dir = workspace_dir.join(".brains");
    let (config, _) = BrainConfig::load_from_dir(&brains_dir)?;

    let targets = options.target_override.unwrap_or(config.compiler.targets);
    if targets.is_empty() {
        return Err("No targets configured in brain.toml or specified on CLI.".into());
    }

    // Collect all .brn files
    let mut brn_files = Vec::new();
    for entry in WalkDir::new(&brains_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "brn") {
            brn_files.push(path.to_path_buf());
        }
    }

    if brn_files.is_empty() {
        println!("No .brn files found in {}", brains_dir.display());
        return Ok(BuildResult {
            files_compiled: 0,
            skills_written: 0,
            written_paths: Vec::new(),
        });
    }

    let mut skills_written = 0;
    let mut written_paths = Vec::new();

    for target in &targets {
        let base_out_dir = workspace_dir.join(target.default_dir_name());

        for file_path in &brn_files {
            let emitted = compile_file(file_path, &brains_dir, target.clone())?;

            for (skill_name, markdown) in emitted {
                let skill_dir = base_out_dir.join(&skill_name);
                let out_file = skill_dir.join("SKILL.md");

                let safety = check_file_safety(&out_file)?;
                match safety {
                    SafetyStatus::LocallyModified(path_str) => {
                        if !options.force {
                            return Err(format!(
                                "Refusing to overwrite manually modified skill at {}.\nLocal edits would be lost. Use --force to overwrite.",
                                path_str
                            ).into());
                        }
                    }
                    SafetyStatus::ForeignConflict => {
                        if !options.force && !config.safety.overwrite_foreign_skills {
                            return Err(format!(
                                "Refusing to overwrite existing unmanaged skill at {}.\nUse --force to overwrite.",
                                out_file.display()
                            ).into());
                        }
                    }
                    SafetyStatus::SafeToCreate | SafetyStatus::SafeToOverwrite => {}
                }

                fs::create_dir_all(&skill_dir)?;
                fs::write(&out_file, &markdown)?;
                written_paths.push(out_file);
                skills_written += 1;

                // Write separate companion hash file for clean, zero-comment SKILL.md
                let hash_file = skill_dir.join(HASH_FILE_NAME);
                let source_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                let hash_record = format!(
                    "source: {}\nsha256: {}\n",
                    source_name,
                    hex::encode(&sha2::Sha256::digest(markdown.as_bytes())[..8])
                );
                fs::write(&hash_file, hash_record)?;

                // If the source file lived in a skill subfolder, copy supporting files/directories (excluding .brn files)
                if let Some(src_parent) = file_path.parent()
                    && src_parent != brains_dir
                {
                    copy_supporting_files(src_parent, &skill_dir)?;
                }
            }
        }
    }

    Ok(BuildResult {
        files_compiled: brn_files.len(),
        skills_written,
        written_paths,
    })
}

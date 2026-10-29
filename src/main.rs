use clap::{Parser as ClapParser, Subcommand};
use dialoguer::Select;
use notify::{RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::time::Duration;

use brain::builder::{BuildOptions, build_project};
use brain::config::{BrainConfig, TargetPlatform};

#[derive(ClapParser, Debug)]
#[command(name = "brain")]
#[command(about = "Compiler for strictly typed Brain (.brn) AI skills", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize .brains folder and detect OpenCode / Claude targets
    Init {
        /// Workspace root (defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Compile .brn files into OpenCode and Claude skills
    Build {
        /// Workspace root (defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Force overwriting non-brain generated skills
        #[arg(short, long)]
        force: bool,

        /// Specific target platform (opencode or claude)
        #[arg(short, long)]
        target: Option<String>,
    },
    /// Watch .brains folder and automatically rebuild on change
    Watch {
        /// Workspace root (defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Force overwriting non-brain generated skills
        #[arg(short, long)]
        force: bool,
    },
    /// Create a new skill source folder and template
    New {
        /// Name of the new skill (e.g. code-reviewer or git-commit)
        name: String,

        /// Workspace root (defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,
    },
}

fn handle_init(workspace: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let brains_dir = workspace.join(".brains");
    if brains_dir.exists() {
        println!(
            "Note: .brains directory already exists at {}",
            brains_dir.display()
        );
    } else {
        std::fs::create_dir_all(&brains_dir)?;
    }

    // Detect harnesses
    let has_opencode = workspace.join(".opencode").exists();
    let has_claude = workspace.join(".claude").exists();

    let mut detected_targets = Vec::new();
    if has_opencode {
        detected_targets.push(TargetPlatform::OpenCode);
        println!("✓ Detected OpenCode harness (.opencode)");
    }
    if has_claude {
        detected_targets.push(TargetPlatform::Claude);
        println!("✓ Detected Claude harness (.claude)");
    }

    if detected_targets.is_empty() {
        println!(
            "No AI harness directory (.opencode or .claude) detected in {}",
            workspace.display()
        );
        let selections = &[
            "OpenCode (.opencode/skills)",
            "Claude (.claude/skills)",
            "Both OpenCode and Claude",
        ];

        let selection = Select::new()
            .with_prompt("Choose target harness(es) to configure")
            .items(selections)
            .default(2)
            .interact()?;

        match selection {
            0 => detected_targets.push(TargetPlatform::OpenCode),
            1 => detected_targets.push(TargetPlatform::Claude),
            2 => {
                detected_targets.push(TargetPlatform::OpenCode);
                detected_targets.push(TargetPlatform::Claude);
            }
            _ => {}
        }
    }

    let config = BrainConfig::new("brain-skills".to_string(), detected_targets);
    let cfg_file = config.save_to_dir(&brains_dir)?;
    println!("✓ Saved config to {}", cfg_file.display());

    // Create a starter template if none exists
    let starter_dir = brains_dir.join("starter-skill");
    let starter_file = starter_dir.join("Starter.brn");
    if !starter_file.exists() && !brains_dir.join("Starter.brn").exists() {
        std::fs::create_dir_all(&starter_dir)?;
        let starter_content = r#"[Skill("starter-skill", "Example starter AI skill written in Brain")]
[Trigger("run starter", "starter")]
[Requires("shell")]
[Rule("Always confirm actions before executing dangerous commands")]
public class StarterSkill
{
    [Precondition]
    public bool CheckPrerequisites()
    {
        return true;
    }

    public void Run(string task = "greeting", bool verbose = false)
    {
        if (verbose)
        {
            Prompt($"Starting starter skill with task: {task}");
        }

        try
        {
            PerformTask(task);
        }
        catch (ExecutionError)
        {
            Prompt("Encountered an issue executing task, falling back to safe status check.");
        }
    }

    private void PerformTask(string task)
    {
        Prompt($"Execute action for task: {task}");
    }

    [Example(Input = "run starter greeting", Output = "Executed greeting")]
    public void Examples() {}
}
"#;
        std::fs::write(&starter_file, starter_content)?;
        println!("✓ Created starter template at {}", starter_file.display());
    }

    println!("\nInitialized successfully! Run `brain build` to compile skills.");
    Ok(())
}

fn handle_build(
    workspace: &Path,
    force: bool,
    target_arg: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let target_override = if let Some(t) = target_arg {
        match TargetPlatform::from_str_loose(&t) {
            Some(target) => Some(vec![target]),
            None => {
                return Err(format!("Unknown target '{}'. Use 'opencode' or 'claude'.", t).into());
            }
        }
    } else {
        None
    };

    let result = build_project(
        workspace,
        BuildOptions {
            force,
            target_override,
        },
    )?;
    println!(
        "✓ Compiled {} source file(s) -> Generated {} skill(s):",
        result.files_compiled, result.skills_written
    );
    for p in result.written_paths {
        println!("  - {}", p.display());
    }
    Ok(())
}

fn handle_watch(workspace: &Path, force: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("Initial build...");
    let _ = handle_build(workspace, force, None);

    let brains_dir = workspace.join(".brains");
    if !brains_dir.exists() {
        return Err(format!(
            "Directory {} does not exist. Run 'brain init' first.",
            brains_dir.display()
        )
        .into());
    }

    println!(
        "\nWatching {} for changes... (Press Ctrl+C to exit)",
        brains_dir.display()
    );

    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = notify::recommended_watcher(tx)?;
    watcher.watch(&brains_dir, RecursiveMode::Recursive)?;

    loop {
        match rx.recv() {
            Ok(Ok(event)) => {
                if event
                    .paths
                    .iter()
                    .any(|p| p.extension().is_some_and(|e| e == "brn" || e == "toml"))
                {
                    println!("\nChange detected, recompiling...");
                    std::thread::sleep(Duration::from_millis(100)); // debounce
                    if let Err(e) = handle_build(workspace, force, None) {
                        eprintln!("Error during build: {}", e);
                    }
                }
            }
            Ok(Err(e)) => eprintln!("Watch error: {}", e),
            Err(e) => eprintln!("Channel error: {}", e),
        }
    }
}

fn handle_new(name: &str, workspace: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let brains_dir = workspace.join(".brains");
    if !brains_dir.exists() {
        return Err(format!(
            "Directory {} does not exist. Run 'brain init' first.",
            brains_dir.display()
        )
        .into());
    }

    let sanitized_name = name.to_lowercase().replace(' ', "-");
    let skill_folder = brains_dir.join(&sanitized_name);
    std::fs::create_dir_all(&skill_folder)?;

    let class_name = sanitized_name
        .split('-')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join("")
        + "Skill";

    let brn_file = skill_folder.join(format!("{}.brn", class_name));
    if brn_file.exists() {
        println!("Skill file already exists at {}", brn_file.display());
        return Ok(());
    }

    let template = format!(
        r#"[Skill("{name}", "Description for {name}")]
[Trigger("{name}")]
[Requires("shell")]
[Rule("Follow user instructions strictly")]
[Precondition("Workspace is initialized")]
[Example(Input = "{name}", Output = "Executed {name}")]
public class {class}
{{
    public void Run()
    {{
        Prompt("Execute instructions for {name}.");
    }}
}}
"#,
        name = sanitized_name,
        class = class_name
    );

    std::fs::write(&brn_file, template)?;
    println!("✓ Created new skill at {}", brn_file.display());
    println!(
        "  (You can add supporting scripts, schemas, or docs inside: {})",
        skill_folder.display()
    );
    Ok(())
}

fn main() {
    let cli = Cli::parse();

    let res = match cli.command {
        Commands::Init { path } => handle_init(&path),
        Commands::Build {
            path,
            force,
            target,
        } => handle_build(&path, force, target),
        Commands::Watch { path, force } => handle_watch(&path, force),
        Commands::New { name, path } => handle_new(&name, &path),
    };

    if let Err(e) = res {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

use anyhow::{Context, Result};
use clap::Parser;
use console::{style, Term};
use dialoguer::{theme::ColorfulTheme, MultiSelect, Select};
use directories::ProjectDirs;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Skills Manager CLI
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {}

const REPO_URL: &str = "https://github.com/trendlink/skills.git"; // Update this with actual repo URL if known, or prompt user.
const SKILLS_DIR_NAME: &str = "antigravity/skills";

fn main() -> Result<()> {
    let term = Term::stdout();
    term.clear_screen()?;

    println!("{}", style("Welcome to the Skills Manager").bold().green());

    let dirs = ProjectDirs::from("com", "gemini", "antigravity")
        .context("Could not determine config directory")?;
    let cache_dir = dirs.cache_dir().join("skills");

    // 1. Initial Setup / Update
    setup_skills_repo(&cache_dir)?;

    // 2. Skill Discovery
    let skills = discover_skills(&cache_dir)?;
    if skills.is_empty() {
        println!("{}", style("No skills found in the repository.").yellow());
        return Ok(());
    }

    // 3. Interactive Selection
    let selected_indices = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select skills to install (Space to toggle, Enter to confirm)")
        .items(&skills)
        .interact()?;

    if selected_indices.is_empty() {
        println!("No skills selected.");
        return Ok(());
    }

    // 4. Destination & Installation
    let pwd = std::env::current_dir()?;
    let local_skills_dir = pwd.join(".agent/skills");
    let global_skills_dir = dirs.config_local_dir().join("skills"); // ~/.gemini/antigravity/skills

    for &index in &selected_indices {
        let skill_name = &skills[index];
        let selections = &["Global (~/.gemini/antigravity/skills/)", "Local (./.agent/skills/)"];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("Install '{}' to?", skill_name))
            .default(0)
            .items(&selections[..])
            .interact()?;

        let source_path = cache_dir.join(skill_name);
        let target_path = if selection == 0 {
            &global_skills_dir
        } else {
            &local_skills_dir
        };
        
        install_skill(&source_path, target_path, skill_name)?;
    }

    println!("{}", style("\nAll done!").bold().green());
    Ok(())
}

fn setup_skills_repo(path: &Path) -> Result<()> {
    if path.exists() {
        println!("Skills repository found at {:?}", path);
        let update = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Do you want to update the skills repository?")
            .default(0)
            .items(&["Yes", "No"])
            .interact()?;
        
        if update == 0 {
            println!("Updating repository...");
            Command::new("git")
                .current_dir(path)
                .args(&["pull"])
                .status()
                .context("Failed to pull git repository")?;
        }
    } else {
        println!("Skills repository not found.");
        let clone = Select::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("Do you want to clone the skills repository to {:?}?", path))
            .default(0)
            .items(&["Yes", "No"])
            .interact()?;
            
        if clone == 0 {
            fs::create_dir_all(path.parent().unwrap())?;
            // Note: In a real scenario, we might want to prompt for the URL if REPO_URL is just a placeholder.
            // For now, assuming REPO_URL (or asking user to input it)
            println!("Cloning repository...");
             Command::new("git")
                .args(&["clone", "--branch", "main", REPO_URL, path.to_str().unwrap()])
                .status()
                .context("Failed to clone git repository")?;
        }
    }
    Ok(())
}

fn discover_skills(path: &Path) -> Result<Vec<String>> {
    let mut skills = Vec::new();
    if !path.exists() {
        return Ok(skills);
    }
    
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if !name.starts_with('.') && name != "skill-manager" && name != "target" {
                    skills.push(name.to_string());
                }
            }
        }
    }
    skills.sort();
    Ok(skills)
}

fn install_skill(source: &Path, target_parent: &Path, name: &str) -> Result<()> {
    let target = target_parent.join(name);
    
    if !target_parent.exists() {
        fs::create_dir_all(target_parent)?;
    }

    if target.exists() {
        println!("Skill '{}' already exists in target. Overwriting...", name);
        fs::remove_dir_all(&target)?;
    }

    println!("Installing '{}' to {:?}...", name, target);
    
    // Recursive copy
    copy_dir_recursive(source, &target)?;
    
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
           copy_dir_recursive(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use displaydoc::Display;
use git2::Repository;
use log::{debug, info, trace};
use thiserror::Error;
use walkdir::WalkDir;

use self::GenerateGitError as E;
use crate::{
    args::GenerateGitConfig,
    tasks::{
        git::{GitConfig, GitRemote},
        task::Task,
        ResolveEnv,
    },
};

use super::GENERATED_PRELUDE_COMMENT;

pub fn run(generate_git_configs: Vec<GenerateGitConfig>) -> Result<()> {
    for config in generate_git_configs {
        run_single(&config)?;
    }
    Ok(())
}

pub fn run_single(generate_git_config: &GenerateGitConfig) -> Result<()> {
    debug!(
        "Generating git config for: {path}",
        path = generate_git_config.path.display()
    );
    let mut git_task = Task::from(&generate_git_config.path)?;
    debug!("Existing git config: {:?}", git_task);
    let mut git_configs = Vec::new();
    for path in find_repos(
        &generate_git_config.search_paths,
        generate_git_config.excludes.as_ref(),
    )? {
        git_configs.push(parse_git_config(&path)?);
    }
    // TODO(gib): keep old branch names.
    git_configs.sort_unstable_by(|c1, c2| c1.path.cmp(&c2.path));
    let toml_configs = git_configs
        .into_iter()
        .map(toml::Value::try_from)
        .collect::<Result<Vec<_>, _>>()?;
    git_task.config.data = Some(toml_configs.into());
    debug!("New git config: {:?}", git_task);
    let mut serialized_task = GENERATED_PRELUDE_COMMENT.to_owned();
    serialized_task.push_str(&toml::to_string_pretty(&git_task.config)?);
    trace!("New toml file: <<<{}>>>", serialized_task);
    fs::write(&generate_git_config.path, serialized_task)?;
    info!(
        "Git repo layout generated for task '{}' and written to '{:?}'",
        git_task.name, generate_git_config.path
    );
    Ok(())
}

impl ResolveEnv for Vec<GenerateGitConfig> {
    fn resolve_env<F>(&mut self, env_fn: F) -> Result<()>
    where
        F: Fn(&str) -> Result<String>,
    {
        for config in self.iter_mut() {
            config.path = PathBuf::from(env_fn(&config.path.to_string_lossy())?);

            let mut new_search_paths = Vec::new();
            for search_path in &config.search_paths {
                new_search_paths.push(PathBuf::from(env_fn(&search_path.to_string_lossy())?));
            }
            config.search_paths = new_search_paths;

            if let Some(excludes) = config.excludes.as_ref() {
                let mut new_excludes = Vec::new();
                for exclude in excludes {
                    new_excludes.push(env_fn(&exclude)?);
                }
                config.excludes = Some(new_excludes);
            }
        }
        Ok(())
    }
}

fn find_repos(search_paths: &[PathBuf], excludes: Option<&Vec<String>>) -> Result<Vec<PathBuf>> {
    let mut repo_paths = Vec::new();
    for path in search_paths {
        trace!("Searching in '{}'", &path.display());
        for entry in WalkDir::new(path)
            .into_iter()
            .filter_entry(|e| {
                if let Some(ex) = excludes {
                    let s = e.path().to_str().unwrap_or("");
                    for exclude in ex {
                        if s.contains(exclude) {
                            return false;
                        }
                    }
                    true
                } else {
                    true
                }
            })
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_dir() && e.file_name() == ".git")
        {
            trace!("Entry: {:?}", &entry);
            let mut repo_path = entry.into_path();
            repo_path.pop();
            repo_paths.push(repo_path);
        }
    }
    debug!("Found repo paths: {:?}", repo_paths);
    Ok(repo_paths)
}

fn parse_git_config(path: &Path) -> Result<GitConfig> {
    let repo = Repository::open(&path)?;
    let mut remotes = Vec::new();
    for opt_name in &repo.remotes()? {
        let name = opt_name.ok_or(E::InvalidUTF8)?;
        let remote = repo.find_remote(name).with_context(|| E::InvalidRemote {
            name: name.to_owned(),
        })?;
        let git_remote = GitRemote::from(&remote)?;
        remotes.push(git_remote);
    }
    let config = GitConfig {
        path: path.to_string_lossy().to_string(),
        branch: None,
        remotes,
    };
    trace!("Parsed GitConfig: {:?}", &config);
    Ok(config)
}

#[derive(Error, Debug, Display)]
/// Errors thrown by this file.
pub enum GenerateGitError {
    /// Invalid UTF-8.
    InvalidUTF8,
    /// Invalid remote '{name}'.
    InvalidRemote { name: String },
}
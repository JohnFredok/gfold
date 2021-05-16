use crate::util;
use anyhow::Result;
use log::{debug, warn};
use std::{
    cmp::Ordering,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct Config {
    pub enable_unpushed_check: bool,
    pub include_non_repos: bool,
    pub no_color: bool,
    pub shallow: bool,
    pub show_email: bool,
    pub skip_sort: bool,
}

pub struct TableWrapper {
    pub path_string: String,
    pub table: prettytable::Table,
}

pub struct Results(Vec<TableWrapper>);

impl Results {
    pub fn new(path: &Path, config: &Config) -> Result<Results> {
        debug!("Running with config: {:#?}", &config);
        debug!("Running in path: {:#?}", &path);
        let mut results = Results(Vec::new());
        results.execute_in_directory(&config, path)?;
        if !&config.skip_sort {
            results.sort_results();
        }
        Ok(results)
    }

    pub fn print_results(self) {
        debug!("Printing results with {} tables...", self.0.len());
        match self.0.len().cmp(&1) {
            Ordering::Greater => {
                for table_wrapper in self.0 {
                    println!("\n{}", table_wrapper.path_string);
                    table_wrapper.table.printstd();
                }
            }
            Ordering::Equal => {
                self.0[0].table.printstd();
            }
            _ => {}
        };
    }

    // Sequential exeuction has benchmarked faster than concurrent implementations.
    fn execute_in_directory(&mut self, config: &Config, dir: &Path) -> Result<()> {
        let mut repos: Vec<PathBuf> = Vec::new();
        let mut non_repos: Vec<PathBuf> = Vec::new();

        for entry in (fs::read_dir(dir)?).flatten() {
            let file_name_buf = entry.file_name();
            let file_name = match file_name_buf.to_str() {
                Some(o) => o,
                None => continue,
            };
            if !file_name.starts_with('.') && entry.file_type()?.is_dir() {
                let entry_path = entry.path();
                match git2::Repository::open(&entry_path) {
                    Ok(_) => repos.push(entry_path),
                    Err(e) => {
                        debug!(
                            "Tried to open {:#?} as git repository: {:#?}",
                            entry_path,
                            e.message()
                        );
                        if config.include_non_repos {
                            non_repos.push(entry_path.clone());
                        }
                        if !config.shallow {
                            if let Err(e) = self.execute_in_directory(&config, &entry_path) {
                                warn!(
                                    "Encountered error during recursive walk into {:#?}: {:#?}",
                                    &entry_path, e
                                );
                            }
                        }
                    }
                }
            }
        }

        debug!("Git repositories found: {:#?}", repos);
        if config.include_non_repos {
            debug!("Standard directories found: {:#?}", non_repos);
        }
        if !repos.is_empty() {
            if !&config.skip_sort {
                repos.sort();
            }
            if let Some(table_wrapper) = util::create_table_from_paths(
                repos,
                non_repos,
                &dir,
                &config.enable_unpushed_check,
                &config.no_color,
                &config.show_email,
            ) {
                self.0.push(table_wrapper);
            }
        }
        Ok(())
    }

    fn sort_results(&mut self) {
        debug!("Sorting {:#?} tables...", self.0.len());
        if self.0.len() >= 2 {
            // FIXME: find a way to do this without "clone()".
            self.0.sort_by_key(|table| table.path_string.clone());
        }
    }
}
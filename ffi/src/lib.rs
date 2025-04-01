use gitlens_core::analytics;
use gitlens_core::models;
use gitlens_core::repository::Repository;
use gitlens_core::types::{BranchName, GitUrl, Result};

use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::path::Path;
use std::str::FromStr;

// Create custom exception
create_exception!(gitlens, GitLensError, PyException);

/// Converts GitLens error to Python exception
fn to_py_err(err: gitlens_core::error::GitError) -> PyErr {
    GitLensError::new_err(err.to_string())
}

/// A Git repository for analysis.
#[pyclass]
struct PyRepository {
    inner: Repository,
}

#[pymethods]
impl PyRepository {
    /// Open an existing Git repository.
    #[staticmethod]
    fn open(path: &str) -> PyResult<Self> {
        let result = Repository::open(path).map_err(to_py_err)?;
        Ok(PyRepository { inner: result })
    }

    /// Clone a Git repository.
    #[staticmethod]
    fn clone(url: &str, path: &str, options: Option<&PyDict>) -> PyResult<Self> {
        let git_url = GitUrl::from_str(url).map_err(to_py_err)?;

        let clone_options = if let Some(opts) = options {
            let mut clone_opts = gitlens_core::repository::CloneOptions::default();

            if let Ok(Some(shallow)) = opts.get_item("shallow") {
                if let Ok(shallow_val) = shallow.extract::<bool>() {
                    clone_opts.shallow = shallow_val;
                }
            }

            if let Ok(Some(no_blob_content)) = opts.get_item("no_blob_content") {
                if let Ok(val) = no_blob_content.extract::<bool>() {
                    clone_opts.no_blob_content = val;
                }
            }

            if let Ok(Some(single_branch)) = opts.get_item("single_branch") {
                if let Ok(branch) = single_branch.extract::<&str>() {
                    if let Ok(branch_name) = BranchName::from_str(branch) {
                        clone_opts.single_branch = Some(branch_name);
                    }
                }
            }

            if let Ok(Some(bare)) = opts.get_item("bare") {
                if let Ok(val) = bare.extract::<bool>() {
                    clone_opts.bare = val;
                }
            }

            if let Ok(Some(mirror)) = opts.get_item("mirror") {
                if let Ok(val) = mirror.extract::<bool>() {
                    clone_opts.mirror = val;
                }
            }

            Some(clone_opts)
        } else {
            None
        };

        let result = Repository::clone(git_url, Path::new(path), clone_options).map_err(to_py_err)?;
        Ok(PyRepository { inner: result })
    }

    /// List all branches in the repository.
    fn list_branches(&self) -> PyResult<Vec<String>> {
        let branches = self.inner.list_branches().map_err(to_py_err)?;
        Ok(branches.iter().map(|b| b.to_string()).collect())
    }

    /// List all tracked files in the repository.
    fn list_tracked(&self) -> PyResult<Vec<String>> {
        self.inner.list_tracked().map_err(to_py_err)
    }

    /// Get the current HEAD commit hash.
    fn get_hash(&self, short: Option<bool>) -> PyResult<String> {
        let short_val = short.unwrap_or(false);
        let hash = self.inner.get_hash(short_val).map_err(to_py_err)?;
        Ok(hash.to_string())
    }

    /// Get information about a commit.
    fn get_commit<'py>(&self, py: Python<'py>, commit_ref: Option<&str>) -> PyResult<&'py PyDict> {
        let commit = self.inner.get_commit(commit_ref).map_err(to_py_err)?;

        let result = PyDict::new(py);
        result.set_item("hash", commit.hash.to_string())?;
        result.set_item("short_hash", commit.short_hash.to_string())?;
        result.set_item("author_name", commit.author_name)?;
        result.set_item("author_email", commit.author_email)?;
        result.set_item("timestamp", commit.timestamp)?;
        result.set_item("message", commit.message)?;

        let parents = PyList::new(py, commit.parents.iter().map(|p| p.to_string()));
        result.set_item("parents", parents)?;

        Ok(result)
    }

    /// Get the current status of the repository.
    fn status<'py>(&self, py: Python<'py>) -> PyResult<&'py PyDict> {
        let status = self.inner.status().map_err(to_py_err)?;

        let result = PyDict::new(py);
        result.set_item("branch", status.branch.map(|b| b.to_string()))?;
        result.set_item("is_clean", status.is_clean)?;
        result.set_item("merging", status.merging)?;
        result.set_item("rebasing", status.rebasing)?;
        result.set_item("cherry_picking", status.cherry_picking)?;

        let files = PyList::new(py, status.files.iter().map(|f| {
            let file_dict = PyDict::new(py);
            file_dict.set_item("path", f.path.to_string_lossy().to_string()).unwrap();
            file_dict.set_item("status", format!("{:?}", f.status)).unwrap();
            file_dict.set_item("original_path", f.original_path.as_ref().map(|p| p.to_string_lossy().to_string())).unwrap();
            file_dict
        }));
        result.set_item("files", files)?;

        Ok(result)
    }

    /// Get the commit history of the repository.
    fn get_commit_history<'py>(
        &self,
        py: Python<'py>,
        limit: Option<usize>,
        skip: Option<usize>,
        branch: Option<&str>,
    ) -> PyResult<Vec<&'py PyDict>> {
        let branch_name = if let Some(b) = branch {
            Some(BranchName::from_str(b).map_err(to_py_err)?)
        } else {
            None
        };

        let commits = self.inner.get_commit_history(
            limit,
            skip,
            branch_name.as_ref(),
        ).map_err(to_py_err)?;

        let result = commits.iter().map(|commit| {
            let commit_dict = PyDict::new(py);
            commit_dict.set_item("hash", commit.hash.to_string()).unwrap();
            commit_dict.set_item("short_hash", commit.short_hash.to_string()).unwrap();
            commit_dict.set_item("author_name", &commit.author_name).unwrap();
            commit_dict.set_item("author_email", &commit.author_email).unwrap();
            commit_dict.set_item("timestamp", commit.timestamp).unwrap();
            commit_dict.set_item("message", &commit.message).unwrap();

            let parents = PyList::new(py, commit.parents.iter().map(|p| p.to_string()));
            commit_dict.set_item("parents", parents).unwrap();

            commit_dict
        }).collect();

        Ok(result)
    }

    /// Analyze contributor statistics for the repository.
    fn analyze_contributions<'py>(
        &self,
        py: Python<'py>,
        limit: Option<usize>,
    ) -> PyResult<&'py PyDict> {
        let stats = self.inner.analyze_contributions(limit).map_err(to_py_err)?;

        let result = PyDict::new(py);
        result.set_item("total_commits", stats.total_commits)?;
        result.set_item("total_authors", stats.total_authors)?;
        result.set_item("total_added", stats.total_added)?;
        result.set_item("total_removed", stats.total_removed)?;
        result.set_item("total_files_changed", stats.total_files_changed)?;

        let authors = PyDict::new(py);
        for (author, author_stats) in stats.by_author {
            let author_dict = PyDict::new(py);
            author_dict.set_item("commits", author_stats.commits)?;
            author_dict.set_item("added_lines", author_stats.added_lines)?;
            author_dict.set_item("removed_lines", author_stats.removed_lines)?;
            author_dict.set_item("files_changed", author_stats.files_changed)?;
            author_dict.set_item("first_commit", author_stats.first_commit)?;
            author_dict.set_item("last_commit", author_stats.last_commit)?;

            authors.set_item(author, author_dict)?;
        }
        result.set_item("by_author", authors)?;

        Ok(result)
    }

    /// Analyze file change frequency in the repository.
    fn analyze_change_frequency<'py>(
        &self,
        py: Python<'py>,
        path_filter: Option<&str>,
    ) -> PyResult<Vec<&'py PyDict>> {
        let frequency = self.inner.analyze_change_frequency(path_filter).map_err(to_py_err)?;

        let result = frequency.iter().map(|f| {
            let freq_dict = PyDict::new(py);
            freq_dict.set_item("path", f.path.to_string_lossy().to_string()).unwrap();
            freq_dict.set_item("change_count", f.change_count).unwrap();
            freq_dict.set_item("last_modified", f.last_modified).unwrap();

            let authors = PyList::new(py, &f.authors);
            freq_dict.set_item("authors", authors).unwrap();
            freq_dict.set_item("primary_owner", &f.primary_owner).unwrap();

            freq_dict
        }).collect();

        Ok(result)
    }

    /// Calculate branch divergence metrics.
    fn calculate_branch_divergence<'py>(
        &self,
        py: Python<'py>,
        source: &str,
        target: &str,
    ) -> PyResult<&'py PyDict> {
        let source_branch = BranchName::from_str(source).map_err(to_py_err)?;
        let target_branch = BranchName::from_str(target).map_err(to_py_err)?;

        let divergence = self.inner.calculate_branch_divergence(
            &source_branch,
            &target_branch,
        ).map_err(to_py_err)?;

        let result = PyDict::new(py);
        result.set_item("source", divergence.source.to_string())?;
        result.set_item("target", divergence.target.to_string())?;
        result.set_item("merge_base", divergence.merge_base.to_string())?;
        result.set_item("ahead_count", divergence.ahead_count)?;
        result.set_item("behind_count", divergence.behind_count)?;
        result.set_item("differing_files", divergence.differing_files)?;

        Ok(result)
    }
}

/// Python module configuration
#[pymodule]
fn _gitlens(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyRepository>()?;
    m.add("GitLensError", py.get_type::<GitLensError>())?;

    Ok(())
}
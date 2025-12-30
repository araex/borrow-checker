use crate::ssh_keys::get_private_key_path;
use crate::structs;
use crate::traits::{PersistenceError, PersistenceRepository};
use git2::{
    Cred, MergeOptions, ObjectType, PushOptions, RebaseOperationType, RemoteCallbacks, Repository,
    Tree,
};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::str;
use std::sync::Mutex;
use uuid::Uuid;

/// Git-backed implementation of PersistenceRepository.
///
/// The repository is opened during construction. A map from ledger UUID -> relative path
/// (path relative to repository root, e.g. "ledgers/39C3") is maintained and built when
/// listing/loading ledgers.
pub struct GitPersistence {
    repo: Mutex<Repository>,
    /// Map from ledger id -> relative path (under repo root, e.g. "ledgers/39C3")
    ledger_map: Mutex<HashMap<Uuid, PathBuf>>,

    /// The path under repo root where ledgers live (default "ledgers")
    ledgers_root: PathBuf,
}

impl GitPersistence {
    /// Open a repository at the given path.
    pub fn new(repo_path: PathBuf) -> Result<Self, PersistenceError> {
        let path = repo_path;

        let repo = Repository::open(&path)
            .map_err(|e| PersistenceError::RepositoryError(format!("{}: {}", path.display(), e)))?;

        Ok(GitPersistence {
            repo: Mutex::new(repo),
            ledger_map: Mutex::new(HashMap::new()),
            ledgers_root: PathBuf::from("ledgers"),
        })
    }

    /// Resolve refs/heads/main or HEAD and return the repository root tree.
    fn get_root_tree<'repo>(
        &self,
        repo: &'repo Repository,
    ) -> Result<Tree<'repo>, PersistenceError> {
        let reference = repo
            .find_reference("refs/heads/main")
            .or_else(|_| repo.head())
            .map_err(|e| {
                PersistenceError::RepositoryError(format!("failed to find main or HEAD: {}", e))
            })?;

        let target_oid = reference.target().ok_or_else(|| {
            PersistenceError::RepositoryError("reference does not point to an object".into())
        })?;

        let commit = repo.find_commit(target_oid).map_err(|e| {
            PersistenceError::RepositoryError(format!("failed to find commit: {}", e))
        })?;

        Ok(commit
            .tree()
            .map_err(|e| PersistenceError::RepositoryError(format!("failed to get tree: {}", e)))?)
    }

    /// Given a tree and a path, return the subtree at that path.
    fn subtree_from_tree<'repo>(
        &self,
        repo: &'repo Repository,
        tree: &Tree<'repo>,
        path: &Path,
    ) -> Result<Tree<'repo>, PersistenceError> {
        let entry = tree.get_path(path).map_err(|e| {
            PersistenceError::RepositoryError(format!("failed to find path {:?}: {}", path, e))
        })?;

        match entry.kind() {
            Some(ObjectType::Tree) => {
                let obj = entry.to_object(repo).map_err(|e| {
                    PersistenceError::RepositoryError(format!("to_object failed: {}", e))
                })?;
                obj.peel_to_tree().map_err(|e| {
                    PersistenceError::RepositoryError(format!("peel_to_tree failed: {}", e))
                })
            }
            _ => Err(PersistenceError::RepositoryError(format!(
                "path {:?} is not a tree",
                path
            ))),
        }
    }

    /// Helper: read a blob at `path_in_repo` (relative path under repo root) and return its text.
    fn read_blob_text(
        &self,
        repo: &Repository,
        path_in_repo: &Path,
    ) -> Result<String, PersistenceError> {
        let root_tree = self.get_root_tree(repo)?;

        let entry = root_tree.get_path(path_in_repo).map_err(|_| {
            PersistenceError::NotFound(format!("{} not found", path_in_repo.display()))
        })?;

        let blob = repo.find_blob(entry.id()).map_err(|e| {
            PersistenceError::RepositoryError(format!("failed to read blob: {}", e))
        })?;

        let text = std::str::from_utf8(blob.content())
            .map_err(|e| PersistenceError::DataError(format!("UTF-8 decode error: {}", e)))?;

        Ok(text.to_string())
    }

    /// Build the ledger map by scanning the ledgers folder.
    ///
    /// This will delegate to list_ledgers which updates the internal map for successfully parsed ledgers.
    #[allow(dead_code)]
    fn build_ledger_map(&self) -> Result<(), PersistenceError> {
        // list_ledgers updates the internal ledger_map with relative paths for successfully parsed ledgers.
        let _ = self.list_ledgers()?;
        Ok(())
    }

    /// Helper: Persist changes to a file and commit them to git
    ///
    /// Writes the serialized content to disk, stages it in git index, and creates a commit.
    fn persist_and_commit(
        &self,
        repo: &Repository,
        rel_file_path: &Path,
        content: &str,
        commit_message: &str,
    ) -> Result<(), PersistenceError> {
        let workdir = repo.workdir().ok_or_else(|| {
            PersistenceError::RepositoryError("repository has no working directory".into())
        })?;

        let full_fs_path = workdir.join(rel_file_path);

        // Write to filesystem
        fs::write(&full_fs_path, content.as_bytes()).map_err(|e| {
            PersistenceError::DataError(format!(
                "failed to write {}: {}",
                full_fs_path.display(),
                e
            ))
        })?;

        // Update the git index
        let mut index = repo.index().map_err(|e| {
            PersistenceError::RepositoryError(format!("failed to get index: {}", e))
        })?;

        index.add_path(rel_file_path).map_err(|e| {
            PersistenceError::RepositoryError(format!("failed to add path to index: {}", e))
        })?;

        index.write().map_err(|e| {
            PersistenceError::RepositoryError(format!("failed to write index: {}", e))
        })?;

        let tree_oid = index.write_tree().map_err(|e| {
            PersistenceError::RepositoryError(format!("failed to write tree: {}", e))
        })?;

        let tree = repo.find_tree(tree_oid).map_err(|e| {
            PersistenceError::RepositoryError(format!("failed to find tree: {}", e))
        })?;

        // Create signature
        let sig = repo.signature().map_err(|e| {
            PersistenceError::RepositoryError(format!("failed to create signature: {}", e))
        })?;

        // Determine parent commit (HEAD)
        let parent_commit = {
            let head = repo.head().map_err(|e| {
                PersistenceError::RepositoryError(format!("failed to get HEAD: {}", e))
            })?;
            let target = head.target().ok_or_else(|| {
                PersistenceError::RepositoryError("HEAD has no target commit".into())
            })?;
            repo.find_commit(target).map_err(|e| {
                PersistenceError::RepositoryError(format!("failed to find parent commit: {}", e))
            })?
        };

        // Commit
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            commit_message,
            &tree,
            &[&parent_commit],
        )
        .map_err(|e| {
            PersistenceError::RepositoryError(format!("failed to create commit: {}", e))
        })?;

        Ok(())
    }

    fn repo_has_local_changes(repo: &Repository) -> Result<bool, PersistenceError> {
        let local_main = repo
            .find_reference("refs/heads/main")
            .and_then(|r| {
                r.target()
                    .ok_or_else(|| git2::Error::from_str("main has no target"))
            })
            .map_err(|e| {
                PersistenceError::RepositoryError(format!(
                    "failed to resolve refs/heads/main: {}",
                    e
                ))
            })?;

        let origin_main = repo
            .find_reference("refs/remotes/origin/main")
            .and_then(|r| {
                r.target()
                    .ok_or_else(|| git2::Error::from_str("origin/main has no target"))
            })
            .map_err(|e| {
                PersistenceError::RepositoryError(format!(
                    "failed to resolve refs/remotes/origin/main: {}",
                    e
                ))
            })?;

        Ok(local_main != origin_main)
    }
}

impl PersistenceRepository for GitPersistence {
    // ---------------- Group Operations ----------------

    fn load_group(&self) -> Result<structs::Group, PersistenceError> {
        let repo = &self
            .repo
            .lock()
            .map_err(|e| PersistenceError::RepositoryError(format!("Can not lock repo: {e}")))?;

        let text = self.read_blob_text(&repo, Path::new("group.toml"))?;
        let group: structs::Group = toml::from_str(&text)
            .map_err(|e| PersistenceError::DataError(format!("TOML parse error: {}", e)))?;
        Ok(group)
    }

    fn save_group(&self, group: &structs::Group) -> Result<(), PersistenceError> {
        let repo = &self
            .repo
            .lock()
            .map_err(|e| PersistenceError::RepositoryError(format!("Can not lock repo: {e}")))?;

        let toml_text = toml::to_string_pretty(group).map_err(|e| {
            PersistenceError::DataError(format!("failed to serialize group: {}", e))
        })?;

        self.persist_and_commit(
            &repo,
            Path::new("group.toml"),
            &toml_text,
            "Update group configuration",
        )
    }

    // ---------------- Ledger Operations ----------------

    fn list_ledgers(&self) -> Result<Vec<structs::Ledger>, PersistenceError> {
        let repo = &self
            .repo
            .lock()
            .map_err(|e| PersistenceError::RepositoryError(format!("Can not lock repo: {e}")))?;

        let root_tree = self.get_root_tree(&repo)?;
        let ledgers_tree = self.subtree_from_tree(&repo, &root_tree, &self.ledgers_root)?;

        let mut results = Vec::new();
        // Collect successful ledger id -> relative-path entries so we can update the ledger_map as we go.
        let mut successful_map_entries: Vec<(Uuid, PathBuf)> = Vec::new();

        for entry in ledgers_tree.iter() {
            let ledger_dir_name = match entry.name() {
                Some(n) => n.to_string(),
                None => {
                    log::warn!("skipping ledger entry with no name");
                    continue;
                }
            };

            log::info!("child entry: {:?}", entry.name());
            match entry.kind() {
                Some(ObjectType::Tree) => {
                    // peel to child tree
                    let child_obj = match entry.to_object(&repo) {
                        Ok(o) => o,
                        Err(e) => {
                            log::error!("failed to read ledger folder {}: {}", ledger_dir_name, e);
                            continue;
                        }
                    };
                    log::info!("child: {:?}", child_obj);
                    let child_tree = match child_obj.peel_to_tree() {
                        Ok(t) => t,
                        Err(e) => {
                            log::error!(
                                "failed to peel ledger folder {} to tree: {}",
                                ledger_dir_name,
                                e
                            );
                            continue;
                        }
                    };

                    // find .ledger.toml marker
                    let mut ledger_blob_entry_opt: Option<git2::TreeEntry> = None;
                    for child_entry in child_tree.iter() {
                        if let Some(n) = child_entry.name() {
                            if n == ".ledger.toml" {
                                ledger_blob_entry_opt = Some(child_entry);
                                break;
                            }
                        }
                    }

                    let ledger_blob_entry = match ledger_blob_entry_opt {
                        Some(e) => e,
                        None => continue, // not a ledger folder
                    };

                    // Ensure .ledger.toml is a blob
                    let blob = match ledger_blob_entry.kind() {
                        Some(ObjectType::Blob) => match repo.find_blob(ledger_blob_entry.id()) {
                            Ok(b) => b,
                            Err(e) => {
                                return Err(PersistenceError::RepositoryError(format!(
                                    "unable to read .ledger.toml in {}: {}",
                                    ledger_dir_name, e
                                )));
                            }
                        },
                        Some(k) => {
                            return Err(PersistenceError::RepositoryError(format!(
                                ".ledger.toml in {} is not a blob (kind={:?})",
                                ledger_dir_name, k
                            )));
                        }
                        None => {
                            return Err(PersistenceError::RepositoryError(format!(
                                ".ledger.toml in {} has no object type",
                                ledger_dir_name
                            )));
                        }
                    };

                    // Parse blob content
                    let text = match str::from_utf8(blob.content()) {
                        Ok(t) => t,
                        Err(e) => {
                            return Err(PersistenceError::DataError(format!(
                                ".ledger.toml in {} is not valid utf8: {}",
                                ledger_dir_name, e
                            )));
                        }
                    };

                    match toml::from_str::<structs::Ledger>(text) {
                        Ok(ledger) => {
                            // record successful ledger; store relative path using the tree entry name (ledger folder name)
                            let rel_path = self.ledgers_root.join(ledger_dir_name.clone());
                            successful_map_entries.push((ledger.id, rel_path));
                            results.push(ledger);
                        }
                        Err(e) => {
                            return Err(PersistenceError::ParseLedger {
                                ledger_name: ledger_dir_name,
                                message: format!("{}", e),
                            });
                        }
                    }
                }
                Some(kind) => {
                    log::warn!(
                        "skipping non-tree ledger entry {}: {:?}",
                        ledger_dir_name,
                        kind
                    );
                }
                None => {
                    eprintln!("ledger entry {} has no object type", ledger_dir_name);
                }
            }
        }

        // Update internal map with successfully parsed ledgers (store relative paths).
        {
            let mut map = self.ledger_map.lock().map_err(|e| {
                PersistenceError::RepositoryError(format!("Can not lock ledger_map: {e}"))
            })?;

            // Clear existing map and insert all successful entries to keep in sync.
            map.clear();
            for (id, rel_path) in successful_map_entries {
                log::info!("entry: {:?}, {:?}", id, rel_path);
                map.insert(id, rel_path);
            }
            log::info!("ledger map: {:?}", map)
        }

        Ok(results)
    }

    fn create_ledger(&self, ledger: structs::Ledger) -> Result<Uuid, PersistenceError> {
        let repo = &self
            .repo
            .lock()
            .map_err(|e| PersistenceError::RepositoryError(format!("Can not lock repo: {e}")))?;

        let ledger_id = ledger.id;
        // Use display_name as the folder name (sanitize if needed)
        let folder_name = &ledger.display_name;
        let ledger_rel_path = self.ledgers_root.join(folder_name);
        let marker_file_path = ledger_rel_path.join(".ledger.toml");

        // Ensure repository has a working directory
        let workdir = repo.workdir().ok_or_else(|| {
            PersistenceError::RepositoryError("repository has no working directory".into())
        })?;

        let ledger_dir = workdir.join(&ledger_rel_path);

        // Create the ledger directory if it doesn't exist
        fs::create_dir_all(&ledger_dir).map_err(|e| {
            PersistenceError::DataError(format!("failed to create ledger directory: {}", e))
        })?;

        // Serialize ledger to TOML
        let toml_text = toml::to_string_pretty(&ledger).map_err(|e| {
            PersistenceError::DataError(format!("failed to serialize ledger {}: {}", ledger_id, e))
        })?;

        // Persist and commit
        self.persist_and_commit(
            &repo,
            &marker_file_path,
            &toml_text,
            &format!("Create ledger {} ({})", ledger.display_name, ledger_id),
        )?;

        // Update the ledger_map with the new ledger
        {
            let mut map = self.ledger_map.lock().map_err(|e| {
                PersistenceError::RepositoryError(format!("Can not lock ledger_map: {e}"))
            })?;

            map.insert(ledger_id, ledger_rel_path);
        }

        Ok(ledger_id)
    }

    fn update_ledger(&self, ledger: structs::Ledger) -> Result<(), PersistenceError> {
        let repo = &self
            .repo
            .lock()
            .map_err(|e| PersistenceError::RepositoryError(format!("Can not lock repo: {e}")))?;

        // Find ledger relative path in map (path is relative to repo root)
        let ledger_id = ledger.id;
        let ledger_rel_path = self
            .ledger_map
            .lock()
            .map_err(|e| PersistenceError::RepositoryError(format!("Can not lock ledger_map: {e}")))
            .and_then(|map| {
                map.get(&ledger_id).cloned().ok_or_else(|| {
                    PersistenceError::NotFound(format!("ledger id {} not found", ledger_id))
                })
            })?;

        // Relative path to the marker file in the repo (e.g. "ledgers/39C3/.ledger.toml")
        let rel_file_path = ledger_rel_path.join(".ledger.toml");

        // Serialize ledger to TOML
        let toml_text = toml::to_string_pretty(&ledger).map_err(|e| {
            PersistenceError::DataError(format!("failed to serialize ledger {}: {}", ledger_id, e))
        })?;

        self.persist_and_commit(
            &repo,
            &rel_file_path,
            &toml_text,
            &format!("Update ledger {}", ledger_id),
        )
    }

    fn delete_ledger(&self, id: Uuid) -> Result<(), PersistenceError> {
        let repo = &self
            .repo
            .lock()
            .map_err(|e| PersistenceError::RepositoryError(format!("Can not lock repo: {e}")))?;

        // Find ledger relative path in map
        let ledger_rel_path = self
            .ledger_map
            .lock()
            .map_err(|e| PersistenceError::RepositoryError(format!("Can not lock ledger_map: {e}")))
            .and_then(|map| {
                map.get(&id).cloned().ok_or_else(|| {
                    PersistenceError::NotFound(format!("ledger id {} not found", id))
                })
            })?;

        // Ensure repository has a working directory
        let workdir = repo.workdir().ok_or_else(|| {
            PersistenceError::RepositoryError("repository has no working directory".into())
        })?;

        let ledger_dir = workdir.join(&ledger_rel_path);

        // Remove the ledger directory
        fs::remove_dir_all(&ledger_dir).map_err(|e| {
            PersistenceError::DataError(format!("failed to delete ledger directory: {}", e))
        })?;

        // Update git index to remove all files in the ledger directory
        let mut index = repo.index().map_err(|e| {
            PersistenceError::RepositoryError(format!("failed to get index: {}", e))
        })?;

        // Remove the ledger path from index
        index.remove_dir(&ledger_rel_path, 0).map_err(|e| {
            PersistenceError::RepositoryError(format!("failed to remove from index: {}", e))
        })?;

        index.write().map_err(|e| {
            PersistenceError::RepositoryError(format!("failed to write index: {}", e))
        })?;

        let tree_oid = index.write_tree().map_err(|e| {
            PersistenceError::RepositoryError(format!("failed to write tree: {}", e))
        })?;

        let tree = repo.find_tree(tree_oid).map_err(|e| {
            PersistenceError::RepositoryError(format!("failed to find tree: {}", e))
        })?;

        // Create signature
        let sig = repo.signature().map_err(|e| {
            PersistenceError::RepositoryError(format!("failed to create signature: {}", e))
        })?;

        // Determine parent commit (HEAD)
        let parent_commit = {
            let head = repo.head().map_err(|e| {
                PersistenceError::RepositoryError(format!("failed to get HEAD: {}", e))
            })?;
            let target = head.target().ok_or_else(|| {
                PersistenceError::RepositoryError("HEAD has no target commit".into())
            })?;
            repo.find_commit(target).map_err(|e| {
                PersistenceError::RepositoryError(format!("failed to find parent commit: {}", e))
            })?
        };

        // Commit
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            &format!("Delete ledger {}", id),
            &tree,
            &[&parent_commit],
        )
        .map_err(|e| {
            PersistenceError::RepositoryError(format!("failed to create commit: {}", e))
        })?;

        // Remove from ledger_map
        {
            let mut map = self.ledger_map.lock().map_err(|e| {
                PersistenceError::RepositoryError(format!("Can not lock ledger_map: {e}"))
            })?;

            map.remove(&id);
        }

        Ok(())
    }

    // ---------------- Transaction Operations ----------------

    fn list_transactions(
        &self,
        ledger_id: Uuid,
    ) -> Result<Vec<structs::Transaction>, PersistenceError> {
        let repo = &self
            .repo
            .lock()
            .map_err(|e| PersistenceError::RepositoryError(format!("Can not lock repo: {e}")))?;

        // Find ledger relative path in map (path is relative to repo root)
        let map = self.ledger_map.lock().map_err(|e| {
            PersistenceError::RepositoryError(format!("Can not lock ledger_map: {e}"))
        })?;

        let ledger_path = match map.get(&ledger_id) {
            Some(p) => p.clone(),
            None => {
                return Err(PersistenceError::NotFound(format!(
                    "ledger id {} not found. Known ledgers: {:?}",
                    ledger_id, map
                )));
            }
        };
        drop(map);

        // Get root tree and find the subtree for the ledger relative path
        let root_tree = self.get_root_tree(&repo)?;
        let ledger_tree = self.subtree_from_tree(&repo, &root_tree, &ledger_path)?;

        let mut transactions = Vec::new();

        for entry in ledger_tree.iter() {
            let name = match entry.name() {
                Some(n) => n,
                None => {
                    eprintln!("skipping entry with no name");
                    continue;
                }
            };

            // skip hidden files and marker file
            if name.starts_with('.') {
                continue;
            }

            match entry.kind() {
                Some(ObjectType::Blob) => {
                    let blob = repo.find_blob(entry.id()).map_err(|e| {
                        PersistenceError::RepositoryError(format!(
                            "failed to read blob {}: {}",
                            name, e
                        ))
                    })?;
                    let text = str::from_utf8(blob.content()).map_err(|e| {
                        PersistenceError::DataError(format!("UTF-8 decode error: {}", e))
                    })?;
                    match toml::from_str::<structs::Transaction>(text) {
                        Ok(tx) => transactions.push(tx),
                        Err(e) => eprintln!("failed to parse {}: {}", name, e),
                    }
                }
                Some(kind) => {
                    eprintln!("skipping non-blob entry {}: {:?}", name, kind);
                }
                None => {
                    eprintln!("entry {} has no object type", name);
                }
            }
        }

        Ok(transactions)
    }

    fn create_transaction(
        &self,
        ledger_id: Uuid,
        transaction: structs::Transaction,
    ) -> Result<Uuid, PersistenceError> {
        let repo = &self
            .repo
            .lock()
            .map_err(|e| PersistenceError::RepositoryError(format!("Can not lock repo: {e}")))?;

        // Find ledger relative path in map (path is relative to repo root)
        let ledger_rel_path = {
            let map = self.ledger_map.lock().map_err(|e| {
                PersistenceError::RepositoryError(format!("Can not lock ledger_map: {e}"))
            })?;

            match map.get(&ledger_id) {
                Some(p) => p.clone(),
                None => {
                    return Err(PersistenceError::NotFound(format!(
                        "ledger id {} not found",
                        ledger_id
                    )));
                }
            }
        };

        let transaction_id = transaction.id;
        let rel_file_path = ledger_rel_path.join(format!("{}.toml", transaction_id));

        // Serialize transaction to TOML
        let toml_text = toml::to_string_pretty(&transaction).map_err(|e| {
            PersistenceError::DataError(format!(
                "failed to serialize transaction {}: {}",
                transaction_id, e
            ))
        })?;

        self.persist_and_commit(
            &repo,
            &rel_file_path,
            &toml_text,
            &format!("Add transaction {} to ledger {}", transaction_id, ledger_id),
        )?;

        Ok(transaction_id)
    }

    fn update_transaction(
        &self,
        ledger_id: Uuid,
        transaction: structs::Transaction,
    ) -> Result<(), PersistenceError> {
        let repo = &self
            .repo
            .lock()
            .map_err(|e| PersistenceError::RepositoryError(format!("Can not lock repo: {e}")))?;

        // Find ledger relative path in map (path is relative to repo root)
        let ledger_rel_path = self
            .ledger_map
            .lock()
            .map_err(|e| PersistenceError::RepositoryError(format!("Can not lock ledger_map: {e}")))
            .and_then(|map| {
                map.get(&ledger_id).cloned().ok_or_else(|| {
                    PersistenceError::NotFound(format!("ledger id {} not found", ledger_id))
                })
            })?;

        let transaction_id = transaction.id;
        let rel_file_path = ledger_rel_path.join(format!("{}.toml", transaction_id));

        // Serialize transaction to TOML
        let toml_text = toml::to_string_pretty(&transaction).map_err(|e| {
            PersistenceError::DataError(format!(
                "failed to serialize transaction {}: {}",
                transaction_id, e
            ))
        })?;

        self.persist_and_commit(
            &repo,
            &rel_file_path,
            &toml_text,
            &format!(
                "Update transaction {} in ledger {}",
                transaction_id, ledger_id
            ),
        )
    }

    fn delete_transaction(
        &self,
        ledger_id: Uuid,
        transaction_id: Uuid,
    ) -> Result<(), PersistenceError> {
        let repo = &self
            .repo
            .lock()
            .map_err(|e| PersistenceError::RepositoryError(format!("Can not lock repo: {e}")))?;

        // Find ledger relative path in map
        let ledger_rel_path = self
            .ledger_map
            .lock()
            .map_err(|e| PersistenceError::RepositoryError(format!("Can not lock ledger_map: {e}")))
            .and_then(|map| {
                map.get(&ledger_id).cloned().ok_or_else(|| {
                    PersistenceError::NotFound(format!("ledger id {} not found", ledger_id))
                })
            })?;

        let rel_file_path = ledger_rel_path.join(format!("{}.toml", transaction_id));

        // Ensure repository has a working directory
        let workdir = repo.workdir().ok_or_else(|| {
            PersistenceError::RepositoryError("repository has no working directory".into())
        })?;

        let full_fs_path = workdir.join(&rel_file_path);

        // Remove the transaction file
        fs::remove_file(&full_fs_path).map_err(|e| {
            PersistenceError::DataError(format!("failed to delete transaction file: {}", e))
        })?;

        // Update git index to remove the file
        let mut index = repo.index().map_err(|e| {
            PersistenceError::RepositoryError(format!("failed to get index: {}", e))
        })?;

        index.remove_path(&rel_file_path).map_err(|e| {
            PersistenceError::RepositoryError(format!("failed to remove from index: {}", e))
        })?;

        index.write().map_err(|e| {
            PersistenceError::RepositoryError(format!("failed to write index: {}", e))
        })?;

        let tree_oid = index.write_tree().map_err(|e| {
            PersistenceError::RepositoryError(format!("failed to write tree: {}", e))
        })?;

        let tree = repo.find_tree(tree_oid).map_err(|e| {
            PersistenceError::RepositoryError(format!("failed to find tree: {}", e))
        })?;

        // Create signature
        let sig = repo.signature().map_err(|e| {
            PersistenceError::RepositoryError(format!("failed to create signature: {}", e))
        })?;

        // Determine parent commit (HEAD)
        let parent_commit = {
            let head = repo.head().map_err(|e| {
                PersistenceError::RepositoryError(format!("failed to get HEAD: {}", e))
            })?;
            let target = head.target().ok_or_else(|| {
                PersistenceError::RepositoryError("HEAD has no target commit".into())
            })?;
            repo.find_commit(target).map_err(|e| {
                PersistenceError::RepositoryError(format!("failed to find parent commit: {}", e))
            })?
        };

        // Commit
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            &format!(
                "Delete transaction {} from ledger {}",
                transaction_id, ledger_id
            ),
            &tree,
            &[&parent_commit],
        )
        .map_err(|e| {
            PersistenceError::RepositoryError(format!("failed to create commit: {}", e))
        })?;

        Ok(())
    }

    // ---------------- Storage Operations ----------------

    fn has_local_changes(&self) -> Result<bool, PersistenceError> {
        let repo = &self
            .repo
            .lock()
            .map_err(|e| PersistenceError::RepositoryError(format!("Can not lock repo: {e}")))?;
        Self::repo_has_local_changes(repo)
    }

    fn refresh(&self) -> Result<crate::traits::RefreshResult, PersistenceError> {
        let has_changes = {
            let repo_guard = self.repo.lock().map_err(|e| {
                PersistenceError::RepositoryError(format!("Can not lock repo: {e}"))
            })?;
            let repo = &*repo_guard;

            log::info!("Starting repository refresh...");

            let old_head = repo
                .head()
                .and_then(|h| {
                    h.target()
                        .ok_or_else(|| git2::Error::from_str("HEAD has no target"))
                })
                .map_err(|e| {
                    PersistenceError::RepositoryError(format!(
                        "failed to read HEAD before refresh: {e}"
                    ))
                })?;

            let private_key_path = get_private_key_path().map_err(|e| {
                log::error!("Failed to resolve SSH key path: {e}");
                PersistenceError::RepositoryError(format!("failed to get private key path: {e}"))
            })?;
            let private_key_path = private_key_path.to_owned();

            let fetch_key_path = private_key_path.clone();
            let mut callbacks = RemoteCallbacks::new();
            callbacks.credentials(move |url, username_from_url, allowed_types| {
                log::info!("Credentials requested for fetch from URL: {}", url);
                log::info!("Allowed credential types: {:?}", allowed_types);
                
                // Check if SSH key authentication is allowed
                if allowed_types.contains(git2::CredentialType::SSH_KEY) {
                    log::info!("Providing SSH credentials for git fetch");
                    Cred::ssh_key(
                        username_from_url.unwrap_or("git"),
                        None,
                        &fetch_key_path,
                        None,
                    )
                } else if allowed_types.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
                    log::info!("Attempting default git credentials for HTTPS");
                    Cred::credential_helper(&git2::Config::open_default().ok().unwrap(), url, username_from_url)
                } else {
                    log::error!("No supported credential type available");
                    Err(git2::Error::from_str("No supported credential type"))
                }
            });

            let mut fetch_options = git2::FetchOptions::new();
            fetch_options.remote_callbacks(callbacks);

            let mut remote = repo.find_remote("origin").map_err(|e| {
                log::error!("Unable to locate remote 'origin': {e}");
                PersistenceError::RepositoryError(format!("failed to find remote 'origin': {e}"))
            })?;

            log::info!("Fetching origin/main...");
            remote
                .fetch(&["main"], Some(&mut fetch_options), None)
                .map_err(|e| {
                    log::error!("Fetch failed: {e}");
                    PersistenceError::RepositoryError(format!("failed to fetch origin/main: {e}"))
                })?;

            log::info!("Fetch complete; remote tracking branch updated");
            drop(remote);

            let upstream_ref = repo
                .find_reference("refs/remotes/origin/main")
                .map_err(|e| {
                    PersistenceError::RepositoryError(format!(
                        "failed to find refs/remotes/origin/main after fetch: {e}"
                    ))
                })?;
            let upstream_annotated =
                repo.reference_to_annotated_commit(&upstream_ref)
                    .map_err(|e| {
                        log::error!("Failed to create annotated commit for origin/main: {e}");
                        PersistenceError::RepositoryError(format!(
                            "failed to create annotated commit for origin/main: {e}"
                        ))
                    })?;

            let sig = repo.signature().map_err(|e| {
                log::error!("Failed to resolve git signature: {e}");
                PersistenceError::RepositoryError(format!("failed to get git signature: {e}"))
            })?;

            let mut merge_opt = MergeOptions::new();
            merge_opt.fail_on_conflict(true);
            let mut rb = repo
                .rebase(
                    None, // current branch/HEAD
                    Some(&upstream_annotated),
                    None, // no explicit onto
                    Some(&mut git2::RebaseOptions::new().merge_options(merge_opt)),
                )
                .map_err(|e| {
                    log::error!("Failed to start rebase: {e}");
                    PersistenceError::RepositoryError(format!("failed to start rebase: {e}"))
                })?;

            let mut rebase_operations = 0usize;
            while let Some(op) = rb.next().transpose().map_err(|e| {
                log::error!("Rebase operation failed: {e}");
                PersistenceError::RepositoryError(format!("rebase apply failed: {e}"))
            })? {
                rebase_operations += 1;
                let oid = op.id();
                log::info!("Applied rebase operation for commit {oid}");

                if op.kind() != Some(RebaseOperationType::Exec) {
                    rb.commit(None, &sig, None).map_err(|e| {
                        log::error!("Failed to commit rebased change: {e}");
                        PersistenceError::RepositoryError(format!(
                            "failed to commit rebase step: {e}"
                        ))
                    })?;
                } else {
                    log::info!("Rebase operation was EXEC; skipping commit");
                }
            }

            log::info!("Rebase applied {rebase_operations} operations; finalizing");

            rb.finish(Some(&sig)).map_err(|e| {
                log::error!("Failed to finish rebase: {e}");
                PersistenceError::RepositoryError(format!("failed to finish rebase: {e}"))
            })?;

            repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
                .map_err(|e| {
                    log::error!("Failed to checkout rebased HEAD: {e}");
                    PersistenceError::RepositoryError(format!(
                        "failed to checkout HEAD after rebase: {e}"
                    ))
                })?;

            let head_after_rebase = repo.head().map_err(|e| {
                PersistenceError::RepositoryError(format!("failed to read HEAD after refresh: {e}"))
            })?;
            let head_ref_name_opt = head_after_rebase.name().map(|n| n.to_string());
            let new_head = head_after_rebase
                .target()
                .ok_or_else(|| git2::Error::from_str("HEAD has no target"))
                .map_err(|e| {
                    PersistenceError::RepositoryError(format!(
                        "failed to read HEAD after refresh: {e}"
                    ))
                })?;

            if Self::repo_has_local_changes(repo)? {
                log::info!("Local repository has commits not on origin/main; pushing");

                let branch_ref = match head_ref_name_opt {
                    Some(name) => name,
                    None => {
                        log::error!("HEAD reference has no name; cannot push to origin");
                        return Err(PersistenceError::RepositoryError(
                            "HEAD reference missing name; cannot push".into(),
                        ));
                    }
                };

                let refspec = format!("{branch_ref}:{branch_ref}");
                
                let push_key_path = private_key_path.clone();
                let mut push_callbacks = RemoteCallbacks::new();
                push_callbacks.credentials(move |url, username_from_url, allowed_types| {
                    log::info!("Credentials requested for push to URL: {}", url);
                    log::info!("Allowed credential types: {:?}", allowed_types);
                    
                    // Check if SSH key authentication is allowed
                    if allowed_types.contains(git2::CredentialType::SSH_KEY) {
                        log::info!("Providing SSH credentials for git push");
                        Cred::ssh_key(
                            username_from_url.unwrap_or("git"),
                            None,
                            &push_key_path,
                            None,
                        )
                    } else if allowed_types.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
                        log::info!("Attempting default git credentials for HTTPS");
                        Cred::credential_helper(&git2::Config::open_default().ok().unwrap(), url, username_from_url)
                    } else {
                        log::error!("No supported credential type available");
                        Err(git2::Error::from_str("No supported credential type"))
                    }
                });

                let mut push_options = PushOptions::new();
                push_options.remote_callbacks(push_callbacks);

                let mut remote = repo.find_remote("origin").map_err(|e| {
                    log::error!("Unable to locate remote 'origin' for push: {e}");
                    PersistenceError::RepositoryError(format!(
                        "failed to find remote 'origin' for push: {e}"
                    ))
                })?;

                remote
                    .push(&[refspec.as_str()], Some(&mut push_options))
                    .map_err(|e| {
                        log::error!("Failed to push to origin/main: {e}");
                        PersistenceError::RepositoryError(format!(
                            "failed to push to origin/main: {e}"
                        ))
                    })?;
                log::info!("Push to origin/main completed successfully");
            } else {
                log::info!("No push required; local matches origin/main");
            }

            old_head != new_head
        };

        log::info!("Refresh complete; rebuilding ledger map");
        self.build_ledger_map()?;
        log::info!("Ledger map rebuild finished");

        Ok(crate::traits::RefreshResult { has_changes })
    }
}

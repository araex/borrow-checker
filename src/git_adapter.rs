use crate::structs;
use crate::traits::{PersistenceError, PersistenceRepository};
use git2::{ObjectType, Repository, Tree};
use std::collections::HashMap;
use std::env;
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
    repo: Repository,
    /// Map from ledger id -> relative path (under repo root, e.g. "ledgers/39C3")
    ledger_map: Mutex<HashMap<Uuid, PathBuf>>,

    /// The path under repo root where ledgers live (default "ledgers")
    ledgers_root: PathBuf,
}

impl GitPersistence {
    /// Open a repository. If `repo_path` is None the default
    /// "data/borrow-checker-testdata/" relative to the current working directory is used.
    pub fn new(repo_path: Option<PathBuf>) -> Result<Self, PersistenceError> {
        let path = match repo_path {
            Some(p) => p,
            None => {
                let cwd = env::current_dir().map_err(|e| PersistenceError::DataError(format!("{}", e)))?;
                cwd.join("data/borrow-checker-testdata/")
            }
        };

        let repo = Repository::open(&path)
            .map_err(|e| PersistenceError::RepositoryError(format!("{}: {}", path.display(), e)))?;

        Ok(GitPersistence {
            repo,
            ledger_map: Mutex::new(HashMap::new()),
            ledgers_root: PathBuf::from("ledgers"),
        })
    }

    /// Resolve refs/heads/main or HEAD and return the repository root tree.
    fn get_root_tree<'repo>(&'repo self) -> Result<Tree<'repo>, PersistenceError> {
        let reference = self
            .repo
            .find_reference("refs/heads/main")
            .or_else(|_| self.repo.head())
            .map_err(|e| PersistenceError::RepositoryError(format!("failed to find main or HEAD: {}", e)))?;

        let target_oid = reference.target().ok_or_else(|| {
            PersistenceError::RepositoryError("reference does not point to an object".into())
        })?;

        let commit = self
            .repo
            .find_commit(target_oid)
            .map_err(|e| PersistenceError::RepositoryError(format!("failed to find commit: {}", e)))?;

        let root_tree = commit
            .tree()
            .map_err(|e| PersistenceError::RepositoryError(format!("failed to get tree: {}", e)))?;
        Ok(root_tree)
    }

    /// Given a tree and a path, return the subtree at that path.
    fn subtree_from_tree<'repo>(
        &'repo self,
        tree: &Tree<'repo>,
        path: &Path,
    ) -> Result<Tree<'repo>, PersistenceError> {
        let entry = tree
            .get_path(path)
            .map_err(|e| PersistenceError::RepositoryError(format!("failed to find path {:?}: {}", path, e)))?;

        match entry.kind() {
            Some(ObjectType::Tree) => {
                let obj = entry
                    .to_object(&self.repo)
                    .map_err(|e| PersistenceError::RepositoryError(format!("to_object failed: {}", e)))?;
                Ok(obj
                    .peel_to_tree()
                    .map_err(|e| PersistenceError::RepositoryError(format!("peel_to_tree failed: {}", e)))?)
            }
            Some(kind) => Err(PersistenceError::RepositoryError(format!(
                "path {:?} is not a tree (kind={:?})",
                path, kind
            ))),
            None => Err(PersistenceError::RepositoryError(format!(
                "entry at {:?} has no object type",
                path
            ))),
        }
    }

    /// Helper: read a blob at `path_in_repo` (relative path under repo root) and return its text.
    fn read_blob_text(&self, path_in_repo: &Path) -> Result<String, PersistenceError> {
        let root_tree = self.get_root_tree()?;
        let entry = root_tree.get_path(path_in_repo).map_err(|_| {
            PersistenceError::NotFound(format!("{} not found", path_in_repo.display()))
        })?;

        match entry.kind() {
            Some(ObjectType::Blob) => {
                let blob = self
                    .repo
                    .find_blob(entry.id())
                    .map_err(|e| PersistenceError::RepositoryError(format!("failed to read blob: {}", e)))?;
                let text = str::from_utf8(blob.content())
                    .map_err(|e| PersistenceError::DataError(format!("UTF-8 decode error: {}", e)))?;
                Ok(text.to_string())
            }
            Some(kind) => Err(PersistenceError::RepositoryError(format!(
                "expected blob at {}, got {:?}",
                path_in_repo.display(),
                kind
            ))),
            None => Err(PersistenceError::RepositoryError(format!(
                "entry at {} has no object type",
                path_in_repo.display()
            ))),
        }
    }

    /// Build the ledger map by scanning the ledgers folder.
    ///
    /// This will delegate to list_ledgers which updates the internal map for successfully parsed ledgers.
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
        rel_file_path: &Path,
        content: &str,
        commit_message: &str,
    ) -> Result<(), PersistenceError> {
        // Ensure repository has a working directory
        let workdir = self
            .repo
            .workdir()
            .ok_or_else(|| PersistenceError::RepositoryError("repository has no working directory".into()))?;

        let full_fs_path = workdir.join(rel_file_path);

        // Write to filesystem
        fs::write(&full_fs_path, content.as_bytes()).map_err(|e| {
            PersistenceError::DataError(format!("failed to write {}: {}", full_fs_path.display(), e))
        })?;

        // Update the git index
        let mut index = self
            .repo
            .index()
            .map_err(|e| PersistenceError::RepositoryError(format!("failed to get index: {}", e)))?;

        index
            .add_path(rel_file_path)
            .map_err(|e| PersistenceError::RepositoryError(format!("failed to add path to index: {}", e)))?;

        index
            .write()
            .map_err(|e| PersistenceError::RepositoryError(format!("failed to write index: {}", e)))?;

        let tree_oid = index
            .write_tree()
            .map_err(|e| PersistenceError::RepositoryError(format!("failed to write tree: {}", e)))?;

        let tree = self
            .repo
            .find_tree(tree_oid)
            .map_err(|e| PersistenceError::RepositoryError(format!("failed to find tree: {}", e)))?;

        // Create signature
        let sig = self
            .repo
            .signature()
            .map_err(|e| PersistenceError::RepositoryError(format!("failed to create signature: {}", e)))?;

        // Determine parent commit (HEAD)
        let parent_commit = {
            let head = self
                .repo
                .head()
                .map_err(|e| PersistenceError::RepositoryError(format!("failed to get HEAD: {}", e)))?;
            let target = head
                .target()
                .ok_or_else(|| PersistenceError::RepositoryError("HEAD has no target commit".into()))?;
            self.repo.find_commit(target).map_err(|e| {
                PersistenceError::RepositoryError(format!("failed to find parent commit: {}", e))
            })?
        };

        // Commit
        self.repo
            .commit(
                Some("HEAD"),
                &sig,
                &sig,
                commit_message,
                &tree,
                &[&parent_commit],
            )
            .map_err(|e| PersistenceError::RepositoryError(format!("failed to create commit: {}", e)))?;

        Ok(())
    }
}

impl PersistenceRepository for GitPersistence {
    // ---------------- Group Operations ----------------

    fn load_group(&self) -> Result<structs::Group, PersistenceError> {
        let text = self.read_blob_text(Path::new("group.toml"))?;
        let group: structs::Group =
            toml::from_str(&text).map_err(|e| PersistenceError::DataError(format!("TOML parse error: {}", e)))?;
        Ok(group)
    }

    fn save_group(&self, group: &structs::Group) -> Result<(), PersistenceError> {
        // Serialize group to TOML
        let toml_text = toml::to_string_pretty(&group).map_err(|e| {
            PersistenceError::DataError(format!("failed to serialize group: {}", e))
        })?;

        self.persist_and_commit(Path::new("group.toml"), &toml_text, "Update group configuration")
    }

    // ---------------- Ledger Operations ----------------

    fn list_ledgers(&self) -> Result<Vec<structs::Ledger>, PersistenceError> {
        // Get root tree
        let root_tree = self.get_root_tree()?;

        // Find ledgers root subtree (e.g. "ledgers")
        let ledgers_tree = match self.subtree_from_tree(&root_tree, &self.ledgers_root) {
            Ok(t) => t,
            Err(e) => {
                return Err(PersistenceError::RepositoryError(format!(
                    "failed to get ledgers subtree: {}",
                    e
                )));
            }
        };

        let mut results = Vec::new();
        // Collect successful ledger id -> relative-path entries so we can update the ledger_map as we go.
        let mut successful_map_entries: Vec<(Uuid, PathBuf)> = Vec::new();

        for entry in ledgers_tree.iter() {
            let ledger_dir_name = match entry.name() {
                Some(n) => n.to_string(),
                None => {
                    eprintln!("skipping ledger entry with no name");
                    continue;
                }
            };

            match entry.kind() {
                Some(ObjectType::Tree) => {
                    // peel to child tree
                    let child_obj = match entry.to_object(&self.repo) {
                        Ok(o) => o,
                        Err(e) => {
                            eprintln!("failed to read ledger folder {}: {}", ledger_dir_name, e);
                            continue;
                        }
                    };
                    let child_tree = match child_obj.peel_to_tree() {
                        Ok(t) => t,
                        Err(e) => {
                            eprintln!(
                                "failed to peel ledger folder {} to tree: {}",
                                ledger_dir_name, e
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
                        Some(ObjectType::Blob) => match self.repo.find_blob(ledger_blob_entry.id()) {
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
                    eprintln!(
                        "skipping non-tree ledger entry {}: {:?}",
                        ledger_dir_name, kind
                    );
                }
                None => {
                    eprintln!("ledger entry {} has no object type", ledger_dir_name);
                }
            }
        }

        // Update internal map with successfully parsed ledgers (store relative paths).
        {
            let mut map = self.ledger_map.lock().unwrap();
            // Clear existing map and insert all successful entries to keep in sync.
            map.clear();
            for (id, rel_path) in successful_map_entries {
                map.insert(id, rel_path);
            }
        }

        Ok(results)
    }

    fn create_ledger(&self, _ledger: structs::Ledger) -> Result<Uuid, PersistenceError> {
        Err(PersistenceError::UnsupportedOperation(
            "create_ledger is not implemented for GitPersistence".into(),
        ))
    }

    fn update_ledger(&self, ledger: structs::Ledger) -> Result<(), PersistenceError> {
        // Find ledger relative path in map (path is relative to repo root)
        let ledger_id = ledger.id;
        let ledger_rel_path = {
            let map = self.ledger_map.lock().unwrap();
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

        // Relative path to the marker file in the repo (e.g. "ledgers/39C3/.ledger.toml")
        let rel_file_path = ledger_rel_path.join(".ledger.toml");

        // Serialize ledger to TOML
        let toml_text = toml::to_string_pretty(&ledger).map_err(|e| {
            PersistenceError::DataError(format!("failed to serialize ledger {}: {}", ledger_id, e))
        })?;

        self.persist_and_commit(&rel_file_path, &toml_text, &format!("Update ledger {}", ledger_id))
    }

    fn delete_ledger(&self, _id: Uuid) -> Result<(), PersistenceError> {
        Err(PersistenceError::UnsupportedOperation(
            "delete_ledger is not implemented for GitPersistence".into(),
        ))
    }

    // ---------------- Transaction Operations ----------------

    fn list_transactions(
        &self,
        ledger_id: Uuid,
    ) -> Result<Vec<structs::Transaction>, PersistenceError> {
        // Find ledger relative path in map (path is relative to repo root)
        let map = self.ledger_map.lock().unwrap();
        let ledger_path = match map.get(&ledger_id) {
            Some(p) => p.clone(),
            None => {
                return Err(PersistenceError::NotFound(format!(
                    "ledger id {} not found",
                    ledger_id
                )));
            }
        };
        drop(map);

        // Get root tree and find the subtree for the ledger relative path
        let root_tree = self.get_root_tree()?;
        let ledger_tree = self.subtree_from_tree(&root_tree, &ledger_path)?;

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
                    let blob = self.repo.find_blob(entry.id()).map_err(|e| {
                        PersistenceError::RepositoryError(format!("failed to read blob {}: {}", name, e))
                    })?;
                    let text = str::from_utf8(blob.content())
                        .map_err(|e| PersistenceError::DataError(format!("UTF-8 decode error: {}", e)))?;
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
        // Find ledger relative path in map (path is relative to repo root)
        let ledger_rel_path = {
            let map = self.ledger_map.lock().unwrap();
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
            PersistenceError::DataError(format!("failed to serialize transaction {}: {}", transaction_id, e))
        })?;

        self.persist_and_commit(
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
        // Find ledger relative path in map (path is relative to repo root)
        let ledger_rel_path = {
            let map = self.ledger_map.lock().unwrap();
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
            PersistenceError::DataError(format!("failed to serialize transaction {}: {}", transaction_id, e))
        })?;

        self.persist_and_commit(
            &rel_file_path,
            &toml_text,
            &format!("Update transaction {} in ledger {}", transaction_id, ledger_id),
        )
    }

    fn delete_transaction(
        &self,
        _ledger_id: Uuid,
        _transaction_id: Uuid,
    ) -> Result<(), PersistenceError> {
        Err(PersistenceError::UnsupportedOperation(
            "delete_transaction is not implemented for GitPersistence".into(),
        ))
    }

    // ---------------- Storage Operations ----------------

    fn refresh(&self) -> Result<crate::traits::RefreshResult, PersistenceError> {
        // For now, rebuild ledger map from current HEAD tree.
        self.build_ledger_map()?;
        Ok(crate::traits::RefreshResult { has_changes: true })
    }
}

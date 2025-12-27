use crate::structs;
use crate::traits::{PersistenceError, PersistenceRepository};
use git2::{ObjectType, Repository, Tree};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::str;
use std::sync::Mutex;
use uuid::Uuid;

/// Git-backed implementation of PersistenceRepository.
///
/// The repository is opened during construction. A map from ledger UUID -> path in the repo
/// is maintained and built when listing/loading ledgers.
pub struct GitPersistence {
    repo: Repository,
    /// Map from ledger id -> path (relative path under repo root, e.g. "ledgers/39C3")
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
                let cwd = env::current_dir().map_err(|e| PersistenceError::Io(format!("{}", e)))?;
                cwd.join("data/borrow-checker-testdata/")
            }
        };

        let repo = Repository::open(&path)
            .map_err(|e| PersistenceError::RepoOpen(format!("{}: {}", path.display(), e)))?;

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
            .map_err(|e| PersistenceError::Git(format!("failed to find main or HEAD: {}", e)))?;

        let target_oid = reference.target().ok_or_else(|| {
            PersistenceError::Other("reference does not point to an object".into())
        })?;

        let commit = self
            .repo
            .find_commit(target_oid)
            .map_err(|e| PersistenceError::Git(format!("failed to find commit: {}", e)))?;

        let root_tree = commit
            .tree()
            .map_err(|e| PersistenceError::Git(format!("failed to get tree: {}", e)))?;
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
            .map_err(|e| PersistenceError::Git(format!("failed to find path {:?}: {}", path, e)))?;

        match entry.kind() {
            Some(ObjectType::Tree) => {
                let obj = entry
                    .to_object(&self.repo)
                    .map_err(|e| PersistenceError::Git(format!("to_object failed: {}", e)))?;
                Ok(obj
                    .peel_to_tree()
                    .map_err(|e| PersistenceError::Git(format!("peel_to_tree failed: {}", e)))?)
            }
            Some(kind) => Err(PersistenceError::InvalidObjectType(format!(
                "path {:?} is not a tree (kind={:?})",
                path, kind
            ))),
            None => Err(PersistenceError::InvalidObjectType(format!(
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
                    .map_err(|e| PersistenceError::Git(format!("failed to read blob: {}", e)))?;
                let text = str::from_utf8(blob.content())
                    .map_err(|e| PersistenceError::Utf8(format!("{}", e)))?;
                Ok(text.to_string())
            }
            Some(kind) => Err(PersistenceError::InvalidObjectType(format!(
                "expected blob at {}, got {:?}",
                path_in_repo.display(),
                kind
            ))),
            None => Err(PersistenceError::InvalidObjectType(format!(
                "entry at {} has no object type",
                path_in_repo.display()
            ))),
        }
    }

    /// Build the ledger map (ledger UUID -> path) by scanning the ledgers folder.
    ///
    /// This will clear and repopulate the internal ledger_map.
    fn build_ledger_map(&self) -> Result<(), PersistenceError> {
        let ledgers = self.list_ledgers()?; // list_ledgers will populate entries in result
        let mut map = self.ledger_map.lock().unwrap();
        map.clear();
        for ledger in ledgers {
            map.insert(
                ledger.id,
                self.ledgers_root.clone().join(ledger.display_name.clone()),
            );
        }
        Ok(())
    }
}

impl PersistenceRepository for GitPersistence {
    // ---------------- Group Operations ----------------

    fn load_group(&self) -> Result<structs::Group, PersistenceError> {
        let text = self.read_blob_text(Path::new("group.toml"))?;
        let group: structs::Group =
            toml::from_str(&text).map_err(|e| PersistenceError::Toml(format!("{}", e)))?;
        Ok(group)
    }

    fn save_group(&self, _group: &structs::Group) -> Result<(), PersistenceError> {
        // For a git-backed read-only implementation in the testdata tree we don't support writes yet.
        Err(PersistenceError::UnsupportedOperation(
            "save_group is not implemented for GitPersistence".into(),
        ))
    }

    // ---------------- Ledger Operations ----------------

    fn list_ledgers(&self) -> Result<Vec<structs::Ledger>, PersistenceError> {
        // Get root tree
        let root_tree = self.get_root_tree()?;

        // Find ledgers root subtree (e.g. "ledgers")
        let ledgers_tree = match self.subtree_from_tree(&root_tree, &self.ledgers_root) {
            Ok(t) => t,
            Err(e) => {
                return Err(PersistenceError::Git(format!(
                    "failed to get ledgers subtree: {}",
                    e
                )));
            }
        };

        let mut results = Vec::new();

        for entry in ledgers_tree.iter() {
            let ledger_name = match entry.name() {
                Some(n) => n.to_string(),
                None => {
                    eprintln!("skipping ledger entry with no name");
                    continue;
                }
            };

            match entry.kind() {
                Some(ObjectType::Tree) => {
                    // peel to child tree
                    let child_obj = entry.to_object(&self.repo).map_err(|e| {
                        PersistenceError::Git(format!(
                            "failed to read ledger folder {}: {}",
                            ledger_name, e
                        ))
                    })?;
                    let child_tree = child_obj.peel_to_tree().map_err(|e| {
                        PersistenceError::Git(format!(
                            "failed to peel ledger folder {} to tree: {}",
                            ledger_name, e
                        ))
                    })?;

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

                    match ledger_blob_entry.kind() {
                        Some(ObjectType::Blob) => {
                            let blob =
                                self.repo.find_blob(ledger_blob_entry.id()).map_err(|e| {
                                    PersistenceError::Git(format!(
                                        "failed to read .ledger.toml blob in {}: {}",
                                        ledger_name, e
                                    ))
                                })?;
                            let text = str::from_utf8(blob.content())
                                .map_err(|e| PersistenceError::Utf8(format!("{}", e)))?;
                            match toml::from_str::<structs::Ledger>(text) {
                                Ok(ledger) => results.push(ledger),
                                Err(e) => {
                                    eprintln!(
                                        "failed to parse .ledger.toml in {}: {}",
                                        ledger_name, e
                                    );
                                    // continue, but register parse error
                                    return Err(PersistenceError::ParseLedger {
                                        ledger_name,
                                        message: format!("{}", e),
                                    });
                                }
                            }
                        }
                        Some(k) => {
                            eprintln!(
                                "found .ledger.toml in {} but it's not a blob (kind={:?})",
                                ledger_name, k
                            );
                        }
                        None => {
                            eprintln!(".ledger.toml entry in {} has no object type", ledger_name);
                        }
                    }
                }
                Some(kind) => {
                    eprintln!("skipping non-tree ledger entry {}: {:?}", ledger_name, kind);
                }
                None => {
                    eprintln!("ledger entry {} has no object type", ledger_name);
                }
            }
        }

        // rebuild internal map
        {
            let mut map = self.ledger_map.lock().unwrap();
            map.clear();
            for ledger in &results {
                map.insert(
                    ledger.id,
                    self.ledgers_root.clone().join(ledger.display_name.clone()),
                );
            }
        }

        Ok(results)
    }

    fn create_ledger(&self, _ledger: structs::Ledger) -> Result<Uuid, PersistenceError> {
        Err(PersistenceError::UnsupportedOperation(
            "create_ledger is not implemented for GitPersistence".into(),
        ))
    }

    fn update_ledger(&self, _ledger: structs::Ledger) -> Result<(), PersistenceError> {
        Err(PersistenceError::UnsupportedOperation(
            "update_ledger is not implemented for GitPersistence".into(),
        ))
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
        // Find ledger path in map
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

        // Get root tree and find the subtree for the ledger path
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
                        PersistenceError::Git(format!("failed to read blob {}: {}", name, e))
                    })?;
                    let text = str::from_utf8(blob.content())
                        .map_err(|e| PersistenceError::Utf8(format!("{}", e)))?;
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
        _ledger_id: Uuid,
        _transaction: structs::Transaction,
    ) -> Result<Uuid, PersistenceError> {
        Err(PersistenceError::UnsupportedOperation(
            "create_transaction is not implemented for GitPersistence".into(),
        ))
    }

    fn update_transaction(
        &self,
        _ledger_id: Uuid,
        _transaction: structs::Transaction,
    ) -> Result<(), PersistenceError> {
        Err(PersistenceError::UnsupportedOperation(
            "update_transaction is not implemented for GitPersistence".into(),
        ))
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

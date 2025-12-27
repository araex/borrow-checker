pub mod git_adapter {
    use core::str;
    use std::env;
    use std::path::{Path, PathBuf};

    use git2::{ObjectType, Repository, Tree};

    use crate::structs;

    /// Given a repository and a tree, return the subtree at `path`.
    fn subtree_from_tree<'repo>(
        repo: &'repo Repository,
        tree: &Tree<'repo>,
        path: &Path,
    ) -> Result<Tree<'repo>, git2::Error> {
        // tree.get_path supports nested paths like "ledgers/39C3"
        let entry = tree.get_path(path)?; // returns TreeEntry

        // Ensure the TreeEntry is a tree and peel it into a Tree
        match entry.kind() {
            Some(ObjectType::Tree) => {
                let obj = entry.to_object(repo)?;
                obj.peel_to_tree()
            }
            Some(kind) => Err(git2::Error::from_str(&format!(
                "path is not a tree: {:?}",
                kind
            ))),
            None => Err(git2::Error::from_str("entry has no object type")),
        }
    }

    /// Resolve refs/heads/main or HEAD and return the repository root tree.
    fn get_root_tree<'repo>(repo: &'repo Repository) -> Result<Tree<'repo>, &'static str> {
        let reference = repo
            .find_reference("refs/heads/main")
            .or_else(|_| repo.head())
            .map_err(|_| "failed to find main or HEAD")?;

        let target_oid = reference
            .target()
            .ok_or("reference does not point to an object")?;

        let commit = repo
            .find_commit(target_oid)
            .map_err(|_| "failed to find commit")?;

        let root_tree = commit.tree().map_err(|_| "failed to get tree")?;
        Ok(root_tree)
    }

    pub fn get_repo() -> Repository {
        let path = env::current_dir().unwrap();
        let buf = PathBuf::from(path);
        let repo_path = buf.join("data/borrow-checker-testdata/");
        return Repository::open(repo_path).unwrap();
    }

    pub fn get_transactions(
        repo: &Repository,
        ledger_path: &Path,
    ) -> Result<Vec<structs::Transaction>, &'static str> {
        // Get the repository root tree (prefers refs/heads/main, falls back to HEAD)
        let root_tree = get_root_tree(&repo)?;

        // Find subtree for ledger_path
        let ledger_tree = match subtree_from_tree(&repo, &root_tree, ledger_path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("failed to get subtree at {:?}: {}", ledger_path, e);
                return Err("failed to get ledger subtree");
            }
        };

        // Collect transactions parsed from ledger files (skip hidden files)
        let mut transactions: Vec<structs::Transaction> = Vec::new();

        for entry in ledger_tree.iter() {
            let name = match entry.name() {
                Some(n) => n,
                None => {
                    eprintln!("skipping entry with no name");
                    continue;
                }
            };

            // Filter hidden files (names starting with '.')
            if name.starts_with('.') {
                continue;
            }
            match entry.kind() {
                Some(ObjectType::Blob) => match repo.find_blob(entry.id()) {
                    Ok(blob) => match str::from_utf8(blob.content()) {
                        Ok(text) => match toml::from_str::<structs::Transaction>(text) {
                            Ok(tx) => transactions.push(tx),
                            Err(e) => eprintln!("failed to parse {}: {}", name, e),
                        },
                        Err(e) => eprintln!("blob {} is not valid utf8: {}", name, e),
                    },
                    Err(e) => eprintln!("failed to read blob {}: {}", name, e),
                },
                Some(kind) => {
                    eprintln!("skipping non-blob entry {}: {:?}", name, kind);
                }
                None => {
                    eprintln!("entry {} has no object type", name);
                }
            }
        }

        println!("Transactions: ({})", transactions.len());
        for t in &transactions {
            println!("Desc: {}", t.description);
        }

        Ok(transactions)
    }

    /// Scan the repository for ledgers under `ledgers_path`.
    ///
    /// A ledger is any subfolder of `ledgers_path` that contains a ".ledger.toml" blob.
    /// For each found ledger this returns a tuple of (path_to_ledger, parsed Ledger).
    ///
    /// Note: `path_to_ledger` is a PathBuf built as `ledgers_path.join(<ledger-folder-name>)`.
    pub fn list_ledgers(
        repo: &Repository,
        ledgers_path: &Path,
    ) -> Result<Vec<(PathBuf, structs::Ledger)>, &'static str> {
        // Get the repository root tree (prefers refs/heads/main, falls back to HEAD)
        let root_tree = get_root_tree(repo)?;

        // Find the tree that corresponds to the ledgers_path
        let ledgers_tree = match subtree_from_tree(repo, &root_tree, ledgers_path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("failed to get ledgers subtree at {:?}: {}", ledgers_path, e);
                return Err("failed to get ledgers subtree");
            }
        };

        let mut results: Vec<(PathBuf, structs::Ledger)> = Vec::new();

        for entry in ledgers_tree.iter() {
            let ledger_name = match entry.name() {
                Some(n) => n,
                None => {
                    eprintln!("skipping ledger entry with no name");
                    continue;
                }
            };

            // We're only interested in sub-trees (folders)
            match entry.kind() {
                Some(ObjectType::Tree) => {
                    // Obtain the child tree for this ledger folder
                    let child_obj = match entry.to_object(repo) {
                        Ok(o) => o,
                        Err(e) => {
                            eprintln!("failed to read ledger folder {}: {}", ledger_name, e);
                            continue;
                        }
                    };
                    let child_tree = match child_obj.peel_to_tree() {
                        Ok(t) => t,
                        Err(e) => {
                            eprintln!(
                                "failed to peel ledger folder {} to tree: {}",
                                ledger_name, e
                            );
                            continue;
                        }
                    };

                    // Look for the marker file ".ledger.toml" inside the ledger folder
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
                        None => {
                            // not a ledger folder (no marker file)
                            continue;
                        }
                    };

                    // Ensure it's a blob and parse it
                    match ledger_blob_entry.kind() {
                        Some(ObjectType::Blob) => match repo.find_blob(ledger_blob_entry.id()) {
                            Ok(blob) => match str::from_utf8(blob.content()) {
                                Ok(text) => match toml::from_str::<structs::Ledger>(text) {
                                    Ok(ledger) => {
                                        let ledger_path = ledgers_path.join(ledger_name);
                                        results.push((ledger_path, ledger));
                                    }
                                    Err(e) => eprintln!(
                                        "failed to parse .ledger.toml in {}: {}",
                                        ledger_name, e
                                    ),
                                },
                                Err(e) => eprintln!(
                                    ".ledger.toml in {} is not valid utf8: {}",
                                    ledger_name, e
                                ),
                            },
                            Err(e) => eprintln!(
                                "failed to read .ledger.toml blob in {}: {}",
                                ledger_name, e
                            ),
                        },
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
                    // ignore non-tree entries at ledgers root
                    eprintln!("skipping non-tree ledger entry {}: {:?}", ledger_name, kind);
                }
                None => {
                    eprintln!("ledger entry {} has no object type", ledger_name);
                }
            }
        }

        Ok(results)
    }
}

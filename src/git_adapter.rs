pub mod git_adapter {
    use core::str;
    use std::path::Path;

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

    pub fn get_transactions() -> Result<Vec<structs::Transaction>, &'static str> {
        let repo_path = "/home/serafin/dev/borrow-checker/data/borrow-checker-testdata";
        let repo = match Repository::open(repo_path) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("failed to open repo {}: {}", repo_path, e);
                return Err("failed to open repository");
            }
        };

        // Try to find refs/heads/main, otherwise fall back to HEAD
        let reference = repo
            .find_reference("refs/heads/main")
            .or_else(|_| repo.head())
            .map_err(|_| "failed to find main or HEAD")
            .unwrap();

        // Resolve the reference to the commit OID
        let target_oid = reference
            .target()
            .ok_or("reference does not point to an object")
            .unwrap();

        // Find the commit and its tree
        let commit = repo
            .find_commit(target_oid)
            .map_err(|_| "failed to find commit")
            .unwrap();
        let root_tree = commit.tree().map_err(|_| "failed to get tree").unwrap();

        // The hard-coded ledger path
        let ledger_path = Path::new("ledgers/39C3");

        // Find subtree for ledger_path
        let ledger_tree = match subtree_from_tree(&repo, &root_tree, ledger_path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("failed to get subtree at {:?}: {}", ledger_path, e);
                return Err("failed to get ledger subtree");
            }
        };

        // Example: iterate entries in the subtree and print them.
        // Replace this with your parsing logic to build Transaction objects.
        // println!("Entries in ledger subtree (ledgers/39C3):");
        // for entry in ledger_tree.iter() {
        //     println!(
        //         "- name: {:<30} kind: {:?} id: {}",
        //         entry.name().unwrap_or(""),
        //         entry.kind(),
        //         entry.id()
        //     );
        // }

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
}

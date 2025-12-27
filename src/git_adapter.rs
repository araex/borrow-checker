pub mod git_adapter {
    use core::str;
    use std::path::Path;

    use git2::WorktreeAddOptions;

    use crate::structs;

    pub fn get_transactions() -> Result<Vec<structs::Transaction>, &'static str> {
        use git2::Repository;

        let repo =
            match Repository::open("/home/serafin/dev/borrow-checker/data/borrow-checker-testdata")
            {
                Ok(repo) => repo,
                Err(e) => panic!("failed to open: {}", e),
            };

        let head = repo.head();
        let wtops = WorktreeAddOptions::new();
        // let tree = repo
        //     .worktree("test", Path::new("39C3"), Some(&wtops))
        //     .unwrap();
        let repo_ref = repo.find_reference("refs/heads/main");

        println!("Head {}", head.unwrap().name().unwrap());
        // println!(
        //     "ref {}",
        //     repo_ref
        //         .clone()
        //         .unwrap()
        //         .peel_to_tree()
        //         .unwrap()
        //         .get_path(Path::new("ledgers/39C3"))
        //         .unwrap()
        //         .name()
        //         .unwrap()
        // );

        let ledger_tree = repo_ref
            .unwrap()
            .peel_to_tree()
            .unwrap()
            .get_path(Path::new("ledgers/39C3"))
            .unwrap();

        //  ledger_tree.

        return Err("Not implemented");
    }
}

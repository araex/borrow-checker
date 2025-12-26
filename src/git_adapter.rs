pub mod git_backend {
    use core::str;

    use git2::{DescribeFormatOptions, DescribeOptions, Repository};

    use super::super::structs;

    //pub fn get_transactions() -> Result<Vec<structs::Transaction>, &'static str> {
    pub fn get_transactions() -> Result<Vec<String>, &'static str> {
        use git2::Repository;

        let repo =
            match Repository::open("/home/serafin/dev/borrow-checker/data/borrow-checker-testdata")
            {
                Ok(repo) => repo,
                Err(e) => panic!("failed to open: {}", e),
            };

        let opts = DescribeOptions::new();
        let head = repo.head();
        println!("Head {}", head.unwrap().name().unwrap());

        return Err("Not implemented");
    }
}

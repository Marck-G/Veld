# Git Source Fetching

Veld fetches git-based dependencies using **libgit2** via the `git2` Rust crate. Fetched repositories are stored in a local cache for reuse across projects.

## Architecture

```
crates/veld-fetcher-git/
  src/
    lib.rs              # Re-exports
    find_repo.rs        # URL existence check
    fetch/
      fetcher.rs        # Trait + data types
      git.rs            # LibGit2Fetcher implementation
```

## Core Types

### `SourceRef`

Identifies which revision to check out:

```rust
pub enum SourceRef {
    Tag(String),
    Branch(String),
    Commit(String),
}
```

Display format: `tag:v1.0.0`, `branch:main`, `commit:abc123...`

### `FetchedSource`

Result of a successful fetch:

```rust
pub struct FetchedSource {
    pub path: PathBuf,           // Local path to the fetched source
    pub commit: String,          // 40-character SHA-1 commit hash
    pub symbolic_ref: Option<String>,  // The resolved tag or branch name
}
```

### `GitFetcher` Trait

```rust
pub trait GitFetcher {
    fn fetch(&self, url: &str, source_ref: &SourceRef, dest: &Path) -> VeldResult<FetchedSource>;
}
```

## `LibGit2Fetcher` Implementation

The `LibGit2Fetcher` struct implements the full fetch pipeline:

### Fetch Pipeline

```
fetch(url, source_ref, dest)
    |
    +--> open_or_clone(url, dest)
    |       |
    |       +--> If .git exists: open repo, fetch all remotes
    |       +--> If not: clone(url, dest)
    |
    +--> resolve_ref(repo, source_ref)
    |       |
    |       +--> Commit: revparse_single() + peel_to_commit()
    |       +--> Tag: lookup refs/tags/{tag} + peel to commit
    |       +--> Branch: try refs/remotes/origin/{branch}, then refs/heads/{branch}
    |
    +--> checkout(repo, commit)
    |       |
    |       +--> Find commit -> get tree -> force checkout -> set HEAD detached
    |
    +--> FetchedSource { path, commit, symbolic_ref }
```

### `open_or_clone()`

Checks if the destination already has a `.git` directory:
- **Yes**: Opens the existing repo and fetches all remotes to get new refs.
- **No**: Clones the repository fresh.

This enables incremental updates: if you change a dependency's tag, only the fetch step runs (not a full re-clone).

### `clone()`

Uses `git2::build::RepoBuilder` with default fetch options:

```rust
let mut builder = git2::build::RepoBuilder::new();
builder.fetch_options(self.default_fetch_options());
builder.clone(url, dest)?;
```

### `fetch_all_remotes()`

Iterates all remotes and performs a fetch with default options:

```rust
for remote_name in repo.remotes()?.iter().flatten() {
    let mut remote = repo.find_remote(remote_name)?;
    remote.fetch(&[] as &[&str], Some(&mut self.default_fetch_options()), None)?;
}
```

### `resolve_ref()`

Different strategies for each revision type:

| SourceRef | Resolution Strategy |
|-----------|-------------------|
| `Commit(hash)` | `repo.revparse_single(hash)` then `peel_to_commit()` |
| `Tag(name)` | Look up `refs/tags/{name}`, peel to commit |
| `Branch(name)` | Try `refs/remotes/origin/{name}` first, then `refs/heads/{name}` |

### `checkout()`

1. Find the commit by OID.
2. Get the tree from the commit.
3. Perform a **force checkout** of the tree to the working directory.
4. Set HEAD to a **detached state** pointing at the commit.

```rust
let obj = repo.find_object(commit_oid, Some(ObjectType::Commit))?;
repo.checkout_tree(&obj, Some(&mut opts))?;
repo.set_head_detached(commit_oid)?;
```

### `default_fetch_options()`

Sets `download_tags(Auto)` which fetches tags along with branches:

```rust
let mut callbacks = RemoteCallbacks::new();
let mut fetch_opts = FetchOptions::new();
fetch_opts.remote_callbacks(callbacks);
fetch_opts.download_tags(git2::AutotagOption::Auto);
```

## URL Existence Check

The `repo_exists()` function in `find_repo.rs` performs a lightweight HTTP check:

```rust
pub fn repo_exists(url: &str) -> VeldResult<bool> {
    match reqwest::blocking::get(url) {
        Ok(resp) => Ok(resp.status().as_u16() < 400),
        Err(_) => Ok(false),
    }
}
```

This uses `reqwest` for a blocking HTTP GET request. Network errors are treated as "repo not found" (returns `Ok(false)`).

## Error Handling

Git operations produce specific error codes:

| Error Code | Trigger |
|------------|---------|
| `E030` | Clone operation failed |
| `E031` | Reference (tag/branch/commit) not found |
| `E032` | Checkout operation failed |
| `E033` | Fetch operation failed |
| `E034` | Failed to open existing repository |
| `E035` | Failed to peel reference to commit |

## Source Directory Resolution

After fetching, the source directory is determined:

- If `subdir` is specified in the dependency source: `{dest}/{subdir}`
- Otherwise: `{dest}`

This source path is stored in the lockfile and used by the CMake generator.

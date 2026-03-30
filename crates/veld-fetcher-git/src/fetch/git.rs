use std::path::Path;

use git2::{build::CheckoutBuilder, FetchOptions, Repository};
use veld_error::{map_veld_errin, map_veld_error, veld_error, ErrorCode, ErrorContext, VeldResult};

use crate::fetch::fetcher::{FetchedSource, GitFetcher, SourceRef};
pub struct LibGit2Fetcher;

impl GitFetcher for LibGit2Fetcher {
    fn fetch<S: AsRef<str>, P: AsRef<std::path::Path>>(
        &self,
        url: S,
        source_ref: &super::fetcher::SourceRef,
        dest: P,
    ) -> veld_error::VeldResult<super::fetcher::FetchedSource> {
        let repo = open_or_clone(url.as_ref(), dest.as_ref())?;
        let commit_oid = resolve_ref(&repo, url.as_ref(), source_ref)?;
        checkout(&repo, commit_oid)?;

        let symbolic_ref = match source_ref {
            SourceRef::Tag(s) | SourceRef::Branch(s) => Some(s.clone()),
            SourceRef::Commit(_) => None,
        };

        Ok(FetchedSource {
            path: dest.as_ref().to_path_buf(),
            commit: commit_oid.to_string(),
            symbolic_ref,
        })
    }
}
fn open_or_clone(url: &str, dest: &Path) -> VeldResult<Repository> {
    if dest.join(".git").exists() {
        let repo = map_veld_errin!(Repository::open(dest), |e: git2::Error| veld_error!(
            ErrorCode::GitOpenFailed(dest.display().to_string(), e.to_string())
        ))?;
        fetch_all_remotes(&repo)?;
        Ok(repo)
    } else {
        clone(url, dest)
    }
}

fn clone(url: &str, dest: &Path) -> VeldResult<Repository> {
    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(default_fetch_options());

    map_veld_errin!(builder.clone(url, dest), |e: git2::Error| veld_error!(
        ErrorCode::GitCloneFailed(url.to_owned(), e.to_string())
    ))
}

fn fetch_all_remotes(repo: &Repository) -> VeldResult<()> {
    for remote_name in repo.remotes().unwrap().iter().flatten() {
        let mut remote =
            map_veld_errin!(repo.find_remote(remote_name), |e: git2::Error| veld_error!(
                ErrorCode::GitFetchFailed(remote_name.to_owned(), e.to_string())
            ))?;

        map_veld_errin!(
            remote.fetch(&[] as &[&str], Some(&mut default_fetch_options()), None),
            |e: git2::Error| veld_error!(ErrorCode::GitFetchFailed(
                remote_name.to_owned(),
                e.to_string()
            ))
        )?;
    }
    Ok(())
}

fn resolve_ref(repo: &Repository, url: &str, source_ref: &SourceRef) -> VeldResult<git2::Oid> {
    match source_ref {
        SourceRef::Commit(hash) => {
            map_veld_errin!(
                repo.revparse_single(hash)
                    .and_then(|obj| obj.peel_to_commit())
                    .map(|c| c.id()),
                |e: git2::Error| veld_error!(
                    ErrorCode::GitInvalidCommit(hash.clone(), e.to_string()),
                    ErrorContext::new().with_value(hash)
                )
            )
        }

        SourceRef::Tag(tag) => peel_ref_to_commit(repo, &format!("refs/tags/{tag}"), url, tag),

        SourceRef::Branch(branch) => {
            let remote_ref = format!("refs/remotes/origin/{branch}");
            let local_ref = format!("refs/heads/{branch}");

            peel_ref_to_commit(repo, &remote_ref, url, branch)
                .or_else(|_| peel_ref_to_commit(repo, &local_ref, url, branch))
        }
    }
}

fn peel_ref_to_commit(
    repo: &Repository,
    refname: &str,
    url: &str,
    display_name: &str,
) -> VeldResult<git2::Oid> {
    // We discard the git2 error here — the useful info is the ref name and url,
    // not the internal libgit2 message, so map_veld_error! is the right fit.
    let reference = map_veld_error!(
        repo.find_reference(refname),
        ErrorCode::GitRefNotFound(display_name.to_owned(), url.to_owned()),
        ErrorContext::new().with_value(display_name)
    )?;

    map_veld_errin!(
        reference.peel_to_commit().map(|c| c.id()),
        |e: git2::Error| veld_error!(
            ErrorCode::GitPeelingFailed(refname.to_owned(), e.to_string()),
            ErrorContext::new().with_value(refname)
        )
    )
}

fn checkout(repo: &Repository, oid: git2::Oid) -> VeldResult<()> {
    let commit = map_veld_errin!(repo.find_commit(oid), |e: git2::Error| veld_error!(
        ErrorCode::GitCheckoutFailed(oid.to_string(), e.to_string())
    ))?;

    let tree = map_veld_errin!(commit.tree(), |e: git2::Error| veld_error!(
        ErrorCode::GitCheckoutFailed(oid.to_string(), e.to_string())
    ))?;

    let mut opts = CheckoutBuilder::new();
    opts.force();

    map_veld_errin!(
        repo.checkout_tree(tree.as_object(), Some(&mut opts)),
        |e: git2::Error| veld_error!(ErrorCode::GitCheckoutFailed(oid.to_string(), e.to_string()))
    )?;

    map_veld_errin!(repo.set_head_detached(oid), |e: git2::Error| veld_error!(
        ErrorCode::GitCheckoutFailed(oid.to_string(), e.to_string())
    ))
}

fn default_fetch_options() -> FetchOptions<'static> {
    let mut opts = FetchOptions::new();
    opts.download_tags(git2::AutotagOption::Auto);
    opts
}

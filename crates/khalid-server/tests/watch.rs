//! Watching the site folder: a saved post shows without a restart, and
//! files Khalid writes itself, under `data/`, never cause a rebuild.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use khalid_server::{
    FolderLook, LivePages, MediaType, Page, PagePath, Render, Rendered, poll, render_pages, watch,
};

/// How often the tests look at the folder.
const EVERY: Duration = Duration::from_millis(50);

/// How long a test waits for a change before it fails.
const PATIENCE: Duration = Duration::from_secs(10);

/// Renders each file in `content/en/posts/` as `/blog/<name>/`, and counts
/// how many times it ran.
#[derive(Debug, Default)]
struct Posts {
    site: PathBuf,
    renders: AtomicUsize,
}

#[cfg(test)]
impl Render for Posts {
    type Error = std::io::Error;

    fn render(&self) -> Result<Rendered, Self::Error> {
        self.renders.fetch_add(1, Ordering::SeqCst);
        let mut pages = Vec::new();
        for entry in fs::read_dir(under(&self.site, "content/en/posts"))? {
            let path = entry?.path();
            let text = fs::read_to_string(&path)?;
            if text.contains("{{ broken") {
                return Err(std::io::Error::other("a template does not close"));
            }
            let name = path.file_stem().unwrap().to_str().unwrap().to_owned();
            let address = PagePath::try_new(&format!("/blog/{name}/")).unwrap();
            pages.push(Page::new(address, MediaType::Html, text));
        }
        Ok(Rendered::new(pages, "<h1>Page not found</h1>"))
    }
}

#[cfg(test)]
fn under(folder: &Path, relative: &str) -> PathBuf {
    let mut path = folder.to_owned();
    path.push(relative);
    path
}

/// A site folder with one post, its renderer, and the pages it serves.
#[cfg(test)]
async fn site() -> (tempfile::TempDir, Arc<Posts>, LivePages, FolderLook) {
    let folder = tempfile::tempdir().unwrap();
    fs::create_dir_all(under(folder.path(), "content/en/posts")).unwrap();
    fs::create_dir_all(under(folder.path(), "data")).unwrap();
    let post = under(folder.path(), "content/en/posts/what-a-sitemap-does.md");
    fs::write(post, "<h1>What a sitemap does</h1>").unwrap();
    let renderer = Arc::new(Posts {
        site: folder.path().to_owned(),
        renders: AtomicUsize::new(0),
    });
    let look = FolderLook::take(folder.path());
    let live = LivePages::new(render_pages(&renderer, None).await.unwrap());
    (folder, renderer, live, look)
}

/// Waits until the post reads `want`, or fails after [`PATIENCE`].
#[cfg(test)]
async fn wait_for(live: &LivePages, path: &str, want: Option<&str>) {
    let read = || {
        let pages = live.current();
        let page = pages.get(path)?;
        Some(String::from_utf8(page.body().unwrap().to_vec()).unwrap())
    };
    let waited = tokio::time::timeout(PATIENCE, async {
        while read().as_deref() != want {
            tokio::time::sleep(EVERY).await;
        }
    });
    assert!(waited.await.is_ok(), "{path} never became {want:?}");
}

#[tokio::test]
async fn a_saved_post_shows_without_a_restart() {
    let (folder, renderer, live, look) = site().await;
    let watching = watch(folder.path(), &renderer, &live, look, EVERY);
    let post = under(folder.path(), "content/en/posts/what-a-sitemap-does.md");
    tokio::time::sleep(EVERY * 3).await;

    fs::write(&post, "<h1>What a sitemap is for</h1>").unwrap();
    wait_for(
        &live,
        "/blog/what-a-sitemap-does/",
        Some("<h1>What a sitemap is for</h1>"),
    )
    .await;

    let added = under(folder.path(), "content/en/posts/what-hreflang-does.md");
    fs::write(&added, "<h1>What hreflang does</h1>").unwrap();
    wait_for(
        &live,
        "/blog/what-hreflang-does/",
        Some("<h1>What hreflang does</h1>"),
    )
    .await;

    fs::remove_file(&added).unwrap();
    wait_for(&live, "/blog/what-hreflang-does/", None).await;
    watching.stop().await;
}

#[tokio::test]
async fn a_saved_post_shows_at_once_through_file_events() {
    let (folder, renderer, live, look) = site().await;
    // Looking only once an hour, so only a file event can show the save.
    let watching = watch(
        folder.path(),
        &renderer,
        &live,
        look,
        Duration::from_secs(3600),
    );
    let post = under(folder.path(), "content/en/posts/what-a-sitemap-does.md");
    tokio::time::sleep(EVERY * 3).await;

    fs::write(&post, "<h1>What a sitemap is for</h1>").unwrap();
    wait_for(
        &live,
        "/blog/what-a-sitemap-does/",
        Some("<h1>What a sitemap is for</h1>"),
    )
    .await;
    watching.stop().await;
}

#[tokio::test]
async fn looking_at_the_folder_still_shows_a_save() {
    let (folder, renderer, live, look) = site().await;
    let watching = poll(folder.path(), &renderer, &live, look, EVERY);
    let post = under(folder.path(), "content/en/posts/what-a-sitemap-does.md");
    tokio::time::sleep(EVERY * 3).await;

    fs::write(&post, "<h1>What a sitemap is for</h1>").unwrap();
    wait_for(
        &live,
        "/blog/what-a-sitemap-does/",
        Some("<h1>What a sitemap is for</h1>"),
    )
    .await;
    watching.stop().await;
}

/// Like [`Posts`], but its full render takes [`SLOW`], and it renders a
/// saved post's page alone at once.
#[derive(Debug)]
struct SlowSite(Posts);

/// How long a full render of [`SlowSite`] takes.
const SLOW: Duration = Duration::from_secs(2);

#[cfg(test)]
impl Render for SlowSite {
    type Error = std::io::Error;

    fn render(&self) -> Result<Rendered, Self::Error> {
        std::thread::sleep(SLOW);
        self.0.render()
    }

    fn render_saved(&self, files: &[PathBuf]) -> Result<Option<Vec<Page>>, Self::Error> {
        let mut pages = Vec::new();
        for file in files {
            let mut path = self.0.site.clone();
            path.push(file);
            let name = file.file_stem().unwrap().to_str().unwrap();
            let address = PagePath::try_new(&format!("/blog/{name}/")).unwrap();
            pages.push(Page::new(address, MediaType::Html, fs::read(path)?));
        }
        Ok(Some(pages))
    }
}

#[tokio::test]
async fn a_saved_post_shows_before_the_full_render_ends() {
    let (folder, posts, live, look) = site().await;
    let renderer = Arc::new(SlowSite(Posts {
        site: posts.site.clone(),
        renders: AtomicUsize::new(0),
    }));
    let watching = watch(folder.path(), &renderer, &live, look, EVERY);
    let post = under(folder.path(), "content/en/posts/what-a-sitemap-does.md");
    tokio::time::sleep(EVERY * 3).await;

    for edit in [
        "<h1>What a sitemap is for</h1>",
        "<h1>Why a sitemap helps</h1>",
    ] {
        fs::write(&post, edit).unwrap();
        let shown = tokio::time::timeout(SLOW / 2, async {
            let page = || live.current().get("/blog/what-a-sitemap-does/").cloned();
            while page().is_none_or(|page| page.body().unwrap() != edit) {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        });
        assert!(shown.await.is_ok(), "{edit} waited for the full render");
    }
    watching.stop().await;
    // The full render that ran last read the last save, so it kept it.
    wait_for(
        &live,
        "/blog/what-a-sitemap-does/",
        Some("<h1>Why a sitemap helps</h1>"),
    )
    .await;
}

#[tokio::test]
async fn a_post_saved_before_the_watch_first_looks_still_shows() {
    let (folder, renderer, live, look) = site().await;
    let post = under(folder.path(), "content/en/posts/what-a-sitemap-does.md");
    fs::write(&post, "<h1>Saved while the server started</h1>").unwrap();
    let watching = watch(folder.path(), &renderer, &live, look, EVERY);
    wait_for(
        &live,
        "/blog/what-a-sitemap-does/",
        Some("<h1>Saved while the server started</h1>"),
    )
    .await;
    watching.stop().await;
}

#[tokio::test]
async fn files_khalid_writes_itself_cause_no_rebuild() {
    let (folder, renderer, live, look) = site().await;
    let watching = watch(folder.path(), &renderer, &live, look, EVERY);
    tokio::time::sleep(EVERY * 3).await;
    let before = renderer.renders.load(Ordering::SeqCst);

    for round in 0..5 {
        fs::write(
            under(folder.path(), "data/site.db"),
            format!("round {round}"),
        )
        .unwrap();
        fs::write(under(folder.path(), ".git-index"), format!("round {round}")).unwrap();
        tokio::time::sleep(EVERY * 2).await;
    }
    assert_eq!(renderer.renders.load(Ordering::SeqCst), before);
    watching.stop().await;
}

#[tokio::test]
async fn a_broken_save_keeps_watching() {
    let (folder, renderer, live, look) = site().await;
    let watching = watch(folder.path(), &renderer, &live, look, EVERY);
    let post = under(folder.path(), "content/en/posts/what-a-sitemap-does.md");
    let address = "/blog/what-a-sitemap-does/";
    tokio::time::sleep(EVERY * 3).await;

    let before = renderer.renders.load(Ordering::SeqCst);
    fs::write(&post, "<h1>{{ broken</h1>").unwrap();
    let tried = tokio::time::timeout(PATIENCE, async {
        while renderer.renders.load(Ordering::SeqCst) == before {
            tokio::time::sleep(EVERY).await;
        }
    });
    assert!(tried.await.is_ok(), "the broken save was never rendered");
    wait_for(&live, address, Some("<h1>What a sitemap does</h1>")).await;

    fs::write(&post, "<h1>What a sitemap is for</h1>").unwrap();
    wait_for(&live, address, Some("<h1>What a sitemap is for</h1>")).await;
    watching.stop().await;
}

// SPDX-License-Identifier: Apache-2.0
//! Actual temporary files through the native read-only coding boundary.

use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use agentmage_platform_linux::{LinuxCodingReadError, LinuxCodingReadScope};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Files(PathBuf);
impl Files {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "agentmage-code-read-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        fs::create_dir(root.join("src")).unwrap();
        fs::write(
            root.join("src/calc.py"),
            "def add(a, b):\n    return a - b\n",
        )
        .unwrap();
        Self(root)
    }
    fn paths() -> Vec<String> {
        vec!["src/calc.py".into()]
    }
    fn scope(&self) -> LinuxCodingReadScope {
        LinuxCodingReadScope::select(&self.0, &Self::paths()).unwrap()
    }
}
impl Drop for Files {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn reads_only_exact_selected_paths_and_observes_later_in_scope_edits() {
    let files = Files::new();
    let scope = files.scope();
    assert_eq!(scope.paths(), Files::paths());
    assert_eq!(
        scope.read(&Files::paths()).unwrap()[0].content,
        "def add(a, b):\n    return a - b\n"
    );
    fs::write(files.0.join("src/calc.py"), "changed by person").unwrap();
    assert_eq!(
        scope.read(&Files::paths()).unwrap()[0].content,
        "changed by person"
    );
    fs::write(files.0.join("unselected.txt"), "private").unwrap();
    for paths in [
        vec![],
        vec!["unselected.txt".into()],
        vec!["src/calc.py".into(), "src/calc.py".into()],
    ] {
        assert_eq!(
            scope.read(&paths).err(),
            Some(LinuxCodingReadError::Selection)
        );
    }
}

#[test]
fn malformed_reserved_case_colliding_and_oversized_selections_are_refused() {
    let files = Files::new();
    for path in [
        "../outside",
        "/absolute",
        "src//calc.py",
        "src/./calc.py",
        "src/../calc.py",
        "...",
        ".git/config",
        ".GIT/config",
        ".git./config",
        "src/calc.py.",
        "a\\b",
        "a:b",
        "a%2fb",
        "a\0b",
    ] {
        assert_eq!(
            LinuxCodingReadScope::select(&files.0, &[path.into()]).err(),
            Some(LinuxCodingReadError::Selection),
            "{path:?}"
        );
    }
    assert_eq!(
        LinuxCodingReadScope::select(
            &PathBuf::from(format!("/{}", "a".repeat(4096))),
            &Files::paths()
        )
        .err(),
        Some(LinuxCodingReadError::Selection)
    );
    for paths in [
        vec![],
        vec!["a".into(), "A".into()],
        vec!["src/calc.py".into(); 17],
    ] {
        assert_eq!(
            LinuxCodingReadScope::select(&files.0, &paths).err(),
            Some(LinuxCodingReadError::Selection)
        );
    }
    assert_eq!(
        LinuxCodingReadScope::select(std::path::Path::new("relative"), &Files::paths()).err(),
        Some(LinuxCodingReadError::Boundary)
    );
}

#[test]
fn symlink_hardlink_directory_and_special_file_never_become_source() {
    let files = Files::new();
    let outside = Files::new();
    symlink(outside.0.join("src/calc.py"), files.0.join("link.py")).unwrap();
    symlink(outside.0.join("src"), files.0.join("linked")).unwrap();
    fs::hard_link(outside.0.join("src/calc.py"), files.0.join("hard.py")).unwrap();
    rustix::fs::mkfifoat(
        rustix::fs::CWD,
        files.0.join("pipe.py"),
        rustix::fs::Mode::RUSR,
    )
    .unwrap();
    for path in [
        "link.py",
        "linked/calc.py",
        "hard.py",
        "src",
        "pipe.py",
        "missing.py",
    ] {
        assert_eq!(
            LinuxCodingReadScope::select(&files.0, &[path.into()]).err(),
            Some(LinuxCodingReadError::Boundary)
        );
    }
    assert_eq!(
        LinuxCodingReadScope::select(&files.0.join("linked"), &["calc.py".into()]).err(),
        Some(LinuxCodingReadError::Boundary)
    );
    let scope = files.scope();
    fs::remove_file(files.0.join("src/calc.py")).unwrap();
    symlink(outside.0.join("src/calc.py"), files.0.join("src/calc.py")).unwrap();
    assert_eq!(
        scope.read(&Files::paths()).err(),
        Some(LinuxCodingReadError::Boundary)
    );
}

#[test]
fn non_text_single_file_and_aggregate_budgets_refuse_whole_result() {
    let files = Files::new();
    for bytes in [vec![], vec![0], vec![255]] {
        fs::write(files.0.join("bad.py"), bytes).unwrap();
        assert_eq!(
            LinuxCodingReadScope::select(&files.0, &["bad.py".into()]).err(),
            Some(LinuxCodingReadError::Content)
        );
    }
    fs::write(files.0.join("bad.py"), vec![b'a'; 128 * 1024 + 1]).unwrap();
    assert_eq!(
        LinuxCodingReadScope::select(&files.0, &["bad.py".into()]).err(),
        Some(LinuxCodingReadError::Boundary)
    );
    for path in ["a.py", "b.py"] {
        fs::write(files.0.join(path), vec![b'a'; 70 * 1024]).unwrap();
    }
    assert_eq!(
        LinuxCodingReadScope::select(&files.0, &["a.py".into(), "b.py".into()]).err(),
        Some(LinuxCodingReadError::Content)
    );
    let scope = files.scope();
    fs::write(files.0.join("src/calc.py"), [0]).unwrap();
    assert_eq!(
        scope.read(&Files::paths()).err(),
        Some(LinuxCodingReadError::Content)
    );
}

#[test]
fn root_selection_holds_directory_identity_across_rename_and_replacement() {
    let files = Files::new();
    fs::create_dir(files.0.join("selected")).unwrap();
    fs::write(files.0.join("selected/source.py"), "selected").unwrap();
    let paths = vec!["source.py".into()];
    let scope = LinuxCodingReadScope::select(&files.0.join("selected"), &paths).unwrap();
    fs::rename(files.0.join("selected"), files.0.join("moved")).unwrap();
    fs::create_dir(files.0.join("selected")).unwrap();
    fs::write(files.0.join("selected/source.py"), "replacement").unwrap();
    assert_eq!(scope.read(&paths).unwrap()[0].content, "selected");
}

#[test]
fn concurrent_symlink_swaps_never_return_outside_source() {
    let files = Files::new();
    let outside = Files::new();
    fs::write(outside.0.join("src/calc.py"), "outside secret").unwrap();
    let scope = files.scope();
    let stop = Arc::new(AtomicBool::new(false));
    let worker = {
        let stop = stop.clone();
        let root = files.0.clone();
        let target = outside.0.join("src/calc.py");
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                symlink(&target, root.join("staging")).unwrap();
                fs::rename(root.join("staging"), root.join("src/calc.py")).unwrap();
                fs::write(root.join("staging"), "safe source").unwrap();
                fs::rename(root.join("staging"), root.join("src/calc.py")).unwrap();
            }
        })
    };
    for _ in 0..100 {
        match scope.read(&Files::paths()) {
            Ok(sources) => assert!(
                sources[0].content == "safe source" || sources[0].content.contains("return a - b")
            ),
            Err(error) => assert_eq!(error, LinuxCodingReadError::Boundary),
        }
    }
    stop.store(true, Ordering::Relaxed);
    worker.join().unwrap();
}

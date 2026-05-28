use std::fs;

fn create_cargo_project(dir: &std::path::Path) {
    fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"test_project\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/main.rs"), "fn main() {}").unwrap();
}

fn create_pyproject_project(dir: &std::path::Path) {
    fs::write(
        dir.join("pyproject.toml"),
        "[project]\nname = \"test_py\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    fs::create_dir_all(dir.join("src")).unwrap();
}

fn create_go_project(dir: &std::path::Path) {
    fs::write(dir.join("go.mod"), "module example.com/test\n\ngo 1.21\n").unwrap();
    fs::create_dir_all(dir.join("cmd")).unwrap();
}

fn create_package_json_project(dir: &std::path::Path) {
    fs::write(
        dir.join("package.json"),
        "{\"name\": \"test-js\", \"version\": \"1.0.0\"}\n",
    )
    .unwrap();
    fs::create_dir_all(dir.join("src")).unwrap();
}

#[test]
fn test_find_workspace_root_cargo() {
    let temp = tempfile::tempdir().unwrap();
    create_cargo_project(temp.path());

    let subdir = temp.path().join("src").join("deep").join("nested");
    fs::create_dir_all(&subdir).unwrap();
    let file = subdir.join("main.rs");
    fs::write(&file, "fn main() {}").unwrap();

    let root = splice::workspace::find_workspace_root(&file).unwrap();
    assert_eq!(root, temp.path());
}

#[test]
fn test_find_workspace_root_pyproject() {
    let temp = tempfile::tempdir().unwrap();
    create_pyproject_project(temp.path());

    let subdir = temp.path().join("src").join("pkg");
    fs::create_dir_all(&subdir).unwrap();
    let file = subdir.join("mod.py");
    fs::write(&file, "def hello(): pass").unwrap();

    let root = splice::workspace::find_workspace_root(&file).unwrap();
    assert_eq!(root, temp.path());
}

#[test]
fn test_find_workspace_root_go_mod() {
    let temp = tempfile::tempdir().unwrap();
    create_go_project(temp.path());

    let subdir = temp.path().join("cmd").join("server");
    fs::create_dir_all(&subdir).unwrap();
    let file = subdir.join("main.go");
    fs::write(&file, "package main\nfunc main() {}\n").unwrap();

    let root = splice::workspace::find_workspace_root(&file).unwrap();
    assert_eq!(root, temp.path());
}

#[test]
fn test_find_workspace_root_package_json() {
    let temp = tempfile::tempdir().unwrap();
    create_package_json_project(temp.path());

    let subdir = temp.path().join("src").join("utils");
    fs::create_dir_all(&subdir).unwrap();
    let file = subdir.join("index.ts");
    fs::write(&file, "export function foo() {}").unwrap();

    let root = splice::workspace::find_workspace_root(&file).unwrap();
    assert_eq!(root, temp.path());
}

#[test]
fn test_find_workspace_root_no_manifest() {
    let temp = tempfile::tempdir().unwrap();
    let subdir = temp.path().join("some").join("dir");
    fs::create_dir_all(&subdir).unwrap();
    let file = subdir.join("readme.txt");
    fs::write(&file, "hello").unwrap();

    let result = splice::workspace::find_workspace_root(&file);
    assert!(result.is_err(), "should fail when no manifest found");
}

#[test]
fn test_detect_project_language_rust() {
    let temp = tempfile::tempdir().unwrap();
    create_cargo_project(temp.path());

    let lang = splice::workspace::detect_project_language(temp.path());
    assert_eq!(lang, Some("rust"));
}

#[test]
fn test_detect_project_language_python() {
    let temp = tempfile::tempdir().unwrap();
    create_pyproject_project(temp.path());

    let lang = splice::workspace::detect_project_language(temp.path());
    assert_eq!(lang, Some("python"));
}

#[test]
fn test_detect_project_language_go() {
    let temp = tempfile::tempdir().unwrap();
    create_go_project(temp.path());

    let lang = splice::workspace::detect_project_language(temp.path());
    assert_eq!(lang, Some("go"));
}

#[test]
fn test_detect_project_language_javascript() {
    let temp = tempfile::tempdir().unwrap();
    create_package_json_project(temp.path());

    let lang = splice::workspace::detect_project_language(temp.path());
    assert_eq!(lang, Some("javascript"));
}

#[test]
fn test_detect_project_language_unknown() {
    let temp = tempfile::tempdir().unwrap();
    let lang = splice::workspace::detect_project_language(temp.path());
    assert_eq!(lang, None);
}

#[test]
fn test_detect_include_paths_from_workspace() {
    let temp = tempfile::tempdir().unwrap();
    create_cargo_project(temp.path());

    let paths = splice::workspace::detect_include_paths(temp.path());
    assert!(
        !paths.is_empty(),
        "should detect at least src/ for a Cargo project"
    );
    assert!(
        paths.iter().any(|p| p.contains("src")),
        "should include src/ path, got: {:?}",
        paths
    );
}

#[test]
fn test_boundary_stops_at_root() {
    let temp = tempfile::tempdir().unwrap();
    let subdir = temp.path().join("empty").join("dir");
    fs::create_dir_all(&subdir).unwrap();
    let file = subdir.join("file.txt");
    fs::write(&file, "data").unwrap();

    let result = splice::workspace::find_workspace_root(&file);
    assert!(result.is_err());
}

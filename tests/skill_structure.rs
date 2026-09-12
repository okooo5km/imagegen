use std::{fs, path::Path};
#[test]
fn skill_frontmatter_is_discoverable() {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("imagegen/SKILL.md");
    let s = fs::read_to_string(p).unwrap();
    assert!(s.starts_with("---\n"));
    let front = s.split("---").nth(1).unwrap();
    assert!(front.lines().any(|l| l == "name: imagegen"));
    let description = front
        .lines()
        .find_map(|l| l.strip_prefix("description: "))
        .unwrap();
    assert!(!description.is_empty() && description.len() <= 1024);
    assert!(s.lines().count() < 500);
}
#[test]
fn skill_local_links_resolve() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("imagegen");
    let mut docs = vec![root.join("SKILL.md")];
    docs.extend(
        fs::read_dir(root.join("references"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == "md")),
    );
    for doc in docs {
        let text = fs::read_to_string(&doc).unwrap();
        for tail in text.split("](").skip(1) {
            let target = tail.split(')').next().unwrap();
            if target.contains("://") || target.starts_with('#') {
                continue;
            }
            let target = target.split('#').next().unwrap();
            assert!(
                doc.parent().unwrap().join(target).is_file(),
                "Broken link: {} -> {target}",
                doc.display()
            );
        }
    }
}
#[test]
fn licenses_travel_with_skill() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    assert_eq!(
        fs::read(root.join("LICENSE")).unwrap(),
        fs::read(root.join("imagegen/LICENSE.txt")).unwrap()
    );
    assert_eq!(
        fs::read(root.join("NOTICE")).unwrap(),
        fs::read(root.join("imagegen/NOTICE")).unwrap()
    );
}

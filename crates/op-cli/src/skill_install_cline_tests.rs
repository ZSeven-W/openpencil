use std::fs;
use std::path::PathBuf;

use super::{install, uninstall, OWNER_MARKER};
use crate::skill_install_cli::{cline_root, SkillBundle};
use crate::skill_install_error::SkillInstallError;

fn temp_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("op-cli-cline-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("temp root");
    dir
}

fn bundle(skills: &[&str]) -> SkillBundle {
    let mut files = vec![("package.json".to_string(), "{}".to_string())];
    for name in skills {
        files.push((
            format!("skills/{name}/SKILL.md"),
            format!("---\nname: {name}\n---\n"),
        ));
        files.push((format!("skills/{name}/references/a.md"), "ref".to_string()));
    }
    SkillBundle {
        version: "0.0.0-test".into(),
        files,
    }
}

#[test]
fn install_places_each_skill_one_level_below_skills_dir() {
    let root = temp_root("layout");
    install(&root, &bundle(&["openpencil-design"])).expect("install");

    // Exactly where Cline's scanner looks: <root>/skills/<name>/SKILL.md,
    // with sibling resources still reachable.
    assert!(root.join("skills/openpencil-design/SKILL.md").is_file());
    assert!(root
        .join("skills/openpencil-design/references/a.md")
        .is_file());
    assert!(!root.join("skills/openpencil-skill").exists());
    assert!(root
        .join("openpencil-skill/skills/openpencil-design/SKILL.md")
        .is_file());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn reinstall_is_idempotent_and_drops_stale_bundle_files() {
    let root = temp_root("reinstall");
    install(&root, &bundle(&["openpencil-design"])).expect("first install");
    fs::write(root.join("openpencil-skill/stale.txt"), "old").expect("seed stale");
    install(&root, &bundle(&["openpencil-design"])).expect("second install");

    assert!(root.join("skills/openpencil-design/SKILL.md").is_file());
    assert!(!root.join("openpencil-skill/stale.txt").exists());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn install_refuses_to_replace_a_user_owned_skill() {
    let root = temp_root("conflict");
    let user_skill = root.join("skills/openpencil-design");
    fs::create_dir_all(&user_skill).expect("user skill dir");
    fs::write(user_skill.join("SKILL.md"), "mine").expect("user skill");

    let error = install(&root, &bundle(&["openpencil-design"])).expect_err("conflict");
    assert!(matches!(error, SkillInstallError::NotOwned(_)), "{error}");
    assert_eq!(
        fs::read_to_string(user_skill.join("SKILL.md")).expect("still there"),
        "mine"
    );
    // Nothing was written before the refusal.
    assert!(!root.join("openpencil-skill").exists());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn uninstall_removes_only_owned_entries() {
    let root = temp_root("uninstall");
    install(&root, &bundle(&["openpencil-design"])).expect("install");
    let other = root.join("skills/my-skill");
    fs::create_dir_all(&other).expect("other skill");
    fs::write(other.join("SKILL.md"), "mine").expect("other skill file");

    uninstall(&root).expect("uninstall");

    assert!(fs::symlink_metadata(root.join("skills/openpencil-design")).is_err());
    assert!(!root.join("openpencil-skill").exists());
    assert!(other.join("SKILL.md").is_file());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn uninstall_cleans_entries_from_an_older_bundle_and_copied_fallbacks() {
    let root = temp_root("legacy");
    install(&root, &bundle(&["old-skill", "openpencil-design"])).expect("install");
    // Simulate the copy fallback (e.g. Windows without symlink rights).
    let copied = root.join("skills/copied-skill");
    fs::create_dir_all(&copied).expect("copied dir");
    fs::write(copied.join(OWNER_MARKER), "openpencil-skill").expect("marker");

    uninstall(&root).expect("uninstall");

    assert!(fs::symlink_metadata(root.join("skills/old-skill")).is_err());
    assert!(!copied.exists());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn uninstall_without_prior_install_is_a_no_op() {
    let root = temp_root("noop");
    uninstall(&root).expect("uninstall on empty root");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn cline_root_honors_cline_dir_like_cline_does() {
    let home = PathBuf::from("/home/u");
    assert_eq!(cline_root(&home, None), home.join(".cline"));
    assert_eq!(cline_root(&home, Some("  ".into())), home.join(".cline"));
    assert_eq!(
        cline_root(&home, Some("/opt/cline".into())),
        PathBuf::from("/opt/cline")
    );
}

#[test]
fn cline_is_a_parseable_install_target() {
    let root = temp_root("target");
    // `install_target_at_home` resolves the root from `CLINE_DIR`; only
    // assert the default layout when the variable is not set here.
    if std::env::var_os("CLINE_DIR").is_none() {
        crate::skill_install_cli::install_target_at_home("cline", &root).expect("install");
        assert!(root
            .join(".cline/skills/openpencil-design/SKILL.md")
            .is_file());
        crate::skill_install_cli::uninstall_target_at_home("cline", &root).expect("uninstall");
        assert!(!root.join(".cline/openpencil-skill").exists());
    }
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn reinstall_prunes_skills_removed_from_the_bundle() {
    let root = temp_root("prune");
    install(&root, &bundle(&["old-skill", "openpencil-design"])).expect("old install");
    install(&root, &bundle(&["openpencil-design"])).expect("new install");

    assert!(fs::symlink_metadata(root.join("skills/old-skill")).is_err());
    assert!(root.join("skills/openpencil-design/SKILL.md").is_file());
    let _ = fs::remove_dir_all(&root);
}

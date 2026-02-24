use tempfile::tempdir;
use versi_backend::ShellInitOptions;
use versi_shell::{ShellConfig, ShellType};

#[test]
fn guarded_add_init_is_idempotent_on_disk() {
    let temp_dir = tempdir().expect("create temp dir");
    let config_path = temp_dir
        .path()
        .join("nested")
        .join(".config")
        .join("fish")
        .join("config.fish");

    let init_line = "fnm env --shell fish | source";

    let mut config = ShellConfig::load(ShellType::Fish, config_path.clone()).expect("load config");
    if !config.has_init("fnm env") {
        let edit = config.add_init(init_line, "fnm");
        assert!(edit.has_changes());
        config.apply_edit(&edit).expect("apply first init edit");
    }

    let content_after_first = std::fs::read_to_string(&config_path).expect("read first content");

    let mut config =
        ShellConfig::load(ShellType::Fish, config_path.clone()).expect("reload config");
    if !config.has_init("fnm env") {
        let edit = config.add_init(init_line, "fnm");
        config.apply_edit(&edit).expect("apply second init edit");
    }

    let content_after_second = std::fs::read_to_string(&config_path).expect("read second content");

    assert_eq!(content_after_first, content_after_second);
    assert_eq!(content_after_second.matches("fnm env").count(), 1);
}

#[test]
fn update_flags_without_changes_preserves_file_contents() {
    let temp_dir = tempdir().expect("create temp dir");
    let config_path = temp_dir.path().join(".zshrc");
    let original = "export PATH=$PATH:/usr/local/bin\n\
                    eval \"$(fnm env --use-on-cd --resolve-engines --shell zsh)\"\n";
    std::fs::write(&config_path, original).expect("write config");

    let mut config = ShellConfig::load(ShellType::Zsh, config_path.clone()).expect("load config");
    let options = ShellInitOptions {
        use_on_cd: true,
        resolve_engines: true,
        corepack_enabled: false,
    };
    let edit = config.update_flags("fnm env", &options);
    assert!(!edit.has_changes());

    config.apply_edit(&edit).expect("apply no-op edit");
    let reloaded = std::fs::read_to_string(&config_path).expect("read config after no-op edit");
    assert_eq!(reloaded, original);
}

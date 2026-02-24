#![cfg(target_os = "windows")]

use versi_platform::{detect_wsl_distros, execute_in_wsl};

fn integration_enabled() -> bool {
    std::env::var_os("VERSI_IT_WSL").is_some()
}

fn selected_distro() -> Option<String> {
    if let Ok(distro) = std::env::var("VERSI_IT_WSL_DISTRO") {
        if !distro.trim().is_empty() {
            return Some(distro);
        }
    }

    let distros = detect_wsl_distros(&[]);
    distros
        .iter()
        .find(|distro| distro.is_default)
        .or_else(|| distros.first())
        .map(|distro| distro.name.clone())
}

#[test]
#[ignore = "requires wsl.exe and at least one configured distro"]
fn detect_wsl_distros_lists_configured_distributions() {
    if !integration_enabled() {
        eprintln!("Skipping: set VERSI_IT_WSL=1 to run WSL integration tests");
        return;
    }

    let distros = detect_wsl_distros(&[]);
    assert!(
        !distros.is_empty(),
        "expected at least one WSL distro, got none"
    );
    assert!(
        distros.iter().all(|distro| !distro.name.trim().is_empty()),
        "all distros should have non-empty names: {distros:?}"
    );
}

#[tokio::test]
#[ignore = "requires wsl.exe and at least one configured distro"]
async fn execute_in_wsl_runs_commands_in_selected_distro() {
    if !integration_enabled() {
        eprintln!("Skipping: set VERSI_IT_WSL=1 to run WSL integration tests");
        return;
    }

    let distro = selected_distro()
        .expect("no WSL distro found; set VERSI_IT_WSL_DISTRO to target a specific distro");
    let output = execute_in_wsl(&distro, "printf versi-wsl-it")
        .await
        .expect("execute command in WSL");
    assert!(
        output.contains("versi-wsl-it"),
        "unexpected WSL output for distro {distro}: {output:?}"
    );
}

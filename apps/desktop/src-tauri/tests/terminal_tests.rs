use desktop_lib::{migrations, terminal};

#[test]
fn default_shell_returns_program() {
    let info = terminal::terminal_default_shell().expect("shell info");
    assert!(
        !info.program.trim().is_empty(),
        "expected shell program to be present"
    );
}

#[test]
fn migrations_include_run_status() {
    let definitions = migrations::definitions();
    assert!(
        definitions.iter().any(|migration| migration.version == 2),
        "expected run status migration to be registered"
    );
}

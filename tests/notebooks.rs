use std::{env, fs, path, process};

const INPUT_NOTEBOOK: &str = "tests/notebook.nb";

/// Get name of the binary
fn bin() -> path::PathBuf {
    let root = env::current_exe()
        .unwrap()
        .parent()
        .expect("executable's directory")
        .parent()
        .expect("executable's directory")
        .to_path_buf();
    if cfg!(target_os = "windows") {
        root.join("mathematica-notebook-filter.exe")
    } else {
        root.join("mathematica-notebook-filter")
    }
}

/// Check that the minimized file is smaller than the original file.
fn check_is_minimized<P: AsRef<path::Path>>(min_file: P) {
    let orig_size = fs::metadata(INPUT_NOTEBOOK)
        .expect("Input file metadata.")
        .len();
    let min_size = fs::metadata(min_file).expect("Output file metadata.").len();

    assert!(
        min_size < orig_size,
        "Minimized file is not smaller (original: {}, minimzed: {}).",
        orig_size,
        min_size
    );
}

#[test]
fn valid_notebook() {
    let mut cmd = process::Command::new(bin());
    cmd.arg("-i");
    cmd.arg(INPUT_NOTEBOOK);
    cmd.arg("-o");
    cmd.arg("tests/out-notebook.nb");

    match cmd.status() {
        Err(e) => {
            println!("Error: {}.", e);
            panic!();
        }
        Ok(status) => {
            println!("Exited with code {:?}.", status.code());
            assert!(status.success());
        }
    }

    check_is_minimized("tests/out-notebook.nb");
}

#[test]
fn valid_pipe() {
    let in_file = fs::File::open(INPUT_NOTEBOOK).expect("input notebook file");
    let out_file = fs::File::create("tests/out-pipe.nb").expect("output notebook file");

    let mut cmd = process::Command::new(bin());
    cmd.stdin(in_file);
    cmd.stdout(out_file);

    match cmd.status() {
        Err(e) => {
            println!("Error: {}.", e);
            panic!();
        }
        Ok(status) => {
            println!("Exited with code {:?}.", status.code());
            assert!(status.success());
        }
    }

    check_is_minimized("tests/out-pipe.nb");
}

#[test]
fn valid_notebook_v() {
    let mut cmd = process::Command::new(bin());
    cmd.arg("-i");
    cmd.arg(INPUT_NOTEBOOK);
    cmd.arg("-o");
    cmd.arg("tests/out-v.nb");
    cmd.arg("-v");

    match cmd.status() {
        Err(e) => {
            println!("Error: {}.", e);
            panic!();
        }
        Ok(status) => {
            println!("Exited with code {:?}.", status.code());
            assert!(status.success());
        }
    }

    check_is_minimized("tests/out-v.nb");
}

#[test]
fn valid_notebook_vv() {
    let mut cmd = process::Command::new(bin());
    cmd.arg("-i");
    cmd.arg(INPUT_NOTEBOOK);
    cmd.arg("-o");
    cmd.arg("tests/out-vv.nb");
    cmd.arg("-vv");

    match cmd.status() {
        Err(e) => {
            println!("Error: {}.", e);
            panic!();
        }
        Ok(status) => {
            println!("Exited with code {:?}.", status.code());
            assert!(status.success());
        }
    }

    check_is_minimized("tests/out-vv.nb");
}

#[test]
fn valid_notebook_vvv() {
    let mut cmd = process::Command::new(bin());
    cmd.arg("-i");
    cmd.arg(INPUT_NOTEBOOK);
    cmd.arg("-o");
    cmd.arg("tests/out-vvv.nb");
    cmd.arg("-vvv");

    match cmd.status() {
        Err(e) => {
            println!("Error: {}.", e);
            panic!();
        }
        Ok(status) => {
            println!("Exited with code {:?}.", status.code());
            assert!(status.success());
        }
    }

    check_is_minimized("tests/out-vvv.nb");
}

#[test]
fn invalid_argument() {
    let mut cmd = process::Command::new(bin());
    cmd.arg("--foobar");

    match cmd.status() {
        Err(e) => {
            println!("Error: {}.", e);
            panic!();
        }
        Ok(status) => {
            println!("Exited with code {:?}.", status.code());
            assert!(!status.success());
        }
    }
}

#[test]
fn inexistent_notebook() {
    let mut cmd = process::Command::new(bin());
    cmd.arg("-i");
    cmd.arg("tests/not-a-notebook.nb");
    cmd.arg("-o");
    cmd.arg("tests/out-inexistent.nb");

    match cmd.status() {
        Err(e) => {
            println!("Error: {}.", e);
            panic!();
        }
        Ok(status) => {
            println!("Exited with code {:?}.", status.code());
            assert!(!status.success());
        }
    }
}

#[test]
fn not_notebook() {
    let mut cmd = process::Command::new(bin());
    cmd.arg("-i");
    cmd.arg("Cargo.toml");
    cmd.arg("-o");
    cmd.arg("tests/out-not.nb");

    match cmd.status() {
        Err(e) => {
            println!("Error: {}.", e);
            panic!();
        }
        Ok(status) => {
            println!("Exited with code {:?}.", status.code());
            assert!(!status.success());
        }
    }
}

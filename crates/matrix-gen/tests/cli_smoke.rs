use std::process::Command;

#[test]
fn cli_runs_with_scripted_backend_and_writes_jsonl() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let profiles = format!("{manifest_dir}/examples/profiles.json");
    let scripted = format!("{manifest_dir}/examples/scripted.json");
    let out = tempfile::NamedTempFile::new().unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_matrix-gen"))
        .args([
            "--profiles",
            &profiles,
            "--ticks",
            "6",
            "--pairs",
            "4",
            "--clusters",
            "2",
            "--backend",
            "mock-scripted",
            "--scripted",
            &scripted,
            "--output",
            out.path().to_str().unwrap(),
        ])
        .status()
        .expect("spawn matrix-gen");
    assert!(status.success(), "matrix-gen exited non-zero");
    let body = std::fs::read_to_string(out.path()).unwrap();
    let lines: Vec<_> = body.lines().collect();
    assert_eq!(
        lines.len(),
        4,
        "expected 4 JSONL pairs, got {}",
        lines.len()
    );
    for line in lines {
        let v: serde_json::Value = serde_json::from_str(line).unwrap();
        assert!(v.get("instruction").is_some());
        assert!(v.get("response").is_some());
    }
}

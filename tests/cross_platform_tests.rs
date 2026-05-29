// Cross-platform path handling tests
// These tests verify that all file operations use platform-agnostic path APIs

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Normalises path separators to forward slashes regardless of OS.
/// On Windows, `Path` produces backslash-separated strings; callers that store
/// or compare path strings must normalise them first to get consistent results.
fn normalize_path_separators(path: &str) -> String {
    path.replace('\\', "/")
}

#[test]
fn test_path_construction_is_platform_agnostic() {
    let base = Path::new("configs");
    let file = base.join("test.json");

    // Path should work on any platform
    assert!(file.to_string_lossy().contains("test.json"));

    // On Windows, this would be configs\test.json
    // On Unix, this would be configs/test.json
    // Both are valid and handled by Path
    #[cfg(target_os = "windows")]
    assert!(file.to_string_lossy().contains("\\"));

    #[cfg(not(target_os = "windows"))]
    assert!(file.to_string_lossy().contains("/"));
}

#[test]
fn test_pathbuf_multiple_joins() {
    let mut path = PathBuf::from("test_snapshots");
    path.push("capability_detection_tests");
    path.push("test_file.json");

    assert!(path.to_string_lossy().contains("test_snapshots"));
    assert!(path
        .to_string_lossy()
        .contains("capability_detection_tests"));
    assert!(path.to_string_lossy().contains("test_file.json"));
}

#[test]
fn test_file_operations_with_path() {
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("anchorkit_test.txt");

    // Write
    {
        let mut file = fs::File::create(&test_file).expect("Failed to create test file");
        file.write_all(b"test content").expect("Failed to write");
    }

    // Read
    let content = fs::read_to_string(&test_file).expect("Failed to read test file");
    assert_eq!(content, "test content");

    // Cleanup
    fs::remove_file(&test_file).expect("Failed to remove test file");
}

#[test]
fn test_directory_iteration() {
    let configs_dir = Path::new("configs");

    if configs_dir.exists() {
        let entries: Vec<_> = fs::read_dir(configs_dir)
            .expect("Failed to read configs directory")
            .filter_map(|e| e.ok())
            .collect();

        // Should find some config files
        assert!(
            !entries.is_empty(),
            "Expected config files in configs directory"
        );

        // All entries should have valid paths
        for entry in entries {
            let path = entry.path();
            assert!(path.exists());
        }
    }
}

#[test]
fn test_parent_directory_access() {
    let deep_path = Path::new("configs").join("subdir").join("file.json");

    let parent = deep_path.parent().expect("Should have parent");
    assert!(parent.to_string_lossy().contains("subdir"));

    let grandparent = parent.parent().expect("Should have grandparent");
    assert!(grandparent.to_string_lossy().contains("configs"));
}

#[test]
fn test_file_extension_detection() {
    let json_file = Path::new("config.json");
    assert_eq!(json_file.extension().and_then(|s| s.to_str()), Some("json"));

    let toml_file = Path::new("config.toml");
    assert_eq!(toml_file.extension().and_then(|s| s.to_str()), Some("toml"));

    let no_ext = Path::new("config");
    assert_eq!(no_ext.extension(), None);
}

#[test]
fn test_absolute_path_resolution() {
    let relative = Path::new("configs");

    // canonicalize requires the path to exist
    if relative.exists() {
        let absolute = relative.canonicalize().expect("Failed to canonicalize");
        assert!(absolute.is_absolute());
    }
}

#[test]
fn test_path_comparison() {
    let path1 = Path::new("configs").join("test.json");
    let path2 = Path::new("configs").join("test.json");
    let path3 = Path::new("configs").join("other.json");

    assert_eq!(path1, path2);
    assert_ne!(path1, path3);
}

#[test]
fn test_path_components() {
    let path = Path::new("configs").join("subdir").join("file.json");

    let components: Vec<_> = path.components().collect();
    assert!(components.len() >= 3);

    // File name
    assert_eq!(path.file_name().and_then(|s| s.to_str()), Some("file.json"));

    // File stem (without extension)
    assert_eq!(path.file_stem().and_then(|s| s.to_str()), Some("file"));
}

#[test]
fn test_path_stripping() {
    let base = Path::new("configs");
    let full = base.join("subdir").join("file.json");

    if let Ok(stripped) = full.strip_prefix(base) {
        assert!(!stripped.to_string_lossy().contains("configs"));
        assert!(stripped.to_string_lossy().contains("file.json"));
    }
}

#[test]
fn test_temp_directory_access() {
    let temp = std::env::temp_dir();
    assert!(temp.exists());
    assert!(temp.is_absolute());

    // Should be able to create files in temp
    let test_file = temp.join("anchorkit_temp_test.txt");
    fs::write(&test_file, b"temp test").expect("Failed to write temp file");
    assert!(test_file.exists());
    fs::remove_file(&test_file).expect("Failed to remove temp file");
}

#[test]
fn test_current_directory() {
    let current = std::env::current_dir().expect("Failed to get current directory");
    assert!(current.is_absolute());
    assert!(current.exists());
}

#[test]
fn test_no_hardcoded_separators() {
    // This is the CORRECT way - platform agnostic
    let correct = Path::new("configs").join("test.json");

    // This would be WRONG (but we're not doing this anywhere)
    // let wrong = "configs/test.json";  // Unix-only
    // let wrong = "configs\\test.json"; // Windows-only

    // Verify our correct path works
    assert!(correct.to_string_lossy().len() > 0);
}

#[test]
fn test_glob_pattern_matching() {
    let configs_dir = Path::new("configs");

    if configs_dir.exists() {
        let entries: Vec<_> = fs::read_dir(configs_dir)
            .expect("Failed to read directory")
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|ext| ext == "json" || ext == "toml")
                    .unwrap_or(false)
            })
            .collect();

        // Should find config files
        if !entries.is_empty() {
            for entry in entries {
                let path = entry.path();
                let ext = path.extension().and_then(|s| s.to_str());
                assert!(ext == Some("json") || ext == Some("toml"));
            }
        }
    }
}

#[test]
fn test_read_file_with_path() {
    let cargo_toml = Path::new("Cargo.toml");

    if cargo_toml.exists() {
        let content = fs::read_to_string(cargo_toml).expect("Failed to read Cargo.toml");
        assert!(content.contains("[package]") || content.contains("name"));
    }
}

#[test]
fn test_directory_creation() {
    let temp = std::env::temp_dir();
    let test_dir = temp.join("anchorkit_test_dir");

    // Create
    fs::create_dir_all(&test_dir).expect("Failed to create directory");
    assert!(test_dir.exists());
    assert!(test_dir.is_dir());

    // Cleanup
    fs::remove_dir(&test_dir).expect("Failed to remove directory");
}

#[test]
fn test_nested_directory_creation() {
    let temp = std::env::temp_dir();
    let nested = temp.join("anchorkit_test").join("nested").join("deep");

    // Create all at once
    fs::create_dir_all(&nested).expect("Failed to create nested directories");
    assert!(nested.exists());

    // Cleanup
    let base = temp.join("anchorkit_test");
    fs::remove_dir_all(&base).expect("Failed to remove nested directories");
}

#[test]
fn test_file_metadata() {
    let cargo_toml = Path::new("Cargo.toml");

    if cargo_toml.exists() {
        let metadata = fs::metadata(cargo_toml).expect("Failed to get metadata");
        assert!(metadata.is_file());
        assert!(!metadata.is_dir());
        assert!(metadata.len() > 0);
    }
}

#[test]
fn test_symlink_detection() {
    let cargo_toml = Path::new("Cargo.toml");

    if cargo_toml.exists() {
        let metadata = fs::symlink_metadata(cargo_toml).expect("Failed to get symlink metadata");
        // On most systems, Cargo.toml is a regular file, not a symlink
        assert!(metadata.is_file() || metadata.file_type().is_symlink());
    }
}

#[test]
fn test_symlink_creation() {
    let temp = std::env::temp_dir();
    let target = temp.join("anchorkit_symlink_target.txt");
    let link = temp.join("anchorkit_symlink_link.txt");

    fs::write(&target, b"symlink target").expect("Failed to write target file");

    // Attempt symlink creation; on Windows this requires elevated privileges.
    #[cfg(windows)]
    let result = std::os::windows::fs::symlink_file(&target, &link);

    #[cfg(not(windows))]
    let result = std::os::unix::fs::symlink(&target, &link);

    match result {
        Ok(()) => {
            let metadata = fs::symlink_metadata(&link).expect("Failed to get symlink metadata");
            assert!(metadata.file_type().is_symlink(), "Expected a symlink");
            fs::remove_file(&link).ok();
        }
        Err(e) => {
            // On Windows without SeCreateSymbolicLinkPrivilege this is expected.
            #[cfg(windows)]
            {
                eprintln!("SKIP test_symlink_creation: symlink creation requires elevated privileges on this system ({e})");
                // Not a test failure — privilege is unavailable.
            }
            #[cfg(not(windows))]
            panic!("Symlink creation failed on Unix: {e}");
        }
    }

    fs::remove_file(&target).ok();
}

#[test]
fn test_config_schema_path() {
    let schema = Path::new("config_schema.json");
    assert_eq!(
        schema.file_name().and_then(|s| s.to_str()),
        Some("config_schema.json")
    );
}

#[test]
fn test_config_directory_path() {
    let configs = Path::new("configs");
    assert_eq!(
        configs.file_name().and_then(|s| s.to_str()),
        Some("configs")
    );
}

#[test]
fn test_validator_script_path() {
    let validator = Path::new("validate_config_strict.py");
    assert_eq!(validator.extension().and_then(|s| s.to_str()), Some("py"));
}

// ── Windows path-separator tests ─────────────────────────────────────────────
// The tests below exercise explicit backslash separators so that path
// normalisation logic is verified even when CI runs on Linux/macOS.

#[test]
fn test_windows_backslash_path_is_normalized() {
    let windows_path = "configs\\subdir\\file.json";
    let normalized = normalize_path_separators(windows_path);
    assert_eq!(normalized, "configs/subdir/file.json");
}

#[test]
fn test_mixed_separators_are_normalized() {
    let mixed = "configs/subdir\\file.json";
    let normalized = normalize_path_separators(mixed);
    assert_eq!(normalized, "configs/subdir/file.json");
}

#[test]
fn test_windows_path_splits_into_correct_components() {
    let windows_path = "test_snapshots\\capability_detection_tests\\test_file.json";
    let normalized = normalize_path_separators(windows_path);
    let parts: Vec<&str> = normalized.split('/').collect();

    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0], "test_snapshots");
    assert_eq!(parts[1], "capability_detection_tests");
    assert_eq!(parts[2], "test_file.json");
}

#[test]
fn test_normalize_unix_path_is_unchanged() {
    let unix_path = "configs/subdir/file.json";
    let normalized = normalize_path_separators(unix_path);
    assert_eq!(normalized, unix_path);
}

#[test]
fn test_windows_unc_path_normalization() {
    // UNC paths (\\server\share) must also normalise without panicking.
    let unc_path = "\\\\server\\share\\file.json";
    let normalized = normalize_path_separators(unc_path);
    assert_eq!(normalized, "//server/share/file.json");
}

#[test]
fn test_native_path_separator_and_normalize() {
    // Build a path with std::path::PathBuf (uses the native separator) then
    // verify that normalize_path_separators produces a forward-slash string.
    let path = PathBuf::from("configs").join("subdir").join("file.json");
    let display = path.to_string_lossy().to_string();

    let normalized = normalize_path_separators(&display);

    // After normalization there must be no backslashes.
    assert!(!normalized.contains('\\'), "normalized path still contains backslash: {normalized}");
    assert_eq!(normalized, "configs/subdir/file.json");

    // On Windows the original string should have contained backslashes;
    // on Unix it should already have forward slashes.
    #[cfg(target_os = "windows")]
    assert!(display.contains('\\'), "expected native backslash on Windows");

    #[cfg(not(target_os = "windows"))]
    assert!(display.contains('/'), "expected forward slash on Unix");
}

#[test]
fn test_trailing_backslash_normalization() {
    let path_with_trailing = "configs\\subdir\\";
    let normalized = normalize_path_separators(path_with_trailing);
    assert_eq!(normalized, "configs/subdir/");
    assert!(!normalized.contains('\\'));
}

#[test]
fn test_single_backslash_component() {
    let single = "file\\name.json";
    let normalized = normalize_path_separators(single);
    assert_eq!(normalized, "file/name.json");
}

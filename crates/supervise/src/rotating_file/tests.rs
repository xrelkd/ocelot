use std::{
    io::Read,
    os::unix::fs::PermissionsExt,
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use flate2::read::GzDecoder;
use lzzzz::lz4f;
use tempfile::tempdir;
use tokio::{fs, io::AsyncWriteExt, time::sleep};

use super::{LogCompression, LogRotationConfig, RotatingFile};

#[tokio::test]
async fn test_size_rotation() -> std::io::Result<()> {
    let dir = tempdir()?;
    let file_path = dir.path().join("log.txt");
    let rotation = LogRotationConfig {
        max_size_bytes: Some(100),
        rotation_interval_secs: None,
        max_files: None,
        max_age_days: None,
        mode: None,
        compression: LogCompression::None,
    };
    let mut rf = RotatingFile::new(file_path.clone(), rotation).await?;

    let data1 = b"A".repeat(50);
    rf.write_all(&data1).await?;
    assert_eq!(rf.current_size, 50);

    let data2 = b"B".repeat(60);
    rf.write_all(&data2).await?;
    assert_eq!(rf.current_size, 60);

    let metadata = fs::metadata(&file_path).await?;
    assert_eq!(metadata.len(), 60);

    let mut rotated_count = 0;
    let mut entries = fs::read_dir(dir.path()).await?;
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name();
        if let Some(s) = name.to_str()
            && s.starts_with("log.txt.")
        {
            rotated_count += 1;
        }
    }
    assert_eq!(rotated_count, 1);
    Ok(())
}

#[tokio::test]
async fn test_max_files_cleanup() -> std::io::Result<()> {
    let dir = tempdir()?;
    let file_path = dir.path().join("log.txt");
    let rotation = LogRotationConfig {
        max_size_bytes: Some(10),
        rotation_interval_secs: None,
        max_files: Some(2),
        max_age_days: None,
        mode: None,
        compression: LogCompression::None,
    };
    let mut rf = RotatingFile::new(file_path.clone(), rotation).await?;

    for i in 0..3 {
        let data = b"X".repeat(20);
        rf.write_all(&data).await?;
        if i < 2 {
            sleep(Duration::from_secs(1)).await;
        }
    }

    let mut rotated_files = Vec::new();
    let mut entries = fs::read_dir(dir.path()).await?;
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name();
        if let Some(s) = name.to_str()
            && s.starts_with("log.txt.")
        {
            rotated_files.push(s.to_string());
        }
    }
    assert_eq!(rotated_files.len(), 2);
    Ok(())
}

#[tokio::test]
async fn test_max_age_deletion() -> std::io::Result<()> {
    let dir = tempdir()?;
    let file_path = dir.path().join("log.txt");

    let rotation = LogRotationConfig {
        max_size_bytes: None,
        rotation_interval_secs: None,
        max_files: None,
        max_age_days: Some(0),
        mode: None,
        compression: LogCompression::None,
    };

    let rotated_timestamp =
        SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let rotated_path = format!("{}.{}", file_path.display(), rotated_timestamp);
    fs::write(&rotated_path, b"old log data").await?;

    sleep(Duration::from_millis(100)).await;

    let mut rf = RotatingFile::new(file_path.clone(), rotation).await?;
    rf.write_all(b"new data").await?;

    let mut entries = fs::read_dir(dir.path()).await?;
    let mut rotated_count = 0;
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name();
        if let Some(s) = name.to_str()
            && s.starts_with("log.txt.")
        {
            rotated_count += 1;
        }
    }
    assert_eq!(rotated_count, 0, "old file should have been deleted due to max_age_days=0");
    Ok(())
}

#[tokio::test]
async fn test_file_mode() -> std::io::Result<()> {
    let dir = tempdir()?;
    let file_path = dir.path().join("log.txt");
    let rotation = LogRotationConfig {
        max_size_bytes: None,
        rotation_interval_secs: None,
        max_files: None,
        max_age_days: None,
        mode: Some(0o600),
        compression: LogCompression::None,
    };
    let mut rf = RotatingFile::new(file_path.clone(), rotation).await?;

    rf.write_all(b"test data").await?;

    let metadata = fs::metadata(&file_path).await?;
    let permissions = metadata.permissions();
    let mode = permissions.mode();

    #[cfg(unix)]
    {
        let expected_mode = 0o600;
        let actual_mode = mode & 0o777;
        assert_eq!(actual_mode, expected_mode, "file mode should be 0o600");
    }

    Ok(())
}

#[tokio::test]
async fn test_gzip_compression() -> std::io::Result<()> {
    let dir = tempdir()?;
    let file_path = dir.path().join("log.txt");
    let rotation = LogRotationConfig {
        max_size_bytes: Some(10),
        rotation_interval_secs: None,
        max_files: None,
        max_age_days: None,
        mode: None,
        compression: LogCompression::Gzip,
    };
    let mut rf = RotatingFile::new(file_path.clone(), rotation).await?;

    // First write: 5 bytes (under threshold)
    rf.write_all(&b"A".repeat(5)).await?;
    assert_eq!(rf.current_size, 5);

    // Second write: 10 bytes, triggers rotation before write because 5+10 > 10
    rf.write_all(&b"B".repeat(10)).await?;
    // After rotation and write, current_size should be 10 (the second write)
    assert_eq!(rf.current_size, 10);

    // Find the compressed rotated file (should contain the first 5 'A's)
    let mut entries = fs::read_dir(dir.path()).await?;
    let mut compressed_path = None;
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name();
        if let Some(s) = name.to_str()
            && s.starts_with("log.txt.")
            && Path::new(s).extension().is_some_and(|ext| ext.eq_ignore_ascii_case("gz"))
        {
            compressed_path = Some(entry.path());
            break;
        }
    }

    let compressed_path = compressed_path.expect("compressed file should exist");
    let compressed_data = fs::read(&compressed_path).await?;
    let mut decoder = GzDecoder::new(compressed_data.as_slice());
    let mut decompressed = Vec::new();
    let _unused = decoder.read_to_end(&mut decompressed)?;
    let decompressed_str = String::from_utf8(decompressed).expect("valid utf8");
    assert_eq!(decompressed_str, "AAAAA");
    Ok(())
}

#[tokio::test]
async fn test_lz4_compression() -> std::io::Result<()> {
    let dir = tempdir()?;
    let file_path = dir.path().join("log.txt");
    let rotation = LogRotationConfig {
        max_size_bytes: Some(10),
        rotation_interval_secs: None,
        max_files: None,
        max_age_days: None,
        mode: None,
        compression: LogCompression::Lz4,
    };
    let mut rf = RotatingFile::new(file_path.clone(), rotation).await?;

    // First write: 5 bytes (under threshold)
    rf.write_all(&b"A".repeat(5)).await?;
    assert_eq!(rf.current_size, 5);

    // Second write: 10 bytes, triggers rotation before write because 5+10 > 10
    rf.write_all(&b"B".repeat(10)).await?;
    // After rotation and write, current_size should be 10 (the second write)
    assert_eq!(rf.current_size, 10);

    // Find the compressed rotated file (should contain the first 5 'A's)
    let mut entries = fs::read_dir(dir.path()).await?;
    let mut compressed_path = None;
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name();
        if let Some(s) = name.to_str()
            && s.starts_with("log.txt.")
            && Path::new(s).extension().is_some_and(|ext| ext.eq_ignore_ascii_case("lz4"))
        {
            compressed_path = Some(entry.path());
            break;
        }
    }

    let compressed_path = compressed_path.expect("compressed file should exist");
    let compressed_data = fs::read(&compressed_path).await?;
    let mut decompressed = Vec::new();
    let n = lz4f::decompress_to_vec(&compressed_data, &mut decompressed)?;
    let decompressed_str = String::from_utf8(decompressed[..n].to_vec()).expect("valid utf8");
    assert_eq!(decompressed_str, "AAAAA");
    Ok(())
}

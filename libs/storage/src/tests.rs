use crate::{StorageConfig, create_operator};

#[tokio::test]
async fn test_fs_storage_config() {
    let config = StorageConfig::Fs {
        root: std::env::temp_dir().to_string_lossy().into_owned(),
    };
    let result = create_operator(&config);
    assert!(result.is_ok(), "Failed to create fs storage operator");
}

#[test]
fn test_fs_config_parsing() {
    let toml_str = r#"
type = "fs"
root = "./storage"
"#;
    let config: StorageConfig = toml::from_str(toml_str).expect("Failed to parse TOML config");
    let StorageConfig::Fs { root } = config else {
        panic!("Expected Fs variant");
    };
    assert_eq!(root, "./storage");
}

#[tokio::test]
async fn test_s3_storage_config() {
    let config = StorageConfig::S3 {
        bucket: "test-bucket".to_string(),
        root: "/test".to_string(),
        region: Some("us-east-1".to_string()),
        endpoint: Some("http://localhost:9000".to_string()),
        access_key_id: Some("minioadmin".to_string()),
        secret_access_key: Some("minioadmin".to_string()),
        session_token: None,
        disable_config_load: true,
        enable_virtual_host_style: false,
    };

    let result = create_operator(&config);
    assert!(result.is_ok(), "Failed to create S3 storage operator");
}

#[test]
fn test_storage_config_serialization() {
    let config = StorageConfig::S3 {
        bucket: "my-bucket".to_string(),
        root: "/recordings".to_string(),
        region: Some("us-west-2".to_string()),
        endpoint: None,
        access_key_id: Some("AKIA...".to_string()),
        secret_access_key: Some("secret...".to_string()),
        session_token: None,
        disable_config_load: false,
        enable_virtual_host_style: true,
    };

    let serialized = toml::to_string(&config).expect("Failed to serialize config");
    let deserialized: StorageConfig =
        toml::from_str(&serialized).expect("Failed to deserialize config");

    let StorageConfig::S3 { bucket: b1, .. } = &config else {
        panic!("Expected S3 variant");
    };
    let StorageConfig::S3 { bucket: b2, .. } = &deserialized else {
        panic!("Expected S3 variant");
    };
    assert_eq!(b1, b2, "Bucket names should match");
}

#[test]
fn test_default_storage_config() {
    let config = StorageConfig::default();
    let StorageConfig::Fs { root } = config else {
        panic!("Default config should be Fs variant");
    };
    assert_eq!(root, "./storage");
}

#[test]
fn test_s3_config_parsing() {
    let toml_str = r#"
type = "s3"
bucket = "test-bucket"
root = "/recordings"
region = "us-east-1"
access_key_id = "test-key"
secret_access_key = "test-secret"
enable_virtual_host_style = true
"#;

    let config: StorageConfig = toml::from_str(toml_str).expect("Failed to parse TOML config");

    let StorageConfig::S3 {
        bucket,
        root,
        region,
        enable_virtual_host_style,
        ..
    } = config
    else {
        panic!("Expected S3 variant");
    };
    assert_eq!(bucket, "test-bucket");
    assert_eq!(root, "/recordings");
    assert_eq!(region, Some("us-east-1".to_string()));
    assert!(enable_virtual_host_style);
}

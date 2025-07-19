//! JSON Schema generation for configuration validation
//!
//! This module provides JSON Schema generation for the TOML configuration
//! to enable programmatic validation and documentation.

use schemars::{schema_for, JsonSchema};
use serde_json::Value;
use std::fs;
use std::path::Path;
use thiserror::Error;

use crate::config::*;

/// Schema generation error types
#[derive(Debug, Error)]
pub enum SchemaError {
    /// Failed to generate JSON schema
    #[error("Failed to generate schema: {0}")]
    GenerationError(String),

    /// Failed to write schema to file
    #[error("Failed to write schema file: {0}")]
    WriteError(#[from] std::io::Error),

    /// Failed to serialize schema to JSON
    #[error("Failed to serialize schema: {0}")]
    SerializationError(#[from] serde_json::Error),
}

/// Generate JSON Schema for the configuration
pub fn generate_config_schema() -> Result<Value, SchemaError> {
    let schema = schema_for!(Config);
    let json = serde_json::to_value(&schema)?;
    Ok(json)
}

/// Write JSON Schema to a file
pub fn write_schema_to_file(path: &Path) -> Result<(), SchemaError> {
    let schema = generate_config_schema()?;
    let pretty_json = serde_json::to_string_pretty(&schema)?;
    fs::write(path, pretty_json)?;
    Ok(())
}

/// Validate a configuration against the schema
pub fn validate_config_json(config_json: &Value) -> Result<(), SchemaError> {
    use jsonschema::JSONSchema;

    let schema = generate_config_schema()?;
    let compiled =
        JSONSchema::compile(&schema).map_err(|e| SchemaError::GenerationError(e.to_string()))?;

    let result = compiled.validate(config_json);
    if let Err(errors) = result {
        let error_messages: Vec<String> = errors
            .map(|e| format!("{}: {}", e.instance_path, e))
            .collect();
        return Err(SchemaError::GenerationError(error_messages.join("; ")));
    }

    Ok(())
}

/// Example configuration as JSON
pub fn example_config_json() -> Value {
    serde_json::json!({
        "general": {
            "verbose": false,
            "log_file": "~/.airgapsync/sync.log",
            "threads": 0
        },
        "source": {
            "path": "/Users/username/Documents",
            "exclude": ["*.tmp", ".DS_Store", "node_modules/"],
            "follow_symlinks": false,
            "include_hidden": false
        },
        "device": [{
            "id": "USB001",
            "name": "Secure Backup USB",
            "mount_point": "/Volumes/SecureUSB",
            "encryption": {
                "algorithm": "aes256-gcm",
                "key_derivation": "pbkdf2",
                "iterations": 100000
            }
        }],
        "policy": {
            "retain_snapshots": 7,
            "retain_days": 30,
            "gc_interval_hours": 24,
            "verify_after_write": true,
            "compression_level": 3,
            "chunk_size_mb": 1,
            "parallel_files": 4,
            "buffer_size_kb": 1024
        },
        "security": {
            "key_rotation_days": 90,
            "require_authentication": true,
            "audit_level": "full",
            "audit_retention_days": 365
        },
        "notifications": {
            "notify_on_start": false,
            "notify_on_complete": true,
            "notify_on_error": true,
            "sound_on_complete": true,
            "sound_on_error": true
        },
        "advanced": {
            "snapshot_version": 1,
            "experimental_dedup": false,
            "experimental_delta_sync": false,
            "debug_encryption": false,
            "debug_performance": false,
            "save_sync_report": true
        }
    })
}

/// Configuration documentation structure
pub struct ConfigDoc {
    /// Field path (e.g. "general.verbose")
    pub field: String,
    /// Human-readable description
    pub description: String,
    /// Data type (boolean, string, integer, etc.)
    pub field_type: String,
    /// Default value if any
    pub default: Option<String>,
    /// Whether field is required
    pub required: bool,
}

/// Generate schema validation rules using JsonSchema
pub fn create_schema_validator() -> Result<jsonschema::JSONSchema, SchemaError> {
    let schema = generate_config_schema()?;
    let compiled = jsonschema::JSONSchema::compile(&schema)
        .map_err(|e| SchemaError::GenerationError(e.to_string()))?;
    Ok(compiled)
}

/// Generate enhanced schema with custom validation rules
pub fn generate_enhanced_schema() -> Result<Value, SchemaError> {
    // Generate the basic schema first
    let basic_schema = generate_config_schema()?;
    
    // Create enhanced schema by adding custom validation rules
    let mut enhanced = basic_schema.clone();
    
    // Add device ID pattern validation to the schema JSON directly
    if let Some(properties) = enhanced.get_mut("properties") {
        if let Some(device_prop) = properties.get_mut("device") {
            if let Some(items) = device_prop.get_mut("items") {
                if let Some(device_properties) = items.get_mut("properties") {
                    if let Some(id_prop) = device_properties.get_mut("id") {
                        if let Some(id_obj) = id_prop.as_object_mut() {
                            id_obj.insert("pattern".to_string(), 
                                         serde_json::Value::String("^[A-Z0-9]{3,20}$".to_string()));
                        }
                    }
                }
            }
        }
        
        // Add path format validation for source.path
        if let Some(source_prop) = properties.get_mut("source") {
            if let Some(source_properties) = source_prop.get_mut("properties") {
                if let Some(path_prop) = source_properties.get_mut("path") {
                    if let Some(path_obj) = path_prop.as_object_mut() {
                        path_obj.insert("format".to_string(), 
                                       serde_json::Value::String("path".to_string()));
                    }
                }
            }
        }
    }
    
    Ok(enhanced)
}

/// Generate schema for a specific type using JsonSchema trait
pub fn generate_type_schema<T: JsonSchema>() -> Value {
    let schema = schemars::schema_for!(T);
    serde_json::to_value(&schema).unwrap_or_default()
}

/// Get JSON schema properties for introspection
pub fn get_schema_properties(schema: &Value) -> Vec<String> {
    let mut properties = Vec::new();
    
    if let Some(props) = schema.get("properties") {
        if let Some(obj) = props.as_object() {
            for key in obj.keys() {
                properties.push(key.clone());
            }
        }
    }
    
    properties
}

/// Generate documentation for configuration fields
pub fn generate_config_docs() -> Vec<ConfigDoc> {
    vec![
        ConfigDoc {
            field: "general.verbose".to_string(),
            description: "Enable verbose logging output".to_string(),
            field_type: "boolean".to_string(),
            default: Some("false".to_string()),
            required: false,
        },
        ConfigDoc {
            field: "general.log_file".to_string(),
            description: "Path to log file (optional)".to_string(),
            field_type: "string".to_string(),
            default: None,
            required: false,
        },
        ConfigDoc {
            field: "general.threads".to_string(),
            description: "Number of worker threads (0 = auto-detect)".to_string(),
            field_type: "integer".to_string(),
            default: Some("0".to_string()),
            required: false,
        },
        ConfigDoc {
            field: "source.path".to_string(),
            description: "Source directory to sync".to_string(),
            field_type: "string".to_string(),
            default: None,
            required: true,
        },
        ConfigDoc {
            field: "source.exclude".to_string(),
            description: "Patterns to exclude (gitignore syntax)".to_string(),
            field_type: "array[string]".to_string(),
            default: Some("[]".to_string()),
            required: false,
        },
        ConfigDoc {
            field: "device[].id".to_string(),
            description: "Unique device identifier".to_string(),
            field_type: "string".to_string(),
            default: None,
            required: true,
        },
        ConfigDoc {
            field: "device[].mount_point".to_string(),
            description: "Device mount point path".to_string(),
            field_type: "string".to_string(),
            default: None,
            required: true,
        },
        ConfigDoc {
            field: "policy.retain_snapshots".to_string(),
            description: "Number of snapshots to retain".to_string(),
            field_type: "integer".to_string(),
            default: Some("7".to_string()),
            required: false,
        },
        ConfigDoc {
            field: "security.key_rotation_days".to_string(),
            description: "Days between automatic key rotation".to_string(),
            field_type: "integer".to_string(),
            default: Some("90".to_string()),
            required: false,
        },
        ConfigDoc {
            field: "security.audit_level".to_string(),
            description: "Audit logging level (none, basic, full)".to_string(),
            field_type: "string".to_string(),
            default: Some("full".to_string()),
            required: false,
        },
    ]
}

/// Generate markdown documentation for configuration
pub fn generate_markdown_docs() -> String {
    let mut doc = String::from("# Configuration Reference\n\n");
    doc.push_str("## Configuration Fields\n\n");
    doc.push_str("| Field | Type | Required | Default | Description |\n");
    doc.push_str("|-------|------|----------|---------|-------------|\n");

    for field_doc in generate_config_docs() {
        doc.push_str(&format!(
            "| `{}` | {} | {} | {} | {} |\n",
            field_doc.field,
            field_doc.field_type,
            if field_doc.required { "Yes" } else { "No" },
            field_doc.default.unwrap_or_else(|| "-".to_string()),
            field_doc.description
        ));
    }

    doc.push_str("\n## Example Configuration\n\n```toml\n");
    doc.push_str(include_str!("../../config.example.toml"));
    doc.push_str("```\n");

    doc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_generation() {
        let schema = generate_config_schema().unwrap();
        assert!(schema.is_object());
        assert!(schema.get("title").is_some());
        assert!(schema.get("properties").is_some());
    }

    #[test]
    fn test_example_validation() {
        let example = example_config_json();
        let result = validate_config_json(&example);
        if let Err(e) = &result {
            eprintln!("Validation error: {}", e);
            eprintln!("Example config: {}", serde_json::to_string_pretty(&example).unwrap());
        }
        assert!(result.is_ok(), "Validation failed: {:?}", result.err());
    }

    #[test]
    fn test_invalid_config_validation() {
        let invalid = serde_json::json!({
            "source": {
                // Missing required 'path' field
                "exclude": ["*.tmp"]
            },
            "device": []  // Empty device array
        });

        let result = validate_config_json(&invalid);
        assert!(result.is_err());
    }
}

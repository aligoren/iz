use std::collections::HashMap;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use regex::Regex;

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct IzConfig {
    pub commands: HashMap<String, String>,
    #[serde(default)]
    pub temp_dir: Option<String>,
    #[serde(default)]
    pub keep: Option<bool>,
}

pub fn parse_key_val(s: &str) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let pos = s.find('=')
        .ok_or_else(|| format!("Invalid KEY=value format: {}", s))?;
    Ok((s[..pos].to_string(), s[pos + 1..].to_string()))
}

pub fn substitute_variables(template: &str, params: &HashMap<String, String>) -> Result<String> {
    let re = Regex::new(r"#\{(\w+)\}").unwrap();
    let mut result = template.to_string();
    
    for caps in re.captures_iter(template) {
        let var_name = &caps[1];
        let full_match = &caps[0];
        
        if let Some(value) = params.get(var_name) {
            result = result.replace(full_match, value);
        } else {
            return Err(anyhow::anyhow!("Required parameter not found: {}", var_name));
        }
    }
    
    Ok(result)
}

pub fn read_config_from_path(config_path: &std::path::Path) -> Result<IzConfig> {
    if !config_path.exists() {
        return Err(anyhow::anyhow!("izconfig.json not found. Example content:\n{}", 
            serde_json::to_string_pretty(&IzConfig {
                commands: {
                    let mut map = HashMap::new();
                    map.insert("run".to_string(), "dotnet run".to_string());
                    map.insert("build".to_string(), "dotnet build".to_string());
                    map.insert("test".to_string(), "dotnet test".to_string());
                    map
                },
                temp_dir: Some(".iztemp".to_string()),
                keep: Some(false),
            })?));
    }
    
    let content = std::fs::read_to_string(config_path)
        .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;
    
    let config: IzConfig = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", config_path.display()))?;
    
    Ok(config)
}

pub fn read_config() -> Result<IzConfig> {
    let config_path = std::env::current_dir()?.join("izconfig.json");
    read_config_from_path(&config_path)
}

use anyhow::Context;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    use std::fs;

    #[test]
    fn test_parse_key_val_success() {
        let result = parse_key_val("name=Ali").unwrap();
        assert_eq!(result, ("name".to_string(), "Ali".to_string()));
        
        let result = parse_key_val("port=8080").unwrap();
        assert_eq!(result, ("port".to_string(), "8080".to_string()));
        
        let result = parse_key_val("key=value=with=equals").unwrap();
        assert_eq!(result, ("key".to_string(), "value=with=equals".to_string()));
    }

    #[test]
    fn test_parse_key_val_failure() {
        let result = parse_key_val("invalid_format");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid KEY=value format"));
    }

    #[test]
    fn test_substitute_variables_success() {
        let mut params = HashMap::new();
        params.insert("name".to_string(), "Ali".to_string());
        params.insert("port".to_string(), "8080".to_string());
        
        let result = substitute_variables("echo 'Merhaba #{name}!'", &params).unwrap();
        assert_eq!(result, "echo 'Merhaba Ali!'");
        
        let result = substitute_variables("server --port #{port}", &params).unwrap();
        assert_eq!(result, "server --port 8080");
        
        let result = substitute_variables("greet #{name} on port #{port}", &params).unwrap();
        assert_eq!(result, "greet Ali on port 8080");
    }

    #[test]
    fn test_substitute_variables_no_variables() {
        let params = HashMap::new();
        let result = substitute_variables("echo 'Hello World'", &params).unwrap();
        assert_eq!(result, "echo 'Hello World'");
    }

    #[test]
    fn test_substitute_variables_missing_param() {
        let params = HashMap::new();
        let result = substitute_variables("echo 'Hello #{name}'", &params);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Required parameter not found: name"));
    }

    #[test]
    fn test_read_config_from_path_success() {
        let temp_dir = std::env::temp_dir().join("iz-test-config");
        fs::create_dir_all(&temp_dir).unwrap();
        let config_path = temp_dir.join("izconfig.json");
        
        let config_content = r#"
        {
            "commands": {
                "run": "dotnet run",
                "test": "dotnet test"
            }
        }"#;
        
        fs::write(&config_path, config_content).unwrap();
        
        let config = read_config_from_path(&config_path).unwrap();
        assert_eq!(config.commands.get("run").unwrap(), "dotnet run");
        assert_eq!(config.commands.get("test").unwrap(), "dotnet test");
        
        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_read_config_from_path_file_not_found() {
        let temp_dir = std::env::temp_dir().join("iz-test-nonexistent");
        fs::create_dir_all(&temp_dir).unwrap();
        let config_path = temp_dir.join("nonexistent.json");
        
        let result = read_config_from_path(&config_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("izconfig.json not found"));
    }

    #[test]
    fn test_read_config_from_path_invalid_json() {
        let temp_dir = std::env::temp_dir().join("iz-test-invalid");
        fs::create_dir_all(&temp_dir).unwrap();
        let config_path = temp_dir.join("izconfig.json");
        
        fs::write(&config_path, "invalid json content").unwrap();
        
        let result = read_config_from_path(&config_path);
        assert!(result.is_err());
        
        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_iz_config_serde() {
        let mut commands = HashMap::new();
        commands.insert("run".to_string(), "dotnet run".to_string());
        commands.insert("build".to_string(), "cargo build".to_string());
        
        let config = IzConfig { 
            commands,
            temp_dir: None,
            keep: None,
        };
        
        // Serialize
        let json = serde_json::to_string(&config).unwrap();
        
        // Deserialize
        let deserialized: IzConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, deserialized);
    }

    #[test]
    fn test_iz_config_with_temp_dir_and_keep() {
        let temp_dir = std::env::temp_dir().join("iz-test-config-extended");
        fs::create_dir_all(&temp_dir).unwrap();
        let config_path = temp_dir.join("izconfig.json");
        
        let config_content = r#"
        {
            "commands": {
                "run": "dotnet run",
                "test": "dotnet test"
            },
            "temp_dir": "/tmp/iz-custom",
            "keep": true
        }"#;
        
        fs::write(&config_path, config_content).unwrap();
        
        let config = read_config_from_path(&config_path).unwrap();
        assert_eq!(config.commands.get("run").unwrap(), "dotnet run");
        assert_eq!(config.temp_dir.as_ref().unwrap(), "/tmp/iz-custom");
        assert_eq!(config.keep.unwrap(), true);
        
        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_iz_config_without_optional_fields() {
        let temp_dir = std::env::temp_dir().join("iz-test-config-minimal");
        fs::create_dir_all(&temp_dir).unwrap();
        let config_path = temp_dir.join("izconfig.json");
        
        let config_content = r#"
        {
            "commands": {
                "run": "dotnet run"
            }
        }"#;
        
        fs::write(&config_path, config_content).unwrap();
        
        let config = read_config_from_path(&config_path).unwrap();
        assert_eq!(config.commands.get("run").unwrap(), "dotnet run");
        assert!(config.temp_dir.is_none());
        assert!(config.keep.is_none());
        
        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }
} 
use clap::Parser;
use std::collections::HashMap;
use std::process::Command;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use tempfile::TempDir;
use git2::Repository;
use regex::Regex;

#[derive(Parser)]
#[command(
    name = "iz",
    about = "Git commit'lerini geçici klasörde test etmek için CLI aracı",
    version = "0.1.0"
)]
struct Cli {
    /// Git commit ID'si
    commit_id: String,
    
    /// Çalıştırılacak komut
    command: String,
    
    /// Geçici klasörü sakla
    #[arg(long)]
    keep: bool,
    
    /// Ek parametreler (--key=value formatında)
    #[arg(long, value_parser = parse_key_val)]
    param: Vec<(String, String)>,
}

#[derive(Deserialize, Serialize, Debug)]
struct IzConfig {
    commands: HashMap<String, String>,
}

fn parse_key_val(s: &str) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let pos = s.find('=')
        .ok_or_else(|| format!("Geçersiz KEY=value formatı: {}", s))?;
    Ok((s[..pos].to_string(), s[pos + 1..].to_string()))
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    println!("🔄 İz CLI başlatılıyor...");
    
    // izconfig.json dosyasını oku
    let config = read_config().context("izconfig.json dosyası okunamadı")?;
    
    // Komutun var olup olmadığını kontrol et
    let command_template = config.commands.get(&cli.command)
        .ok_or_else(|| anyhow::anyhow!("'{}' komutu izconfig.json'da bulunamadı", cli.command))?;
    
    // Parametreleri HashMap'e çevir
    let params: HashMap<String, String> = cli.param.into_iter().collect();
    
    // Komutu parametrelerle doldur
    let final_command = substitute_variables(command_template, &params)?;
    
    println!("🎯 Commit: {}", cli.commit_id);
    println!("📝 Komut: {}", final_command);
    
    // Geçici klasör oluştur
    let temp_dir = TempDir::new().context("Geçici klasör oluşturulamadı")?;
    let temp_path = temp_dir.path();
    
    println!("📁 Geçici klasör: {}", temp_path.display());
    
    // Git işlemleri
    checkout_commit_to_temp(&cli.commit_id, temp_path).context("Commit çıkarılamadı")?;
    
    // Komutu çalıştır
    println!("🚀 Komut çalıştırılıyor...");
    execute_command(&final_command, temp_path).context("Komut çalıştırılamadı")?;
    
    if cli.keep {
        println!("💾 Geçici klasör saklandı: {}", temp_path.display());
        // TempDir'i drop etmeden çıkış yap
        std::mem::forget(temp_dir);
    } else {
        println!("🧹 Geçici klasör temizlendi");
    }
    
    println!("✅ İşlem tamamlandı!");
    Ok(())
}

fn read_config() -> Result<IzConfig> {
    let config_path = "izconfig.json";
    
    if !std::path::Path::new(config_path).exists() {
        return Err(anyhow::anyhow!("izconfig.json dosyası bulunamadı. Örnek içerik:\n{}", 
            serde_json::to_string_pretty(&IzConfig {
                commands: {
                    let mut map = HashMap::new();
                    map.insert("run".to_string(), "dotnet run".to_string());
                    map.insert("build".to_string(), "dotnet build".to_string());
                    map.insert("test".to_string(), "dotnet test".to_string());
                    map
                }
            })?));
    }
    
    let content = std::fs::read_to_string(config_path)
        .context("izconfig.json dosyası okunamadı")?;
    
    let config: IzConfig = serde_json::from_str(&content)
        .context("izconfig.json dosyası parse edilemedi")?;
    
    Ok(config)
}

fn substitute_variables(template: &str, params: &HashMap<String, String>) -> Result<String> {
    let re = Regex::new(r"#\{(\w+)\}").unwrap();
    let mut result = template.to_string();
    
    for caps in re.captures_iter(template) {
        let var_name = &caps[1];
        let full_match = &caps[0];
        
        if let Some(value) = params.get(var_name) {
            result = result.replace(full_match, value);
        } else {
            return Err(anyhow::anyhow!("Gerekli parametre bulunamadı: {}", var_name));
        }
    }
    
    Ok(result)
}

fn checkout_commit_to_temp(commit_id: &str, temp_path: &std::path::Path) -> Result<()> {
    // Mevcut Git repository'yi aç
    let repo = Repository::open(".").context("Git repository bulunamadı")?;
    
    // Commit'i bul
    let oid = git2::Oid::from_str(commit_id)
        .context("Geçersiz commit ID")?;
    
    let commit = repo.find_commit(oid)
        .context("Commit bulunamadı")?;
    
    let tree = commit.tree()
        .context("Commit tree'si alınamadı")?;
    
    // Dosyaları geçici klasöre çıkar
    let mut checkout_builder = git2::build::CheckoutBuilder::new();
    checkout_builder.target_dir(temp_path);
    checkout_builder.force();
    
    repo.checkout_tree(tree.as_object(), Some(&mut checkout_builder))
        .context("Dosyalar çıkarılamadı")?;
    
    Ok(())
}

fn execute_command(command: &str, working_dir: &std::path::Path) -> Result<()> {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return Err(anyhow::anyhow!("Boş komut"));
    }
    
    let mut cmd = Command::new(parts[0]);
    if parts.len() > 1 {
        cmd.args(&parts[1..]);
    }
    
    cmd.current_dir(working_dir);
    
    let output = cmd.output()
        .context("Komut çalıştırılamadı")?;
    
    if !output.stdout.is_empty() {
        println!("📄 Çıktı:");
        println!("{}", String::from_utf8_lossy(&output.stdout));
    }
    
    if !output.stderr.is_empty() {
        eprintln!("⚠️  Hata çıktısı:");
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
    }
    
    if !output.status.success() {
        return Err(anyhow::anyhow!("Komut başarısız oldu: {}", output.status));
    }
    
    Ok(())
}

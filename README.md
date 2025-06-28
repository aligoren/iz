# İz CLI 🚀

Git commit'lerini geçici klasörde test etmek için güçlü bir CLI aracı.

## Nedir?

`iz`, geçmiş commit'lerinizi aktif branch'inizi değiştirmeden test etmenizi sağlar. Herhangi bir commit'in dosyalarını geçici bir klasöre çıkarır, istediğiniz komutu o klasörde çalıştırır ve işlem bitince klasörü temizler.

## Kurulum

```bash
cargo build --release
# Binary dosyası target/release/iz konumunda oluşur
```

## Kullanım

### Temel Kullanım

```bash
iz <commit-id> <komut>
```

### Örnekler

```bash
# Belirli bir commit'te dotnet run çalıştır
iz 30b5302 run

# Build komutunu çalıştır
iz abc1234 build

# Test komutunu çalıştır
iz def5678 test
```

### Parametreli Komutlar

```bash
# Port parametresi ile çalıştır
iz 30b5302 serve --param port=8080

# Birden fazla parametre
iz 30b5302 echo --param name=Ali --param surname=Veli
```

### Geçici Klasörü Saklamak

```bash
# --keep bayrağı ile geçici klasör silinmez
iz 30b5302 run --keep
```

## Konfigürasyon

Projenizin kök dizininde `izconfig.json` dosyası oluşturun:

```json
{
    "commands": {
        "run": "dotnet run",
        "build": "dotnet build",
        "test": "dotnet test",
        "dev": "npm start",
        "serve": "python -m http.server #{port}",
        "echo": "echo 'Merhaba #{name}!'"
    }
}
```

### Değişken Desteği

Komutlarınızda `#{değişken}` formatında değişkenler kullanabilirsiniz:

```json
{
    "commands": {
        "serve": "python -m http.server #{port}",
        "greet": "echo 'Merhaba #{name} #{surname}!'"
    }
}
```

Bu değişkenleri `--param` ile geçebilirsiniz:

```bash
iz 30b5302 serve --param port=3000
iz 30b5302 greet --param name=Ali --param surname=Veli
```

## Avantajları

- ✅ Aktif branch'inizi değiştirmez
- ✅ Commit geçmişinizi güvenle test edebilirsiniz  
- ✅ Geçici klasörler otomatik temizlenir
- ✅ Değişken desteği ile esnek komutlar
- ✅ Basit JSON konfigürasyonu
- ✅ Cross-platform (Windows, macOS, Linux)

## Gereksinimler

- Rust (derleme için)
- Git repository (çalışacağınız projede)
- izconfig.json dosyası

## Lisans

MIT

## Katkıda Bulunma

Pull request'ler ve issue'lar memnuniyetle karşılanır! 
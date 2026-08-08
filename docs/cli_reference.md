# NetLang CLI Reference

NetLang komut satırı arayüzü (CLI), NetLang dosyalarını (`.nl`) derlemek, elektriksel kural denetimi (ERC) yapmak ve simülasyon koşmak için kullanılır. Otomasyon ve yapay zeka ajanları ile tam uyumlu çalışması için JSON çıktısı da destekler.

## Kurulum ve Çalıştırma
Uygulama derlendikten sonra `netlang` komutu ile çalıştırılabilir.
```bash
cargo run --bin netlang -- [KOMUT] [DOSYA] [SEÇENEKLER]
```

## Alt Komutlar (Subcommands)

### 1. `check`
Sadece sözdizimi ve yapısal kuralları (ERC) denetler. Herhangi bir çıktı dosyası (.spice vb.) üretmez.
```bash
netlang check examples/demo_circuit.nl
```

### 2. `compile`
Dosyayı derler, ERC kontrollerini yapar ve başarılı olursa devre dizininde `*.spice` uzantılı bir SPICE netlist dosyası oluşturur.
```bash
netlang compile examples/demo_circuit.nl
```

### 3. `simulate`
Derleme ve ERC adımlarını geçer, `*.spice` dosyasını oluşturur ve ardından gömülü ngspice motorunu kullanarak devrenin simülasyonunu çalıştırır. Simülasyon çıktıları doğrudan terminale basılır.
```bash
netlang simulate examples/demo_circuit.nl
```

### 4. `render` *(Geliştirme Aşamasında - Faz 3)*
Derleme aşamalarından sonra devrenin şemasını SVG formatında çıktı olarak verir.

## Format Desteği ve Yapay Zeka (AI) Entegrasyonu

Otomasyon araçları ve AI ajanları için `--format json` seçeneği eklenmiştir. Bu argüman eklendiğinde uygulama (hata veya başarı durumlarında) renkli metin yerine, ayrıştırılabilir bir JSON objesi döndürür.

```bash
netlang --format json check examples/test_amp.nl
```

**Örnek JSON Çıktısı (Hata Durumu):**
```json
{
  "status": "error",
  "diagnostics": [
    {
      "code": "NL-E003",
      "severity": "Error",
      "message": "Floating Pin: Q1.c is not connected to anything.",
      "component": "Q1",
      "pin": "c"
    }
  ],
  "spice_file": null
}
```

**Örnek JSON Çıktısı (Başarı Durumu):**
```json
{
  "status": "success",
  "diagnostics": [],
  "spice_file": "examples/demo_circuit.spice"
}
```

## Exit Kodları (Çıkış Kodları)
Terminal otomasyonlarında güvenilir kullanım için NetLang aşağıdaki standart exit kodlarını kullanır:
- **`0`**: Başarılı. Herhangi bir kural ihlali veya sözdizimi hatası yok.
- **`1`**: ERC Hatası. Devre sözdizimi olarak doğru ancak tanımsız bileşen veya ucu açık pin gibi elektriksel/yapısal kural ihlalleri var.
- **`2`**: Parse / I-O Hatası. Dosya bulunamadı, okunamadı veya NetLang sözdizimine (syntax) uygun değil.
- **`3`**: Simülasyon Hatası. Ngspice motoru başlatılamadı veya simülasyon çöktü.

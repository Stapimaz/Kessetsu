# NetLang CLI Reference

NetLang CLI, `.nl` kaynaklarını ortak Rust derleme hattından geçirir; ERC, SPICE üretimi, Ngspice çalıştırma ve assertion değerlendirme komutları sunar. Human çıktı insanlar, sürümlü JSON çıktı otomasyon ve AI ajanları içindir.

## Kullanım

```bash
netlang [--format human|json] <COMMAND> [OPTIONS] <FILE>
```

`--format` gerçek bir global seçenektir; alt komuttan önce veya sonra yazılabilir:

```bash
netlang --format json check examples/demo_circuit.nl
netlang check examples/demo_circuit.nl --format json
```

## Komutlar

### `check`

Parse, semantic validation ve ERC çalıştırır; dosya üretmez.

```bash
netlang check examples/demo_circuit.nl
```

### `compile`

Kontroller başarılıysa SPICE netlist üretir.

```bash
netlang compile examples/demo_circuit.nl
netlang compile examples/demo_circuit.nl --output build/demo.spice
```

Varsayılan hedef kaynak dosyanın `.spice` uzantılı halidir. Var olan dosya sessizce ezilmez; bilinçli overwrite için `--force` gerekir:

```bash
netlang compile examples/demo_circuit.nl --force
```

`--output` yolu çalışma dizinine göre çözülür. Hedef kaynak dosyanın kendisiyse `--force` verilse bile işlem reddedilir. CLI eksik parent dizinlerini otomatik oluşturmaz.

### `simulate`

Derler, aynı output politikasına göre SPICE dosyasını yazar ve ortak simulation runner üzerinden Ngspice'ı batch modunda çalıştırır. Runner executable sürümünü doğrular, her çalışma için benzersiz temporary directory kullanır ve varsayılan 30 saniyelik timeout uygular. Simulator process status veya fatal/error çıktısı başarısızsa exit `3` döner; JSON modunda simulator logları stdout'a karışmaz.

Dil seviyesinde `op`, `tran`, `ac` ve bağımsız voltage/current source için `dc` sweep desteklenir. Analysis argümanları ve fiziksel birimleri semantic aşamada doğrulanır; desteklenmeyen veya hatalı analysis `NL-C009` verir ve simulator başlatılmaz.

```bash
netlang simulate examples/demo_circuit.nl --force
```

### `test`

Derleme ve simülasyondan sonra kaynak içindeki assertion'ları değerlendirir. Simülasyon problemi exit `3`, başarısız assertion exit `4` üretir.

```bash
netlang test examples/test_features.nl --force
```

Her assertion kaynak sırasına göre deterministik bir `NL-Txxx` kodu alır. Durumlar `PASS`, `FAIL`, `ERROR` ve `SKIPPED` olarak ayrılır: eşik sağlanmıyorsa `FAIL`, ölçüm bulunamıyor veya metric desteklenmiyorsa `ERROR`, simülasyon tamamlanmadıysa `SKIPPED` üretilir. Bu durumların herhangi biri varsa komut exit `4` döner.

Desteklenen temel metric'ler `value`, `min`, `max`, `peak`, `average`/`avg` ve `rms`'tir. `peak`, signed maksimum değil `max(abs(x))` anlamına gelir. OP analizinde `value`, `min`, `max` ve `average` signed skaler değeri; `peak` ve `rms` mutlak büyüklüğü verir. Equality ve inclusive comparator'lar küçük numeric sapmalar için tanımlı absolute/relative tolerans uygular; strict `<`/`>` sınırları gevşetilmez.

Akım işareti component'in canonical pozitif/reference pinine giren yönü pozitif kabul eder. Voltage source için bu `plus` pinidir; dolayısıyla güç veren bir kaynağın ölçülen akımı çoğu zaman negatiftir. Human çıktı engineering prefix ve fiziksel birimi birlikte gösterir; JSON aynı typed assertion raporunu ve sayısal summary'yi taşır.

Ngspice executable discovery gerektiğinde `NETLANG_NGSPICE` environment variable ile açık bir executable yoluna yönlendirilebilir. Yol başlatılamazsa simülasyon exit `3` ile fail-closed olur.

### `render`

SVG renderer henüz uygulanmadığı için komut fail-closed davranır: çıktı üretmez, `NL-F001` verir ve exit `2` döner. Başarı stub'ı değildir.

```bash
netlang render examples/demo_circuit.nl
```

## JSON sözleşmesi

JSON stdout her çalıştırmada tek bir JSON objesidir; progress ve simulator logları stdout'a yazılmaz. Compile raporu şema sürümü `netlang.compile.v1`, assertion raporu şema sürümü `netlang.assertion.v1` kullanır. CLI, canonical `CompileReport` alanlarına `status`, `spice_file` ve gerektiğinde `assertions` alanını ekler.

Başarılı `check` özeti:

```json
{
  "status": "success",
  "schema_version": "netlang.compile.v1",
  "ast": {},
  "ir": {},
  "diagnostics": [],
  "graph": {},
  "spice_netlist": null,
  "layout": null,
  "kicad_sch": null,
  "spice_file": null,
  "assertions": null
}
```

`test` sonucundaki assertion alanı ayrı sürümlü ve özetlidir:

```json
{
  "schema_version": "netlang.assertion.v1",
  "assertions": [
    {
      "code": "NL-T001",
      "status": "PASS",
      "metric": "peak",
      "signal": "I(V1)",
      "actual": 0.005,
      "threshold": 0.1,
      "unit": "A"
    }
  ],
  "summary": {
    "total": 1,
    "passed": 1,
    "failed": 0,
    "errors": 0,
    "skipped": 0
  }
}
```

Diagnostic alanları bütün aşamalarda ortaktır:

```json
{
  "code": "NL-E003",
  "severity": "error",
  "stage": "erc",
  "message": "Floating Pin: R1.p1 is not connected to anything.",
  "component": "R1",
  "pin": "p1",
  "field": null,
  "line": null,
  "column": null
}
```

Compile error varsa SPICE/layout/KiCad backend alanları `null` olur. Başarılı `compile` JSON'u `spice_netlist` içeriğini ve yazılan yolun `spice_file` değerini birlikte taşır. I/O veya runtime hatalarında `status` hiçbir zaman `success` değildir.

## Exit kodları

- `0`: Başarı.
- `1`: Flatten, semantic validation veya ERC hatası.
- `2`: Parse, I/O, güvenli output politikası veya henüz uygulanmamış frontend komutu hatası.
- `3`: Simulator başlatma/process/runtime hatası.
- `4`: Bir veya daha fazla assertion başarısız.

Human ve JSON formatları aynı kod yolunu ve exit semantiğini kullanır.

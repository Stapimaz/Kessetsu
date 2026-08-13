# NetLang CLI Reference

NetLang CLI, `.nl` kaynaklarını ortak Rust derleme hattından geçirir; ERC, SPICE üretimi, Ngspice çalıştırma ve assertion değerlendirme komutları sunar. Human çıktı insanlar, sürümlü JSON çıktı otomasyon ve AI ajanları içindir.

## Kullanım

```bash
netlang [--format human|json] [--schema-version netlang.cli.v1] \
  [--include ast,ir,graph,spice,datasets,models,raw-log] <COMMAND> [OPTIONS] <FILE>
```

`--format` gerçek bir global seçenektir; alt komuttan önce veya sonra yazılabilir:

```bash
netlang --format json check examples/demo_circuit.nl
netlang check examples/demo_circuit.nl --format json
```

`--schema-version` ve `--include` da global seçeneklerdir. `--include` virgülle ayrılarak veya tekrarlanarak kullanılabilir. Bilinmeyen schema sürümü `NL-F002` ve exit `2` ile, kaynak okunmadan ve output oluşturulmadan reddedilir.

## Stdin ve dosyasız agent kullanımı

Dosya yolu yerine `-` verildiğinde NetLang source stdin'den okunur:

```bash
netlang check - --format json < circuit.nl
netlang compile - --format json --include spice < circuit.nl
netlang simulate - --format json < circuit.nl
netlang test - --format json < circuit.nl
```

Stdin ile `compile`, `simulate` ve `test`, açık `--output` yoksa çalışma dizinine SPICE dosyası yazmaz. `compile` JSON çağrısında generated netlist gerekiyorsa `--include spice` kullanılır. Human `compile -` netlist'i stdout'a basar. Kalıcı artifact istenirse normal güvenli overwrite sözleşmesiyle `--output result.spice` verilebilir.

Side-effect-free stdin + JSON kullanımı agent retry'ları için idempotenttir: aynı source, seçenekler ve deterministic simulator sonucu byte-for-byte aynı envelope'u üretir. Dosya çıktısı isteyen caller, mevcut artifact'i bilinçli biçimde yenilemek için `--force` kullanmalıdır.

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

Derler, aynı output politikasına göre SPICE dosyasını yazar ve ortak simulation runner üzerinden Ngspice'ı batch modunda çalıştırır. Runner executable sürümünü doğrular, her çalışma için benzersiz temporary directory kullanır ve varsayılan 30 saniyelik timeout uygular. Simulator process status veya fatal/error çıktısı başarısızsa exit `3` döner. Human ve JSON renderer aynı typed `SimulationResult` nesnesini kullanır; raw simulator log yalnız açık `--include raw-log` ile gösterilir.

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

Canonical Schematic IR üzerinden SVG, PNG veya tek sayfa vector PDF üretir. Format output uzantısından seçilir; PNG ölçeği `0.25..8`, arka plan `white|transparent` olabilir.

```bash
netlang render circuit.nl --output circuit.svg
netlang render circuit.nl --output circuit.png --scale 3 --background transparent
netlang render circuit.nl --output circuit.pdf
```

### `export`

Versioned ortak exporter sözleşmesinden makine-okunabilir veya düzenlenebilir çıktı üretir:

```bash
netlang export circuit.nl --target schematic-json --output circuit.netlang.json
netlang export circuit.nl --target spice --output circuit.spice
netlang export circuit.nl --target kicad --output circuit.kicad_sch
netlang export circuit.nl --target ltspice --output circuit.asc
```

`render` ve `export`, canonical connectivity doğrulanmadıysa veya hedef bir özelliği güvenle temsil edemiyorsa `NL-X...` diagnostic ile çıktı üretmeden durur. Mevcut dosyayı yenilemek için `--force` gerekir. `--format json` artifact schema/version, MIME, SHA-256, byte length, connectivity, capability, warning ve loss alanlarını bildirir. Formatların sınırları [export matrix](export_formats.md) içinde tanımlıdır.

## JSON sözleşmesi

JSON stdout her çalıştırmada tek bir JSON objesidir; progress ve simulator logları stdout'a yazılmaz. Varsayılan agent envelope sürümü `netlang.cli.v1`'dir. Compile raporu `netlang.compile.v3`, canonical şema `netlang.schematic.v1`, simulation sonucu `netlang.simulation.v1`, engineering measurement modeli `netlang.measurement.v1`, assertion raporu `netlang.assertion.v1` kullanır; geçerli alt sözleşmeler `domain_versions` alanında görünür.

Assertion primitive'leri, derived metric formülleri, analiz gereksinimleri ve sign convention için [engineering measurement sözleşmesine](engineering_measurements.md) bakın.

Varsayılan çıktı bilinçli olarak kompakttır. Yalnız `status`, diagnostics, summary, measurements, assertions ve artifact referanslarına ek olarak command/schema metadata'sı taşır. Canonical AST/IR/graph/SPICE, analysis dataset'leri, model manifest/lock ve raw log `debug` altında ancak ilgili `--include` seçilirse bulunur.

Başarılı `check` özeti:

```json
{
  "schema_version": "netlang.cli.v1",
  "command": "check",
  "status": "success",
  "domain_versions": {
    "compile": "netlang.compile.v3",
    "simulation": null,
    "measurement": null,
    "assertion": null
  },
  "diagnostics": [],
  "summary": {
    "errors": 0,
    "warnings": 0,
    "info": 0,
    "analyses": 0,
    "measurements": 0,
    "assertions": null
  },
  "measurements": {},
  "assertions": null,
  "artifacts": []
}
```

Debug alanlarını isteme örneği:

```bash
netlang compile circuit.nl --format json --include ast,ir,graph,spice
netlang simulate circuit.nl --format json --include datasets,raw-log --force
netlang compile circuit.nl --format json --include models --force
```

İlk komutta `debug.ast`, `debug.ir`, `debug.graph` ve `debug.spice_netlist`; ikincide `debug.datasets` ve `debug.raw_log`; üçüncüde `debug.models.manifest` ve `debug.models.lock` oluşur. Seçilmeyen hacimli alanlar `null` yazılmak yerine tamamen dışarıda bırakılır.

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
  "pin": "p1"
}
```

Başarılı `compile`, yazılan SPICE dosyasını `artifacts` içinde `spice_netlist` olarak bildirir. Kullanılan bir model/subcircuit varsa aynı dizindeki deterministic `netlang.lock` ayrıca `model_lock` artifact'i olur. Netlist metni yalnız `--include spice`, model provenance ve lock içeriği yalnız `--include models` ile döner. I/O veya runtime hatalarında `status` hiçbir zaman `success` değildir.

## Model ve subcircuit kullanımı

User-defined device model ve op-amp subcircuit'leri raw SPICE değil typed declaration'dır:

```netlang
model diode SafeD version=1.0.0 license=MIT Is=2e-9 Rs=0.5
model bjt SafeN npn version=1.0.0 license=MIT Is=1e-12 Bf=100
model mosfet SafeP pmos version=1.0.0 license=MIT Vto=-2 Kp=4
subcircuit opamp SafeOp (in_p,in_n,vcc,vee,out) version=1.0.0 license=MIT gain=100k bandwidth=2MHz
```

Exact packaged model seçimi:

```netlang
model_include netlang_analog 1.0.0
opamp U1 NLANG_PACKAGE_OPAMP
```

`version` ve `license` user declaration'larında zorunlu, `source` opsiyoneldir. İzinli parametreler kind'e göre sınırlıdır; bilinmeyen parametre, yanlış polarity/kind, hatalı pin sırası, duplicate ad veya raw directive payload compile aşamasında structured `NL-C010..013` diagnostic'i üretir. Builtin generic doğrulama yolları `NLANG_OPAMP_V1`, `NLANG_PMOS_V1` ve `NLANG_POWER_NPN_V1`'dir.

## Exit kodları

- `0`: Başarı.
- `1`: Flatten, semantic validation veya ERC hatası.
- `2`: Parse, I/O, güvenli output politikası veya export/render frontend hatası.
- `3`: Simulator başlatma/process/runtime hatası.
- `4`: Bir veya daha fazla assertion başarısız.

Human ve JSON formatları aynı kod yolunu ve exit semantiğini kullanır.

## Agent döngüsü

Ayrı daemon veya özel bir Agent API gerekmez. Bir ajan şu döngüyü yalnız JSON alanlarını okuyarak kurabilir:

1. `check - --format json` ile syntax/semantic/ERC diagnostic'lerini alır.
2. `compile - --format json --include spice` ile canonical netlist'i gerektiğinde inspect eder.
3. `simulate - --format json` ile typed measurement ve analysis summary'yi okur.
4. `test - --format json` içindeki `assertions[].status`, `actual`, `threshold` ve summary alanlarından hedef farkını çıkarır.
5. Source'u revize edip aynı stdin çağrısını tekrarlar.

Repository contract suite'i bu akışı başarısız 10 Ω adayından ölçülen 200 mA sonucunu okuyup direnci 100 Ω'a revize eden ve 20 mA ile assertion'ı geçen platformlar arası fixture ile doğrular; test human terminal metni parse etmez.

# NetLang Projesi Mimari Anayasası

Bu doküman, NetLang projesinin çekirdek algoritmalarını, derleme süreçlerini ve dil tasarım standartlarını açıklar. **Projeye dahil olan tüm geliştiriciler ve Yapay Zeka Ajanları (AI Agents), projede herhangi bir kod yazmadan önce bu dokümandaki kurallara uymak ZORUNDADIR.**

> **Geliştirme planı ve görev takibi için:** `docs/ROADMAP.md` dosyasına bakınız.

## 1. Sistemin Temel Parçaları

NetLang projesi temelde iki ana parçadan oluşur:
1. **`netlang-core` (Rust):** Dilin ayrıştırıcısı (parser), AST oluşturucusu, Circuit IR dönüştürücüsü, ERC (Electrical Rules Check) motoru, SPICE netlist jeneratörü ve otomatik şema (layout) motorunu barındıran çekirdek kütüphane.
   - Hem bir kütüphane (`lib`), hem CLI (`bin/`) olarak hem de Web için WASM (`wasm.rs`) olarak derlenir.
2. **`webapp` (React/TS):** Kullanıcının kodu yazdığı ve sonuçları (şema/grafik) gördüğü UI. `netlang-core`'u WASM üzerinden tarayıcı içinde gerçek zamanlı çalıştırır.

### Derleme Pipeline'ı

```
NetLang Source (.nl)
    │
    ▼
  Parser (netlang.pest → pest → AST)
    │
    ▼
  Module Flattening → Canonical AST
    │
    ▼
  Circuit IR (typed dönüşüm + semantic validation)
    │
    ▼
  Canonical Graph → ERC
    │
    ├──► SPICE Netlist
    ├──► Schematic IR → Render / EDA Export
    └──► CompileReport (CLI/WASM/API)
```

**Kritik kural:** Tüm backend'ler (SPICE, Layout, ERC, JSON) yalnızca Circuit IR üzerinden çalışır. AST'den doğrudan çıktı üretilmez.

### Tek Compile Sözleşmesi

Çekirdeğin canonical derleme girişi `compile_source(source, options) -> CompileReport` fonksiyonudur. Bu fonksiyon dosya yazmaz, process başlatmaz ve log basmaz; bu yan etkiler CLI gibi frontend'lere aittir. Rapor şeması `netlang.compile.v3` ile sürümlüdür ve opsiyonlara göre flattened AST, typed IR, deterministik graph özeti, SPICE, canonical `netlang.schematic.v1`, SVG, geçici legacy layout ve KiCad çıktıları taşıyabilir.

Parse, flatten, semantic ve ERC hataları ortak `Diagnostic` modeline dönüştürülür. Error severity varsa hiçbir backend çıktısı üretilmez; warning ve info sonuçları başarılı çıktılarla birlikte taşınabilir. CLI ve WASM kendi paralel derleme akışlarını kurmamalı, yalnızca bu entrypoint'in adaptörü olmalıdır.

CLI, canonical raporu `netlang.cli.v1` agent envelope'u içinde render eder; Core raporunun semantiğini değiştirmez. Varsayılan JSON yalnız kompakt durum/diagnostic/summary/measurement/assertion/artifact alanlarını taşır. AST, IR, graph, SPICE, dataset ve raw simulator log açık `--include` olmadan serialize edilmez. Compile, simulation, measurement ve assertion alt sözleşmelerinin sürümleri `domain_versions` içinde ilan edilir; bilinmeyen CLI schema isteği hiçbir compile veya dosya yazma işlemi başlamadan `NL-F002` ile reddedilir. JSON stdout tek bir obje olarak kalır. Dosya yazma frontend sorumluluğudur: mevcut output açık `--force` olmadan ezilmez ve hiçbir generated output kaynak `.nl` dosyasının üzerine yazılamaz.

CLI'da kaynak yolu `-` ise source stdin'den okunur. Stdin tabanlı `compile`, `simulate` ve `test` açık `--output` verilmedikçe SPICE dosyası yazmaz; netlist process içinde simulation'a aktarılır ve JSON caller gerekirse `--include spice` ile metni alır. Böylece tool çağrıları geçici kaynak dosyasına ihtiyaç duymaz ve side-effect-free stdin istekleri aynı input/seçenekler için byte-stable JSON üretir. Dosya tabanlı komutların mevcut güvenli overwrite politikası değişmez.

Simulator executable discovery, dağıtılan binary konumlarını ve sistem fallback'ini dener; otomasyon/packaging ortamları açık bir executable yolu için `NETLANG_NGSPICE` kullanabilir. Bu override derleme hattını değiştirmez ve başlatma/process hataları CLI'da exit `3` olarak kalır.

## 2. Dilin Sözdizimi (Syntax) ve Kurallar

### Desteklenen Bileşenler
| Keyword | Tür | Pinler | SPICE Prefix |
|---|---|---|---|
| `resistor` | Pasif | p1, p2 | R_ |
| `capacitor` | Pasif | p1, p2 | C_ |
| `inductor` | Pasif | p1, p2 | L_ |
| `diode` | Yarı-iletken | p1, p2 | D_ |
| `transistor` | BJT (NPN/PNP) | c, b, e | Q_ |
| `mosfet` | MOSFET (NMOS/PMOS) | d, g, s | M_ |
| `opamp` | Op-Amp | in_p, in_n, vcc, vee, out | X_ |
| `source` | Voltaj Kaynağı | plus, minus | V_ |
| `current_source` | Akım Kaynağı | plus, minus | I_ |

### Temel Kurallar
- `source`, DC ve waveform tabanlı voltaj kaynaklarının canonical component türüdür; SPICE çıktısında `V_` öneki kullanılır.
- **Değerler:** Bileşen değerleri typed olarak parse edilir. SI prefixler desteklenir:
  - `resistor R1 10k` → 10000 Ω
  - `capacitor C1 100uF` → 100µF
  - `source Vin 5V` → 5V DC
  - `source Vin sine(0V, 1V, 1kHz)` → transient sinüs kaynağı
  - `source Vin ac(1V)` → small-signal AC kaynağı
  - `source Vin sine_ac(0V, 1V, 1kHz, 1V)` → transient ve AC analizlerinde ortak kaynak
  - Geriye uyumluluk: `source Vin "SINE(0 1V 1kHz)"` da kabul edilir (string olarak)
- **Simülasyon Komutları:** Analysis komutları raw SPICE metni olarak taşınmaz; semantic aşamada typed `Analysis` varyantlarına çevrilir. Desteklenmeyen komut, arity, birim veya sweep yönü `NL-C009` ile fail-closed reddedilir.
  - `simulate op` — DC Operating Point
  - `simulate tran 10us 1ms` — Transient
  - `simulate ac dec 10 1Hz 1MHz` — AC Analiz
  - `simulate dc V1 0V 5V 100mV` — Bağımsız voltage/current source sweep
- **Assertion'lar (Test):**
  - `assert max(V(out)) < 3.3V`
  - `assert peak(I(D1)) < 100mA`
  - `assert output_power(V(out),RL) > 2W`
  - Primitive, derived metric, analysis ve sign semantiğinin canonical tanımı `docs/engineering_measurements.md` içindedir.

## 3. Circuit IR (Intermediate Representation)

AST ile SPICE/Layout/ERC arasında **typed bir ara katman** bulunur. Bu katmanın amacı:

1. **Tip güvenliği:** `value: String` yerine typed parameters (resistance, capacitance, waveform, vb.)
2. **Statik doğrulama:** IR üzerinde SPICE çalıştırmadan kontrol yapılabilir
3. **Backend bağımsızlığı:** Syntax değişse bile backend'ler etkilenmez
4. **Agent erişimi:** AI ajanları compile raporundaki typed IR ve diagnostics alanlarını okuyabilir; backend input'u olarak raw IR kabul edilmez

**Kural:** Hiçbir backend, IR'yi atlayarak doğrudan AST üzerinden çalışmamalıdır.

## 4. Düğüm (Node) İsimlendirme Algoritması (`graph.rs`)

Component pin adları, canonical backend sırası, SPICE prefix'i ve layout pin koordinatları `core/src/component.rs` kataloğunda tek kez tanımlanır. Graph, ERC, SPICE ve layout kendi ayrı pin listelerini üretmez. Fiziksel bağlantısı olmayan bir pin magic integer ile değil `Option<NetId>::None` ile temsil edilir; `NetId(0)` typed ground kimliğidir.

SPICE motoru için düğüm isimleri rastgele tam sayılar DEĞİLDİR. Okunabilirlik ve determinism için özel bir algoritma kullanılır:

1. Explicit `net GND` hattına bağlı her şey her zaman `"0"` düğümündedir ve bu referans legacy fallback'ten önceliklidir. Explicit GND yoksa lexicographic olarak ilk voltage-source `minus` neti geriye uyumluluk fallback'i olur. Birden fazla bağımsız aday deterministic seçilse bile `NL-E008` ambiguity diagnostic üretilir; belirsizlik sessizce başarılı sayılmaz.
2. **User-named netler birinci sınıf kimliktir.** Kullanıcı `net output` tanımladıysa, o net SPICE'ta `output` olarak görünür.
   - Component ve net aynı exact identifier'ı paylaşamaz (`NL-E006`).
   - Aynı fiziksel nete birden fazla user-name bağlanamaz (`NL-E007`).
3. User ismi olmayan netlerde, kendisine bağlı pinlerin listesi **alfabetik olarak sıralanır** ve en baştaki pinin adı alınır.
4. Pin adındaki nokta `.` karakteri alt çizgiye `_` çevrilip başına `N_` eklenir.
   - *Örnek:* Bir düğüme `R1.p2`, `C1.p1` ve `Q1.b` bağlıysa. Alfabetik sırada ilk gelen `C1.p1`'dir. Düğüm ismi **`N_C1_p1`** olur. SPICE çıktısında voltaj `v(N_C1_p1)` olarak okunur.

**Determinism kuralı:** Aynı canonical graph her zaman aynı net isimlerini üretir. Bu algoritma BOZULMAMALIDIR. Ancak user-named netler bu algoritmanın üstüne eklenir — otomatik isimler yalnızca isimsiz netler için fallback'tir.

**Not:** Algoritma deterministiktir ama edit-stable değildir. Devreye yeni component eklendiğinde otomatik net isimleri değişebilir. Önemli ölçüm noktaları için user-named net kullanılmalıdır.

## 5. SPICE Motoru ve Standart Modeller

Ngspice entegrasyonu Windows'ta repository/release sidecar ile, otomasyon ve diğer paketleme ortamlarında `NETLANG_NGSPICE` override'ı ile çalışır.
- Eğer IR içerisinde `2N3904`, `1N4148`, `IRF540` gibi bilinen bir parça kullanılırsa, SPICE jeneratörü builtin `.model` tanımını otomatik olarak netlist'in sonuna ekler. Kullanıcıların `.model` yazmasına gerek yoktur.
- **Model provenance:** Her modelin kaynağı (builtin/user-defined) ve tipi (NPN/PNP/NMOS/PMOS/D) IR'de belirtilir.
- **Model kalitesi:** Dahili modeller "generic" kalitededir. İleri sürümlerde üretici-spesifik modeller ve kalite seviyeleri eklenecektir.
- **Canonical sayılar:** Generated SPICE sayıları tek formatter kullanır. Orta büyüklükler trimlenmiş decimal, çok küçük/büyük değerler normalize edilmiş lowercase exponent ile yazılır; `-0` ve binary float artıkları output'a taşınmaz.

### Desteklenen Modeller
| Model | Tür | Kaynak |
|---|---|---|
| 2N3904 | BJT NPN | Builtin |
| 2N3906 | BJT PNP | Builtin |
| 2N2222 | BJT NPN | Builtin |
| NLANG_POWER_NPN_V1 | Generic power BJT NPN | Verified builtin |
| NLANG_POWER_PNP_V1 | Generic power BJT PNP | Verified builtin |
| 1N4148 | Diode | Builtin |
| 1N4007 | Diode | Builtin |
| IRF540 | MOSFET NMOS | Builtin |
| NLANG_PMOS_V1 | Generic MOSFET PMOS | Verified builtin |
| NLANG_OPAMP_V1 | Generic op-amp subcircuit | Verified builtin |

Builtin default'lar BJT için `2N3904`/`2N3906`, MOSFET için `IRF540`, diode için `1N4148`, op-amp için `NLANG_OPAMP_V1`'dir. Böylece dilde tanımlı temel component türlerinden hiçbiri bütünüyle kullanılamaz durumda değildir. `NLANG_*` modelleri NetLang'in kendi generic ve lisansı açık doğrulama modelleridir; belirli bir üretici parçasının datasheet eşleniği oldukları iddia edilmez.

### Typed user model ve subcircuit sınırı

NetLang raw `.include`, `.model`, `.subckt` veya control directive kabul etmez. Kullanıcı yalnız typed declaration verir; parameter whitelist, numeric parse, model kind/polarity ve metadata semantic aşamada doğrulandıktan sonra directive Core tarafından canonical biçimde üretilir:

```netlang
model diode SafeD version=1.0.0 license=MIT Is=2e-9 Rs=0.5
model mosfet SafeP pmos version=1.0.0 license=MIT Vto=-2 Kp=4
subcircuit opamp SafeOp (in_p,in_n,vcc,vee,out) version=1.0.0 license=MIT gain=100k bandwidth=2MHz
```

- Device model kind'leri `diode`, `bjt` ve `mosfet`; güvenli subcircuit template'i şu aşamada `opamp` ile sınırlıdır.
- BJT `npn|pnp`, MOSFET `nmos|pmos` polarity ister. Component/model kind veya polarity uyuşmazlığı `NL-C004` olur.
- User model/subcircuit `version` ve `license` metadata'sı taşır; `source` opsiyoneldir. İzinli elektriksel parametreler kind'e göre sabit whitelist'ten gelir. Bilinmeyen/duplicate/non-finite parameter `NL-C010` olur.
- Op-amp pin sırası ortak component kataloğundaki `in_p,in_n,vcc,vee,out` sırasıyla byte-for-byte uyuşur; aksi durum `NL-C012`'dir. Backend kendi ayrı pin listesine güvenmez.
- Builtin, package, user model ve subcircuit adları case-insensitive tek namespace içindedir; çakışma `NL-C013` ile reddedilir.
- Simulator capability şu sözleşmede `ngspice-35+` olarak provenance'a yazılır. Desteklenmeyen paket/sürüm `NL-C011` ile fail-closed olur.

Exact package kullanımı `model_include netlang_analog 1.0.0` biçimindedir. Floating version/range yoktur. Kullanılan model ve paketler `netlang.models.v1` manifest'inde source, license, version, simulator capability ve `sha256:` content hash ile taşınır. Dosya tabanlı compile/simulate/test model kullanıyorsa SPICE artifact'iyle aynı dizine deterministic `netlang.lock` (`netlang.lock.v1`) yazılır ve CLI bunu `model_lock` artifact'i olarak bildirir. Stdin-only çağrı filesystem'e yazmaz; manifest ve lock içeriği `--include models` ile alınabilir.

Quoted parameter içine `.control`, `.include`, shell veya satır sonu saklama girişimleri typed numeric/metadata doğrulamasından geçemez; error varken SPICE backend çalışmaz. Bu injection sınırı regression testleriyle korunur.

## 6. ERC (Electrical Rules Check) Motoru (`erc.rs`)

> **Not:** Daha önceki sürümlerde "DRC" olarak adlandırılıyordu. Schematic seviyesindeki kontroller için doğru terim **ERC** (Electrical Rules Check). DRC, PCB physical design kontrolleri için kullanılır.

### Yapısal Kontroller (Simülasyonsuz)
| Kod | Severity | Açıklama |
|---|---|---|
| NL-E001 | Error | Duplicate component declaration |
| NL-E002 | Error | Undefined component reference |
| NL-E003 | Error | Floating pin (bağlantısız zorunlu pin) |
| NL-E004 | Error | Direct short circuit (source plus=minus) |
| NL-E005 | Error | Component türünde bulunmayan pin referansı |
| NL-E006 | Error | Component/net namespace çakışması |
| NL-E007 | Error | Aynı fiziksel net için birden fazla user-name |
| NL-E008 | Error | Birden fazla bağımsız ground adayı |
| NL-E009 | Error | Duplicate net declaration |

### Runtime Diagnostic'leri
| Kod | Severity | Açıklama |
|---|---|---|
| NL-S001 | Error | Simulator executable başlatılamadı |
| NL-S002 | Error | Simulator process/output başarısızlığı |
| NL-S003 | Warning | Simulator warning veya runtime cleanup uyarısı |
| NL-S004 | Error | Convergence/singular-matrix/timestep başarısızlığı |
| NL-S005 | Error | Fatal veya aborted simulator çıktısı |
| NL-S006 | Error | Measurement veya analysis dataset parse başarısızlığı |

Assertion sonuçları simulation diagnostic'lerinden ayrı, sürümlü `netlang.assertion.v1` raporunda taşınır. Kaynak sırasındaki her assertion deterministik `NL-T001`, `NL-T002`, ... kimliği alır ve `PASS`, `FAIL`, `ERROR` veya `SKIPPED` durumlarından biriyle sonuçlanır. Eksik ya da desteklenmeyen ölçüm `NaN` üretmez; açıklamalı `ERROR` olur. Simulation başarıyla tamamlanmadıysa assertion sonucu uydurulmaz ve `SKIPPED` olarak raporlanır.

### Simulation domain ve runner sınırı

Native simulator process ayrıntıları Core'un ortak simulation sözleşmesine sızdırılmaz. Versioned `SimulationRequest` typed analysis listesi, netlist, timeout ve artifact politikasını; `SimulationResult` ise analysis, simulator/process status, measurement, warning, error, raw log ve artifact referanslarını ayrı alanlarda taşır. Native Ngspice adaptörü ile gelecekteki browser adaptörü aynı `SimulationRunner` sınırını uygular.

Native runner her çalıştırma için benzersiz bir temporary directory oluşturur. Başarılı çalışmanın artifact'ları temizlenir; hata artifact'ları yalnız açık `retain_on_failure` politikasıyla korunur. Runner executable discovery ve version probe uygular, timeout'ta process'i sonlandırır ve paylaşılabilir cancellation token kabul eder. CLI `simulate` ve `test` aynı runner üzerinden çalışır.

Ngspice analysis verileri stdout tablo metninden çıkarılmaz. Generated SPICE her typed analysis sonrasında deterministic isimli `wrdata` çıktısı üretir. OP sonucu sorted scalar map'e, transient/DC sonucu ortak axis ve real signal serilerine, AC sonucu frequency axis ile real/imaginary signal serilerine parse edilir. Parser exponent, decimal-comma ve LF/CRLF farklarını normalize eder; malformed, duplicate veya non-finite veri `NL-S006` ile fail-closed olur.

### Assertion ve ölçüm semantiği

- `value`, serinin son örneğini; `min` ve `max`, signed minimum/maksimumu; `average` (`avg`) aritmetik ortalamayı; `rms`, kareler ortalamasının karekökünü verir.
- `peak` absolute peak'tir: `max(abs(x))`. Pozitif maksimum anlamına gelmez. Generated `.meas` fallback'inde pozitif maksimum ve negatif minimum ayrı ölçülüp mutlak değerce büyüğü seçilir.
- OP tek skaler örnektir. OP üzerinde `value`, `min`, `max` ve `average` aynı signed değeri; `peak` ve `rms` değerin mutlak büyüklüğünü verir.
- Equality ve inclusive sınırlar (`==`, `<=`, `>=`) varsayılan `abs=1e-9`, `rel=1e-6` toleransını kullanır. Strict `<` ve `>` toleransla gevşetilmez.
- Akım yönü Ngspice branch-current kuralını korur: pozitif akım component'in canonical pozitif/reference pinine giren akımdır. Voltage source için bu `plus`, inductor için `p1` pinidir. Bu nedenle yükü besleyen bir voltage source'un OP akımı çoğu devrede negatif görünür. Assertion motoru işareti yalnız `peak`/`rms` gibi açıkça magnitude tanımlı metric'lerde kaldırır.
- AC dataset'i kompleks olduğu için ham `min`/`max`/`peak`/`average`/`rms` reduction şu aşamada fail-closed `ERROR` verir; frequency-domain magnitude/phase metric'leri mühendislik ölçümleri katmanında tanımlanır.

### Çıktı Formatı
- **İnsan modu (varsayılan):** Stage/code/message içeren stderr diagnostic'leri; assertion PASS/FAIL satırlarında terminal rengi
- **JSON modu (`--format json`):** Makine-okunabilir structured diagnostics
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

**Önemli not:** ERC sonuçları, tespit edilen riskleri raporlar. Bu sonuçlar fiziksel doğrulama veya mühendis incelemesinin yerine geçmez. Özellikle thermal davranış, PCB parasitikleri, ESD ve üretici toleransları gibi konular ERC kapsamı dışındadır.

## 7. Şema Sahipliği ve Layout Motoru

Circuit IR elektriksel semantiğin, versioned Schematic IR ise çizim semantiğinin tek gerçek kaynağıdır. Schematic IR, Circuit IR ve canonical graph'tan üretilir; component instance, canonical pin anchor, orientation, wire endpoint/segment, junction, bağlantısız crossing, net label, bounds ve kalite/connectivity raporunu explicit taşır. Aynı Circuit IR için collection/declaration sırasından bağımsız ve byte-stable serialize edilmelidir.

Render/export sahipliği şu sınırı izler:

```text
Circuit IR + Canonical Graph
        → netlang.schematic.v1
        → SVG / PNG / PDF / Schematic JSON / KiCad / LTspice exporters
        → CLI artifact writer veya Web download UI
```

Renderer/exporter'lar AST'ye veya legacy layout shape'ine dönmez; component pinlerini, bağlantıları veya symbol geometrisini yeniden tanımlamaz. Ortak symbol/pin kataloğu `core/src/component.rs` içindedir. Web yalnız Schematic IR/SVG'yi gösterir ve zoom/pan/selection gibi interaction ekler. Dosya yazma, overwrite ve download yan etkileri Core'un saf exporter sonucunun dışındadır.

Legacy `layout.rs` için başlangıç davranışı aşağıdaki gibidir; Faz 4 migration'ı sırasında characterization baseline olarak korunur:

Şematiği çizerken parçaları x/y koordinatlarına yerleştirmek için **chain-based vertical layout** yaklaşımı kullanılır. DFS, layout pipeline'ında traversal ve başlangıç sıralaması için kullanılan heuristic'lerden biridir.

- **Mevcut heuristic:** Voltage-source rail'leri, GND yönü, through-pin ve `is_signal_pin` bilgisi chain sıralamasını ve rotation seçimini etkiler. BJT/MOSFET gibi aktif elemanlar için ayrı yerleşim davranışı vardır; bütün topolojilerde ideal yön garanti edilmez.
- **Canonical kapı:** `netlang.schematic.v1`, her bağlı graph pinini typed wire endpoint veya semantic net label ile temsil eder; eksik/fazla pin/net varsa `NL-L001` ile fail-closed olur.
- **Yerleşim/router:** Deterministik layered placement, shared pin-side metadata, orthogonal cost-based routing ve yüksek fan-out/power netleri için semantic label kullanır. Symbol/wire/label collision, crossing ve bend sayıları versioned kalite raporundadır.
- **Visual regression:** Altı devrelik corpus'un deterministic SVG SHA-256 golden'ları Rust testinde, gerçek browser görüntüsü Playwright corpus testinde korunur.

## 8. NetLang Vizyonu ve Ekosistem Manifestosu

**NetLang**, analog ve karma-sinyal devrelerini yazılım gibi derlemek, simüle etmek ve assertion'larla test etmek için tasarlanmış; native ve browser ortamlarında çalışan, AI-agent odaklı, deterministik bir SPICE derleyicisi ve doğrulama altyapısıdır.

```
Compile, simulate and test circuits like software.
```

Bu ekosistem üç sütun üzerinde yükselir:

### 1. NetLang Core (Rust Çekirdeği)
Projenin kalbi. Parser, IR, ERC, SPICE jeneratör ve layout motoru tek bir Rust crate içinde yaşar. Hem kütüphane (`lib`), hem CLI, hem WASM olarak derlenir. Deterministik davranış sağlar — aynı devre, her platformda aynı sonucu üretir.

### 2. NetLang CLI (Yapay Zeka ve Geliştiriciler İçin Motor)
Derleme/ERC/SPICE üretimi için internet gerektirmeyen Rust CLI'dır. Repository şu anda Windows x86-64 için Ngspice sidecar taşır; diğer platformlarda paketleme tamamlanmamıştır ve açık executable override'ı gerekir.

- **Mevcut dağıtım:** Source build + Windows sidecar. Tek-binary release, `cargo install` ve VS Code extension gelecek dağıtım hedefleridir.
- **Kullanım:** AI ajanları ve donanım mühendisleri devreyi derlemek, test etmek ve otomatik JSON formatında hataları ayıklamak için kullanır. Ayrıntılar [CLI Reference](cli_reference.md) içindedir.
- **TDD Döngüsü:** Ajan, assertion'ları yazılım testleri gibi kullanarak devreyi iteratif olarak düzeltebilir (Self-Healing). Her iterasyonda structured feedback alır.

### 3. NetLang Web Hub (İnsanlar İçin Vitrin ve Oyun Alanı)
Kullanıcıların kayıtsız, indirmesiz kullanabildiği; Rust çekirdeğini WASM ile tarayıcıda çalıştıran arayüz.

- **Mevcut playground:** Kod yaz → WASM compile/ERC + SPICE metni + deneysel SVG/KiCad çıktısı.
- **Planlanan simülasyon:** Browser içinde güvenilir simulator runtime ve structured plot/result modeli henüz uygulanmadı.
- **Planlanan paylaşım:** URL-embedded circuit ve kalıcı paylaşım akışı ürün hedefidir; mevcut Web arayüzünde yoktur.

**Güvenlik notu:** Web playground'da kullanıcı girdisi doğrudan SPICE string olarak netlist'e eklenmez. Tüm girdiler IR üzerinden typed olarak işlenir. Raw SPICE erişimi (ileride `unsafe spice_raw {}`) web sürümünde varsayılan olarak kapalıdır.

**ÖZETLE:** NetLang bir "çizim programı" değil, bir devre derleyicisi ve doğrulama altyapısıdır. Bugünkü ürün CLI'da agent-oriented compile/test geri bildirimi ve Web'de WASM compile/şema playground'u sunar. Güvenilir cross-platform simulator, profesyonel EDA round-trip ve URL tabanlı paylaşım tamamlanması gereken ürün hedefleridir.

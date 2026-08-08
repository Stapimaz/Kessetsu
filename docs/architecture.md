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
    ├──► Layout → KiCad Schematic
    └──► CompileReport (CLI/WASM/API)
```

**Kritik kural:** Tüm backend'ler (SPICE, Layout, ERC, JSON) yalnızca Circuit IR üzerinden çalışır. AST'den doğrudan çıktı üretilmez.

### Tek Compile Sözleşmesi

Çekirdeğin canonical derleme girişi `compile_source(source, options) -> CompileReport` fonksiyonudur. Bu fonksiyon dosya yazmaz, process başlatmaz ve log basmaz; bu yan etkiler CLI gibi frontend'lere aittir. Rapor şeması `netlang.compile.v1` ile sürümlüdür ve opsiyonlara göre flattened AST, typed IR, deterministik graph özeti, SPICE, layout ve KiCad çıktıları taşıyabilir.

Parse, flatten, semantic ve ERC hataları ortak `Diagnostic` modeline dönüştürülür. Error severity varsa hiçbir backend çıktısı üretilmez; warning ve info sonuçları başarılı çıktılarla birlikte taşınabilir. CLI ve WASM kendi paralel derleme akışlarını kurmamalı, yalnızca bu entrypoint'in adaptörü olmalıdır.

CLI JSON çıktısı canonical raporu değiştirmez; `status`, `spice_file` ve assertion sonucu gibi frontend alanlarıyla genişletir. JSON stdout tek bir obje olarak kalır. Dosya yazma frontend sorumluluğudur: mevcut output açık `--force` olmadan ezilmez ve hiçbir generated output kaynak `.nl` dosyasının üzerine yazılamaz.

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
  - `source Vin sine(0V, 1V, 1kHz)` → AC sinüs kaynağı
  - Geriye uyumluluk: `source Vin "SINE(0 1V 1kHz)"` da kabul edilir (string olarak)
- **Simülasyon Komutları:** `simulate <cmd> <args*>`
  - `simulate op` — DC Operating Point
  - `simulate tran 10us 1ms` — Transient
  - `simulate ac dec 10 1Hz 1MHz` — AC Analiz
- **Assertion'lar (Test):**
  - `assert max(V(out)) < 3.3V`
  - `assert peak(I(D1)) < 100mA`

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
| 1N4148 | Diode | Builtin |
| 1N4007 | Diode | Builtin |
| IRF540 | MOSFET NMOS | Builtin |

User-defined model declaration/include syntax'ı henüz tanımlı değildir. Bu nedenle bilinmeyen model adları IR'ye raw string olarak geçirilmez; `NL-C003` semantic diagnostic ile reddedilir. BJT/MOSFET/diode için yukarıdaki builtin modeller ve güvenli default'lar kullanılır. Op-amp syntax'ı parser ve component modelinde tanımlıdır ancak doğrulanmış bir builtin subcircuit henüz bulunmadığı için modelsiz op-amp `NL-C005` ile fail-closed davranır; boş `X_` SPICE satırı üretilmez.

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

Faz 3'te convergence, ölçüm ve assertion durumları daha ayrıntılı ayrı diagnostic/result kodlarına bölünecektir.

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

## 7. Şema (Layout) Motoru (`layout.rs`)

Şematiği çizerken parçaları x/y koordinatlarına yerleştirmek için **chain-based vertical layout** yaklaşımı kullanılır. DFS, layout pipeline'ında traversal ve başlangıç sıralaması için kullanılan heuristic'lerden biridir.

- **Mevcut heuristic:** Voltage-source rail'leri, GND yönü, through-pin ve `is_signal_pin` bilgisi chain sıralamasını ve rotation seçimini etkiler. BJT/MOSFET gibi aktif elemanlar için ayrı yerleşim davranışı vardır; bütün topolojilerde ideal yön garanti edilmez.
- **Henüz açık kabul kapısı:** Layout çıktısının canonical graph ile bağlantısal eşdeğerliğini otomatik kanıtlayan round-trip check mevcut değildir.
- **Bilinen sınırlamalar:** Döngüsel topolojiler (Wheatstone bridge, feedback loop), çok yüksek fan-out ve bidirectional sinyaller için layout kalitesi düşebilir. Bu topolojiler için gelişmiş algoritmalar (Sugiyama/layered) ileride eklenecektir.

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
